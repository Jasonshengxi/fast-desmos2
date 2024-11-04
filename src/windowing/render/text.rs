use crate::text::glyph_data::GpuGlyphData;
use crate::windowing::input::InputTracker;
use crate::windowing::render::util::Devices;
use bytemuck::{Pod, Zeroable};
use dyn_buf::DynamicStorage;
use glam::Vec2;
use std::array;
use wgpu::*;
use winit::dpi::PhysicalSize;
use winit::event::MouseButton;

mod dyn_buf;

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct GlyphInstance {
    pub position: Vec2,
    pub size: Vec2,
    pub index: u32,
    pub _padding: u32,
}

impl GlyphInstance {
    pub fn new(position: Vec2, size: Vec2, index: u32) -> Self {
        Self {
            position,
            size,
            index,
            _padding: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct Transform {
    scale: Vec2,
    offset: Vec2,
    bbox_expansion: Vec2,
}

pub struct ExprRender<'a> {
    devices: &'a Devices,
    inner_size: Vec2,

    pipeline: RenderPipeline,
    index_buffer: Buffer,

    instance_data: DynamicStorage<GlyphInstance>,

    transform: Transform,
    trans_buffer: Buffer,
    trans_bg: BindGroup,

    gpu_glyph_data: GpuGlyphData,

    vertex: Buffer,
}

impl<'a> ExprRender<'a> {
    fn update_transform_with(&mut self, func: impl FnOnce(&mut Transform)) {
        func(&mut self.transform);
        self.devices.queue.write_buffer(
            &self.trans_buffer,
            0,
            bytemuck::cast_slice(array::from_ref(&self.transform)),
        );
    }

    fn update_transform(&mut self, transform: Transform) {
        self.update_transform_with(|trans| *trans = transform)
    }

    pub fn new(devices: &'a Devices, gpu_glyph_data: GpuGlyphData) -> Self {
        let shader = devices
            .device
            .create_shader_module(include_wgsl!("../shaders/expr.wgsl"));

        // let (gpu_glyph_data, cpu_glyph_data) =
        // glyph_data::new(devices, include_bytes!("../../../times_new_roman.ttf"));
        let instance_data = DynamicStorage::new(devices);

        let transform = Transform {
            scale: Vec2::ONE,
            offset: Vec2::ZERO,
            bbox_expansion: Vec2::splat(0.01),
        };
        let (trans_layout, trans_bg, [trans_buffer]) = devices.make_bind_group_and_layout(
            "Transform bg layout",
            "Transform Bind Group",
            ShaderStages::VERTEX,
            [(
                "Transform item",
                bytemuck::cast_slice(array::from_ref(&transform)),
            )],
        );

        let index_buffer = devices.make_index_buffer("Expr Index Buf", &[0u16, 1, 2, 1, 3, 2]);

        let (_, pipeline) = devices.make_render_pipeline_and_layout_no_vertex(
            "Expr Layout",
            "Expr Pipeline",
            &shader,
            &[
                &trans_layout,
                instance_data.layout(),
                gpu_glyph_data.layout(),
            ],
        );

        Self {
            devices,
            pipeline,
            index_buffer,
            instance_data,
            transform,
            trans_bg,
            trans_buffer,
            gpu_glyph_data,
            vertex: devices.make_empty_vertex_buffer("Expr VBO"),
            inner_size: Vec2::ONE,
        }
    }

    pub fn tick(&mut self, input_tracker: &InputTracker) {
        let mut new_trans = self.transform;

        if input_tracker.is_button_down(MouseButton::Right) {
            new_trans.offset += input_tracker.gpu_mouse_delta() / self.inner_size;
        }
        let wheel_y = input_tracker.wheel_delta().y;
        if wheel_y != 0.0 {
            new_trans.scale *= (1.1f32).powf(input_tracker.wheel_delta().y / 10.0);
        }

        self.update_transform(new_trans);
    }

    pub fn set_instances(&mut self, instances: &[GlyphInstance]) {
        self.instance_data.set_new_data(self.devices, instances);
    }

    pub fn resize(&mut self, _: PhysicalSize<u32>, new_size: PhysicalSize<u32>) {
        let new_size = Vec2::new(new_size.width as f32, new_size.height as f32);
        self.inner_size = new_size;

        let min_dim = new_size.x.min(new_size.y);
        let scale = min_dim / new_size;
        self.update_transform(Transform {
            scale,
            offset: Vec2::ZERO,
            //                       3 pixels of expansion
            bbox_expansion: Vec2::splat(6.0 / min_dim) * scale,
        });
    }

    pub fn render(&mut self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.trans_bg, &[]);
        render_pass.set_bind_group(1, self.instance_data.bind_group(), &[]);
        render_pass.set_bind_group(2, self.gpu_glyph_data.bind_group(), &[]);

        render_pass.set_vertex_buffer(0, self.vertex.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..self.instance_data.len());
    }
}
