use std::fs;
use std::time::{Duration, Instant};

use ash::vk;
use copypasta::ClipboardContext;
use imgui::{Context, FontConfig, FontSource, Style, Ui};
use imgui_rs_vulkan_renderer::Renderer;
use imgui_winit_support::WinitPlatform;

use crate::{OverlayOptions, SystemRuntimeController};
use crate::app::window_app::WindowApp;
use crate::clipboard::ClipboardSupport;
use crate::error::Result;
use crate::vulkan_render::{Swapchain, VulkanContext};

struct ImguiWindowInfo {
    pub window: winit::window::Window,
    pub platform: WinitPlatform,
    pub vulkan_context: VulkanContext,
    command_buffer: vk::CommandBuffer,
    swapchain: Swapchain,
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    pub dirty_swapchain: bool,
    fence: vk::Fence,
    pub renderer: Renderer,
    pub runtime_controller: SystemRuntimeController,
}

pub mod window_app {
    use std::intrinsics::transmute;
    use std::time::{Duration, Instant};

    use ash::vk;
    use imgui::{Context, Style, Ui};
    use imgui_rs_vulkan_renderer::{Options, Renderer};
    use imgui_winit_support::HiDpiMode;
    use windows::Win32::Foundation::{COLORREF, HWND};
    use windows::Win32::UI::Controls::MARGINS;
    use windows::Win32::UI::WindowsAndMessaging::{GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST, LWA_ALPHA, SetLayeredWindowAttributes, SetWindowLongA, SetWindowLongPtrA, SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_CLIPSIBLINGS, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP};
    use winit::application::ApplicationHandler;
    use winit::dpi::{LogicalSize, PhysicalSize, Size};
    use winit::event::{Event, StartCause, WindowEvent};
    use winit::event_loop::ActiveEventLoop;
    use winit::platform::windows::{Color, WindowAttributesExtWindows};
    use winit::window::{WindowAttributes, WindowId};

    use crate::{OverlayActiveTracker, SystemRuntimeController};
    use crate::app::{create_imgui_context, ImguiWindowInfo};
    use crate::input::{KeyboardInputSystem, MouseInputSystem};
    use crate::vulkan_render::{record_command_buffers, Swapchain, VulkanContext};
    use crate::window_tracker::WindowTracker;

    /// ### Prepare a parameter structure and rendering function first
    /// #### Parameter structure
    /// ```
    /// let mut options = OverlayOptions {
    ///         target: OverlayTarget::WindowTitle(String::from("计算器")),
    ///         fps: 60,
    ///         ..OverlayOptions::default()
    ///     };
    /// ```
    /// #### A render function is required
    /// ```
    /// let func = move |ui: &mut Ui, style: &mut Style| {
    ///         ui.window("imgui")
    ///             .resizable(false)
    ///             .size([150.0, 100.0], Condition::FirstUseEver)
    ///             .movable(true)
    ///             .build(|| {
    ///                 ui.text(format!("FPS: {:.2}", ui.io().framerate));
    ///                 ui.text("你好世界!");
    ///             });
    ///         true
    ///
    /// ```
    /// #### Finally build the event loop and run the application
    /// ```
    ///  let event_loop = EventLoopBuilder::default().build().unwrap();
    ///  let mut windows_app = app::WindowApp::new(func, &mut options);
    ///  event_loop.run_app(&mut windows_app).unwrap();
    /// ```
    pub struct WindowApp<R> {
        pub title: String,
        pub attached_window: HWND,
        pub last_frame: Instant,
        pub winit_info: Option<ImguiWindowInfo>,
        /// 样式配置，每次配置后重置为None
        pub style_config: Option<Box<dyn Fn(&mut Context) -> ()>>,
        pub last_time: Instant,
        pub target_frame_time: Duration,
        pub render: R,
    }

