use imgui::Condition;
use std::borrow::Cow;

use imgui_rs_overlay::key_down;
use imgui_rs_overlay::window::{FrameRate, Windows, WindowsOptions};

fn main() -> imgui_rs_overlay::Result<()> {
    let mut index = 2usize;
    let items = ["Dark", "Highlight", "Classic"];
    let mut app = Windows::new(&WindowsOptions {
        frame_rate: FrameRate::SYNC_SCREEN,
        ..WindowsOptions::default()
    })?;
    app.run(move |ui, style| {
        ui.window("你好")
            .resizable(true)
            .size([250.0, 100.0], Condition::FirstUseEver)
            .movable(true)
            .build(|| {
                if ui.combo("theme", &mut index, &items, |item| {
                    Cow::Owned(String::from(*item))
                }) {
                    match index {
                        0 => style.use_dark_colors(),
                        1 => style.use_light_colors(),
                        2 => style.use_classic_colors(),
                        _ => style,
                    };
                }
                ui.text(format!("FPS: {:.2}", ui.io().framerate));
                ui.text("hello world!");
            });
        !key_down!(35)
    })?;
    Ok(())
}
