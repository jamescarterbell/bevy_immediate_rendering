use bevy::prelude::*;
use bevy::render::camera::Camera;
use bevy::render::pass::ClearColor;
use bevy::render::draw::{DrawContext, DrawError, FetchDrawContext};
use bevy::render::pipeline::{PipelineSpecialization, PipelineDescriptor, VertexBufferDescriptor, InputStepMode, VertexAttributeDescriptor, VertexFormat };
use bevy::render::{mesh, mesh::*};
use bevy::render::renderer::{BindGroup, RenderResourceBindings, RenderResourceId, BufferInfo, BufferUsage, BufferId};
use std::collections::HashMap;
use bevy::sprite::{QUAD_HANDLE, SPRITE_SHEET_PIPELINE_HANDLE};
use std::borrow::Cow;
use std::ops::Range;
use bevy::ecs::{FetchSystemParam, SystemParam, ResourceIndex, ResourceRef, ResourceRefMut};
use std::any::TypeId;
use bevy::render::texture::SamplerDescriptor;


pub struct SpriteAtlasDrawer<'a>{
    draw: Draw,
    draw_context: DrawContext<'a>,
    meshes: ResourceRef<'a, Assets<Mesh>>,
    texture_atlases: ResourceRef<'a, Assets<TextureAtlas>>,
    render_resource_bindings: ResourceRefMut<'a, RenderResourceBindings>,
}

impl<'a> SpriteAtlasDrawer<'a>{
    pub fn new(mut draw_context: DrawContext<'a>,
    meshes: ResourceRef<'a, Assets<Mesh>>,
    render_resource_bindings: ResourceRefMut<'a, RenderResourceBindings>,
    texture_atlases: ResourceRef<'a, Assets<TextureAtlas>>) -> Self{
        let mut draw = Draw::default();
        draw_context.set_pipeline(
            &mut draw,
            &SPRITE_SHEET_PIPELINE_HANDLE.typed(),
            &PipelineSpecialization{
                sample_count: 1,
                vertex_buffer_descriptor: VertexBufferDescriptor{
                    name: Cow::Borrowed("sprite_texture"),
                    stride: std::mem::size_of::<f32>() as u64 * 8,
                    step_mode: InputStepMode::Vertex,
                    attributes: vec![
                        VertexAttributeDescriptor{
                            name: Cow::Borrowed("Vertex_Position"),
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float3
                        },
                        VertexAttributeDescriptor{
                            name: Cow::Borrowed("Vertex_Normal"),
                            offset: std::mem::size_of::<f32>() as u64 * 3,
                            shader_location: 1,
                            format: VertexFormat::Float3
                        },
                        VertexAttributeDescriptor{
                            name: Cow::Borrowed("Vertex_Uv"),
                            offset: std::mem::size_of::<f32>() as u64 * 6,
                            shader_location: 2,
                            format: VertexFormat::Float3
                        },
                    ]
                },

                ..Default::default()
            }
        );

        
        let render_resource_context = &**draw_context.render_resource_context;

        let index_buffer = render_resource_context.create_buffer_with_data(
            BufferInfo {
                buffer_usage: BufferUsage::INDEX,
                ..Default::default()
            },
            &meshes.get(QUAD_HANDLE.clone()).unwrap().get_index_buffer_bytes().unwrap(),
        );

        render_resource_context.set_asset_resource(
            &QUAD_HANDLE.typed::<Mesh>(),
            RenderResourceId::Buffer(index_buffer),
            INDEX_BUFFER_ASSET_INDEX,
        );

        let interleaved_buffer = &meshes.get(QUAD_HANDLE.clone()).unwrap().get_vertex_buffer_data();

            render_resource_context.set_asset_resource(
                &QUAD_HANDLE.typed::<Mesh>(),
                RenderResourceId::Buffer(render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::VERTEX,
                        ..Default::default()
                    },
                    &interleaved_buffer,
                )),
                VERTEX_ATTRIBUTE_BUFFER_ID,
            );

        Self{
            draw_context,
            draw,
            meshes,
            render_resource_bindings,
            texture_atlases
        }
    }

    fn set_mesh_attributes(&mut self) -> Result<Range<u32>, DrawError>{
        let render_resource_context = &**self.draw_context.render_resource_context;

        let vertex_attribute_buffer_id = render_resource_context
            .get_asset_resource(
                &QUAD_HANDLE.typed::<Mesh>(),
                mesh::VERTEX_ATTRIBUTE_BUFFER_ID,
            ).unwrap().get_buffer().unwrap();
        self.draw.set_vertex_buffer(0, vertex_attribute_buffer_id, 0);

        let mut indices = 0..0;
        let quad_index_buffer = render_resource_context
            .get_asset_resource(
                &QUAD_HANDLE.typed::<Mesh>(),
                mesh::INDEX_BUFFER_ASSET_INDEX,
            ).unwrap().get_buffer().unwrap();
        self.draw.set_index_buffer(quad_index_buffer, 0);
        if let Some(buffer_info) = render_resource_context.get_buffer_info(quad_index_buffer) {
            indices = 0..(buffer_info.size / 4) as u32;
        } else {
            panic!("Expected buffer type.");
        }
        

        // set global bindings
        self
            .draw_context
            .set_bind_groups_from_bindings(
                &mut self.draw, 
                &mut [&mut self.render_resource_bindings])?;
        Ok(indices)
    }

    pub fn draw(&mut self, transform: &Transform, cam_transform: &Transform, cam: &Camera, atlas: &Handle<TextureAtlas>, index: u32, color: Color, sampler: &SamplerDescriptor) -> Result<(), DrawError>{
        let indices = self.set_mesh_attributes()?;
        self.draw_context.set_asset_bind_groups(&mut self.draw, atlas)?;
        
        let sprite = TextureAtlasSprite{
            index,
            color
        };

        // Set 0
        let cam_transform = cam.projection_matrix;
        let cam_transform_buffer = self.draw_context.get_uniform_buffer(&cam_transform).unwrap();
        let cam_bind_group = BindGroup::build()
            .add_binding(0, cam_transform_buffer)
            .finish();
        self.draw_context.create_bind_group_resource(0, &cam_bind_group)?;
        self.draw.set_bind_group(0, &cam_bind_group);

        // Set 1
        let atlas = self.texture_atlases.get(atlas.clone()).unwrap();
        let size = self.draw_context.get_uniform_buffer(&atlas.size).unwrap();
        let textures = self.draw_context.get_uniform_buffer(&atlas.textures).unwrap();
        let tex = self.draw_context.get_uniform_buffer(&atlas.texture).unwrap();
        let render_resource_context = &**self.draw_context.render_resource_context;
        let sampler = render_resource_context.create_sampler(sampler);
        let texture_bind_group = BindGroup::build()
            .add_binding(0, size)
            .add_binding(1, textures)
            .add_binding(2, tex)
            .add_binding(3, bevy::render::renderer::RenderResourceBinding::Sampler(sampler))
            .finish();
        self.draw_context.create_bind_group_resource(1, &texture_bind_group)?;
        self.draw.set_bind_group(1, &texture_bind_group);

        // Set 2
        let transform = transform.compute_matrix();
        let transform_buffer = self.draw_context.get_uniform_buffer(&transform).unwrap();
        let sprite_buffer = self.draw_context.get_uniform_buffer(&sprite).unwrap();
        let sprite_bind_group = BindGroup::build()
            .add_binding(0, transform_buffer)
            .add_binding(1, sprite_buffer)
            .finish();
        self.draw_context.create_bind_group_resource(2, &sprite_bind_group)?;
        self.draw.set_bind_group(2, &sprite_bind_group);
        self.draw.draw_indexed(indices.clone(), 1, 0..1);
        Ok(())
    }
    
    pub fn draw_sprite_atlas_instanced(&mut self, transform: &[Transform], atlas: &Handle<TextureAtlas>, indicies: &[u32]){
        self.draw_context.set_asset_bind_groups(&mut self.draw, atlas);
        todo!()
    }
}