    impl<R> ApplicationHandler for WindowApp<R> where R: FnMut(&mut Ui,&mut Style) -> bool {
        fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
            if let Some(imgui_window) = &mut self.winit_info {
                let now = Instant::now();
                imgui_window.runtime_controller.imgui.io_mut().update_delta_time(now - self.last_frame);
                self.last_frame = now;
            }
        }


        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let attributes = WindowAttributes::default()
                .with_visible(false)
                .with_title(&self.title)
                .with_title_background_color(Some(Color::SYSTEM_DEFAULT))
                .with_inner_size(Size::Logical(LogicalSize { width: 500.0, height: 300.0 }));
            let window = event_loop.create_window(attributes)
                .unwrap();
            let hwnd = unsafe { transmute(window.id()) };
            {
                unsafe {
                    // Make it transparent
                    SetWindowLongA(
                        hwnd,
                        GWL_STYLE,
                        (WS_POPUP | WS_CLIPSIBLINGS).0 as i32,
                    );
                    SetWindowLongPtrA(
                        hwnd,
                        GWL_EXSTYLE,
                        (WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT).0 as isize,
                    );

                    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
                    let margin: MARGINS = MARGINS { cxLeftWidth: -1, cxRightWidth: -1, cyTopHeight: -1, cyBottomHeight: -1 };
                    let _ = windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea(hwnd, &margin);
                    // Move the window to the top
                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_TOPMOST),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
            }
            let (vulkan_context, command_buffer, swapchain, image_available_semaphore, render_finished_semaphore, fence) = {
                let vulkan_context = VulkanContext::new(&window, &self.title).expect("vulkan_context create fail");
                let command_buffer = {
                    let allocate_info = vk::CommandBufferAllocateInfo::default()
                        .command_pool(vulkan_context.command_pool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1);

                    unsafe {
                        vulkan_context
                            .device
                            .allocate_command_buffers(&allocate_info).unwrap()[0]
                    }
                };

                let swapchain = Swapchain::new(&vulkan_context).unwrap();
                let image_available_semaphore = {
                    let semaphore_info = vk::SemaphoreCreateInfo::default();
                    unsafe {
                        vulkan_context
                            .device
                            .create_semaphore(&semaphore_info, None).unwrap()
                    }
                };
                let render_finished_semaphore = {
                    let semaphore_info = vk::SemaphoreCreateInfo::default();
                    unsafe {
                        vulkan_context
                            .device
                            .create_semaphore(&semaphore_info, None).unwrap()
                    }
                };
                let fence = {
                    let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
                    unsafe { vulkan_context.device.create_fence(&fence_info, None).unwrap() }
                };
                (vulkan_context, command_buffer, swapchain, image_available_semaphore, render_finished_semaphore, fence)
            };
            let (mut platform, mut imgui) = create_imgui_context().expect("imgui context create fail");
            platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Default);
            if let Some(func) = &self.style_config {
                func(&mut imgui);
            }
            self.style_config = None;
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
            ).expect("renderer allocator fail");
            self.winit_info = Some(ImguiWindowInfo {
                window,
                platform,
                vulkan_context,
                command_buffer,
                swapchain,
                image_available_semaphore,
                render_finished_semaphore,
                dirty_swapchain: false,
                fence,
                renderer,
                runtime_controller: SystemRuntimeController {
                    hwnd,
                    imgui,

                    active_tracker: OverlayActiveTracker::new(),
                    key_input_system: KeyboardInputSystem::new(),
                    mouse_input_system: MouseInputSystem::new(),
                    window_tracker: WindowTracker { hwnd, current_bounds: Default::default() },
                    frame_count: 0,
                    debug_overlay_shown: false,
                },
            })
        }

        /// 事件循环
        fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, window_event: WindowEvent) {
            let event: Event<()> = Event::WindowEvent { window_id, event: window_event.clone() };
            if let Some(imgui_info) = &mut self.winit_info {
                imgui_info.platform.handle_event(imgui_info.runtime_controller.imgui.io_mut(), &imgui_info.window, &event);
                match window_event {
                    WindowEvent::CloseRequested => { event_loop.exit() }
                    WindowEvent::RedrawRequested => {
                        {
                            {//update
                                if !imgui_info.runtime_controller.update_state(self.attached_window) {
                                    log::info!("Target window has been closed. Exiting overlay.");
                                    event_loop.exit();
                                    return;
                                }
                            }
                            {
                                // render
                                if imgui_info.dirty_swapchain {
                                    let PhysicalSize { width, height } = imgui_info.window.inner_size();
                                    if width > 0 && height > 0 {
                                        imgui_info.swapchain
                                            .recreate(&imgui_info.vulkan_context)
                                            .expect("Failed to recreate swapchain");
                                        imgui_info.renderer
                                            .set_render_pass(imgui_info.swapchain.render_pass)
                                            .expect("Failed to rebuild renderer pipeline");
                                        imgui_info.dirty_swapchain = false;
                                    } else {
                                        return;
                                    }
                                }

                                let style = {
                                    imgui_info.runtime_controller.imgui.style_mut() as *mut Style
                                };
                                let ui = imgui_info.runtime_controller.imgui.new_frame();
                                let run = (self.render)(ui,unsafe{&mut *style});
                                if !run {
                                    event_loop.exit();
                                    return;
                                }
                                let draw_data = imgui_info.runtime_controller.imgui.render();
                                unsafe {
                                    imgui_info.vulkan_context
                                        .device
                                        .wait_for_fences(&[imgui_info.fence], true, u64::MAX)
                                        .expect("Failed to wait ")
                                };

                                // perf.mark("fence");
                                let next_image_result = unsafe {
                                    imgui_info.swapchain.loader.acquire_next_image(
                                        imgui_info.swapchain.khr,
                                        u64::MAX,
                                        imgui_info.image_available_semaphore,
                                        vk::Fence::null(),
                                    )
                                };
                                let image_index = match next_image_result {
                                    Ok((image_index, _)) => image_index,
                                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                                        imgui_info.dirty_swapchain = true;
                                        return;
                                    }
                                    Err(error) => {
                                        panic!("Error while acquiring next image. Cause: {}", error)
                                    }
                                };

                                unsafe {
                                    imgui_info.vulkan_context
                                        .device
                                        .reset_fences(&[imgui_info.fence])
                                        .expect("Failed to reset fences")
                                };

                                let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
                                let wait_semaphores = [imgui_info.image_available_semaphore];
                                let signal_semaphores = [imgui_info.render_finished_semaphore];

                                // Re-record commands to draw geometry
                                record_command_buffers(
                                    &imgui_info.vulkan_context.device,
                                    imgui_info.vulkan_context.command_pool,
                                    imgui_info.command_buffer,
                                    imgui_info.swapchain.framebuffers[image_index as usize],
                                    imgui_info.swapchain.render_pass,
                                    imgui_info.swapchain.extent,
                                    &mut imgui_info.renderer,
                                    &draw_data,
                                )
                                    .expect("Failed to record command buffer");

                                let command_buffers = [imgui_info.command_buffer];
                                let submit_info = [vk::SubmitInfo::default()
                                    .wait_semaphores(&wait_semaphores)
                                    .wait_dst_stage_mask(&wait_stages)
                                    .command_buffers(&command_buffers)
                                    .signal_semaphores(&signal_semaphores)];

                                // perf.mark("before submit");
                                unsafe {
                                    imgui_info.vulkan_context
                                        .device
                                        .queue_submit(imgui_info.vulkan_context.graphics_queue, &submit_info, imgui_info.fence)
                                        .expect("Failed to submit work to gpu.")
                                };
                                let swapchains = [imgui_info.swapchain.khr];
                                let images_indices = [image_index];
                                let present_info = vk::PresentInfoKHR::default()
                                    .wait_semaphores(&signal_semaphores)
                                    .swapchains(&swapchains)
                                    .image_indices(&images_indices);

                                let present_result = unsafe {
                                    imgui_info.swapchain
                                        .loader
                                        .queue_present(imgui_info.vulkan_context.present_queue, &present_info)
                                };
                                match present_result {
                                    Ok(is_suboptimal) if is_suboptimal => {
                                        imgui_info.dirty_swapchain = true;
                                    }
                                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                                        imgui_info.dirty_swapchain = true;
                                    }
                                    Err(error) => panic!("Failed to present queue. Cause: {}", error),
                                    _ => {}
                                }
                                imgui_info.runtime_controller.frame_rendered();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        /// 用于帧率限制
        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            if let Some(imgui_info) = &mut self.winit_info {
                // End of event processing
                imgui_info.platform
                    .prepare_frame(imgui_info.runtime_controller.imgui.io_mut(), &imgui_info.window)
                    .expect("Failed to prepare frame");
                // 计算这一帧所花费的时间
                let frame_time = self.last_time.elapsed();
                // 计算还需要延迟多久才能达到目标帧时间
                let remaining_time = self.target_frame_time.saturating_sub(frame_time);
                // 如果这一帧耗时少于目标帧时间，则延迟剩余的时间
                if remaining_time.as_secs_f64() > 0.0 {
                    std::thread::sleep(remaining_time);
                }
                // 更新上次记录的时间点
                self.last_time = Instant::now();
                imgui_info.window.request_redraw();
            }
        }
    }
}

impl<R> WindowApp<R>
    where {
    pub fn new(r: R, option: &mut OverlayOptions) -> Self
        where R: FnMut(&mut Ui,&mut Style) -> bool {
        Self {
            title: option.title.clone(),
            attached_window: option.target.resolve_target_window().unwrap(),
            last_frame: Instant::now(),
            winit_info: None,
            style_config: option.style_init.take(),
            last_time: Instant::now(),
            target_frame_time: Duration::from_millis(1000 / option.fps as u64),
            render: r,
        }
    }
}

fn create_imgui_context() -> Result<(WinitPlatform, Context)> {
    let mut imgui = Context::create();
    imgui.set_ini_filename(None);
    let platform = WinitPlatform::new(&mut imgui);
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
    if let Ok(vec) = fs::read(r"C:\Windows\Fonts\monbaiti.ttf") {
        imgui.fonts().add_font(&[FontSource::TtfData {
            data: vec.as_slice(),
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
    }
    Ok((platform, imgui))
}
