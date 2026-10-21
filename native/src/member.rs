//! Một pet trên overlay: hành vi, sprite đã đưa lên GPU, visual của nó, và mặt nạ để thử con trỏ có nằm trên phần
//! có hình không.

use crate::gfx::{Gfx, SpriteVisual};
use crate::pet::Pet;
use crate::sprites::{self, Facing};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use windows::Win32::Graphics::Direct2D::ID2D1Bitmap1;

/// Điểm có alpha trên mức này mới tính là trúng pet (như mặt nạ 1 bit của overlay web).
const HIT_ALPHA: u8 = 24;

/// Mặt nạ 1 bit mỗi pixel của frame.
struct Mask {
    width: u32,
    bits: Vec<u64>,
}

impl Mask {
    fn new(width: u32, bgra: &[u8]) -> Self {
        let mut bits = vec![0u64; (bgra.len() / 4).div_ceil(64)];
        for (i, pixel) in bgra.as_chunks::<4>().0.iter().enumerate() {
            if pixel[3] > HIT_ALPHA {
                bits[i / 64] |= 1 << (i % 64);
            }
        }
        Mask { width, bits }
    }

    fn get(&self, x: u32, y: u32) -> bool {
        let i = (y * self.width + x) as usize;
        self.bits.get(i / 64).is_some_and(|word| word & (1 << (i % 64)) != 0)
    }
}

struct Animation {
    bitmaps: Vec<ID2D1Bitmap1>,
    masks: Vec<Mask>,
    fps: f32,
    looped: bool,
}

impl Animation {
    /// Frame cần vẽ sau `time` giây kể từ lúc animation bắt đầu (`frameIndex` trong packages/core).
    fn frame_at(&self, time: f32) -> usize {
        let index = (time.max(0.0) * self.fps).floor() as usize;
        if self.looped {
            index % self.bitmaps.len()
        } else {
            index.min(self.bitmaps.len() - 1)
        }
    }
}

