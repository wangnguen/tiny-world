//! Direct3D 11, Direct2D và DirectComposition. Cửa sổ overlay không có bitmap riêng (`WS_EX_NOREDIRECTIONBITMAP`):
//! DWM ghép thẳng cây visual lên màn hình. Mỗi pet một visual có surface đúng cỡ pet: đổi frame thì vẽ lại surface
//! (5–16 lần mỗi giây), đi lại thì chỉ đổi offset của visual rồi `Commit`, app không vẽ gì.

use anyhow::{bail, Result};
use std::mem::ManuallyDrop;
use windows::core::Interface;
use windows::Win32::Foundation::{HMODULE, HWND, POINT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Bitmap1, ID2D1DeviceContext, ID2D1Factory1, D2D1_BITMAP_OPTIONS_NONE,
    D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC, D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice2, IDCompositionDesktopDevice, IDCompositionSurface, IDCompositionTarget,
    IDCompositionVisual2,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows_numerics::Matrix3x2;

pub struct Gfx {
    /// Chỉ để tạo bitmap; vẽ thì dùng device context mà surface trả về lúc `BeginDraw`.
    dc: ID2D1DeviceContext,
    device: IDCompositionDesktopDevice,
    root: IDCompositionVisual2,
    _target: IDCompositionTarget,
    _d3d: ID3D11Device,
}

impl Gfx {
    pub fn new(hwnd: HWND) -> Result<Self> {
        unsafe {
            let d3d = d3d_device()?;
            let dxgi: IDXGIDevice = d3d.cast()?;
            let factory: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let d2d = factory.CreateDevice(&dxgi)?;
            let dc = d2d.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            let device: IDCompositionDesktopDevice = DCompositionCreateDevice2(&d2d)?;
            let target = device.CreateTargetForHwnd(hwnd, true)?;
            let root = device.CreateVisual()?;
            target.SetRoot(&root)?;
            device.Commit()?;
            Ok(Self { dc, device, root, _target: target, _d3d: d3d })
        }
    }

    /// Bitmap trên GPU từ điểm ảnh BGRA nhân trước alpha.
    pub fn bitmap(&self, width: u32, height: u32, bgra: &[u8]) -> Result<ID2D1Bitmap1> {
        let properties = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
            colorContext: ManuallyDrop::new(None),
        };
        let bitmap = unsafe {
            self.dc.CreateBitmap(D2D_SIZE_U { width, height }, Some(bgra.as_ptr().cast()), width * 4, &properties)?
        };
        Ok(bitmap)
    }

    /// Visual mới cỡ `width`×`height` pixel, nằm trên các visual đã có.
    pub fn sprite(&self, width: u32, height: u32) -> Result<SpriteVisual> {
        unsafe {
            let visual = self.device.CreateVisual()?;
            let surface =
                self.device.CreateSurface(width, height, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_ALPHA_MODE_PREMULTIPLIED)?;
            visual.SetContent(&surface)?;
            self.root.AddVisual(&visual, true, None)?;
            Ok(SpriteVisual { visual, surface, width, height })
        }
    }

    /// Gửi mọi thay đổi (vẽ lại surface, dời visual) cho DWM một lần.
    pub fn commit(&self) -> Result<()> {
        unsafe { self.device.Commit()? };
        Ok(())
    }
}

fn d3d_device() -> Result<ID3D11Device> {
    // Không có GPU dùng được (máy ảo, driver hỏng) thì vẽ bằng CPU qua WARP.
    for driver in [D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
        let mut device = None;
        let created = unsafe {
            D3D11CreateDevice(
                None,
                driver,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
        };
        if let (Ok(()), Some(device)) = (created, device) {
            return Ok(device);
        }
    }
    bail!("không tạo được thiết bị Direct3D 11")
}

pub struct SpriteVisual {
    visual: IDCompositionVisual2,
    surface: IDCompositionSurface,
    width: u32,
    height: u32,
}

impl SpriteVisual {
    /// Vẽ lại cả surface bằng `bitmap` co giãn vừa khít, lật ngang nếu `mirrored`. `nearest`: lấy điểm gần nhất thay
    /// vì nội suy (như `imageSmoothingEnabled = false` của canvas).
    pub fn draw(&self, bitmap: &ID2D1Bitmap1, mirrored: bool, nearest: bool) -> Result<()> {
        unsafe {
            let mut offset = POINT::default();
            let dc: ID2D1DeviceContext = self.surface.BeginDraw(None, &mut offset)?;
            let (x, y, w) = (offset.x as f32, offset.y as f32, self.width as f32);
            let transform = if mirrored {
                Matrix3x2 { M11: -1.0, M12: 0.0, M21: 0.0, M22: 1.0, M31: x + w, M32: y }
            } else {
                Matrix3x2::translation(x, y)
            };
            dc.SetTransform(&transform);
            dc.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
            let mode =
                if nearest { D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR } else { D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC };
            let dest = D2D_RECT_F { left: 0.0, top: 0.0, right: w, bottom: self.height as f32 };
            dc.DrawBitmap(bitmap, Some(&dest), 1.0, mode, None, None);
            self.surface.EndDraw()?;
        }
        Ok(())
    }

    /// Đặt góc trên trái của visual (pixel của overlay). Chỉ có hiệu lực sau `Gfx::commit`.
    pub fn move_to(&self, x: f32, y: f32) -> Result<()> {
        unsafe {
            self.visual.SetOffsetX2(x)?;
            self.visual.SetOffsetY2(y)?;
        }
        Ok(())
    }
}
