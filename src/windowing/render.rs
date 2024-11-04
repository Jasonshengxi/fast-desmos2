use crate::parsing::parse_source;
use crate::text::glyph_data;
use crate::text::render::AstNodeRenderContext;
use crate::windowing::input::InputTracker;
use crate::windowing::render::text::ExprRender;
use main::MainRender;
use pollster::FutureExt;
use self_cell::self_cell;
use std::iter;
use ui::UiRender;
pub use util::Devices;
use wgpu::*;
use winit::dpi::PhysicalSize;
use winit::event::MouseButton;
use winit::window::Window;

mod main;
pub mod text;
mod ui;
mod util;

self_cell! {
    pub struct WindowSurface {
        owner: Window,

        #[covariant]
        dependent: Surface,
    }
}

pub struct RenderApps<'a> {
    ui_render: UiRender<'a>,
    main_render: MainRender,
    expr_render: ExprRender<'a>,
}

self_cell! {
    pub struct OwnedRenderApps {
        owner: Devices,

        #[covariant]
        dependent: RenderApps,
    }
}

pub struct RenderApp {
    instance: Instance,
    adapter: Adapter,
    render_apps: OwnedRenderApps,
    window_surface: WindowSurface,
    surface_config: SurfaceConfiguration,
}

impl RenderApp {
    pub fn new(window: Window) -> Self {
        let instance = Instance::default();

        let window_surface =
            WindowSurface::new(window, |window| instance.create_surface(window).unwrap());

        let surface = window_surface.borrow_dependent();
        let window = window_surface.borrow_owner();

        let adapters = instance.enumerate_adapters(Backends::all());
        println!("available adapters: {}", adapters.len());
        // let adapter = instance
        //     .request_adapter(&RequestAdapterOptions {
        //         compatible_surface: Some(surface),
        //         ..Default::default()
        //     })
        //     .block_on()
        //     .expect("Adapter request failed.");
        let [adapter] = <[_; 1]>::try_from(adapters).unwrap();

        let required_features = Features::empty();
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Main render device"),
                    required_features,
                    ..Default::default()
                },
                None,
            )
            .block_on()
            .unwrap();

        let capability = surface.get_capabilities(&adapter);
        let texture_format = capability
            .formats
            .into_iter()
            .find(TextureFormat::is_srgb)
            .unwrap();

        let size = window.inner_size();
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: Default::default(),
            view_formats: Vec::new(),
        };
        surface.configure(&device, &surface_config);
        let create_info = Devices {
            texture_format,
            device,
            queue,
        };

        let (gpu_glyph_data, cpu_glyph_data) =
            glyph_data::new(&create_info, include_bytes!("../../JetBrainsMono.ttf"));

        let mut render_apps = OwnedRenderApps::new(create_info, |create_info| RenderApps {
            ui_render: UiRender::new(create_info),
            main_render: MainRender::new(create_info),
            expr_render: ExprRender::new(create_info, gpu_glyph_data),
        });

        // let instances: Vec<_> = cpu_glyph_data.layout("Hello world!".chars(), 1.0, Vec2::ZERO).collect();

        let source = r#"(\frac{1+2-3}{x+y})"#;
        let parsed = parse_source(source).unwrap();
        let mut ctx = AstNodeRenderContext::new(&cpu_glyph_data);
        let ast_node = parsed.borrow_dependent().as_ref().unwrap();
        let node = ctx.render(source, ast_node);
        let instances = node.into_instances();

        render_apps.with_dependent_mut(|_, render_apps| {
            render_apps.ui_render.resize(size, size);
            render_apps.expr_render.resize(size, size);
            render_apps.expr_render.set_instances(&instances);
        });

        Self {
            instance,
            adapter,
            render_apps,
            window_surface,
            surface_config,
        }
    }

    pub fn tick(&mut self, input_tracker: &InputTracker) {
        self.render_apps.with_dependent_mut(|_, render_apps| {
            render_apps.expr_render.tick(input_tracker);
        });

        if input_tracker.is_button_down(MouseButton::Left) {
            let width = self.window_surface.borrow_owner().inner_size().width as f32;
            let mouse = input_tracker.mouse_pos().x;

            self.render_apps.with_dependent_mut(|_, render_apps| {
                render_apps.ui_render.change_v_split(mouse / width);
            });
        }
    }

    pub fn request_redraw(&self) {
        self.window_surface.borrow_owner().request_redraw();
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        let old_size = PhysicalSize::new(self.surface_config.width, self.surface_config.height);
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.window_surface
            .borrow_dependent()
            .configure(&self.devices().device, &self.surface_config);
        self.render_apps.with_dependent_mut(|_, render_apps| {
            render_apps.ui_render.resize(old_size, new_size);
            render_apps.expr_render.resize(old_size, new_size);
        });
    }

    pub fn do_render_pass(
        command_encoder: &mut CommandEncoder,
        descriptor: &RenderPassDescriptor,
        mut to_do: impl FnMut(&mut RenderPass),
    ) {
        let mut render_pass = command_encoder.begin_render_pass(descriptor);
        to_do(&mut render_pass);
    }

    fn devices(&self) -> &Devices {
        self.render_apps.borrow_owner()
    }

    pub fn render(&mut self) {
        let mut command_encoder = self
            .devices()
            .device
            .create_command_encoder(&Default::default());
        let texture = self
            .window_surface
            .borrow_dependent()
            .get_current_texture()
            .unwrap();
        let view = texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        Self::do_render_pass(
            &mut command_encoder,
            &RenderPassDescriptor {
                label: Some("Main render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            },
            |render_pass| {
                self.render_apps.with_dependent_mut(|_, render_apps| {
                    // self.ui_render.render(render_pass);
                    // self.main_render.render(render_pass);
                    render_apps.expr_render.render(render_pass);
                })
            },
        );

        self.devices()
            .queue
            .submit(iter::once(command_encoder.finish()));
        texture.present();
    }
}
