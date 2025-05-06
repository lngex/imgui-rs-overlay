use imgui::{Condition, Style, Ui};
use winit::event_loop::EventLoopBuilder;


use imgui_rs_overlay::{OverlayTarget, app::app, OverlayOptions};

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_default_env()
        .init();
    log::info!("Initialize overlay");
    let func = move |ui: &mut Ui, _style: &mut Style| {
        ui.window("imgui")
            .resizable(false)
            .size([150.0, 100.0], Condition::FirstUseEver)
            .movable(true)
            .build(|| {
                ui.text(format!("FPS: {:.2}", ui.io().framerate));
                ui.text("你好世界!");
            });
        true
    };
    let mut options = OverlayOptions {
        target: OverlayTarget::WindowTitle(String::from("计算器")),
        fps: 60,
        ..OverlayOptions::default()
    };
    let event_loop = EventLoopBuilder::default().build().unwrap();
    let mut windows_app = app::WindowApp::new(func, &mut options);
    event_loop.run_app(&mut windows_app).unwrap();
}
