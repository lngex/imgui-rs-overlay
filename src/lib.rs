
use imgui::{Context, FontConfig, FontGlyphRanges, FontSource, Io};

pub use imgui;
pub use winit;

use obfstr::obfstr;
use windows::{

    Win32::{
        Foundation::HWND,
        UI::{
            Input::KeyboardAndMouse::SetActiveWindow,
            WindowsAndMessaging::{
                GetWindowLongPtrA,
                GWL_EXSTYLE,



                SetWindowDisplayAffinity,

                SetWindowLongPtrA,

                ShowWindow,
                SW_SHOWNOACTIVATE,

                WDA_EXCLUDEFROMCAPTURE,
                WDA_NONE,


                WS_EX_NOACTIVATE,

                WS_EX_TRANSPARENT,

            },
        },
    },
};
#[cfg(any(feature = "windows", feature = "windows_service", feature = "windows_wdk"))]
pub use windows;


use windows::Win32::UI::WindowsAndMessaging::{GetDesktopWindow};






pub use error::*;
use input::{
    KeyboardInputSystem,
    MouseInputSystem,
};
pub use perf::PerfTracker;
pub use window_tracker::{OverlayTarget, WINDOWS_RECT};
use window_tracker::WindowTracker;

mod clipboard;
mod error;

mod input;
mod window_tracker;

mod vulkan;

mod perf;

mod vulkan_render;

mod util;
mod vulkan_driver;
pub mod app;


pub struct OverlayOptions {
    /// Draw the window title
    pub title: String,
    /// Attach the window title
    pub target: OverlayTarget,
    /// 帧率,超过1000帧率限制将失效(近似值与实际有差别)
    pub fps: i32,
    /// Initialize the style
    pub style_init: Option<Box<dyn Fn(&mut Context) -> ()>>,
}

impl Default for OverlayOptions {
    fn default() -> OverlayOptions {
        OverlayOptions {
            title: "Imgui overlay".to_string(),
            target: OverlayTarget::Window(unsafe { GetDesktopWindow() }),
            // 帧率
            fps: 5000,
            style_init: Some(Box::new(|imgui| {
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
                        glyph_ranges: FontGlyphRanges::chinese_simplified_common(),
                        rasterizer_multiply: 1.5,
                        oversample_h: 4,
                        oversample_v: 4,
                        ..FontConfig::default()
                    }),
                }]);
            })),
        }
    }
}
/// Toggles the overlay noactive and transparent state
/// according to whenever ImGui wants mouse/cursor grab.
struct OverlayActiveTracker {
    currently_active: bool,
}

impl OverlayActiveTracker {
    pub fn new() -> Self {
        Self {
            currently_active: true,
        }
    }

    pub fn update(&mut self, hwnd: HWND, io: &Io) {
        let window_active = io.want_capture_mouse | io.want_capture_keyboard;
        if window_active == self.currently_active {
            return;
        }

        self.currently_active = window_active;
        unsafe {
            let mut style = GetWindowLongPtrA(hwnd, GWL_EXSTYLE);
            if window_active {
                style &= !((WS_EX_NOACTIVATE | WS_EX_TRANSPARENT).0 as isize);
            } else {
                style |= (WS_EX_NOACTIVATE | WS_EX_TRANSPARENT).0 as isize;
            }

            log::trace!("Set UI active: {window_active}");
            SetWindowLongPtrA(hwnd, GWL_EXSTYLE, style);
            if window_active {
                let _ = SetActiveWindow(hwnd);
            }
        }
    }
}


struct SystemRuntimeController {
    pub hwnd: HWND,
    pub imgui: Context,
    debug_overlay_shown: bool,
    active_tracker: OverlayActiveTracker,
    mouse_input_system: MouseInputSystem,
    key_input_system: KeyboardInputSystem,
    window_tracker: WindowTracker,
    frame_count: u64,
}

impl SystemRuntimeController {
    fn update_state(&mut self, hwnd: HWND) -> bool {
        self.mouse_input_system.update(self.hwnd, self.imgui.io_mut());
        self.key_input_system.update(self.imgui.io_mut());
        self.active_tracker.update(self.hwnd, self.imgui.io());
        if !self.window_tracker.update(hwnd) {
            log::info!("Target window has been closed. Exiting overlay.");
            return false;
        }
        true
    }

    fn frame_rendered(&mut self) {
        self.frame_count += 1;
        if self.frame_count == 1 {
            /* initial frame */
            unsafe { let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE); };
            self.window_tracker.mark_force_update();
        }
    }

    pub fn toggle_screen_capture_visibility(&self, should_be_visible: bool) {
        unsafe {
            let (target_state, state_name) = if should_be_visible {
                (WDA_NONE, "normal")
            } else {
                (WDA_EXCLUDEFROMCAPTURE, "exclude from capture")
            };

            if !SetWindowDisplayAffinity(self.hwnd, target_state).is_ok() {
                log::warn!(
                    "{} '{}'.",
                    obfstr!("Failed to change overlay display affinity to"),
                    state_name
                );
            }
        }
    }

    pub fn toggle_debug_overlay(&mut self, visible: bool) {
        self.debug_overlay_shown = visible;
    }

    pub fn debug_overlay_shown(&self) -> bool {
        self.debug_overlay_shown
    }
}
