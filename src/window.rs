use std::{fs, thread};

use std::os::raw::c_void;

use std::sync::Mutex;


use crate::d3d11::D3d11Render;
use crate::window_tracker::{OverlayTarget, WindowTracker};
use imgui::{ConfigFlags, Context, DrawData, FontConfig, FontGlyphRanges, FontSource, Style, Ui};
use lazy_static::lazy_static;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HMODULE, LPARAM, LRESULT, POINT, TRUE, WPARAM};
use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::{DXGI_PRESENT, DXGI_SWAP_CHAIN_FLAG};
use windows::Win32::Graphics::Gdi::{CreateSolidBrush, ScreenToClient, UpdateWindow, ValidateRect};
use windows::Win32::System::LibraryLoader::{FreeLibraryAndExitThread, GetModuleHandleA};
use windows::Win32::UI::Input::KeyboardAndMouse::SetActiveWindow;
use windows::{
    core::*
    , Win32::Foundation::HWND
    , Win32::UI::WindowsAndMessaging::*,
};

lazy_static! {
    static ref GLOBAL_DATA: Mutex<Option<D3d11Render >> = Mutex::new(None);
}

#[macro_export]
macro_rules! loword {
    ($uint:expr)=>{
        $uint & 0xFFFF
    }
}

#[macro_export]
macro_rules! hiword {
    ($uint:expr)=>{
        ($uint >> 16) & 0xFFFF
    }
}
#[macro_export]
macro_rules! rgb {
    ($r:expr,$g:expr,$b:expr)=>{
        (($b as u32) << 16) | (($g as u32) << 8) | ($r as u32)
    }
}


