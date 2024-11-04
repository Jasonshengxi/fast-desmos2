use crate::windowing::render::util::Devices;
use bytemuck::{Pod, Zeroable};
use wgpu::*;
use winit::dpi::PhysicalSize;

#[repr(C)]
#[derive(Pod, Zeroable, Copy, Clone)]
struct CellData {
    placeholder: u8,
}

pub struct UiRender<'a> {
    devices: &'a Devices,
    pipeline_layout: PipelineLayout,
    pipeline: RenderPipeline,

    bind_group: BindGroup,

    v_split: f32,
    main_uniform: Buffer,
    instance_uniform: Buffer,

    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl<'a> UiRender<'a> {
    pub fn new(devices: &'a Devices) -> Self {
        let (bind_group_layout, bind_group, [main_uniform, instance_data]) = devices
            .make_bind_group_and_layout(
                "UI Group Layout",
                "UI Bind Group",
                ShaderStages::VERTEX,
                [
                    //
                    ("Main Uniform", bytemuck::cast_slice(&[-1.0f32])),
                    (
                        "Instance Data",
                        bytemuck::cast_slice(&[CellData { placeholder: 0 }]),
                    ),
                ],
            );

        let (pipeline_layout, pipeline) = devices.make_render_pipeline_and_layout_no_vertex(
            "UI Pipeline Layout",
            "UI Pipeline",
            &devices
                .device
                .create_shader_module(include_wgsl!("../shaders/ui.wgsl")),
            &[&bind_group_layout],
        );

        let index_buffer = devices.make_index_buffer("UI Index", &[0u16, 1, 2, 1, 3, 2]);

        Self {
            devices,
            pipeline,
            pipeline_layout,
            index_buffer,
            bind_group,
            main_uniform,
            vertex_buffer: devices.make_empty_vertex_buffer("UI Vertex Buffer"),
            v_split: 0.0,
            instance_uniform: instance_data,
        }
    }

    pub fn resize(&mut self, old_size: PhysicalSize<u32>, new_size: PhysicalSize<u32>) {
        let ratio = new_size.width as f32 / old_size.width as f32;
        self.change_v_split(self.v_split / ratio);
    }

    pub fn change_v_split(&mut self, to: f32) {
        self.v_split = to;
        self.devices.queue.write_buffer(
            &self.main_uniform,
            0,
            bytemuck::cast_slice(&[2.0 * to - 1.0]),
        );
    }

    pub fn tick(&mut self) {}

    pub fn render(&mut self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}
