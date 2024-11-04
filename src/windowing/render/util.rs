use bytemuck::NoUninit;
use std::array;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::*;

pub struct Devices {
    pub device: Device,
    pub queue: Queue,
    pub texture_format: TextureFormat,
}

impl Devices {
    pub fn as_parts(&self) -> (&Device, &Queue) {
        (&self.device, &self.queue)
    }
}

fn index_arr<const N: usize>() -> [usize; N] {
    array::from_fn(|i| i)
}

impl Devices {
    pub fn make_texture_2d(
        &self,
        label: &'static str,
        usage: TextureUsages,
        format: TextureFormat,
        size: Extent3d,
    ) -> Texture {
        self.device.create_texture(&TextureDescriptor {
            label: Some(label),
            usage,
            format,
            size,
            view_formats: &[],
            dimension: TextureDimension::D2,
            sample_count: 1,
            mip_level_count: 1,
        })
    }

    pub fn make_buffer_init<A: NoUninit>(
        &self,
        label: &'static str,
        usage: BufferUsages,
        contents: &[A],
    ) -> Buffer {
        self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some(label),
            usage,
            contents: bytemuck::cast_slice(contents),
        })
    }

    pub fn make_uniform_buffer<A: NoUninit>(&self, label: &'static str, contents: &[A]) -> Buffer {
        const USAGE: BufferUsages = BufferUsages::UNIFORM.union(BufferUsages::COPY_DST);
        self.make_buffer_init(label, USAGE, contents)
    }

    pub fn make_storage_buffer<A: NoUninit>(&self, label: &'static str, contents: &[A]) -> Buffer {
        self.make_buffer_init(label, BufferUsages::STORAGE, contents)
    }

    pub fn make_vertex_buffer<A: NoUninit>(&self, label: &'static str, contents: &[A]) -> Buffer {
        self.make_buffer_init(label, BufferUsages::VERTEX, contents)
    }

    pub fn make_index_buffer<A: NoUninit>(&self, label: &'static str, contents: &[A]) -> Buffer {
        self.make_buffer_init(label, BufferUsages::INDEX, contents)
    }

    pub fn make_empty_vertex_buffer(&self, label: &'static str) -> Buffer {
        self.make_vertex_buffer::<u8>(label, &[])
    }

    pub fn make_uniform_bind_group_layout(
        &self,
        label: &'static str,
        visibility: ShaderStages,
        uniform_count: usize,
    ) -> BindGroupLayout {
        let mut entries = Vec::with_capacity(uniform_count);
        for i in 0..uniform_count {
            entries.push(BindGroupLayoutEntry {
                binding: i as u32,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
                visibility,
            })
        }
        let entries = entries.as_slice();

        let label = Some(label);
        self.device
            .create_bind_group_layout(&BindGroupLayoutDescriptor { label, entries })
    }

    pub fn make_uniform_bind_group(
        &self,
        label: &'static str,
        layout: &BindGroupLayout,
        uniforms: &[&Buffer],
    ) -> BindGroup {
        let mut entries = Vec::with_capacity(uniforms.len());

        for (index, buffer) in uniforms.iter().enumerate() {
            entries.push(BindGroupEntry {
                binding: index as u32,
                resource: buffer.as_entire_binding(),
            })
        }

        let entries = entries.as_slice();

        self.device.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout,
            entries,
        })
    }

    pub fn make_bind_group_and_layout<const UNIFORM_COUNT: usize>(
        &self,
        layout_label: &'static str,
        group_label: &'static str,
        visibility: ShaderStages,
        uniform_contents: [(&'static str, &[u8]); UNIFORM_COUNT],
    ) -> (BindGroupLayout, BindGroup, [Buffer; UNIFORM_COUNT]) {
        let layout =
            self.make_uniform_bind_group_layout(layout_label, visibility, uniform_contents.len());

        let uniforms =
            uniform_contents.map(|(label, contents)| self.make_uniform_buffer(label, contents));
        let uniform_refs = index_arr::<UNIFORM_COUNT>().map(|index| &uniforms[index]);

        let group = self.make_uniform_bind_group(group_label, &layout, &uniform_refs);

        (layout, group, uniforms)
    }

    pub fn make_render_pipeline_and_layout_no_vertex(
        &self,
        layout_label: &'static str,
        pipeline_label: &'static str,
        shader: &ShaderModule,
        bind_group_layouts: &[&BindGroupLayout],
    ) -> (PipelineLayout, RenderPipeline) {
        let layout = self.make_render_layout(layout_label, bind_group_layouts);
        let pipeline = self.make_render_pipeline_no_vertex(pipeline_label, &layout, shader);
        (layout, pipeline)
    }

    pub fn make_render_layout(
        &self,
        label: &'static str,
        bind_group_layouts: &[&BindGroupLayout],
    ) -> PipelineLayout {
        self.device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts,
                push_constant_ranges: &[],
            })
    }

    pub fn make_render_pipeline(
        &self,
        label: &'static str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        vertex_buffers: &[VertexBufferLayout],
    ) -> RenderPipeline {
        self.device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                primitive: PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                vertex: VertexState {
                    module: shader,
                    entry_point: None,
                    compilation_options: PipelineCompilationOptions::default(),
                    buffers: vertex_buffers,
                },
                fragment: Some(FragmentState {
                    module: shader,
                    entry_point: None,
                    compilation_options: PipelineCompilationOptions::default(),
                    targets: &[Some(ColorTargetState {
                        format: self.texture_format,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                depth_stencil: None,
                cache: None,
            })
    }

    pub fn make_render_pipeline_no_vertex(
        &self,
        label: &'static str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
    ) -> RenderPipeline {
        self.make_render_pipeline(label, layout, shader, &[])
    }
}
