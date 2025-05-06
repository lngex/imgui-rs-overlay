
## 示例

Cargo.toml
```
imgui-rs-overlay={git = "https://github.com/lngex/imgui-rs-overlay",default-features = false,features = ["imgui","windows"],branch = "vulkan_1.14"}
anyhow = "1.0.89"
env_logger = "0.11.5"
log = "0.4.22"
```
main.rs
```
use imgui::{Condition, FontConfig, FontGlyphRanges, FontSource, Ui};
use winit::event_loop::EventLoopBuilder;


use imgui_rs_overlay::{OverlayTarget, app::app, OverlayOptions};

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_default_env()
        .init();
    log::info!("Initialize overlay");
    let func = move |ui: &mut Ui, style: &mut Style| {
        ui.window("imgui")
            .resizable(false)
            .size([150.0, 100.0], Condition::FirstUseEver)
            .movable(true)
            .build(|| {
                ui.text(format!("FPS: {:.2}", ui.io().framerate));
                ui.text("你好世界!");
            });
        true
    
    let mut options = OverlayOptions {
        target: OverlayTarget::WindowTitle(String::from("计算器")),
        fps: 60,
        ..OverlayOptions::default()
    };
    let event_loop = EventLoopBuilder::default().build().unwrap();
    let mut windows_app = app::WindowApp::new(func, &mut options);
    event_loop.run_app(&mut windows_app).unwrap();
}

```



