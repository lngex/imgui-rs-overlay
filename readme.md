### 核心代码来自[Valthrun](https://github.com/Valthrun/Valthrun)
***
示例
```
use imgui_rs_overlay::imgui::{
    FontConfig,
    FontGlyphRanges,
    FontSource,
};
use imgui_rs_overlay::OverlayTarget;

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_default_env()
        .init();
    log::info!("Initialize overlay");
    let handle = std::thread::spawn(|| {
        let overlay = imgui_rs_overlay::init(&imgui_rs_overlay::OverlayOptions {
            title: "Task Manager Overlay".to_string(),
            target: OverlayTarget::WindowTitle("计算器".into()),
            fps: 60,
            font_init: Some(Box::new(|imgui| {
            })),
        }).unwrap();
        let mut text_input = Default::default();
        overlay.main_loop(
            |controller| {
                controller.toggle_debug_overlay(true);
                true
            },
            move |ui| {
                ui.window("Dummy Window")
                    .resizable(true)
                    .movable(true)
                    .build(|| {
                        ui.text("Taskmanager Overlay!");
                        ui.text(format!("FPS: {:.2}", ui.io().framerate));
                        ui.input_text("Test-Input", &mut text_input).build();

                        ui.text("Привет, мир!");
                        ui.text("Chào thế giới!");
                        ui.text("Chào thế giới!");
                        ui.text("ສະ​ບາຍ​ດີ​ຊາວ​ໂລກ!");
                        ui.text("Салом Ҷаҳон!");
                        ui.text("こんにちは世界!");
                        ui.text("你好世界!");
                    });
                true
            },
        );
    });
    let _ = handle.join();
    Ok(())
}
```



