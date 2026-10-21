//! Một pet trên màn hình (`petView.ts`): lớp vẽ bằng một frame (chừa thêm vài cột mỗi bên cho lúc lảo đảo), di
//! chuyển bằng offset của visual. Chỉ vẽ lại surface khi đổi frame, hướng, độ nghiêng hay phần bị cửa sổ che.
//!
//! Toạ độ tính theo CSS pixel của overlay như bản web; `px` là số pixel màn hình cho một CSS pixel.

use crate::bubble::Bubble;
use crate::dizzy::{dizzy_lean, dizzy_reach, lean_shift, ring_row, DizzyStars};
use crate::gfx::{mirrored, rect, Gfx, Layer};
use crate::sim::pet::{Pet, PetState};
use crate::sim::terrain::{contains, Bounds};
use crate::sim::{js_round, Point, Rect};
use crate::sprites::{self, climb_reach, frame_index, head_of, mouth_of, Facing, Head, Mask, SpriteSet};
use anyhow::Result;
use std::collections::HashMap;
use windows::Win32::Graphics::Direct2D::{
    ID2D1Bitmap1, D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC, D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
};
use windows_numerics::Matrix3x2;

/// Đáy speech bubble (cả đuôi) cách đỉnh đầu pet chừng này CSS pixel.
const BUBBLE_LIFT: f64 = 12.0;

/// Những gì quyết định hình đang vẽ trên surface: đổi thì vẽ lại.
#[derive(Debug, Clone, PartialEq)]
struct Drawn {
    slot: usize,
    frame: usize,
    flip: bool,
    lean: i64,
    cuts: Vec<[i64; 4]>,
    size: (u32, u32),
}

/// Các lớp của một pet theo thứ tự từ dưới lên, do `Member` tạo (giữa chúng có lớp aura, thời tiết...).
pub struct ViewLayers {
    pub stars_back: Layer,
    pub sprite: Layer,
    pub stars_front: Layer,
    pub bubble: Layer,
}

pub struct PetView {
    pub sprite: SpriteSet,
    /// Bitmap của từng frame (theo `SpriteSet::animations`), đã thu nhỏ sẵn nếu pet nhỏ hơn frame.
    bitmaps: Vec<Vec<ID2D1Bitmap1>>,
    /// Bitmap đúng cỡ vẽ (thu nhỏ sẵn): vẽ 1:1.
    prescaled: bool,
    layer: Layer,
    stars: DizzyStars,
    bubble: Bubble,
    /// Cỡ trong Settings (1 là cỡ gốc của pack).
    size: f64,
    /// Số CSS pixel cho một pixel của frame: `scale` của pack nhân với cỡ trong Settings.
    scale: f64,
    /// Số pixel màn hình cho một CSS pixel.
    px: f64,
    /// Kích thước một frame khi vẽ (CSS pixel), không tính phần chừa cho lúc lảo đảo.
    pub width: f64,
    pub height: f64,
    /// Số cột pixel (của frame) chừa thêm mỗi bên surface để phần đầu nghiêng ra lúc lảo đảo không bị cắt.
    pad: f64,
    /// Cỡ surface (pixel màn hình) và số pixel màn hình cho một pixel của frame.
    canvas: (u32, u32),
    density: f64,
    drawn: Option<Drawn>,
    shown_state: Option<PetState>,
    /// Đầu nhân vật trong animation hiện tại, đo lại khi đổi state.
    head: Head,
    /// Miệng nhân vật ở từng frame (pixel của frame, theo hướng nhìn trong sprite), đo lần đầu cần tới.
    mouths: HashMap<(usize, usize), (f64, f64)>,
    slot: Option<usize>,
    frame: usize,
    flip: bool,
    /// Góc trên bên trái của frame (không tính phần chừa), CSS pixel của overlay; NaN khi chưa vẽ lần nào.
    left: f64,
    top: f64,
    /// Điểm chân lúc vẽ lần cuối (CSS pixel của overlay), có thể trễ hơn `pet.x/y` một bước mô phỏng.
    pub foot: Option<Point>,
    /// Đỉnh đầu đang lệch bao nhiêu pixel của frame lúc lảo đảo (dương: sang phải màn hình); 0 là đứng thẳng.
    lean: f64,
    /// Cửa sổ đang che pet (CSS pixel của overlay): không vẽ, không bắt chuột ở đó.
    occluders: Vec<Rect>,
    /// Khoảng từ điểm chân tới tường lúc leo (CSS pixel), đo theo tay nhân vật trong animation `climb`.
    pub reach: f64,
    /// Có cinematic đang vẽ thay sprite (`set_covered`).
    covered: bool,
    /// Pet đang biến mất (`Pet::vanished`), theo lần `update` gần nhất.
    gone: bool,
}