pub struct Member {
    pub pet: Pet,
    animations: HashMap<String, Animation>,
    frame_width: u32,
    frame_height: u32,
    anchor: (f32, f32),
    /// Vẽ bằng điểm gần nhất (pixel art phóng to, hoặc bitmap đã đúng cỡ) thay vì nội suy.
    nearest: bool,
    facing: Facing,
    /// Pixel của overlay cho một pixel của frame.
    k: f32,
    pub width: u32,
    pub height: u32,
    visual: SpriteVisual,
    drawn: Option<(&'static str, usize, bool)>,
    placed: Option<(i32, i32)>,
}

impl Member {
    /// Nạp pack `dir`, đưa mọi frame lên GPU rồi bỏ bản trên RAM, chỉ giữ mặt nạ 1 bit. `size`: cỡ trong Cài đặt;
    /// `px`: pixel vật lý cho một CSS pixel.
    pub fn load(gfx: &Gfx, dir: &Path, size: f32, px: f32, pet: Pet) -> Result<Self> {
        let pack = sprites::load(dir)?;
        let k = pack.scale * size * px;
        let width = (pack.frame_width as f32 * k).ceil() as u32;
        let height = (pack.frame_height as f32 * k).ceil() as u32;
        // Pet nhỏ hơn frame (frame 192 px hiện 96 px ở cỡ 100%): thu nhỏ một lần ở đây rồi vẽ 1:1.
        let shrunk = k < 1.0;
        let mut animations = HashMap::new();
        for (name, animation) in pack.animations {
            let bitmaps = animation
                .frames
                .iter()
                .map(|bgra| {
                    if shrunk {
                        gfx.bitmap(width, height, &shrink(bgra, pack.frame_width, pack.frame_height, width, height))
                    } else {
                        gfx.bitmap(pack.frame_width, pack.frame_height, bgra)
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            let masks = animation.frames.iter().map(|bgra| Mask::new(pack.frame_width, bgra)).collect();
            animations.insert(name, Animation { bitmaps, masks, fps: animation.fps, looped: animation.looped });
        }
        Ok(Member {
            pet,
            animations,
            frame_width: pack.frame_width,
            frame_height: pack.frame_height,
            anchor: pack.anchor,
            // Đã thu nhỏ sẵn thì vẽ 1:1, lấy điểm gần nhất là đúng từng pixel.
            nearest: shrunk || pack.pixel_art,
            facing: pack.facing,
            k,
            width,
            height,
            visual: gfx.sprite(width, height)?,
            drawn: None,
            placed: None,
        })
    }

    /// Nửa chiều ngang của pet: điểm giữa chân cách mép màn hình ít nhất chừng này.
    pub fn half_width(&self) -> f32 {
        self.width as f32 / 2.0
    }

    /// Animation, frame và hướng đang vẽ. Pack thiếu animation thì dùng `idle` (bắt buộc phải có).
    fn current(&self) -> (&'static str, &Animation, usize, bool) {
        let mut name = self.pet.state.animation();
        let animation = match self.animations.get(name) {
            Some(animation) => animation,
            None => {
                name = "idle";
                &self.animations["idle"]
            }
        };
        let mirrored = self.pet.facing_right != (self.facing == Facing::Right);
        (name, animation, animation.frame_at(self.pet.time), mirrored)
    }

    fn origin(&self) -> (i32, i32) {
        let left = (self.pet.x - self.anchor.0 * self.k).round() as i32;
        let top = (self.pet.y - self.anchor.1 * self.k).round() as i32;
        (left, top)
    }

    /// Vẽ lại surface nếu đổi frame hay hướng, dời visual nếu đổi chỗ. Trả về (đã vẽ, đã dời); cần `Gfx::commit`.
    pub fn render(&mut self) -> Result<(bool, bool)> {
        let (name, animation, frame, mirrored) = self.current();
        let key = (name, frame, mirrored);
        let drew = self.drawn != Some(key);
        if drew {
            self.visual.draw(&animation.bitmaps[frame], mirrored, self.nearest)?;
            self.drawn = Some(key);
        }
        let origin = self.origin();
        let moved = self.placed != Some(origin);
        if moved {
            self.visual.move_to(origin.0 as f32, origin.1 as f32)?;
            self.placed = Some(origin);
        }
        Ok((drew, moved))
    }

    /// Điểm (`x`, `y`) của overlay có nằm trên phần có hình của frame đang hiện không.
    pub fn hit(&self, x: f32, y: f32) -> bool {
        let Some((left, top)) = self.placed else { return false };
        let fx = ((x - left as f32) / self.k).floor();
        let fy = ((y - top as f32) / self.k).floor();
        if fx < 0.0 || fy < 0.0 || fx >= self.frame_width as f32 || fy >= self.frame_height as f32 {
            return false;
        }
        let (_, animation, frame, mirrored) = self.current();
        let fx = if mirrored { self.frame_width - 1 - fx as u32 } else { fx as u32 };
        animation.masks[frame].get(fx, fy as u32)
    }
}

/// Thu nhỏ frame bằng cách lấy trung bình từng vùng (như `imageSmoothingQuality = "high"` của canvas). Lấy điểm gần
/// nhất thì nét mảnh như mí mắt, con ngươi đứt đoạn, mỗi frame một kiểu, nhìn như chớp mắt lỗi. Điểm ảnh đã nhân
/// trước alpha nên trung bình từng kênh là đúng, không có viền tối quanh hình.
fn shrink(bgra: &[u8], width: u32, height: u32, to_width: u32, to_height: u32) -> Vec<u8> {
    let frame = image::RgbaImage::from_raw(width, height, bgra.to_vec()).expect("frame đủ width * height * 4 byte");
    image::imageops::thumbnail(&frame, to_width, to_height).into_raw()
}
