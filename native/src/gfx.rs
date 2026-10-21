//! Direct3D 11, Direct2D, DirectWrite và DirectComposition. Cửa sổ overlay không có bitmap riêng
//! (`WS_EX_NOREDIRECTIONBITMAP`): DWM ghép thẳng cây visual lên màn hình. Mỗi thứ vẽ được (pet, bóng thoại, thời
//! tiết...) là một `Layer`: visual có surface đúng cỡ. Đổi frame thì vẽ lại surface, đi lại thì chỉ đổi offset của
//! visual rồi `commit`, app không vẽ gì.

use anyhow::{bail, Result};
use std::mem::ManuallyDrop;
use windows::core::{Interface, HSTRING};
use windows::Win32::Foundation::{HMODULE, HWND, POINT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_OPEN, D2D1_GRADIENT_STOP,
    D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Bitmap1, ID2D1Brush, ID2D1DeviceContext, ID2D1DeviceContext5, ID2D1Factory1,
    ID2D1PathGeometry, ID2D1SolidColorBrush, ID2D1StrokeStyle, ID2D1SvgDocument, D2D1_ANTIALIAS_MODE_ALIASED,
    D2D1_BITMAP_OPTIONS_NONE, D2D1_CAP_STYLE_ROUND, D2D1_DASH_STYLE_SOLID, D2D1_ELLIPSE, D2D1_LINE_JOIN_ROUND,
    D2D1_STROKE_STYLE_PROPERTIES1, D2D1_STROKE_TRANSFORM_TYPE_NORMAL, ID2D1RenderTarget,
    D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_EXTEND_MODE_CLAMP,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_GAMMA_2_2, D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES,
    D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice2, IDCompositionDesktopDevice, IDCompositionSurface, IDCompositionTarget,
    IDCompositionVisual2, IDCompositionVisual3,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, IDWriteTextLayout, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_LINE_SPACING_METHOD_UNIFORM,
    DWRITE_TEXT_METRICS, DWRITE_WORD_WRAPPING_WRAP,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::UI::Shell::SHCreateMemStream;
use windows_numerics::{Matrix3x2, Vector2};

/// Màu RGBA, mỗi kênh 0–1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Từ mã hex `0xRRGGBB` và độ đục.
    pub const fn hex(rgb: u32, a: f32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xff) as f32 / 255.0,
            g: ((rgb >> 8) & 0xff) as f32 / 255.0,
            b: (rgb & 0xff) as f32 / 255.0,
            a,
        }
    }

    /// Như `rgba(r, g, b, a)` của CSS.
    pub fn rgba(r: u8, g: u8, b: u8, a: f64) -> Self {
        Self { r: f32::from(r) / 255.0, g: f32::from(g) / 255.0, b: f32::from(b) / 255.0, a: a.clamp(0.0, 1.0) as f32 }
    }

    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    fn d2d(self) -> D2D1_COLOR_F {
        D2D1_COLOR_F { r: self.r, g: self.g, b: self.b, a: self.a }
    }
}

pub fn rect(x: f32, y: f32, width: f32, height: f32) -> D2D_RECT_F {
    D2D_RECT_F { left: x, top: y, right: x + width, bottom: y + height }
}

pub fn point(x: f32, y: f32) -> Vector2 {
    Vector2 { X: x, Y: y }
}