impl PetView {
    /// `sprite` phải còn điểm ảnh (vừa `sprites::load`); `size`: cỡ trong Settings; `px`: pixel màn hình cho một CSS
    /// pixel.
    pub fn new(gfx: &Gfx, layers: ViewLayers, sprite: SpriteSet, size: f64, px: f64) -> Result<Self> {
        let ViewLayers { stars_back, sprite: layer, stars_front, bubble } = layers;
        let mut view = Self {
            head: head_of(&sprite.animation(PetState::Idle).masks[0]),
            sprite,
            bitmaps: Vec::new(),
            prescaled: false,
            layer,
            stars: DizzyStars::new(stars_back, stars_front)?,
            bubble: Bubble::new(bubble)?,
            size,
            scale: 1.0,
            px,
            width: 0.0,
            height: 0.0,
            pad: 0.0,
            canvas: (1, 1),
            density: 1.0,
            drawn: None,
            shown_state: None,
            mouths: HashMap::new(),
            slot: None,
            frame: 0,
            flip: false,
            left: f64::NAN,
            top: f64::NAN,
            foot: None,
            lean: 0.0,
            occluders: Vec::new(),
            reach: 0.0,
            covered: false,
            gone: false,
        };
        view.configure(gfx)?;
        Ok(view)
    }

    /// Đổi cỡ pet (Settings); gọi `pet.resize(view.width, view.height, view.reach)` sau đó để sim biết cỡ mới.
    pub fn set_size(&mut self, gfx: &Gfx, size: f64) -> Result<()> {
        self.size = size;
        self.configure(gfx)
    }

    /// Màn hình đổi DPI (sang màn hình khác): vẽ lại cho nét.
    pub fn set_px(&mut self, gfx: &Gfx, px: f64) -> Result<()> {
        if px == self.px {
            return Ok(());
        }
        self.px = px;
        self.configure(gfx)
    }

    /// Tính lại cỡ, đưa frame lên GPU theo cỡ mới (nạp lại điểm ảnh nếu đã bỏ).
    fn configure(&mut self, gfx: &Gfx) -> Result<()> {
        let sprite = &self.sprite;
        self.scale = sprite.scale * self.size;
        self.pad = dizzy_reach(sprite.frame_height);
        let (fw, fh) = (f64::from(sprite.frame_width), f64::from(sprite.frame_height));
        self.width = fw * self.scale;
        self.height = fh * self.scale;
        self.reach = climb_reach(sprite) * self.scale;
        let canvas_width = self.width + 2.0 * self.pad * self.scale;
        self.canvas = (
            (js_round(canvas_width * self.px) as u32).max(1),
            (js_round(self.height * self.px) as u32).max(1),
        );
        self.density = f64::from(self.canvas.0) / (fw + 2.0 * self.pad);
        // Sao dùng lưới 48px riêng, không nhỏ đi khi pack tăng độ phân giải ảnh.
        let frame = sprite.frame_width.min(48);
        self.stars.configure(frame, self.scale * (f64::from(sprite.frame_width) / 48.0).max(1.0));
        self.upload(gfx)?;
        self.drawn = None;
        self.left = f64::NAN;
        Ok(())
    }

