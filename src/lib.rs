use std::{
    ffi::CString,
    time::Instant,
};
use std::time::Duration;

use ash::vk;
use copypasta::ClipboardContext;
use imgui::{
    Context,
    FontConfig,
    FontSource,
    Io,
};
#[cfg(feature = "imgui")]
pub use imgui;
use imgui_rs_vulkan_renderer::{
    Options,
    Renderer,
};
use imgui_winit_support::{
    HiDpiMode,
    winit::{
        dpi::PhysicalSize,
        event::{
            Event,
            WindowEvent,
        },
        event_loop::{
            EventLoop,
        },
        platform::windows::WindowExtWindows,
        window::{
            Window,
            WindowBuilder,
        },
    },
    WinitPlatform,
};
use imgui_winit_support::winit::event_loop::{EventLoopBuilder};
use imgui_winit_support::winit::platform::windows::EventLoopBuilderExtWindows;
use obfstr::obfstr;
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{
            BOOL,
            HWND,
        },
        Graphics::{
            Dwm::{
                DWM_BB_BLURREGION,
                DWM_BB_ENABLE,
                DWM_BLURBEHIND,
                DwmEnableBlurBehindWindow,
            },
            Gdi::CreateRectRgn,
        },
        UI::{
            Input::KeyboardAndMouse::SetActiveWindow,
            WindowsAndMessaging::{
                GetWindowLongPtrA,
                GWL_EXSTYLE,
                GWL_STYLE,
                HWND_TOPMOST,
                MB_ICONERROR,
                MB_OK,
                MessageBoxA,
                SetWindowDisplayAffinity,
                SetWindowLongA,
                SetWindowLongPtrA,
                SetWindowPos,
                ShowWindow,
                SW_SHOWNOACTIVATE,
                SWP_NOACTIVATE,
                SWP_NOMOVE,
                SWP_NOSIZE,
                WDA_EXCLUDEFROMCAPTURE,
                WDA_NONE,
                WS_CLIPSIBLINGS,
                WS_EX_LAYERED,
                WS_EX_NOACTIVATE,
                WS_EX_TOOLWINDOW,
                WS_EX_TRANSPARENT,
                WS_POPUP,
                WS_VISIBLE,
            },
        },
    },
};
#[cfg(feature = "windows")]
pub use windows;
use winit::dpi::{LogicalSize, Size};

use clipboard::ClipboardSupport;
pub use error::*;
use input::{
    KeyboardInputSystem,
    MouseInputSystem,
};
pub use perf::PerfTracker;
use vulkan_render::*;
pub use window_tracker::{OverlayTarget,WINDOWS_RECT};
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

pub fn show_error_message(title: &str, message: &str) {
    let title = CString::new(title).unwrap_or_else(|_| CString::new("[[ NulError ]]").unwrap());
    let message = CString::new(message).unwrap_or_else(|_| CString::new("[[ NulError ]]").unwrap());
    unsafe {
        MessageBoxA(
            HWND::default(),
            PCSTR::from_raw(message.as_ptr() as *const u8),
            PCSTR::from_raw(title.as_ptr() as *const u8),
            MB_ICONERROR | MB_OK,
        );
    }
}