impl<'a> SystemParam for SpriteAtlasDrawer<'a>{
    type Fetch = FetchSpriteAtlasDrawer;
}

pub struct FetchSpriteAtlasDrawer;

impl<'a> FetchSystemParam<'a> for FetchSpriteAtlasDrawer{
    type Item = SpriteAtlasDrawer<'a>;

    fn init(system_state: &mut bevy::ecs::SystemState, world: &bevy::prelude::World, resources: &mut bevy::prelude::Resources) { 
        FetchDrawContext::init(system_state, world, resources);
        if system_state.resource_access.is_write(&TypeId::of::<Assets<Mesh>>()){
            panic!(
                "System '{}' has a `LRes<{res}>` parameter that conflicts with \
                another parameter with mutable access to the same `{res}` resource.",
                system_state.name,
                res = std::any::type_name::<Assets<Mesh>>()
            );
        }
        if system_state.resource_access.is_write(&TypeId::of::<RenderResourceBindings>()){
            panic!(
                "System '{}' has a `LRes<{res}>` parameter that conflicts with \
                another parameter with mutable access to the same `{res}` resource.",
                system_state.name,
                res = std::any::type_name::<RenderResourceBindings>()
            );
        }
        if system_state.resource_access.is_write(&TypeId::of::<Assets<TextureAtlas>>()){
            panic!(
                "System '{}' has a `LRes<{res}>` parameter that conflicts with \
                another parameter with mutable access to the same `{res}` resource.",
                system_state.name,
                res = std::any::type_name::<Assets<TextureAtlas>>()
            );
        }
    }

    unsafe fn get_param(system_state: &'a bevy::ecs::SystemState, world: &'a bevy::prelude::World, resources: &'a bevy::prelude::Resources) -> std::option::Option<<Self as bevy::ecs::FetchSystemParam<'a>>::Item> { 
        let mut draw_context = FetchDrawContext::get_param(system_state, world, resources).unwrap();
        Some(Self::Item::new(
            draw_context,
            resources.get::<Assets<Mesh>>().unwrap(),
            resources.get_mut::<RenderResourceBindings>().unwrap(),
            resources.get::<Assets<TextureAtlas>>().unwrap(),
        ))
    }
}