    fn upload(&mut self, gfx: &Gfx) -> Result<()> {
        if !self.sprite.has_pixels() {
            let fresh = sprites::load(self.sprite.id.as_deref());
            if fresh.id == self.sprite.id {
                self.sprite = fresh;
            }
        }
        let k = self.scale * self.px;
        let (fw, fh) = (self.sprite.frame_width, self.sprite.frame_height);
        // Pet nhỏ hơn frame (frame 192 px hiện 96 px ở cỡ 100%): thu nhỏ một lần ở đây bằng cách lấy trung bình từng
        // vùng, rồi vẽ 1:1. Lấy điểm gần nhất thì nét mảnh như mí mắt, con ngươi đứt đoạn, mỗi frame một kiểu, nhìn
        // như chớp mắt lỗi.
        self.prescaled = k < 1.0;
        let (bw, bh) = if self.prescaled {
            ((js_round(f64::from(fw) * k) as u32).max(1), (js_round(f64::from(fh) * k) as u32).max(1))
        } else {
            (fw, fh)
        };
        let mut bitmaps = Vec::new();
        for animation in self.sprite.animations() {
            let mut frames = Vec::new();
            for pixels in &animation.pixels {
                frames.push(if self.prescaled {
                    let frame = image::RgbaImage::from_raw(fw, fh, pixels.clone()).expect("frame đủ cỡ");
                    gfx.bitmap(bw, bh, &image::imageops::thumbnail(&frame, bw, bh).into_raw())?
                } else {
                    gfx.bitmap(fw, fh, pixels)?
                });
            }
            bitmaps.push(frames);
        }
        self.bitmaps = bitmaps;
        self.sprite.release_pixels();
        Ok(())
    }

    /// Một cinematic vẽ thay sprite (Long biến hình, quật đuôi): ẩn riêng sprite, các lớp effect vẫn vẽ.
    pub fn set_covered(&mut self, covered: bool) -> Result<()> {
        self.covered = covered;
        self.sync_hidden()
    }

    /// Sprite đang ẩn (bị cinematic thay, hoặc pet đang biến mất): không bắt chuột, không hiện bubble.
    pub fn sprite_hidden(&self) -> bool {
        !self.layer.visible()
    }

    fn sync_hidden(&mut self) -> Result<()> {
        self.layer.set_visible(!(self.covered || self.gone))
    }

    /// Bề ngang surface (CSS pixel): một frame cộng phần chừa hai bên.
    fn canvas_width(&self) -> f64 {
        self.width + 2.0 * self.pad * self.scale
    }

    /// Khung surface đang vẽ (CSS pixel của overlay); `None` khi chưa vẽ lần nào.
    pub fn bounds(&self) -> Option<Rect> {
        (!self.left.is_nan()).then(|| Rect {
            x: self.left - self.pad * self.scale,
            y: self.top,
            width: self.canvas_width(),
            height: self.height,
        })
    }

    /// Còn bao nhiêu giây nữa thì animation của `pet` đổi sang frame khác; vô cùng nếu không đổi nữa.
    pub fn next_frame_in(&self, pet: &Pet, pace: f64) -> f64 {
        let animation = self.sprite.animation(pet.state());
        let count = animation.len();
        if pet.pose().is_some() || count < 2 {
            return f64::INFINITY;
        }
        let fps = animation_fps(pet, animation.fps, pace);
        let time = pet.state_time().max(0.0);
        let index = (time * fps).floor();
        if !animation.looped && index >= (count - 1) as f64 {
            return f64::INFINITY;
        }
        (index + 1.0) / fps - time
    }

    /// Làm tròn theo pixel thật của màn hình để pixel art không bị nhoè khi di chuyển.
    fn snap(&self, value: f64) -> f64 {
        js_round(value * self.px) / self.px
    }