pub struct Gfx {
    /// Chỉ để tạo bitmap và brush dùng chung; vẽ thì dùng device context mà surface trả về lúc `BeginDraw`.
    dc: ID2D1DeviceContext,
    factory: ID2D1Factory1,
    dwrite: IDWriteFactory,
    device: IDCompositionDesktopDevice,
    root: IDCompositionVisual2,
    target: Option<IDCompositionTarget>,
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
            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let device: IDCompositionDesktopDevice = DCompositionCreateDevice2(&d2d)?;
            let target = device.CreateTargetForHwnd(hwnd, true)?;
            let root = device.CreateVisual()?;
            target.SetRoot(&root)?;
            device.Commit()?;
            Ok(Self { dc, factory, dwrite, device, root, target: Some(target), _d3d: d3d })
        }
    }

    /// Nét bo tròn hai đầu và chỗ gấp (`lineCap = lineJoin = "round"` của canvas).
    pub fn round_stroke(&self) -> Result<ID2D1StrokeStyle> {
        let properties = D2D1_STROKE_STYLE_PROPERTIES1 {
            startCap: D2D1_CAP_STYLE_ROUND,
            endCap: D2D1_CAP_STYLE_ROUND,
            dashCap: D2D1_CAP_STYLE_ROUND,
            lineJoin: D2D1_LINE_JOIN_ROUND,
            miterLimit: 10.0,
            dashStyle: D2D1_DASH_STYLE_SOLID,
            dashOffset: 0.0,
            transformType: D2D1_STROKE_TRANSFORM_TYPE_NORMAL,
        };
        Ok(unsafe { self.factory.CreateStrokeStyle(&properties, None)?.cast()? })
    }

    /// Đường gấp khúc qua `points`.
    pub fn polyline(&self, points: &[(f32, f32)]) -> Result<ID2D1PathGeometry> {
        unsafe {
            let path = self.factory.CreatePathGeometry()?;
            let sink = path.Open()?;
            if let Some((&(x, y), rest)) = points.split_first() {
                sink.BeginFigure(point(x, y), D2D1_FIGURE_BEGIN_HOLLOW);
                for &(x, y) in rest {
                    sink.AddLine(point(x, y));
                }
                sink.EndFigure(D2D1_FIGURE_END_OPEN);
            }
            sink.Close()?;
            Ok(path.cast()?)
        }
    }

    /// Bỏ cây visual khỏi cửa sổ: một cửa sổ chỉ có một target DirectComposition, nên phải gọi trước khi tạo `Gfx`
    /// mới cho cùng cửa sổ (`is_device_lost`).
    pub fn release(&mut self) {
        unsafe {
            let _ = self.root.RemoveAllVisuals();
        }
        self.target = None;
        unsafe {
            let _ = self.device.Commit();
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

    /// Kiểu chữ "Segoe UI" cỡ `size` (pixel), độ đậm `weight` (600, 700...), dòng cao `line` pixel.
    pub fn text_format(&self, size: f32, weight: i32, line: f32) -> Result<IDWriteTextFormat> {
        unsafe {
            let format = self.dwrite.CreateTextFormat(
                &HSTRING::from("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT(weight),
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                &HSTRING::from("vi-vn"),
            )?;
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
            format.SetLineSpacing(DWRITE_LINE_SPACING_METHOD_UNIFORM, line, line * 0.8)?;
            Ok(format)
        }
    }

    /// Dàn chữ `text` trong bề ngang `max_width`; trả về layout và cỡ thật (dòng dài nhất, tổng chiều cao).
    pub fn layout(&self, text: &str, format: &IDWriteTextFormat, max_width: f32) -> Result<(IDWriteTextLayout, f32, f32)> {
        let wide: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            let layout = self.dwrite.CreateTextLayout(&wide, format, max_width, f32::MAX)?;
            let mut metrics = DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&mut metrics)?;
            Ok((layout, metrics.width, metrics.height))
        }
    }

    /// Nhóm visual mới (không có hình), nằm trên mọi thứ đã có.
    pub fn group(&self) -> Result<Group> {
        let visual = unsafe { self.device.CreateVisual()? };
        unsafe { self.root.AddVisual(&visual, true, None)? };
        Ok(Group { visual })
    }

    /// Lớp vẽ mới cỡ 0 (gọi `Layer::resize` trước khi vẽ), nằm trên cùng của `parent` (hoặc của cả overlay).
    pub fn layer(&self, parent: Option<&Group>) -> Result<Layer> {
        let visual = unsafe { self.device.CreateVisual()? };
        let parent = parent.map_or(&self.root, |g| &g.visual);
        unsafe { parent.AddVisual(&visual, true, None)? };
        Ok(Layer { visual, surface: None, width: 0, height: 0, visible: true, offset: (f32::NAN, f32::NAN) })
    }

    /// Đưa `group` lên trên các visual khác của overlay.
    pub fn raise(&self, group: &Group) -> Result<()> {
        unsafe {
            self.root.RemoveVisual(&group.visual)?;
            self.root.AddVisual(&group.visual, true, None)?;
        }
        Ok(())
    }

    /// Bỏ `group` khỏi overlay.
    pub fn remove(&self, group: &Group) {
        unsafe {
            let _ = self.root.RemoveVisual(&group.visual);
        }
    }

    fn surface(&self, width: u32, height: u32) -> Result<IDCompositionSurface> {
        Ok(unsafe {
            self.device.CreateSurface(width, height, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_ALPHA_MODE_PREMULTIPLIED)?
        })
    }

    /// Lớp hiện dần lên trong `seconds` giây (như `animation: ... ease-out` của CSS). DWM tự chạy animation, app
    /// không phải thức dậy vẽ.
    pub fn fade_in(&self, layer: &Layer, seconds: f32) -> Result<()> {
        unsafe {
            let animation = self.device.CreateAnimation()?;
            // Độ đục 1 - (1 - t/T)², chậm dần về cuối.
            animation.AddCubic(0.0, 0.0, 2.0 / seconds, -1.0 / (seconds * seconds), 0.0)?;
            animation.End(f64::from(seconds), 1.0)?;
            let visual: IDCompositionVisual3 = layer.visual.cast()?;
            visual.SetOpacity(&animation)?;
        }
        Ok(())
    }

    /// Gửi mọi thay đổi (vẽ lại surface, dời visual) cho DWM một lần.
    pub fn commit(&self) -> Result<()> {
        unsafe { self.device.Commit()? };
        Ok(())
    }
}