fn create_window(event_loop: &EventLoop<()>, title: &str) -> Result<(Window, HWND)> {
    let window = WindowBuilder::new()
        .with_title(title.to_owned())
        .with_inner_size(Size::Logical(LogicalSize{width:1.0,height:1.0}))
        .build(&event_loop)?;
    let my_hwnd = {
        let id = window.id();
        let string = format!("{:?}", id);
        let mut my_hwnd = String::new();
        for char in string.chars() {
            if char.is_digit(10) {
                my_hwnd.push(char);
            }
        }
        my_hwnd.parse::<isize>().unwrap()
    };
    let hwnd: HWND = HWND(my_hwnd as _);
    {
        unsafe {
            // Make it transparent
            SetWindowLongA(
                hwnd,
                GWL_STYLE,
                (WS_POPUP | WS_VISIBLE | WS_CLIPSIBLINGS).0 as i32,
            );
            SetWindowLongPtrA(
                hwnd,
                GWL_EXSTYLE,
                (WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT).0
                    as isize,
            );

            let mut bb: DWM_BLURBEHIND = Default::default();
            bb.dwFlags = DWM_BB_ENABLE | DWM_BB_BLURREGION;
            bb.fEnable = BOOL::from(true);
            bb.hRgnBlur = CreateRectRgn(0, 0, 1, 1);
            DwmEnableBlurBehindWindow(hwnd, &bb)?;

            // Move the window to the top
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    Ok((window, hwnd))
}

pub struct OverlayOptions {
    pub title: String,
    pub target: OverlayTarget,
    /// 帧率(近似值与实际有差别)
    pub fps: i32,
    pub font_init: Option<Box<dyn Fn(&mut imgui::Context) -> ()>>,
}

fn create_imgui_context(options: &OverlayOptions) -> Result<(WinitPlatform, imgui::Context)> {
    let mut imgui = Context::create();
    imgui.set_ini_filename(None);

    let platform = WinitPlatform::init(&mut imgui);

    match ClipboardContext::new() {
        Ok(backend) => imgui.set_clipboard_backend(ClipboardSupport(backend)),
        Err(error) => log::warn!("Failed to initialize clipboard: {}", error),
    };

    // Fixed font size. Note imgui_winit_support uses "logical
    // pixels", which are physical pixels scaled by the devices
    // scaling factor. Meaning, 13.0 pixels should look the same size
    // on two different screens, and thus we do not need to scale this
    // value (as the scaling is handled by winit)
    let font_size = 18.0;
    imgui.fonts().add_font(&[FontSource::TtfData {
        data: include_bytes!("../resources/HPSimplified_Bd.ttf"),
        size_pixels: font_size,
        config: Some(FontConfig {
            glyph_ranges: imgui::FontGlyphRanges::chinese_full(),
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
    if let Some(callback) = &options.font_init {
        callback(&mut imgui);
    }

    Ok((platform, imgui))
}

pub struct System {
    pub event_loop: EventLoop<()>,

    pub window: Window,
    pub platform: WinitPlatform,

    pub vulkan_context: VulkanContext,
    command_buffer: vk::CommandBuffer,
    swapchain: Swapchain,
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    fence: vk::Fence,

    pub imgui: Context,
    pub renderer: Renderer,

    pub window_tracker: WindowTracker,
    /// 帧率刷新间隔，单位毫秒
    pub fps_time_interval: i32,
    /// 覆盖窗口句柄
    pub hwnd: HWND,
}

pub fn init(options: &OverlayOptions) -> Result<System> {
    let window_tracker = WindowTracker::new(&options.target)?;

    let event_loop = EventLoopBuilder::new()
        .with_any_thread(true)
        .build()
        .expect("事件循环构建失败");
    let (window, hwnd) = create_window(&event_loop, &options.title)?;

    let vulkan_context = VulkanContext::new(&window, &options.title)?;
    let command_buffer = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(vulkan_context.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        unsafe {
            vulkan_context
                .device
                .allocate_command_buffers(&allocate_info)?[0]
        }
    };

    let swapchain = Swapchain::new(&vulkan_context)?;
    let image_available_semaphore = {
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        unsafe {
            vulkan_context
                .device
                .create_semaphore(&semaphore_info, None)?
        }
    };
    let render_finished_semaphore = {
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        unsafe {
            vulkan_context
                .device
                .create_semaphore(&semaphore_info, None)?
        }
    };
    let fence = {
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        unsafe { vulkan_context.device.create_fence(&fence_info, None)? }
    };

    let (mut platform, mut imgui) = create_imgui_context(&options)?;
    platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Default);
    let fps_time_interval = 1000 / options.fps;
    let renderer = Renderer::with_default_allocator(
        &vulkan_context.instance,
        vulkan_context.physical_device,
        vulkan_context.device.clone(),
        vulkan_context.graphics_queue,
        vulkan_context.command_pool,
        swapchain.render_pass,
        &mut imgui,
        Some(Options {
            in_flight_frames: 1,
            ..Default::default()
        }),
    )?;

    /* The Vulkan backend can handle 32bit vertex offsets, but forgets to insert that flag... */
    imgui
        .io_mut()
        .backend_flags
        .insert(imgui::BackendFlags::RENDERER_HAS_VTX_OFFSET);

    Ok(System {
        event_loop,
        window,

        vulkan_context,
        swapchain,
        command_buffer,
        image_available_semaphore,
        render_finished_semaphore,
        fence,

        imgui,
        platform,
        renderer,

        window_tracker,
        fps_time_interval,
        hwnd,
    })
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

const PERF_RECORDS: usize = 2048;

impl System {
    pub fn main_loop<U, R>(self, mut update: U, mut render: R)
        where
            U: FnMut(&mut SystemRuntimeController) -> bool + 'static,
            R: FnMut(&mut imgui::Ui) -> bool + 'static,
    {
        let System {
            event_loop,
            window,
            vulkan_context,
            mut swapchain,
            command_buffer,
            fence,
            image_available_semaphore,
            render_finished_semaphore,
            imgui,
            mut platform,
            mut renderer,
            window_tracker,
            fps_time_interval,
            hwnd,
            ..
        } = self;
        let mut last_frame = Instant::now();

        let mut runtime_controller = SystemRuntimeController {
            hwnd,
            imgui,

            active_tracker: OverlayActiveTracker::new(),
            key_input_system: KeyboardInputSystem::new(),
            mouse_input_system: MouseInputSystem::new(),
            window_tracker,

            frame_count: 0,
            debug_overlay_shown: false,
        };

        let mut dirty_swapchain = false;
        let target_frame_time = Duration::from_millis(fps_time_interval as u64);
        let mut last_time = Instant::now();
        let mut perf = PerfTracker::new(PERF_RECORDS);
        event_loop.run(move |event, control_flow| {
            platform.handle_event(runtime_controller.imgui.io_mut(), &window, &event);
            match event {
                // New frame
                Event::NewEvents(_) => {
                    perf.begin();
                    let now = Instant::now();
                    runtime_controller
                        .imgui
                        .io_mut()
                        .update_delta_time(now - last_frame);
                    last_frame = now;
                }

                // End of event processing
                Event::AboutToWait => {
                    platform
                        .prepare_frame(runtime_controller.imgui.io_mut(), &window)
                        .expect("Failed to prepare frame");
                    // 计算这一帧所花费的时间
                    let frame_time = last_time.elapsed();
                    // 计算还需要延迟多久才能达到目标帧时间
                    let remaining_time = target_frame_time.saturating_sub(frame_time);
                    // 如果这一帧耗时少于目标帧时间，则延迟剩余的时间
                    if remaining_time > Duration::from_secs(0) {
                        std::thread::sleep(remaining_time);
                    }
                    // 更新上次记录的时间点
                    last_time = Instant::now();
                    window.request_redraw();
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => control_flow.exit(),
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    perf.mark("events cleared");

                    /* Update */
                    {
                        if !runtime_controller.update_state(&window, hwnd) {
                            log::info!("Target window has been closed. Exiting overlay.");
                            control_flow.exit();
                            return;
                        }

                        if !update(&mut runtime_controller) {
                            control_flow.exit();
                            return;
                        }

                        perf.mark("update");
                    }

                    /* render */
                    {
                        // If swapchain must be recreated wait for windows to not be minimized anymore
                        if dirty_swapchain {
                            let PhysicalSize { width, height } = window.inner_size();
                            if width > 0 && height > 0 {
                                swapchain
                                    .recreate(&vulkan_context)
                                    .expect("Failed to recreate swapchain");
                                renderer
                                    .set_render_pass(swapchain.render_pass)
                                    .expect("Failed to rebuild renderer pipeline");
                                dirty_swapchain = false;
                            } else {
                                return;
                            }
                        }

                        let ui = runtime_controller.imgui.new_frame();
                        let run = render(ui);
                        if !run {
                            control_flow.exit();
                            return;
                        }
                        if runtime_controller.debug_overlay_shown {
                            ui.window("Render Debug")
                                .position([200.0, 200.0], imgui::Condition::FirstUseEver)
                                .size([400.0, 400.0], imgui::Condition::FirstUseEver)
                                .build(|| {
                                    ui.text(format!("FPS: {: >4.2}", ui.io().framerate));
                                    ui.same_line_with_pos(100.0);

                                    ui.text(format!(
                                        "Frame Time: {:.2}ms",
                                        ui.io().delta_time * 1000.0
                                    ));
                                    ui.same_line_with_pos(275.0);

                                    ui.text("History length:");
                                    ui.same_line();
                                    let mut history_length = perf.history_length();
                                    ui.set_next_item_width(75.0);
                                    if ui
                                        .input_scalar("##history_length", &mut history_length)
                                        .build()
                                    {
                                        perf.set_history_length(history_length);
                                    }
                                    perf.render(ui, ui.content_region_avail());
                                });
                        }
                        perf.mark("render frame");
                        let draw_data = runtime_controller.imgui.render();

                        unsafe {
                            vulkan_context
                                .device
                                .wait_for_fences(&[fence], true, u64::MAX)
                                .expect("Failed to wait ")
                        };

                        perf.mark("fence");
                        let next_image_result = unsafe {
                            swapchain.loader.acquire_next_image(
                                swapchain.khr,
                                std::u64::MAX,
                                image_available_semaphore,
                                vk::Fence::null(),
                            )
                        };
                        let image_index = match next_image_result {
                            Ok((image_index, _)) => image_index,
                            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                                dirty_swapchain = true;
                                return;
                            }
                            Err(error) => {
                                panic!("Error while acquiring next image. Cause: {}", error)
                            }
                        };

                        unsafe {
                            vulkan_context
                                .device
                                .reset_fences(&[fence])
                                .expect("Failed to reset fences")
                        };

                        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
                        let wait_semaphores = [image_available_semaphore];
                        let signal_semaphores = [render_finished_semaphore];

                        // Re-record commands to draw geometry
                        record_command_buffers(
                            &vulkan_context.device,
                            vulkan_context.command_pool,
                            command_buffer,
                            swapchain.framebuffers[image_index as usize],
                            swapchain.render_pass,
                            swapchain.extent,
                            &mut renderer,
                            &draw_data,
                        )
                            .expect("Failed to record command buffer");

                        let command_buffers = [command_buffer];
                        let submit_info = [vk::SubmitInfo::default()
                            .wait_semaphores(&wait_semaphores)
                            .wait_dst_stage_mask(&wait_stages)
                            .command_buffers(&command_buffers)
                            .signal_semaphores(&signal_semaphores)];

                        perf.mark("before submit");
                        unsafe {
                            vulkan_context
                                .device
                                .queue_submit(vulkan_context.graphics_queue, &submit_info, fence)
                                .expect("Failed to submit work to gpu.")
                        };
                        perf.mark("after submit");

                        let swapchains = [swapchain.khr];
                        let images_indices = [image_index];
                        let present_info = vk::PresentInfoKHR::default()
                            .wait_semaphores(&signal_semaphores)
                            .swapchains(&swapchains)
                            .image_indices(&images_indices);

                        let present_result = unsafe {
                            swapchain
                                .loader
                                .queue_present(vulkan_context.present_queue, &present_info)
                        };
                        match present_result {
                            Ok(is_suboptimal) if is_suboptimal => {
                                dirty_swapchain = true;
                            }
                            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                                dirty_swapchain = true;
                            }
                            Err(error) => panic!("Failed to present queue. Cause: {}", error),
                            _ => {}
                        }
                        perf.finish("present");

                        runtime_controller.frame_rendered();
                    }
                }
                _ => {}
            }
        }).expect("循环事件发生错误");
    }
}

pub struct SystemRuntimeController {
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
    fn update_state(&mut self, window: &Window, hwnd: HWND) -> bool {
        self.mouse_input_system.update(window, hwnd, self.imgui.io_mut());
        self.key_input_system.update(window, self.imgui.io_mut());
        self.active_tracker.update(hwnd, self.imgui.io());
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