    /// `occluders`: cửa sổ đang che pet (`World::occluders`), phần bị che không vẽ. `at`: điểm chân để vẽ (vị trí nội
    /// suy giữa hai bước mô phỏng). `pace`: hệ số tốc độ (`World::pace`).
    pub fn update(
        &mut self,
        gfx: &Gfx,
        pet: &Pet,
        bounds: &Bounds,
        occluders: &[Rect],
        at: Point,
        pace: f64,
    ) -> Result<()> {
        self.gone = pet.vanished > 0.0;
        self.sync_hidden()?;
        let state = pet.state();
        let slot = self.sprite.slot_of(state);
        let animation = self.sprite.animation(state);
        let count = animation.len();
        let fps = animation_fps(pet, animation.fps, pace);
        let frame = match pet.pose() {
            None => frame_index(count, fps, animation.looped, pet.state_time()),
            Some(pose) => pose.min(count - 1),
        };
        let flip = (pet.facing == 1) != (self.sprite.facing == Facing::Right);
        if Some(state) != self.shown_state {
            self.enter_state(state)?;
        }
        let (anchor_x, anchor_y) = self.sprite.anchor;
        let frame_width = f64::from(self.sprite.frame_width);
        let scale = self.scale;
        let anchor_x = if flip { frame_width - anchor_x } else { anchor_x };
        // Làm tròn gốc surface một lần; vùng bắt chuột dùng đúng gốc frame đã vẽ.
        let canvas_left = self.snap(at.x - (anchor_x + self.pad) * scale);
        let left = canvas_left + self.pad * scale;
        let top = self.snap(at.y - anchor_y * scale);
        self.foot = Some(at);
        // Choáng: lảo đảo qua lại quanh điểm chân, đỉnh đầu lệch `lean` pixel của frame.
        let lean = if state == PetState::Dizzy {
            let mask = &self.sprite.animation(state).masks[frame];
            let lean = dizzy_lean(pet.state_time(), (anchor_y - self.head.top).max(1.0));
            self.fit_lean(lean, mask, flip, left, bounds)
        } else {
            0.0
        };
        // Chỉ giữ cửa sổ chạm vào surface, toạ độ tính từ góc surface để pet đi mà cửa sổ đứng yên cũng vẽ lại.
        let canvas_width = self.canvas_width();
        let height = self.height;
        let overlaps = |r: &Rect| {
            r.x < canvas_left + canvas_width && canvas_left < r.x + r.width && r.y < top + height && top < r.y + r.height
        };
        self.occluders = occluders.iter().filter(|r| overlaps(r)).copied().collect();
        let cuts: Vec<Rect> = (self.occluders.iter())
            .map(|r| Rect { x: r.x - canvas_left, y: r.y - top, ..*r })
            .collect();
        let quarter = |v: f64| js_round(v * 4.0) as i64;
        let drawn = Drawn {
            slot,
            frame,
            flip,
            lean: lean as i64,
            cuts: cuts.iter().map(|r| [r.x, r.y, r.width, r.height].map(quarter)).collect(),
            size: self.canvas,
        };
        let resized = self.layer.resize(gfx, self.canvas.0, self.canvas.1)?;
        if resized || self.drawn.as_ref() != Some(&drawn) {
            self.slot = Some(slot);
            self.frame = frame;
            self.flip = flip;
            self.lean = lean;
            self.draw(slot, frame, flip, &cuts)?;
            self.drawn = Some(drawn);
        }
        self.left = left;
        self.top = top;
        self.layer.move_to((canvas_left * self.px).round() as f32, (top * self.px).round() as f32)?;
        if state == PetState::Dizzy {
            self.place_stars(gfx, pet.state_time(), flip)?;
        }
        self.place_bubble(gfx, pet, flip, bounds)
    }

    /// Speech bubble ngay trên đầu, luôn nằm trong màn hình; đuôi bubble chỉ vào giữa đầu.
    fn place_bubble(&mut self, gfx: &Gfx, pet: &Pet, flip: bool, bounds: &Bounds) -> Result<()> {
        // Sprite đang ẩn thì cất bubble; câu chưa hết giờ thì hiện lại cùng pet.
        let text = if self.sprite_hidden() { "" } else { pet.speech.as_ref().map_or("", |s| s.text.as_str()) };
        self.bubble.set_text(gfx, text, self.px)?;
        if text.is_empty() {
            return Ok(());
        }
        let frame_width = f64::from(self.sprite.frame_width);
        let (width, height) = (self.bubble.width, self.bubble.height);
        let head_x =
            self.left + (if flip { frame_width - self.head.center_x } else { self.head.center_x }) * self.scale;
        let x = js_round((head_x - width / 2.0).max(bounds.left + 4.0).min(bounds.right - width - 4.0));
        let y = js_round((self.top + self.head.top * self.scale - BUBBLE_LIFT - height).max(bounds.top + 4.0));
        let tail = (head_x - x).max(12.0).min(width - 12.0);
        self.bubble.place(gfx, x, y, tail, self.px)
    }

    /// Đo lại đầu nhân vật cho animation mới, tắt sao choáng khi hết choáng.
    fn enter_state(&mut self, state: PetState) -> Result<()> {
        self.shown_state = Some(state);
        self.head = head_of(&self.sprite.animation(state).masks[0]);
        if state != PetState::Dizzy {
            self.stars.hide()?;
        }
        Ok(())
    }

