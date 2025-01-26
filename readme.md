
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
use imgui::{Condition, FontConfig, FontGlyphRanges, FontSource};
use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;

use imgui_rs_overlay::{OverlayTarget, WINDOWS_RECT};

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_default_env()
        .init();
    log::info!("Initialize overlay");
    std::thread::spawn(|| {
        // 获取要贴附的窗口句柄 这里获取桌面
        let hwnd = unsafe { GetDesktopWindow() };
        let overlay = imgui_rs_overlay::init(&imgui_rs_overlay::OverlayOptions {
            title: "Imgui overlay".to_string(),
            target: OverlayTarget::Window(hwnd),
            // 帧率
            fps: 60,
            font_init: Some(Box::new(|imgui| {
                // 设置主题
                imgui.style_mut().use_classic_colors();
                // 设置圆角
                imgui.style_mut().window_rounding = 12.0;
                // 设置字体
                imgui.fonts().clear();
                imgui.fonts().add_font(&[FontSource::TtfData {
                    data: include_bytes!(r"C:\Windows\Fonts\simhei.ttf"),
                    size_pixels: 12.0,
                    config: Some(FontConfig {
                        glyph_ranges: FontGlyphRanges::chinese_full(),
                        rasterizer_multiply: 1.5,
                        oversample_h: 4,
                        oversample_v: 4,
                        ..FontConfig::default()
                    }),
                }]);
            })),
        }).unwrap();
        overlay.main_loop(
            |controller| {
                // 动态配置样式
                // controller.imgui.style_mut().use_classic_colors();
                true
            },
            move |ui| {
                ui.window("样本")
                    .resizable(false)
                    .size([150.0, 100.0], Condition::FirstUseEver)
                    .position([(unsafe { WINDOWS_RECT.width } / 2) as f32, (unsafe { WINDOWS_RECT.high } / 2) as f32], Condition::FirstUseEver)
                    .movable(true)
                    .build(|| {
                        ui.text(format!("FPS: {:.2}", ui.io().framerate));
                        ui.text("你好世界!");
                    });
                true
            },
        );
    }).join().expect("绘制异常");
}
```



