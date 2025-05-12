use std::borrow::Cow;

use imgui::Condition;

use imgui_rs_overlay::{key_down, OverlayTarget};
use imgui_rs_overlay::window::{FrameRate, Windows, WindowsOptions};

fn main() -> imgui_rs_overlay::Result<()> {
    let mut index = 2usize;
    let items = ["深色", "高亮", "经典"];
    let mut app = Windows::new(&WindowsOptions {
        frame_rate: FrameRate::UN_LIMITED,
        overlay_target:OverlayTarget::WindowTitle(String::from("计算器")),
        ..WindowsOptions::default()
    })?;
    app.run(move |ui, style| {
        ui.window("imgui")
            .resizable(false)
            .size([150.0, 100.0], Condition::FirstUseEver)
            .movable(true)
            .build(|| {
                if ui.combo("主题", &mut index, &items, |item| {
                    Cow::Owned(String::from(*item))
                }) {
                    match index {
                        0 => { style.use_dark_colors() }
                        1 => { style.use_light_colors() }
                        2 => { style.use_classic_colors() }
                        _ => { style }
                    };
                }
                ui.text(format!("FPS: {:.2}", ui.io().framerate));
                ui.text("你好世界!");
            });
        !key_down!(35)
    })?;
    Ok(())
}
