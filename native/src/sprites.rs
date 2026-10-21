//! Đọc sprite pack (`assets/sprites/<pack>/pet.json` và ảnh, định dạng: assets/README.md). Mỗi frame thành điểm
//! ảnh BGRA nhân trước alpha, đúng định dạng bitmap Direct2D nhận.

use anyhow::{bail, Context, Result};
use image::RgbaImage;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    frame_width: u32,
    frame_height: u32,
    #[serde(default = "default_scale")]
    scale: f32,
    #[serde(default)]
    pixel_art: bool,
    #[serde(default)]
    facing: Facing,
    anchor: Option<Anchor>,
    animations: HashMap<String, AnimationSpec>,
}

fn default_scale() -> f32 {
    2.0
}

#[derive(Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Facing {
    Left,
    #[default]
    Right,
}

#[derive(Deserialize)]
struct Anchor {
    x: f32,
    y: f32,
}

#[derive(Deserialize)]
struct AnimationSpec {
    image: String,
    frames: Option<u32>,
    fps: f32,
    #[serde(default)]
    start: u32,
    #[serde(default)]
    row: u32,
    #[serde(default = "yes")]
    r#loop: bool,
}

fn yes() -> bool {
    true
}

pub struct Animation {
    /// Mỗi frame `frame_width * frame_height * 4` byte BGRA, nhân trước alpha.
    pub frames: Vec<Vec<u8>>,
    pub fps: f32,
    pub looped: bool,
}

pub struct Pack {
    pub frame_width: u32,
    pub frame_height: u32,
    /// Số CSS pixel cho một pixel của frame ở cỡ 100%.
    pub scale: f32,
    pub pixel_art: bool,
    pub facing: Facing,
    /// Điểm chân trong frame (pixel của frame).
    pub anchor: (f32, f32),
    pub animations: HashMap<String, Animation>,
}

pub fn load(dir: &Path) -> Result<Pack> {
    let manifest_path = dir.join("pet.json");
    let text = std::fs::read_to_string(&manifest_path).with_context(|| format!("đọc {}", manifest_path.display()))?;
    let manifest: Manifest =
        serde_json::from_str(&text).with_context(|| format!("{} sai định dạng", manifest_path.display()))?;
    if !manifest.animations.contains_key("idle") {
        bail!("{}: thiếu animation idle", manifest_path.display());
    }
    let mut sheets: HashMap<String, RgbaImage> = HashMap::new();
    let mut animations = HashMap::new();
    for (name, spec) in &manifest.animations {
        if spec.image.contains("..") || Path::new(&spec.image).is_absolute() {
            bail!("animations.{name}.image: phải là đường dẫn nằm trong thư mục pack");
        }
        if !sheets.contains_key(&spec.image) {
            let path = dir.join(&spec.image);
            let sheet = image::open(&path).with_context(|| format!("giải mã {}", path.display()))?.into_rgba8();
            sheets.insert(spec.image.clone(), sheet);
        }
        let frames = slice(&sheets[&spec.image], spec, manifest.frame_width, manifest.frame_height)
            .with_context(|| format!("animations.{name}"))?;
        animations.insert(
            name.clone(),
            Animation {
                frames: frames.into_iter().map(premultiplied_bgra).collect(),
                fps: spec.fps.clamp(0.5, 60.0),
                looped: spec.r#loop,
            },
        );
    }
    let anchor = manifest
        .anchor
        .map_or((manifest.frame_width as f32 / 2.0, manifest.frame_height as f32), |a| (a.x, a.y));
    Ok(Pack {
        frame_width: manifest.frame_width,
        frame_height: manifest.frame_height,
        scale: manifest.scale,
        pixel_art: manifest.pixel_art,
        facing: manifest.facing,
        anchor,
        animations,
    })
}

/// Cắt frame như `frameRects` trong packages/core: trái sang phải, hết hàng thì xuống hàng dưới.
fn slice(sheet: &RgbaImage, spec: &AnimationSpec, width: u32, height: u32) -> Result<Vec<RgbaImage>> {
    let columns = sheet.width() / width;
    let rows = sheet.height() / height;
    if columns < 1 || rows <= spec.row {
        bail!("ảnh {}×{} không chứa được frame {width}×{height} ở hàng {}", sheet.width(), sheet.height(), spec.row);
    }
    let available = ((rows - spec.row) * columns).saturating_sub(spec.start);
    let count = spec.frames.unwrap_or(available);
    if count < 1 || count > available {
        bail!("cần {count} frame từ vị trí {} nhưng ảnh chỉ còn {available}", spec.start);
    }
    Ok((0..count)
        .map(|i| {
            let index = spec.start + i;
            let (x, y) = ((index % columns) * width, (spec.row + index / columns) * height);
            image::imageops::crop_imm(sheet, x, y, width, height).to_image()
        })
        .collect())
}

fn premultiplied_bgra(frame: RgbaImage) -> Vec<u8> {
    let mut bytes = frame.into_raw();
    for pixel in bytes.as_chunks_mut::<4>().0 {
        let alpha = pixel[3] as u32;
        let (r, g, b) = (pixel[0] as u32, pixel[1] as u32, pixel[2] as u32);
        pixel[0] = ((b * alpha + 127) / 255) as u8;
        pixel[1] = ((g * alpha + 127) / 255) as u8;
        pixel[2] = ((r * alpha + 127) / 255) as u8;
    }
    bytes
}