    /// Miệng ở frame đang vẽ (chỗ thở ra khói lúc lạnh) theo CSS pixel của overlay, và hướng mặt trên màn hình (1:
    /// sang phải); `None` nếu chưa vẽ lần nào.
    pub fn mouth_at(&mut self) -> Option<(Point, i8)> {
        let slot = self.slot?;
        if self.left.is_nan() {
            return None;
        }
        let sprite = &self.sprite;
        let mouth = *self.mouths.entry((slot, self.frame)).or_insert_with(|| {
            let mask = &sprite.animations()[slot].masks[self.frame];
            let hint = sprite.mouth_y.map(|y| (y, f64::from(sprite.animation(PetState::Idle).masks[0].top)));
            mouth_of(mask, sprite.anchor.1, sprite.facing, hint)
        });
        let frame_width = f64::from(self.sprite.frame_width);
        let x = self.left + (if self.flip { frame_width - mouth.0 } else { mouth.0 }) * self.scale;
        let right = (self.sprite.facing == Facing::Right) != self.flip;
        Some((Point { x, y: self.top + mouth.1 * self.scale }, if right { 1 } else { -1 }))
    }

    /// Phần có hình của nhân vật lúc đứng yên (frame đầu của `idle`, từ đỉnh đầu tới chân) ở chỗ đang đứng, CSS pixel
    /// của overlay: hiệu ứng thời tiết và nhãn nhiệt độ đặt theo thân thật của từng con chứ không theo khung ảnh (có
    /// con chỉ chiếm nửa khung). `None` nếu chưa vẽ lần nào.
    pub fn body(&self) -> Option<Rect> {
        if self.left.is_nan() {
            return None;
        }
        let mask = &self.sprite.animation(PetState::Idle).masks[0];
        let (_, anchor_y) = self.sprite.anchor;
        let frame_width = f64::from(self.sprite.frame_width);
        if mask.right < 0 {
            return Some(Rect { x: self.left, y: self.top, width: self.width, height: anchor_y * self.scale });
        }
        let left = if self.flip { frame_width - f64::from(mask.right + 1) } else { f64::from(mask.left) };
        Some(Rect {
            x: self.left + left * self.scale,
            y: self.top + f64::from(mask.top) * self.scale,
            width: f64::from(mask.right + 1 - mask.left) * self.scale,
            height: (anchor_y - f64::from(mask.top)).max(1.0) * self.scale,
        })
    }

    /// Vòng sao quanh đầu, lệch theo đầu lúc lảo đảo.
    fn place_stars(&mut self, gfx: &Gfx, time: f64, flip: bool) -> Result<()> {
        let frame_width = f64::from(self.sprite.frame_width);
        let head_x = if flip { frame_width - self.head.center_x } else { self.head.center_x };
        let row = ring_row(self.head.top, self.sprite.anchor.1 - self.head.top);
        let x = self.left + (head_x + self.shift(row)) * self.scale;
        let y = self.top + row * self.scale;
        self.stars.update(gfx, time, x, y, self.px)
    }

    /// Con trỏ (CSS pixel của overlay) có nằm trên phần có hình của pet không.
    pub fn hit_test(&self, point: Point) -> bool {
        let Some(slot) = self.slot else {
            return false;
        };
        // Chỗ bị cửa sổ khác che thì click thuộc về cửa sổ đó.
        if self.occluders.iter().any(|r| contains(r, point.x, point.y)) {
            return false;
        }
        let (frame_width, frame_height) = (self.sprite.frame_width as i32, self.sprite.frame_height as i32);
        let scale = self.scale;
        let y = ((point.y - self.top) / scale).floor() as i32;
        if y < 0 || y >= frame_height {
            return false;
        }
        // Bỏ độ lệch lúc lảo đảo để về đúng pixel của frame.
        let mut x = ((point.x - self.left) / scale - self.shift((point.y - self.top) / scale)).floor() as i32;
        if x < 0 || x >= frame_width {
            return false;
        }
        if self.flip {
            x = frame_width - 1 - x;
        }
        self.sprite.animations()[slot].masks[self.frame].has(x, y)
    }

    /// Hàng `y` của frame (có thể lẻ) đang bị đẩy ngang bao nhiêu pixel của frame (lảo đảo).
    fn shift(&self, y: f64) -> f64 {
        lean_shift(self.lean, y, self.sprite.anchor.1, self.head.top, self.density)
    }