extern "C" {
    /// imgui初始化win32
    fn ImGui_ImplWin32_Init(hwnd: *const c_void) -> bool;
    /// 初始化dx11
    fn ImGui_ImplDX11_Init(device: *mut ID3D11Device, ctx: *mut ID3D11DeviceContext) -> bool;
    /// imgui循环事件处理
    fn ImGui_ImplWin32_WndProcHandler(hwnd: *const c_void, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;

    fn ImGui_ImplDX11_NewFrame();
    fn ImGui_ImplWin32_NewFrame();

    fn ImGui_ImplDX11_RenderDrawData(draw_data: *const DrawData);

    fn ImGui_ImplDX11_Shutdown();
    fn ImGui_ImplWin32_Shutdown();
    fn ImGui_ImplWin32_EnableDpiAwareness();
    /// 隐藏边框
    fn ImGui_ImplWin32_EnableAlphaCompositing(hwnd: *const c_void);
}

pub struct FrameRate(u32);

impl FrameRate {
    /// 屏幕同步
    pub const SYNC_SCREEN: FrameRate = FrameRate(1);
    /// 无限制
    pub const UN_LIMITED: FrameRate = FrameRate(0);
}

pub struct WindowsOptions {
    /// imgui绘制窗口
    pub title: String,
    /// 需要覆盖的目标窗口
    pub overlay_target: OverlayTarget,
    /// 帧率
    pub frame_rate: FrameRate,
    pub dll_hinstance: usize,
    /// 初始化样式
    pub style_init: Option<Box<dyn Fn(&mut Context) -> ()>>,
}

impl Default for WindowsOptions {
    fn default() -> WindowsOptions {
        let result = fs::read(r"C:\Windows\Fonts\simhei.ttf");
        let style_init: Option<Box<dyn Fn(&mut Context) -> ()>> = if result.is_err() {
            log::warn!("simhei read fail");
            None
        } else {
            let vec = result.unwrap();
            Some(Box::new(move |imgui| {
                // 设置主题
                imgui.style_mut().use_classic_colors();
                // 设置圆角
                imgui.style_mut().window_rounding = 12.0;
                // 设置字体
                imgui.fonts().clear();
                imgui.fonts().add_font(&[FontSource::TtfData {
                    data: &*vec,
                    size_pixels: 12.0,
                    config: Some(FontConfig {
                        glyph_ranges: FontGlyphRanges::chinese_simplified_common(),
                        rasterizer_multiply: 1.5,
                        oversample_h: 4,
                        oversample_v: 4,
                        ..FontConfig::default()
                    }),
                }]);
            }))
        };
        WindowsOptions {
            title: String::from("lingex_imgui_overlay"),
            overlay_target: OverlayTarget::Window(unsafe {
                GetDesktopWindow()
            }),
            frame_rate: FrameRate(1),
            style_init,
            dll_hinstance: 0,
        }
    }
}

impl WindowsOptions {
    /// 通过窗口创建
    pub fn new(target: OverlayTarget) -> WindowsOptions {
        WindowsOptions {
            overlay_target: target,
            ..WindowsOptions::default()
        }
    }
}

pub struct Windows {
    pub hwnd: HWND,
    window_tracker: WindowTracker,
    wc: WNDCLASSEXW,
    imgui: Context,
    window_is_active: bool,
    sync_interval: u32,
    hinstance: HINSTANCE,
}

impl Windows {
    /// 创建窗口与D3D渲染
    pub fn new(options: &WindowsOptions) -> Result<Windows> {
        let target_hwnd = options.overlay_target.resolve_target_window()?;
        unsafe {
            ImGui_ImplWin32_EnableDpiAwareness();
            let vec = PCWSTR(HSTRING::from(&options.title).as_ptr());
            let window_class = PCWSTR::from_raw(vec.as_ptr());
            let hinstance = if options.dll_hinstance > 0 {
                HMODULE(options.dll_hinstance as _)
            } else {
                GetModuleHandleA(None)?
            };
            let wc = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hInstance: HINSTANCE(hinstance.0),
                lpszClassName: window_class,
                style: CS_VREDRAW | CS_HREDRAW,
                lpfnWndProc: Some(wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hIcon: HICON::default(),
                hbrBackground: CreateSolidBrush(COLORREF(rgb!(0, 0, 0))),
                lpszMenuName: PCWSTR::null(),
                hIconSm: Default::default(),
            };
            RegisterClassExW(&wc);
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
                window_class,
                window_class,
                WS_POPUP | WS_CLIPSIBLINGS,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                300,
                200,
                None,
                None,
                Some(wc.hInstance),
                None,
            )?;

            let result = D3d11Render::bind(hwnd);
            if result.is_err() {
                UnregisterClassW(wc.lpszClassName, Some(wc.hInstance))?;
                return Err(result.err().unwrap());
            }
            ImGui_ImplWin32_EnableAlphaCompositing(hwnd.0);
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);
            let renderer = result?;
            let mut imgui_context = Context::create();
            imgui_context.style_mut().use_classic_colors();
            imgui_context.style_mut().colors[2] = [0.1, 0.1, 0.1, 1.];
            imgui_context.style_mut().window_rounding = 5.0;
            imgui_context.io_mut().config_flags |= ConfigFlags::NAV_ENABLE_KEYBOARD;
            imgui_context.io_mut().config_flags |= ConfigFlags::NAV_ENABLE_GAMEPAD;
            imgui_context.io_mut().config_flags |= ConfigFlags::NAV_ENABLE_SET_MOUSE_POS;
            imgui_context.set_ini_filename(None);
            if let Some(func) = &options.style_init {
                func(&mut imgui_context)
            }
            ImGui_ImplWin32_Init(hwnd.0);
            let (pd3d_device, ctx) = {
                let device = renderer.pd3d_device.as_raw();
                let ctx = renderer.pd3d_device_context.as_raw();
                (device, ctx)
            };
            ImGui_ImplDX11_Init(pd3d_device as _, ctx as _);
            *GLOBAL_DATA.lock().unwrap() = Some(renderer);
            Ok(Windows {
                hwnd,
                window_tracker: WindowTracker {
                    hwnd: target_hwnd,
                    current_bounds: Default::default(),
                },
                wc,
                imgui: imgui_context,
                window_is_active: true,
                sync_interval: options.frame_rate.0,
                hinstance: HINSTANCE(options.dll_hinstance as _),
            })
        }
    }

    /// 进入循环
    /// [render] 渲染函数
    pub fn run<R>(&mut self, mut render: R) -> Result<()>
        where
            R: FnMut(&mut Ui, &mut Style) -> bool + 'static,
    {
        let mut exit = false;
        let style = unsafe {
            &mut *(self.imgui.style_mut() as *mut Style)
        };
        loop {
            let mut message = MSG::default();
            while unsafe { PeekMessageA(&mut message, None, 0, 0, PM_REMOVE) } == TRUE {
                unsafe {
                    let _ = TranslateMessage(&message);
                    let _ = DispatchMessageA(&message);
                    if message.message == WM_QUIT {
                        exit = true;
                        break;
                    }
                };
            }
            if !self.window_tracker.update(self.hwnd) {
                exit = true;
            }
            if exit {
                break;
            }

            unsafe {
                ImGui_ImplDX11_NewFrame();
                ImGui_ImplWin32_NewFrame();
            }
            {
                self.imgui_active_check()?;
            }
            {
                let frame = self.imgui.new_frame();

                exit = !render(frame, style)
            }
            let mut guard = GLOBAL_DATA.lock().unwrap();
            if let Some(ref mut renderer) = *guard {
                unsafe {
                    let view = renderer.p_main_render_target_view.take().unwrap();
                    renderer.pd3d_device_context.OMSetRenderTargets(Some(&[Some(view.clone())]), None);
                    renderer.pd3d_device_context.ClearRenderTargetView(&view, &[0f32; 4]);
                    ImGui_ImplDX11_RenderDrawData(self.imgui.render());
                    let _ = renderer.p_swap_chain.Present(self.sync_interval, DXGI_PRESENT(0));
                    renderer.p_main_render_target_view = Some(view);
                }
            }
        }
        unsafe {
            ImGui_ImplDX11_Shutdown();
            ImGui_ImplWin32_Shutdown();
        }
        {
            let mut guard = GLOBAL_DATA.lock().unwrap();
            if let Some(ref mut r) = *guard {
                r.cleanup_render_target();
            }
        }
        unsafe {
            let _ = UnregisterClassW(self.wc.lpszClassName, Some(self.wc.hInstance));
        }
        self.free();
        Ok(())
    }

    /// imgui窗口检查
    #[inline]
    fn imgui_active_check(&mut self) -> Result<()> {
        {
            let io = self.imgui.io_mut();
            // 活动检查 1.鼠标输入事件 2.鼠标按键事件
            {
                let mut point = POINT::default();
                unsafe {
                    let _ = GetCursorPos(&mut point)?;
                    let _ = ScreenToClient(self.hwnd, &mut point);
                };
                io.add_mouse_pos_event([point.x as _, point.y as _]);
            }
            let imgui_active = io.want_capture_mouse;
            if imgui_active != self.window_is_active
            {
                self.window_is_active = imgui_active;
                if imgui_active {
                    unsafe {
                        let _ = SetWindowLongA(self.hwnd, GWL_EXSTYLE, (WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW).0 as _);
                    }
                } else {
                    unsafe {
                        let _ = SetWindowLongA(self.hwnd, GWL_EXSTYLE, (WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW).0 as _);
                    }
                }
                if imgui_active {
                    unsafe {
                        let _ = SetActiveWindow(self.hwnd);
                    };
                }
            }
            Ok(())
        }
    }

    /// 释放
    #[cfg(feature = "lib")]
    fn free(&self) {
        let ptr = self.hinstance.0 as usize;
        thread::spawn(move || unsafe {
            if let Err(e) = windows::Win32::System::Console::FreeConsole() {
                log::error!("{e:?}");
            }
            *GLOBAL_DATA.lock().unwrap() = None;
            FreeLibraryAndExitThread(HMODULE(ptr as _), 0);
        });
    }

    /// 释放
    #[cfg(not(feature = "lib"))]
    fn free(&self) {}
}


extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if ImGui_ImplWin32_WndProcHandler(window.0, message, wparam, lparam).0 > 0 {
            return LRESULT(0);
        }

        match message {
            WM_PAINT => {
                let _ = ValidateRect(Some(window), None);
                LRESULT(0)
            }
            WM_SIZE => {
                if wparam.0 as u32 != SIZE_MINIMIZED {
                    let mut guard = GLOBAL_DATA.lock().unwrap();
                    if let Some(ref mut renderer) = *guard {
                        renderer.cleanup_render_target();
                        let _ = renderer.p_swap_chain.ResizeBuffers(0, loword!(lparam.0 as u32), hiword!(lparam.0 as u32), DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SWAP_CHAIN_FLAG(0));
                        let _ = renderer.create_render_target();
                        return LRESULT(0);
                    }
                }
                DefWindowProcA(window, message, wparam, lparam)
            }
            WM_SYSCOMMAND => {
                if ((wparam.0 & 0xfff0) as u32) == SC_KEYMENU {
                    LRESULT(0)
                } else {
                    DefWindowProcA(window, message, wparam, lparam)
                }
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_NCHITTEST => {
                LRESULT(1)
            }
            _ => DefWindowProcA(window, message, wparam, lparam),
        }
    }
}

#[cfg(not(feature = "lib"))]
extern "C" {
    pub fn GetAsyncKeyState(key: i32) -> u16;
    pub fn GetCurrentProcessId() -> u32;
}

#[cfg(feature = "lib")]
pub unsafe fn GetAsyncKeyState(key: i32) -> u16 {
    windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(key) as _
}

#[cfg(feature = "lib")]
pub unsafe fn GetCurrentProcessId() -> u32 {
    windows::Win32::System::Threading::GetCurrentProcessId()
}
/// 是否按下按键
#[macro_export]
macro_rules! key_down {
    ($key:expr)=>{
        unsafe{imgui_rs_overlay::window::GetAsyncKeyState($key) == 32769u16}
    }
}
/// 是否长按按键
#[macro_export]
macro_rules! w_key_down {
    ($key:expr)=>{
        unsafe{imgui_rs_overlay::window::GetAsyncKeyState($key) > 0u16}
    }
}