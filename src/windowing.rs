use crate::windowing::input::InputTracker;
use crate::windowing::render::RenderApp;
use color_eyre::Result;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{WindowAttributes, WindowId};

pub use render::Devices;

mod input;
pub mod render;

struct App {
    input_tracker: InputTracker,
    render_app: Option<RenderApp>,
}

impl App {
    fn new() -> Self {
        Self {
            input_tracker: InputTracker::default(),
            render_app: None,
        }
    }

    fn render_app(&self) -> &RenderApp {
        self.render_app.as_ref().unwrap_or_else(|| unreachable!())
    }

    fn render_app_mut(&mut self) -> &mut RenderApp {
        self.render_app.as_mut().unwrap_or_else(|| unreachable!())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default().with_title("fast-desmos 2"))
            .unwrap();
        self.render_app = Some(RenderApp::new(window));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.input_tracker.process_event(&event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.render_app_mut().render(),
            WindowEvent::Resized(new_size) => self.render_app_mut().resize(new_size),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        self.input_tracker.poll();
        self.render_app.as_mut().unwrap().tick(&self.input_tracker);
        self.render_app().request_redraw();
    }
}

pub fn run_app() -> Result<()> {
    let event_loop = EventLoop::new()?;
    // event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    println!("Running app...");
    event_loop.run_app(&mut app)?;

    Ok(())
}
