use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{
            ERROR_INVALID_WINDOW_HANDLE,
            GetLastError,
            HWND,
            LPARAM,
            POINT,
            RECT,
            WPARAM,
        },
        Graphics::Gdi::ClientToScreen,
        UI::{
            Input::KeyboardAndMouse::GetFocus,
            WindowsAndMessaging::{
                FindWindowExA,
                FindWindowW,
                GetClientRect,
                GetWindowRect,
                GetWindowThreadProcessId,
                MoveWindow,
                SendMessageA,
                WM_PAINT,
            },
        },
    },
};

use crate::{
    error::{
        OverlayError,
        Result,
    },
    util,
};

/// 附加的窗口宽高(该属性只允许读，不允许写)
pub static mut WINDOWS_RECT: Rect = Rect { width: 0, high: 0 };

pub struct Rect {
    pub width: i32,
    pub high: i32,
}

pub enum OverlayTarget {
    Window(HWND),
    WindowTitle(String),
    WindowOfProcess(u32),
}

impl OverlayTarget {
    pub(crate) fn resolve_target_window(&self) -> Result<HWND> {
        Ok(match self {
            Self::Window(hwnd) => *hwnd,
            Self::WindowTitle(title) => unsafe {
                FindWindowW(
                    PCWSTR::null(),
                    PCWSTR::from_raw(util::to_wide_chars(title).as_ptr()),
                ).unwrap()
            },
            Self::WindowOfProcess(process_id) => {
                const MAX_ITERATIONS: usize = 1_000_000;
                let mut iterations = 0;
                let mut current_hwnd = HWND::default();
                while iterations < MAX_ITERATIONS {
                    iterations += 1;

                    current_hwnd = unsafe { FindWindowExA(None, current_hwnd, None, None).unwrap() };
                    if current_hwnd.0 as i32 == 0 {
                        break;
                    }

                    let mut window_process_id = 0;
                    let success = unsafe {
                        GetWindowThreadProcessId(current_hwnd, Some(&mut window_process_id)) != 0
                    };
                    if !success || window_process_id != *process_id {
                        continue;
                    }

                    let mut window_rect = RECT::default();
                    let success =
                        unsafe { GetWindowRect(current_hwnd, &mut window_rect) };
                    if !success.is_ok() {
                        continue;
                    }

                    if window_rect.left == 0
                        && window_rect.bottom == 0
                        && window_rect.right == 0
                        && window_rect.top == 0
                    {
                        /* Window is not intendet to be shown. */
                        continue;
                    }

                    log::debug!(
                        "Found window 0x{:?} which belongs to process {}",
                        current_hwnd.0,
                        process_id
                    );
                    return Ok(current_hwnd);
                }

                if iterations == MAX_ITERATIONS {
                    log::warn!("FindWindowExA seems to be cought in a loop.");
                }

                Default::default()
            }
        })
    }
}

/// Track the window and adjust overlay accordingly.
/// This is only required when playing in windowed mode.
pub struct WindowTracker {
    hwnd: HWND,
    current_bounds: RECT,
}

impl WindowTracker {
    pub fn new(target: &OverlayTarget) -> Result<Self> {
        let hwnd = target.resolve_target_window()?;
        if hwnd.0 as i32 == 0 {
            return Err(OverlayError::WindowNotFound);
        }

        Ok(Self {
            hwnd: hwnd,
            current_bounds: Default::default(),
        })
    }

    pub fn mark_force_update(&mut self) {
        self.current_bounds = Default::default();
    }

    pub fn update(&mut self, hwnd: HWND) -> bool {
        let mut rect: RECT = Default::default();
        let success = unsafe { GetClientRect(self.hwnd, &mut rect) };
        if !success.is_ok() {
            let error = unsafe { GetLastError() };
            if error == ERROR_INVALID_WINDOW_HANDLE {
                return false;
            }

            log::warn!("GetClientRect failed for tracked window: {:?}", error);
            return true;
        }

        unsafe {
            let _ = ClientToScreen(self.hwnd, &mut rect.left as *mut _ as *mut POINT);
            let _ = ClientToScreen(self.hwnd, &mut rect.right as *mut _ as *mut POINT);
        }

        if unsafe { GetFocus() } != self.hwnd {
            rect.bottom -= 1;
        }

        if rect == self.current_bounds {
            return true;
        }

        self.current_bounds = rect;
        let width = rect.right - rect.left;
        let high = rect.bottom - rect.top;
        unsafe {
            WINDOWS_RECT.width = width;
            WINDOWS_RECT.high = high;
        }
        unsafe {
            let _ = MoveWindow(
                hwnd,
                rect.left,
                rect.top,
                width,
                high,
                false, // Don't do a complete repaint (may flicker)
            );

            // Request repaint, so we acknoledge the new bounds
            SendMessageA(hwnd, WM_PAINT, WPARAM::default(), LPARAM::default());
        }

        true
    }
}
