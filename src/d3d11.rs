use std::mem::MaybeUninit;

use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{D3D11CreateDeviceAndSwapChain, ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView, ID3D11Resource, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::{core::Result, Win32::Foundation::{HWND, TRUE}, Win32::Graphics::Dxgi::{
    Common::{DXGI_MODE_DESC, DXGI_RATIONAL, DXGI_SAMPLE_DESC}, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH, DXGI_SWAP_EFFECT_DISCARD,
    DXGI_USAGE_RENDER_TARGET_OUTPUT,
}};

pub struct D3d11Renderer {
    pub p_swap_chain: IDXGISwapChain,
    pub pd3d_device: ID3D11Device,
    pub pd3d_device_context: ID3D11DeviceContext,
    pub p_main_render_target_view: Option<ID3D11RenderTargetView>,
}

impl D3d11Renderer {
    /// 绑定到窗口
    pub fn bind(hwnd: HWND) -> Result<D3d11Renderer> {
        let sd = DXGI_SWAP_CHAIN_DESC {
            BufferCount: 2,
            BufferDesc: DXGI_MODE_DESC {
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                RefreshRate: DXGI_RATIONAL { Numerator: 60, Denominator: 1 },
                ..DXGI_MODE_DESC::default()
            },
            Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH.0 as _,
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            OutputWindow: hwnd,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Windowed: TRUE,
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            ..DXGI_SWAP_CHAIN_DESC::default()
        };
        let mut feature_level = D3D_FEATURE_LEVEL::default();
        let feature_level_array = [D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_0];
        let (p_swap_chain, pd3d_device, pd3d_device_context) = unsafe {
            let mut p_swap_chain = MaybeUninit::uninit();
            let mut pd3d_device = MaybeUninit::uninit();
            let mut pd3d_device_context = MaybeUninit::uninit();
            if D3D11CreateDeviceAndSwapChain(None,
                                             D3D_DRIVER_TYPE_HARDWARE,
                                             HMODULE(0 as _),
                                             D3D11_CREATE_DEVICE_FLAG(0),
                                             Some(&feature_level_array),
                                             D3D11_SDK_VERSION,
                                             Some(&sd),
                                             Some(p_swap_chain.as_mut_ptr()),
                                             Some(pd3d_device.as_mut_ptr()),
                                             Some(&mut feature_level),
                                             Some(pd3d_device_context.as_mut_ptr())).is_err() {
                D3D11CreateDeviceAndSwapChain(None,
                                              D3D_DRIVER_TYPE_WARP,
                                              HMODULE(0 as _),
                                              D3D11_CREATE_DEVICE_FLAG(0),
                                              Some(&feature_level_array),
                                              D3D11_SDK_VERSION,
                                              Some(&sd),
                                              Some(p_swap_chain.as_mut_ptr()),
                                              Some(pd3d_device.as_mut_ptr()),
                                              Some(&mut feature_level),
                                              Some(pd3d_device_context.as_mut_ptr()))?
            }
            (p_swap_chain.assume_init().unwrap(), pd3d_device.assume_init().unwrap(), pd3d_device_context.assume_init().unwrap())
        };
        let p_main_render_target_view =
            unsafe {
                let result: ID3D11Resource = p_swap_chain.GetBuffer(0)?;
                let mut p_main_render_target_view = MaybeUninit::uninit();
                pd3d_device.CreateRenderTargetView(&result, None, Some(p_main_render_target_view.as_mut_ptr()))?;
                p_main_render_target_view.assume_init().unwrap()
            };
        Ok(D3d11Renderer {
            p_swap_chain: p_swap_chain,
            pd3d_device: pd3d_device,
            pd3d_device_context: pd3d_device_context,
            p_main_render_target_view: Some(p_main_render_target_view),
        })
    }

    pub fn create_render_target(&mut self) ->Result<()>{
        unsafe {
            let result: ID3D11Resource = self.p_swap_chain.GetBuffer(0)?;
            let mut p_main_render_target_view = MaybeUninit::uninit();
            self.pd3d_device.CreateRenderTargetView(&result, None, Some(p_main_render_target_view.as_mut_ptr()))?;
            self.p_main_render_target_view = Some(p_main_render_target_view.assume_init().unwrap());
            Ok(())
        }
    }

    /// 清理渲染目标
    pub fn cleanup_render_target(&mut self) {
        if self.p_main_render_target_view.is_some() {
            self.p_main_render_target_view = None;
        }
    }
}