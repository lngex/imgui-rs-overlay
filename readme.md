### 核心代码来自[Valthrun](https://github.com/Valthrun/Valthrun)
***
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

use imgui_rs_overlay::imgui::{
    FontConfig,
    FontGlyphRanges,
    FontSource,
};
use imgui_rs_overlay::{imgui, OverlayTarget};

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
            // 自定义字体
            font_init: Some(Box::new(|imgui| {
                imgui.fonts().clear();
                imgui.fonts().add_font(&[FontSource::TtfData {
                    data: include_bytes!(r"C:\Windows\Fonts\monbaiti.ttf"),
                    size_pixels: 15.0,
                    config: Some(FontConfig {
                        glyph_ranges: FontGlyphRanges::chinese_full(),
                        // As imgui-glium-renderer isn't gamma-correct with
                        // it's font rendering, we apply an arbitrary
                        // multiplier to make the font a bit "heavier". With
                        // default imgui-glow-renderer this is unnecessary.
                        rasterizer_multiply: 1.5,
                        // Oversampling font helps improve text rendering at
                        // expense of larger font atlas texture.
                        oversample_h: 4,
                        oversample_v: 4,
                        ..FontConfig::default()
                    }),
                }]);
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



