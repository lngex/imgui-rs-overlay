use std::ptr::{addr_of, addr_of_mut};
use imgui::{FontConfig, FontGlyphRanges, FontSource, TreeNodeFlags};

use imgui_rs_overlay::{OverlayTarget, WINDOWS_RECT};

#[derive(Debug)]
pub struct Point<'a> {
    pub x: &'a mut i32,
    pub y: &'a i32,
}

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
            fps: 1000,
            font_init: Some(Box::new(|imgui| {
                // imgui.fonts().add_font(font_sources)
                // imgui.fonts().add_font(&[FontSource::TtfData {
                //     data: include_bytes!("../resources/unifont-15.1.03.otf"),
                //     size_pixels: 16.0,
                //     config: Some(FontConfig {
                //         glyph_ranges: FontGlyphRanges::from_slice(&[0x0001, 0xFFFF, 0x0000]),
                //         ..FontConfig::default()
                //     }),
                // }]);
            })),
        }).unwrap();
        let mut text_input = Default::default();
        overlay.main_loop(
            |controller| {
                controller.toggle_debug_overlay(false);
                true
            },
            move |ui| {
                unsafe { println!("窗口(宽：{}-高{})", WINDOWS_RECT.width, WINDOWS_RECT.high) };
                ui.window("Dummy Window")
                    .resizable(true)
                    .movable(true)
                    .build(|| {
                        ui.label_text("Taskmanager Overlay!", "test");
                        ui.text(format!("FPS: {:.2}", ui.io().framerate));
                        ui.input_text("Test-Input", &mut text_input).build();
                        ui.text("Привет, мир!");
                        ui.text("Chào thế giới!");
                        ui.text("Chào thế giới!");
                        ui.text("ສະ​ບາຍ​ດີ​ຊາວ​ໂລກ!");
                        ui.text("Салом Ҷаҳон!");
                        ui.text("こんにちは世界!");
                        ui.label_text("你好世界!", "test");
                    });
                true
            },
        );
    });
    let _ = handle.join();
    Ok(())
}