    /// Hàng `row` của surface đang bị đẩy ngang bao nhiêu pixel surface (lảo đảo), luôn là số nguyên.
    fn row_shift(&self, row: u32) -> f64 {
        let ky = f64::from(self.canvas.1) / f64::from(self.sprite.frame_height);
        js_round(self.shift((f64::from(row) + 0.5) / ky) * self.density)
    }

    /// Giữ phần bị đẩy ngang lúc lảo đảo trong phần surface chừa sẵn và trong màn hình: pet đứng sát mép thì nghiêng
    /// ít lại chứ không để mất một phần hình.
    fn fit_lean(&self, lean: f64, mask: &Mask, flip: bool, left: f64, bounds: &Bounds) -> f64 {
        if lean == 0.0 || mask.right < 0 {
            return 0.0;
        }
        let frame_width = f64::from(self.sprite.frame_width);
        let (min, max) = (f64::from(mask.left), f64::from(mask.right));
        // Cột có hình ngoài cùng bên trái / phải, theo chiều trên màn hình.
        let first = if flip { frame_width - 1.0 - max } else { min };
        let last = if flip { frame_width - 1.0 - min } else { max };
        let lowest = (-self.pad - first).max(((bounds.left - left) / self.scale).ceil() - first);
        let highest =
            (frame_width - 1.0 + self.pad - last).min(((bounds.right - left) / self.scale).floor() - last - 1.0);
        if lowest > highest {
            return 0.0;
        }
        highest.min(lowest.max(lean))
    }

    fn draw(&self, slot: usize, frame: usize, flip: bool, cuts: &[Rect]) -> Result<()> {
        let bitmap = &self.bitmaps[slot][frame];
        let (cw, ch) = (self.canvas.0 as f32, self.canvas.1 as f32);
        let x0 = js_round(self.pad * self.density) as f32;
        let (w, h) = if self.prescaled {
            let size = unsafe { bitmap.GetPixelSize() };
            (size.width as f32, size.height as f32)
        } else {
            ((f64::from(self.sprite.frame_width) * self.density) as f32, ch)
        };
        let mode = if self.prescaled || self.sprite.pixel_art {
            D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR
        } else {
            D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC
        };
        let rows: Vec<(u32, u32, f32)> = if self.lean == 0.0 {
            Vec::new()
        } else {
            // Lảo đảo: chép từng dải hàng, mỗi dải dời một số nguyên pixel: pixel art nghiêng thành bậc gọn mà vân
            // không đổi. Nghiêng cả hình bằng biến đổi thì bị nội suy, pet to lên là thấy nhoè.
            let mut strips = Vec::new();
            let mut row = 0;
            while row < self.canvas.1 {
                let shift = self.row_shift(row);
                let mut end = row + 1;
                while end < self.canvas.1 && self.row_shift(end) == shift {
                    end += 1;
                }
                strips.push((row, end, shift as f32));
                row = end;
            }
            strips
        };
        let (kx, ky) = (cw as f64 / self.canvas_width(), ch as f64 / self.height);
        self.layer.draw(|canvas| {
            let paint = |shift: f32| {
                let x = x0 + shift;
                canvas.transform(&if flip { mirrored(x, 0.0, w) } else { Matrix3x2::translation(x, 0.0) });
                unsafe { canvas.dc.DrawBitmap(bitmap, Some(&rect(0.0, 0.0, w, h)), 1.0, mode, None, None) };
            };
            if rows.is_empty() {
                paint(0.0);
            } else {
                for &(row, end, shift) in &rows {
                    canvas.reset();
                    canvas.push_clip(0.0, row as f32, cw, (end - row) as f32);
                    paint(shift);
                    canvas.pop_clip();
                }
            }
            canvas.reset();
            for r in cuts {
                canvas.clear_rect(
                    (r.x * kx) as f32,
                    (r.y * ky) as f32,
                    (r.width * kx) as f32,
                    (r.height * ky) as f32,
                );
            }
            Ok(())
        })
    }
}

/// Đi/chạy/leo nhanh hơn (Settings) hay chậm hơn (ban đêm) thì chân tay cũng theo, không trượt.
fn animation_fps(pet: &Pet, fps: f64, pace: f64) -> f64 {
    if matches!(pet.state(), PetState::Walk | PetState::Run | PetState::Climb) {
        fps * pace
    } else {
        fps
    }
}
