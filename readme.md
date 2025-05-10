# imgui-rs-overlay

# 版本
* [imgui-rs-0.12-DirectX11(当前)](https://github.com/lngex/imgui-rs-overlay/tree/master)
* [imgui-rs-0.12_vulkan-1.16(最新)](https://github.com/lngex/imgui-rs-overlay/tree/vulkan_1.14)
## 示例

Cargo.toml
```
imgui-rs-overlay={git = "https://github.com/lngex/imgui-rs-overlay"}
imgui = "0.12.0"
```
main.rs
```
use imgui::Condition;
use std::borrow::Cow;
use imgui_rs_overlay::{Result, window::{Windows, WindowsOptions}};


fn main() -> Result<()> {
    let mut index = 2usize;
    let items = ["深色", "高亮", "经典"];
    let mut app = Windows::new(&WindowsOptions::default())?;
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
        true
    })?;
    Ok(())
}
```



