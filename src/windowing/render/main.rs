use super::Devices;
use color_eyre::Result;
use glam::Vec2;
use wgpu::*;

pub struct MainRender {
    pipeline: RenderPipeline,

    texture: Texture,
    sampler: Sampler,
    texture_bind_group: BindGroup,

    vertex_buffer: Buffer,
}

impl MainRender {
    fn make_and_init_image(devices: &Devices) -> Result<Texture> {
        // let img = ImageReader::open("galaxy small.png")?.decode()?.to_rgba8();

        // let font = Font::from_first_in_file("times_new_roman.ttf").unwrap();
        // let mut render = FontRender::new(font, 120.0);
        // let img = render.render_image('{').unwrap();

        let img_extent = Extent3d {
            width: 100,
            height: 100,
            depth_or_array_layers: 1,
        };

        let texture = devices.make_texture_2d(
            "Texture",
            TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            TextureFormat::R8Unorm,
            img_extent,
        );

        // devices.queue.write_texture(
        //     texture.as_image_copy(),
        //     img.as_bytes(),
        //     ImageDataLayout {
        //         offset: 0,
        //         bytes_per_row: Some(img.width()),
        //         rows_per_image: Some(img.height()),
        //     },
        //     img_extent,
        // );

        Ok(texture)
    }

    pub fn new(devices: &Devices) -> Self {
        let shader = devices
            .device
            .create_shader_module(include_wgsl!("../shaders/main.wgsl"));

        let texture = Self::make_and_init_image(devices).unwrap();
        let view = texture.create_view(&Default::default());

        let sampler = devices.device.create_sampler(&Default::default());

        let tex_bg_layout = devices
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Texture BG layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let texture_bind_group = devices.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Texture BG"),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
            layout: &tex_bg_layout,
        });

        let pipeline_layout = devices.make_render_layout("Main pipeline Layout", &[&tex_bg_layout]);
        let pipeline = devices.make_render_pipeline(
            "Main Pipeline",
            &pipeline_layout,
            &shader,
            &[VertexBufferLayout {
                step_mode: VertexStepMode::Vertex,
                array_stride: size_of::<Vec2>() as BufferAddress,
                attributes: &vertex_attr_array![0 => Float32x2],
            }],
        );

        let vertex_buffer = devices.make_vertex_buffer(
            "Full-Screen Square",
            &[
                Vec2::new(-1.0, -1.0),
                Vec2::new(-1.0, 1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(-1.0, -1.0),
                Vec2::new(1.0, 1.0),
            ],
        );

        Self {
            pipeline,
            vertex_buffer,
            texture,
            sampler,
            texture_bind_group,
        }
    }

    pub fn render(&mut self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..1);
    }
}
