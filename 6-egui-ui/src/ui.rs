use egui::FontDefinitions;
use egui_demo_lib::DemoWindows;
use egui_wgpu_backend::RenderPass;
use egui_winit_platform::{Platform, PlatformDescriptor};

/// A custom event type for the winit app.
pub enum Event {
    RequestRedraw,
}

/// This is the repaint signal type that egui needs for requesting a repaint from another thread.
/// It sends the custom RequestRedraw event to the winit event loop.
pub struct ExampleRepaintSignal(std::sync::Mutex<winit::event_loop::EventLoopProxy<Event>>);

impl epi::backend::RepaintSignal for ExampleRepaintSignal {
    fn request_repaint(&self) {
        self.0.lock().unwrap().send_event(Event::RequestRedraw).ok();
    }
}

pub fn create_platform(width: u32, height: u32, scale: f64) -> Platform {
    Platform::new(PlatformDescriptor {
        physical_width: width,
        physical_height: height,
        scale_factor: scale,
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    })
}
pub fn create_render_pass(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
) -> RenderPass {
    RenderPass::new(&device, surface_format, 1)
}
pub fn create_app() -> DemoWindows {
    egui_demo_lib::DemoWindows::default()
}