/// Lỗi `error` là do mất thiết bị GPU (driver cập nhật hay khởi động lại, đổi card màn hình): phải tạo lại `Gfx`
/// và mọi lớp vẽ.
pub fn is_device_lost(error: &anyhow::Error) -> bool {
    use windows::Win32::Foundation::D2DERR_RECREATE_TARGET;
    use windows::Win32::Graphics::Dxgi::{DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET};
    error.chain().any(|cause| {
        cause.downcast_ref::<windows::core::Error>().is_some_and(|e| {
            [DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET, D2DERR_RECREATE_TARGET].contains(&e.code())
        })
    })
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

/// Nhóm các lớp của một pet, để đưa cả con lên trên con khác.
pub struct Group {
    visual: IDCompositionVisual2,
}

/// Một visual có surface riêng.
pub struct Layer {
    visual: IDCompositionVisual2,
    surface: Option<IDCompositionSurface>,
    pub width: u32,
    pub height: u32,
    visible: bool,
    offset: (f32, f32),
}

impl Layer {
    /// Đổi cỡ surface (pixel); cỡ mới thì phải vẽ lại cả surface. `true` nếu đã đổi.
    pub fn resize(&mut self, gfx: &Gfx, width: u32, height: u32) -> Result<bool> {
        let (width, height) = (width.max(1), height.max(1));
        if self.surface.is_some() && width == self.width && height == self.height {
            return Ok(false);
        }
        let surface = gfx.surface(width, height)?;
        unsafe { self.visual.SetContent(&surface)? };
        self.surface = Some(surface);
        self.width = width;
        self.height = height;
        Ok(true)
    }

    /// Vẽ lại cả surface: xoá trong suốt rồi gọi `paint` với device context, gốc toạ độ ở góc trên trái surface.
    pub fn draw(&self, paint: impl FnOnce(&Canvas) -> Result<()>) -> Result<()> {
        let Some(surface) = &self.surface else {
            return Ok(());
        };
        unsafe {
            let mut offset = POINT::default();
            let dc: ID2D1DeviceContext = surface.BeginDraw(None, &mut offset)?;
            let canvas = Canvas { dc, origin: (offset.x as f32, offset.y as f32) };
            canvas.reset();
            canvas.dc.Clear(Some(&Color::TRANSPARENT.d2d()));
            canvas.dc.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            let result = paint(&canvas);
            surface.EndDraw()?;
            result
        }
    }

    /// Đặt góc trên trái (pixel của overlay). Chỉ có hiệu lực sau `Gfx::commit`.
    pub fn move_to(&mut self, x: f32, y: f32) -> Result<()> {
        if self.offset == (x, y) {
            return Ok(());
        }
        self.offset = (x, y);
        unsafe {
            self.visual.SetOffsetX2(x)?;
            self.visual.SetOffsetY2(y)?;
        }
        Ok(())
    }

    pub fn set_visible(&mut self, visible: bool) -> Result<()> {
        if self.visible == visible {
            return Ok(());
        }
        self.visible = visible;
        let visual: IDCompositionVisual3 = self.visual.cast()?;
        unsafe { visual.SetVisible(visible)? };
        Ok(())
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn set_opacity(&self, opacity: f32) -> Result<()> {
        let visual: IDCompositionVisual3 = self.visual.cast()?;
        unsafe { visual.SetOpacity2(opacity)? };
        Ok(())
    }

    /// Biến đổi quanh góc trên trái của visual (xoay, lật), như `transform` của CSS sau `translate`.
    pub fn set_transform(&self, transform: &Matrix3x2) -> Result<()> {
        unsafe { self.visual.SetTransform2(transform)? };
        Ok(())
    }
}

/// Device context đang vẽ một surface; toạ độ tính từ góc trên trái surface (đã cộng `origin` của atlas).
pub struct Canvas {
    pub dc: ID2D1DeviceContext,
    origin: (f32, f32),
}

impl Canvas {
    /// Bỏ mọi biến đổi đã đặt, về gốc của surface.
    pub fn reset(&self) {
        self.transform(&Matrix3x2::identity());
    }

    /// Đặt biến đổi `m` (tính trong toạ độ của surface).
    pub fn transform(&self, m: &Matrix3x2) {
        let shift = Matrix3x2::translation(self.origin.0, self.origin.1);
        unsafe { self.dc.SetTransform(&(*m * shift)) };
    }

    pub fn brush(&self, color: Color) -> Result<ID2D1SolidColorBrush> {
        Ok(unsafe { self.dc.CreateSolidColorBrush(&color.d2d(), None)? })
    }

    fn stops(&self, stops: &[(f32, Color)]) -> Result<windows::Win32::Graphics::Direct2D::ID2D1GradientStopCollection> {
        let stops: Vec<D2D1_GRADIENT_STOP> =
            stops.iter().map(|(position, color)| D2D1_GRADIENT_STOP { position: *position, color: color.d2d() }).collect();
        let target: &ID2D1RenderTarget = &self.dc;
        Ok(unsafe { target.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP)? })
    }

    /// Như `createRadialGradient(cx, cy, 0, cx, cy, r)` của canvas.
    pub fn radial(&self, cx: f32, cy: f32, r: f32, stops: &[(f32, Color)]) -> Result<ID2D1Brush> {
        let properties = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES {
            center: point(cx, cy),
            gradientOriginOffset: point(0.0, 0.0),
            radiusX: r,
            radiusY: r,
        };
        let collection = self.stops(stops)?;
        let brush = unsafe {
            self.dc.CreateRadialGradientBrush(&properties, None, &collection)?
        };
        Ok(brush.cast()?)
    }

    /// Như `createLinearGradient(x0, y0, x1, y1)` của canvas.
    pub fn linear(&self, from: (f32, f32), to: (f32, f32), stops: &[(f32, Color)]) -> Result<ID2D1Brush> {
        let properties =
            D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES { startPoint: point(from.0, from.1), endPoint: point(to.0, to.1) };
        let collection = self.stops(stops)?;
        let brush = unsafe {
            self.dc.CreateLinearGradientBrush(&properties, None, &collection)?
        };
        Ok(brush.cast()?)
    }

    pub fn fill_rect(&self, x: f32, y: f32, width: f32, height: f32, brush: &ID2D1Brush) {
        unsafe { self.dc.FillRectangle(&rect(x, y, width, height), brush) };
    }

    /// Xoá vùng chữ nhật về trong suốt (phần pet bị cửa sổ che).
    pub fn clear_rect(&self, x: f32, y: f32, width: f32, height: f32) {
        unsafe {
            self.dc.PushAxisAlignedClip(&rect(x, y, width, height), D2D1_ANTIALIAS_MODE_ALIASED);
            self.dc.Clear(Some(&Color::TRANSPARENT.d2d()));
            self.dc.PopAxisAlignedClip();
        }
    }

    /// Chỉ vẽ trong vùng chữ nhật cho tới `pop_clip`.
    pub fn push_clip(&self, x: f32, y: f32, width: f32, height: f32) {
        unsafe { self.dc.PushAxisAlignedClip(&rect(x, y, width, height), D2D1_ANTIALIAS_MODE_ALIASED) };
    }

    pub fn pop_clip(&self) {
        unsafe { self.dc.PopAxisAlignedClip() };
    }

    pub fn line(&self, from: (f32, f32), to: (f32, f32), brush: &ID2D1Brush, width: f32) {
        unsafe { self.dc.DrawLine(point(from.0, from.1), point(to.0, to.1), brush, width, None) };
    }

    /// Như `line` với kiểu nét `stroke` (bo tròn...).
    pub fn styled_line(&self, from: (f32, f32), to: (f32, f32), brush: &ID2D1Brush, width: f32, stroke: &ID2D1StrokeStyle) {
        unsafe { self.dc.DrawLine(point(from.0, from.1), point(to.0, to.1), brush, width, stroke) };
    }

    pub fn stroke(&self, path: &ID2D1PathGeometry, brush: &ID2D1Brush, width: f32, stroke: &ID2D1StrokeStyle) {
        unsafe { self.dc.DrawGeometry(path, brush, width, stroke) };
    }

    /// Hình tròn tâm (`cx`, `cy`) bán kính `r`.
    pub fn fill_circle(&self, cx: f32, cy: f32, r: f32, brush: &ID2D1Brush) {
        let ellipse = D2D1_ELLIPSE { point: point(cx, cy), radiusX: r, radiusY: r };
        unsafe { self.dc.FillEllipse(&ellipse, brush) };
    }

    pub fn text(&self, layout: &IDWriteTextLayout, x: f32, y: f32, brush: &ID2D1Brush) {
        unsafe {
            self.dc.DrawTextLayout(
                point(x, y),
                layout,
                brush,
                windows::Win32::Graphics::Direct2D::D2D1_DRAW_TEXT_OPTIONS_NONE,
            )
        };
    }

    /// Vẽ hình SVG `svg` (chuỗi cố định trong code) vừa khung `size`×`size` tại (`x`, `y`).
    pub fn svg(&self, svg: &str, x: f32, y: f32, size: f32) -> Result<()> {
        let dc5: ID2D1DeviceContext5 = self.dc.cast()?;
        let document: ID2D1SvgDocument = unsafe {
            let stream = SHCreateMemStream(Some(svg.as_bytes())).ok_or_else(|| anyhow::anyhow!("SHCreateMemStream"))?;
            dc5.CreateSvgDocument(&stream, D2D_SIZE_F { width: size, height: size })?
        };
        let mut previous = Matrix3x2::default();
        unsafe {
            self.dc.GetTransform(&mut previous);
            self.dc.SetTransform(&(Matrix3x2::translation(x, y) * previous));
            dc5.DrawSvgDocument(&document);
            self.dc.SetTransform(&previous);
        }
        Ok(())
    }
}

/// Lật ngang quanh trục giữa khung rộng `width`, rồi dời tới (`x`, `y`).
pub fn mirrored(x: f32, y: f32, width: f32) -> Matrix3x2 {
    Matrix3x2 { M11: -1.0, M12: 0.0, M21: 0.0, M22: 1.0, M31: x + width, M32: y }
}

/// Xoay `angle` radian quanh điểm (`cx`, `cy`).
pub fn rotation(angle: f32, cx: f32, cy: f32) -> Matrix3x2 {
    let (sin, cos) = angle.sin_cos();
    Matrix3x2 {
        M11: cos,
        M12: sin,
        M21: -sin,
        M22: cos,
        M31: cx - cos * cx + sin * cy,
        M32: cy - sin * cx - cos * cy,
    }
}

/// Phóng to quanh gốc.
pub fn scale(sx: f32, sy: f32) -> Matrix3x2 {
    Matrix3x2 { M11: sx, M12: 0.0, M21: 0.0, M22: sy, M31: 0.0, M32: 0.0 }
}
