//! Sprite pack (`assets/sprites/<pack>/pet.json` và ảnh, định dạng: assets/README.md): đọc và kiểm tra pet.json như
//! `parseSpriteManifest` của packages/core, cắt frame, mặt nạ để thử con trỏ, animation thay thế cho pack thiếu, pet
//! tạm vẽ bằng code khi không có pack nào. Điểm ảnh là BGRA nhân trước alpha, đúng định dạng bitmap Direct2D nhận.

use crate::assets;
use crate::sim::pet::PetState;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;
use tinyworld_backend::settings::MAX_PETS;

/// Thứ tự animation như `ANIMATION_NAMES` của packages/core.
pub const STATES: [PetState; 12] = [
    PetState::Idle,
    PetState::Walk,
    PetState::Run,
    PetState::Sleep,
    PetState::Dragged,
    PetState::Fall,
    PetState::Land,
    PetState::React,
    PetState::Dizzy,
    PetState::Climb,
    PetState::Perch,
    PetState::Jump,
];

const MAX_FRAME_SIZE: f64 = 1024.0;
const MAX_FRAMES: f64 = 256.0;
/// Độ đục tối thiểu (0–255) để một pixel được tính là có hình khi kiểm tra con trỏ.
const ALPHA_THRESHOLD: u8 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationSpec {
    /// File PNG/WebP, đường dẫn tương đối so với pet.json.
    pub image: String,
    /// Số frame. `None`: lấy hết các frame từ `start` tới cuối ảnh.
    pub frames: Option<u32>,
    pub fps: f64,
    pub start: u32,
    pub row: u32,
    /// `false`: chạy một lần rồi dừng ở frame cuối.
    pub looped: bool,
}

/// Nội dung pet.json.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub scale: f64,
    pub pixel_art: bool,
    pub facing: Facing,
    pub anchor: (f64, f64),
    pub mouth_y: Option<f64>,
    pub animations: Vec<(PetState, AnimationSpec)>,
}

impl Manifest {
    fn spec(&self, state: PetState) -> Option<&AnimationSpec> {
        self.animations.iter().find(|(s, _)| *s == state).map(|(_, spec)| spec)
    }
}

/// Đọc và kiểm tra pet.json. Sai chỗ nào thì trả lỗi chỉ rõ trường đó.
pub fn parse_manifest(value: &Value) -> Result<Manifest, String> {
    let root = object(value, "pet.json")?;
    let frame_width = integer(&root["frameWidth"], "frameWidth", 1.0, MAX_FRAME_SIZE)?;
    let frame_height = integer(&root["frameHeight"], "frameHeight", 1.0, MAX_FRAME_SIZE)?;
    let mut animations = Vec::new();
    for (name, spec) in object(&root["animations"], "animations")? {
        let Some(state) = STATES.into_iter().find(|s| s.name() == name) else {
            let names: Vec<_> = STATES.iter().map(|s| s.name()).collect();
            return Err(format!("animations.{name}: tên không hợp lệ, chỉ nhận {}.", names.join(", ")));
        };
        animations.push((state, parse_animation(spec, &format!("animations.{name}"))?));
    }
    if !animations.iter().any(|(s, _)| *s == PetState::Idle) {
        return Err("animations.idle: bắt buộc phải có.".into());
    }
    let (fw, fh) = (f64::from(frame_width), f64::from(frame_height));
    let anchor = match optional(root, "anchor") {
        None => (fw / 2.0, fh),
        Some(v) => {
            let point = object(v, "anchor")?;
            (number(&point["x"], "anchor.x", 0.0, fw)?, number(&point["y"], "anchor.y", 0.0, fh)?)
        }
    };
    let facing = match optional(root, "facing").map(Value::as_str) {
        None => Facing::Right,
        Some(Some("left")) => Facing::Left,
        Some(Some("right")) => Facing::Right,
        Some(_) => return Err("facing: chỉ nhận left hoặc right.".into()),
    };
    Ok(Manifest {
        name: optional(root, "name").map_or(Ok("pet".to_owned()), |v| string(v, "name"))?,
        frame_width,
        frame_height,
        scale: optional(root, "scale").map_or(Ok(2.0), |v| number(v, "scale", 0.25, 16.0))?,
        pixel_art: optional(root, "pixelArt").map_or(Ok(true), |v| boolean(v, "pixelArt"))?,
        facing,
        anchor,
        mouth_y: optional(root, "mouthY").map(|v| number(v, "mouthY", 0.0, fh)).transpose()?,
        animations,
    })
}

fn parse_animation(value: &Value, path: &str) -> Result<AnimationSpec, String> {
    let spec = object(value, path)?;
    Ok(AnimationSpec {
        image: image_path(&spec["image"], &format!("{path}.image"))?,
        frames: optional(spec, "frames").map(|v| integer(v, &format!("{path}.frames"), 1.0, MAX_FRAMES)).transpose()?,
        fps: number(&spec["fps"], &format!("{path}.fps"), 0.5, 60.0)?,
        start: optional(spec, "start")
            .map_or(Ok(0), |v| integer(v, &format!("{path}.start"), 0.0, MAX_FRAMES * 16.0))?,
        row: optional(spec, "row").map_or(Ok(0), |v| integer(v, &format!("{path}.row"), 0.0, MAX_FRAME_SIZE))?,
        looped: optional(spec, "loop").map_or(Ok(true), |v| boolean(v, &format!("{path}.loop")))?,
    })
}

/// Trường `key` nếu có (`null` vẫn tính là có, và sai kiểu), như `undefined` của bản TS.
fn optional<'a>(object: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a Value> {
    object.get(key)
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value.as_object().ok_or_else(|| format!("{path}: phải là object."))
}

fn number(value: &Value, path: &str, min: f64, max: f64) -> Result<f64, String> {
    match value.as_f64() {
        Some(n) if n.is_finite() && n >= min && n <= max => Ok(n),
        _ => Err(format!("{path}: phải là số trong khoảng {min}–{max}.")),
    }
}

fn integer(value: &Value, path: &str, min: f64, max: f64) -> Result<u32, String> {
    let n = number(value, path, min, max)?;
    if n.fract() != 0.0 {
        return Err(format!("{path}: phải là số nguyên."));
    }
    Ok(n as u32)
}

fn boolean(value: &Value, path: &str) -> Result<bool, String> {
    value.as_bool().ok_or_else(|| format!("{path}: phải là true hoặc false."))
}

fn string(value: &Value, path: &str) -> Result<String, String> {
    match value.as_str().map(str::trim) {
        Some(text) if !text.is_empty() => Ok(text.to_owned()),
        _ => Err(format!("{path}: phải là chuỗi không rỗng.")),
    }
}

/// Đường dẫn ảnh tương đối, không cho ra ngoài thư mục pack.
fn image_path(value: &Value, path: &str) -> Result<String, String> {
    let mut file = string(value, path)?.replace('\\', "/");
    while let Some(rest) = file.strip_prefix("./") {
        file = rest.to_owned();
    }
    if file.starts_with('/') || file.contains(':') || file.split('/').any(|part| part == "..") {
        return Err(format!("{path}: phải là đường dẫn nằm trong thư mục pack."));
    }
    Ok(file)
}

/// Vị trí một frame trong ảnh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRect {
    pub x: u32,
    pub y: u32,
}

/// Vị trí từng frame trong ảnh: trái sang phải, hết hàng thì xuống hàng dưới (`frameRects` của packages/core).
pub fn frame_rects(
    spec: &AnimationSpec,
    frame_width: u32,
    frame_height: u32,
    image_width: u32,
    image_height: u32,
) -> Result<Vec<FrameRect>, String> {
    let columns = image_width / frame_width;
    let rows = image_height / frame_height;
    if columns < 1 || rows <= spec.row {
        return Err(format!(
            "{}: ảnh {image_width}×{image_height} không chứa được frame {frame_width}×{frame_height} ở hàng {}.",
            spec.image, spec.row
        ));
    }
    let available = i64::from((rows - spec.row) * columns) - i64::from(spec.start);
    let count = spec.frames.map_or(available, i64::from);
    if count < 1 || count > available {
        return Err(format!(
            "{}: cần {count} frame từ vị trí {} nhưng ảnh chỉ còn {}.",
            spec.image,
            spec.start,
            available.max(0)
        ));
    }
    Ok((0..count as u32)
        .map(|i| {
            let index = spec.start + i;
            FrameRect { x: (index % columns) * frame_width, y: (spec.row + index / columns) * frame_height }
        })
        .collect())
}

/// Frame cần vẽ sau `time` giây kể từ lúc animation bắt đầu (`frameIndex` của packages/core).
pub fn frame_index(count: usize, fps: f64, looped: bool, time: f64) -> usize {
    let index = (time.max(0.0) * fps).floor() as usize;
    if looped {
        index % count
    } else {
        index.min(count - 1)
    }
}

/// Pixel nào của một frame có hình, 1 bit mỗi pixel. Tính sẵn khung chứa phần có hình để khỏi quét lại cả frame.
#[derive(Debug, Clone)]
pub struct Mask {
    pub width: u32,
    pub height: u32,
    bits: Vec<u64>,
    /// Cột ngoài cùng bên trái, bên phải có hình; frame trống thì `left` > `right` (`right` là -1).
    pub left: i32,
    pub right: i32,
    /// Hàng trên cùng có hình (frame trống thì bằng `height`).
    pub top: i32,
}

impl Mask {
    /// `bgra`: `width * height` điểm ảnh BGRA.
    pub fn new(width: u32, height: u32, bgra: &[u8]) -> Self {
        let count = (width * height) as usize;
        let mut bits = vec![0u64; count.div_ceil(64)];
        let (mut left, mut right, mut top) = (width as i32, -1, height as i32);
        for (i, pixel) in bgra.as_chunks::<4>().0.iter().take(count).enumerate() {
            if pixel[3] <= ALPHA_THRESHOLD {
                continue;
            }
            bits[i / 64] |= 1 << (i % 64);
            let (x, y) = ((i as u32 % width) as i32, (i as u32 / width) as i32);
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
        }
        Self { width, height, bits, left, right, top }
    }

    pub fn has(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }
        let i = (y as u32 * self.width + x as u32) as usize;
        self.bits[i / 64] & (1 << (i % 64)) != 0
    }
}

/// Một animation đã cắt frame: mặt nạ (giữ suốt), điểm ảnh (bỏ sau khi đưa lên GPU, `SpriteSet::release_pixels`).
#[derive(Debug, Clone)]
pub struct Animation {
    pub masks: Vec<Mask>,
    /// Mỗi frame `frame_width * frame_height * 4` byte BGRA nhân trước alpha; rỗng sau `release_pixels`.
    pub pixels: Vec<Vec<u8>>,
    pub fps: f64,
    pub looped: bool,
}

impl Animation {
    pub fn len(&self) -> usize {
        self.masks.len()
    }
}

/// Sprite đã nạp xong, sẵn sàng để vẽ.
#[derive(Debug, Clone)]
pub struct SpriteSet {
    /// Tên thư mục pack, `None` là pet tạm vẽ bằng code.
    pub id: Option<String>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub scale: f64,
    pub pixel_art: bool,
    pub facing: Facing,
    pub anchor: (f64, f64),
    pub mouth_y: Option<f64>,
    /// Animation thật của pack (và frame đứng yên cho ngủ, choáng mà pack không có).
    animations: Vec<Animation>,
    /// Mỗi state dùng animation nào trong `animations` (theo thứ tự `STATES`).
    slots: [usize; 12],
}

impl SpriteSet {
    pub fn animation(&self, state: PetState) -> &Animation {
        &self.animations[self.slots[slot(state)]]
    }

    /// Chỗ của animation dùng cho `state` trong `animations()`: hai state mượn chung một animation thì cùng chỗ.
    pub fn slot_of(&self, state: PetState) -> usize {
        self.slots[slot(state)]
    }

    pub fn animations(&self) -> &[Animation] {
        &self.animations
    }

    /// Bỏ điểm ảnh trên RAM (đã đưa lên GPU), chỉ giữ mặt nạ.
    pub fn release_pixels(&mut self) {
        for animation in &mut self.animations {
            animation.pixels = Vec::new();
        }
    }

    pub fn has_pixels(&self) -> bool {
        self.animations.iter().all(|a| a.pixels.len() == a.masks.len())
    }
}

fn slot(state: PetState) -> usize {
    STATES.iter().position(|s| *s == state).unwrap_or(0)
}

/// Pack thiếu animation thì mượn animation gần giống trước khi dùng `idle` (bảng thay thế: assets/README.md).
fn similar(state: PetState) -> Option<PetState> {
    Some(match state {
        PetState::Walk => PetState::Run,
        PetState::Run => PetState::Walk,
        PetState::Dragged => PetState::Fall,
        PetState::Fall => PetState::Dragged,
        // Bám tường trông gần giống bị nhấc lên (tay chân buông thõng), nhảy thì như nhảy lên khi bị click.
        PetState::Climb => PetState::Dragged,
        PetState::Jump => PetState::React,
        _ => return None,
    })
}

/// Điền đủ mọi animation cho pack chỉ có một phần (`withFallback`). Ngủ và choáng mà pack không có thì đứng yên ở
/// frame đầu của `idle`.
fn with_fallback(mut animations: Vec<Animation>, present: &[(PetState, usize)]) -> (Vec<Animation>, [usize; 12]) {
    let find = |state: PetState| present.iter().find(|(s, _)| *s == state).map(|(_, i)| *i);
    let idle = find(PetState::Idle).expect("pack luôn có idle");
    let mut still = None;
    let mut slots = [idle; 12];
    for (i, state) in STATES.into_iter().enumerate() {
        slots[i] = match find(state).or_else(|| similar(state).and_then(find)) {
            Some(index) => index,
            None if matches!(state, PetState::Sleep | PetState::Dizzy) => *still.get_or_insert_with(|| {
                let source = &animations[idle];
                let first = Animation {
                    masks: vec![source.masks[0].clone()],
                    pixels: source.pixels.first().cloned().into_iter().collect(),
                    fps: 1.0,
                    looped: source.looped,
                };
                animations.push(first);
                animations.len() - 1
            }),
            None => idle,
        };
    }
    (animations, slots)
}

/// Một nhân vật chọn được trong Cài đặt.
#[derive(Debug, Clone)]
pub struct PackInfo {
    /// Tên thư mục trong assets/sprites/.
    pub id: String,
    pub manifest: Manifest,
}

/// Pack đọc được `pet.json`, xếp theo tên thư mục; pack lỗi bị bỏ qua (ghi lý do vào log).
static PACKS: LazyLock<Vec<PackInfo>> = LazyLock::new(|| {
    let mut packs = Vec::new();
    for (path, bytes) in assets::FILES {
        let Some(id) = path.strip_prefix("sprites/").and_then(|rest| rest.strip_suffix("/pet.json")) else {
            continue;
        };
        let manifest = serde_json::from_slice(bytes).map_err(|e| e.to_string()).and_then(|v| parse_manifest(&v));
        match manifest {
            Ok(manifest) => packs.push(PackInfo { id: id.to_owned(), manifest }),
            Err(e) => log!("Sprite pack {id} lỗi, bỏ qua: {e}"),
        }
    }
    packs.sort_by(|a, b| a.id.cmp(&b.id));
    packs
});

pub fn pack(id: &str) -> Option<&'static PackInfo> {
    PACKS.iter().find(|p| p.id == id)
}

/// Pack sẽ được dùng cho `id`: đúng pack đó nếu còn, không thì pack đầu tiên; không có pack nào thì `None`.
pub fn resolve_pack(id: Option<&str>) -> Option<String> {
    id.and_then(pack).or_else(|| PACKS.first()).map(|p| p.id.clone())
}

/// Các pack sẽ hiện cho `Settings::pets`: bỏ pack không còn và pack trùng, tối đa `MAX_PETS`, giữ thứ tự chọn. Không
/// còn pack nào thì pack đầu tiên; chưa có pack nào thì `[None]` (một pet tạm).
pub fn resolve_packs(ids: &[String]) -> Vec<Option<String>> {
    let mut known: Vec<Option<String>> = Vec::new();
    for id in ids {
        if pack(id).is_some() && !known.iter().any(|k| k.as_deref() == Some(id)) {
            known.push(Some(id.clone()));
        }
    }
    if known.is_empty() {
        return vec![resolve_pack(None)];
    }
    known.truncate(MAX_PETS);
    known
}

/// Nạp pack `id` (xem `resolve_pack`). Chưa có pack hoặc pack lỗi thì dùng pet tạm.
pub fn load(id: Option<&str>) -> SpriteSet {
    let Some(info) = resolve_pack(id).and_then(|id| pack(&id)) else {
        return placeholder();
    };
    match load_pack(info) {
        Ok(sprite) => sprite,
        Err(e) => {
            log!("Sprite pack {} lỗi, dùng pet tạm: {e}", info.id);
            placeholder()
        }
    }
}

fn load_pack(info: &PackInfo) -> Result<SpriteSet, String> {
    let manifest = &info.manifest;
    let (fw, fh) = (manifest.frame_width, manifest.frame_height);
    let mut sheets: HashMap<String, image::RgbaImage> = HashMap::new();
    let mut animations = Vec::new();
    let mut present = Vec::new();
    for state in STATES {
        let Some(spec) = manifest.spec(state) else {
            continue;
        };
        if !sheets.contains_key(&spec.image) {
            let path = format!("sprites/{}/{}", info.id, spec.image);
            let bytes = assets::get(&path).ok_or_else(|| format!("Không thấy file {} trong {}.", spec.image, info.id))?;
            let sheet = image::load_from_memory(bytes).map_err(|e| format!("{}: {e}", spec.image))?.into_rgba8();
            sheets.insert(spec.image.clone(), sheet);
        }
        let sheet = &sheets[&spec.image];
        let rects = frame_rects(spec, fw, fh, sheet.width(), sheet.height())?;
        let pixels: Vec<Vec<u8>> = rects
            .iter()
            .map(|r| premultiplied_bgra(image::imageops::crop_imm(sheet, r.x, r.y, fw, fh).to_image()))
            .collect();
        let masks = pixels.iter().map(|p| Mask::new(fw, fh, p)).collect();
        present.push((state, animations.len()));
        animations.push(Animation { masks, pixels, fps: spec.fps, looped: spec.looped });
    }
    let (animations, slots) = with_fallback(animations, &present);
    Ok(SpriteSet {
        id: Some(info.id.clone()),
        frame_width: fw,
        frame_height: fh,
        scale: manifest.scale,
        pixel_art: manifest.pixel_art,
        facing: manifest.facing,
        anchor: manifest.anchor,
        mouth_y: manifest.mouth_y,
        animations,
        slots,
    })
}

/// RGBA thường thành BGRA nhân trước alpha.
pub fn premultiplied_bgra(frame: image::RgbaImage) -> Vec<u8> {
    let mut bytes = frame.into_raw();
    for pixel in bytes.as_chunks_mut::<4>().0 {
        let alpha = u32::from(pixel[3]);
        let (r, g, b) = (u32::from(pixel[0]), u32::from(pixel[1]), u32::from(pixel[2]));
        pixel[0] = ((b * alpha + 127) / 255) as u8;
        pixel[1] = ((g * alpha + 127) / 255) as u8;
        pixel[2] = ((r * alpha + 127) / 255) as u8;
    }
    bytes
}

/// Đỉnh đầu nhân vật trong frame (pixel của frame), để đặt hiệu ứng ngay trên đầu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Head {
    /// Hàng pixel trên cùng có hình.
    pub top: f64,
    /// Giữa đầu theo chiều ngang.
    pub center_x: f64,
}

pub fn head_of(mask: &Mask) -> Head {
    let (width, height, top) = (mask.width as i32, mask.height as i32, mask.top);
    if mask.right < 0 {
        return Head { top: 0.0, center_x: f64::from(width) / 2.0 };
    }
    // Lấy vài hàng trên cùng (tai, đỉnh đầu) chứ không lấy cả hình, để đuôi hay tay không kéo lệch tâm.
    let rows = (f64::from(height) / 8.0).round().max(1.0) as i32;
    let (mut min, mut max) = (width, -1);
    for y in top..height.min(top + rows) {
        for x in mask.left..=mask.right {
            if mask.has(x, y) {
                min = min.min(x);
                max = max.max(x);
            }
        }
    }
    Head { top: f64::from(top), center_x: f64::from(min + max + 1) / 2.0 }
}

/// Miệng (chỗ thở ra khói lúc lạnh) ở frame có hình `mask`, pixel của frame: mép mặt phía trước ở hàng miệng.
/// `hint`: hàng miệng `y` đo ở frame đầu của `idle` (`mouthY` trong pet.json) và đỉnh đầu `top` của frame đó; frame
/// này đầu cao thấp hơn bao nhiêu thì hàng miệng lệch theo bấy nhiêu. Không có thì đoán: chỗ nhô ra phía trước nhất
/// trong khoảng 25–65% chiều cao từ đỉnh đầu tới chân (`bottom`). `facing`: hướng nhân vật nhìn trong sprite.
pub fn mouth_of(mask: &Mask, bottom: f64, facing: Facing, hint: Option<(f64, f64)>) -> (f64, f64) {
    let top = f64::from(mask.top);
    if mask.right < 0 {
        return (f64::from(mask.width) / 2.0, f64::from(mask.height) / 2.0);
    }
    let tall = (bottom - top).max(1.0);
    let row = hint.map_or(-1.0, |(y, hint_top)| (y + top - hint_top).round());
    let from = if hint.is_some() { top.max(row - 2.0) } else { (top + tall * 0.25).round() };
    let to = f64::from(mask.height - 1).min(if hint.is_some() { row + 2.0 } else { (top + tall * 0.65).round() });
    let right = facing == Facing::Right;
    let ahead = |x: i32| if right { x } else { -x };
    let mut best: Option<(i32, i32)> = None;
    let mut y = from as i32;
    while f64::from(y) <= to {
        // Pixel có hình phía trước nhất của hàng này.
        for i in 0..=(mask.right - mask.left) {
            let x = if right { mask.right - i } else { mask.left + i };
            if !mask.has(x, y) {
                continue;
            }
            if best.is_none_or(|(bx, _)| ahead(x) > ahead(bx)) {
                best = Some((x, y));
            }
            break;
        }
        y += 1;
    }
    let front =
        best.unwrap_or_else(|| (if right { mask.right } else { mask.left }, ((from + to) / 2.0).round() as i32));
    // Ngay mép ngoài của pixel đó, đúng hàng miệng nếu đã biết.
    let x = if right { front.0 + 1 } else { front.0 };
    (f64::from(x), if hint.is_some() { row } else { f64::from(front.1) })
}

/// Khoảng từ điểm chân tới mép tay xa nhất trong animation `climb` (pixel của frame): chỗ tay chạm tường.
pub fn climb_reach(sprite: &SpriteSet) -> f64 {
    let mut reach: f64 = 0.0;
    for mask in &sprite.animation(PetState::Climb).masks {
        if mask.right < 0 {
            continue;
        }
        let edge = if sprite.facing == Facing::Right {
            f64::from(mask.right + 1) - sprite.anchor.0
        } else {
            sprite.anchor.0 - f64::from(mask.left)
        };
        reach = reach.max(edge);
    }
    reach.max(f64::from(sprite.frame_width) * 0.1)
}

// Pet tạm: một cục blob pixel art 16×16, dùng khi assets/sprites chưa có sprite pack nào.
const SIZE: u32 = 16;
const EMPTY: &str = "................";
const BASE: [&str; 16] = [
    EMPTY,
    EMPTY,
    EMPTY,
    ".....oooooo.....",
    "...oowwwwhwoo...",
    "..owwwwwwwhwwo..",
    ".owwwwwwwwwwwwo.",
    ".owwkwwwwwwkwwo.",
    "owwwkwwwwwwkwwwo",
    "owppwwwwwwwwppwo",
    "owwwwwwkkwwwwwwo",
    "owwwwwwwwwwwwwwo",
    "owswwwwwwwwwwswo",
    ".osswwwwwwwwsso.",
    "..oossssssssoo..",
    "....oooooooo....",
];

fn placeholder_color(c: char) -> Option<[u8; 3]> {
    Some(match c {
        'o' => [0x5a, 0x3d, 0x2b],
        'w' => [0xff, 0xe3, 0xb3],
        's' => [0xf5, 0xc9, 0x8a],
        'h' => [0xff, 0xff, 0xff],
        'k' => [0x2b, 0x1d, 0x14],
        'p' => [0xff, 0x9a, 0xa2],
        _ => return None,
    })
}

fn replace_rows(rows: &[&'static str], changes: &[(usize, &'static str)]) -> Vec<&'static str> {
    rows.iter()
        .enumerate()
        .map(|(i, row)| changes.iter().find(|(r, _)| *r == i).map_or(*row, |(_, c)| *c))
        .collect()
}

/// Thở: bớt một hàng giữa thân, chân vẫn chạm đất.
fn breathe(rows: &[&'static str]) -> Vec<&'static str> {
    let mut out = vec![EMPTY];
    out.extend_from_slice(&rows[..6]);
    out.extend_from_slice(&rows[7..]);
    out
}

fn placeholder_frames() -> Vec<Vec<&'static str>> {
    let base = BASE.to_vec();
    // Nhắm mắt: chớp mắt khi đứng, và lúc ngủ.
    let blink = replace_rows(&base, &[(7, BASE[6]), (8, "owwkkkwwwwkkkwwo")]);
    // Bị click hoặc tiếp đất: bẹp xuống, mắt cười ^ ^.
    let happy = replace_rows(&base, &[(8, "owwkwkwwwwkwkwwo")]);
    let mut squash = vec![EMPTY, EMPTY];
    squash.extend_from_slice(&happy[..6]);
    squash.extend_from_slice(&happy[7..11]);
    squash.extend_from_slice(&happy[12..]);
    // Đang rơi hoặc bị nhấc lên: miệng há tròn.
    let surprised = replace_rows(&base, &[(11, "owwwwwwkkwwwwwwo")]);
    // Choáng: mắt ✕.
    let dizzy = replace_rows(&base, &[(6, ".owkwkwwwwkwkwo."), (8, "owwkwkwwwwkwkwwo")]);
    vec![base.clone(), breathe(&base), blink.clone(), squash, breathe(&blink), surprised, dizzy]
}

fn paint(rows: &[&str]) -> Vec<u8> {
    let mut bytes = vec![0u8; (SIZE * SIZE * 4) as usize];
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            if let Some([r, g, b]) = placeholder_color(c) {
                let i = (y * SIZE as usize + x) * 4;
                bytes[i..i + 4].copy_from_slice(&[b, g, r, 255]);
            }
        }
    }
    bytes
}

/// Pet tạm vẽ bằng code (`createPlaceholderSprite`).
pub fn placeholder() -> SpriteSet {
    let frames: Vec<Vec<u8>> = placeholder_frames().iter().map(|rows| paint(rows)).collect();
    let (base, breathe, blink, squash, sleep, surprised, dizzy) = (0, 1, 2, 3, 4, 5, 6);
    let anim = |indices: &[usize], fps: f64, looped: bool| {
        let pixels: Vec<Vec<u8>> = indices.iter().map(|&i| frames[i].clone()).collect();
        let masks = pixels.iter().map(|p| Mask::new(SIZE, SIZE, p)).collect();
        Animation { masks, pixels, fps, looped }
    };
    let specs: [(PetState, Animation); 9] = [
        (PetState::Idle, anim(&[base, base, breathe, breathe, base, base, breathe, blink], 4.0, true)),
        // Blob không có chân: đi và chạy là nảy tưng tưng.
        (PetState::Walk, anim(&[base, breathe], 6.0, true)),
        (PetState::Run, anim(&[base, breathe], 12.0, true)),
        (PetState::Sleep, anim(&[blink, sleep], 1.0, true)),
        (PetState::React, anim(&[squash, squash, base], 6.0, false)),
        (PetState::Fall, anim(&[surprised], 1.0, true)),
        (PetState::Dragged, anim(&[surprised], 1.0, true)),
        (PetState::Land, anim(&[squash], 1.0, false)),
        (PetState::Dizzy, anim(&[dizzy], 1.0, true)),
    ];
    let mut animations = Vec::new();
    let mut present = Vec::new();
    for (state, animation) in specs {
        present.push((state, animations.len()));
        animations.push(animation);
    }
    let (animations, slots) = with_fallback(animations, &present);
    SpriteSet {
        id: None,
        frame_width: SIZE,
        frame_height: SIZE,
        scale: 3.0,
        pixel_art: true,
        facing: Facing::Right,
        anchor: (f64::from(SIZE) / 2.0, f64::from(SIZE)),
        mouth_y: None,
        animations,
        slots,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec(frames: Option<u32>, start: u32, row: u32) -> AnimationSpec {
        AnimationSpec { image: "a.png".into(), frames, fps: 8.0, start, row, looped: true }
    }

    #[test]
    fn pet_json_dung_va_mac_dinh() {
        let manifest = parse_manifest(&json!({
            "frameWidth": 32, "frameHeight": 32,
            "animations": { "idle": { "image": "./idle.png", "fps": 4 } }
        }))
        .unwrap();
        assert_eq!(manifest.name, "pet");
        assert_eq!(manifest.scale, 2.0);
        assert!(manifest.pixel_art);
        assert_eq!(manifest.facing, Facing::Right);
        assert_eq!(manifest.anchor, (16.0, 32.0));
        assert_eq!(manifest.animations[0].1.image, "idle.png");
        assert!(manifest.animations[0].1.looped);
    }

    #[test]
    fn pet_json_sai_thi_bao_ro_truong() {
        let err = |v: Value| parse_manifest(&v).unwrap_err();
        assert!(err(json!([])).starts_with("pet.json"));
        assert!(err(json!({ "frameWidth": 0, "frameHeight": 32, "animations": {} })).starts_with("frameWidth"));
        assert!(err(json!({ "frameWidth": 32, "frameHeight": 32, "animations": {} })).starts_with("animations.idle"));
        let bad = json!({ "frameWidth": 32, "frameHeight": 32, "animations": { "fly": { "image": "a.png", "fps": 4 } } });
        assert!(err(bad).starts_with("animations.fly"));
        let up = json!({ "frameWidth": 32, "frameHeight": 32, "animations": { "idle": { "image": "../a.png", "fps": 4 } } });
        assert!(err(up).starts_with("animations.idle.image"));
        let fps = json!({ "frameWidth": 32, "frameHeight": 32, "animations": { "idle": { "image": "a.png", "fps": 0 } } });
        assert!(err(fps).starts_with("animations.idle.fps"));
    }

    #[test]
    fn cat_frame_trai_sang_phai_het_hang_xuong_hang() {
        let rects = frame_rects(&spec(Some(3), 1, 0), 10, 10, 20, 20).unwrap();
        assert_eq!(rects, [FrameRect { x: 10, y: 0 }, FrameRect { x: 0, y: 10 }, FrameRect { x: 10, y: 10 }]);
        assert_eq!(frame_rects(&spec(None, 0, 1), 10, 10, 30, 20).unwrap().len(), 3);
        assert!(frame_rects(&spec(Some(5), 0, 0), 10, 10, 20, 20).is_err());
        assert!(frame_rects(&spec(None, 0, 2), 10, 10, 20, 20).is_err());
    }

    #[test]
    fn frame_theo_thoi_gian() {
        assert_eq!(frame_index(4, 8.0, true, 0.6), 0);
        assert_eq!(frame_index(4, 8.0, false, 0.6), 3);
        assert_eq!(frame_index(4, 8.0, true, -1.0), 0);
    }

    #[test]
    fn moi_pack_co_san_nap_duoc_du_animation() {
        assert!(!PACKS.is_empty());
        for info in PACKS.iter() {
            let sprite = load_pack(info).unwrap_or_else(|e| panic!("{}: {e}", info.id));
            for state in STATES {
                assert!(sprite.animation(state).len() > 0, "{} {}", info.id, state.name());
            }
            let head = head_of(&sprite.animation(PetState::Idle).masks[0]);
            assert!(head.top > 0.0 && head.top < sprite.anchor.1);
        }
    }

    #[test]
    fn pack_khong_con_thi_dung_pack_dau_tien() {
        let first = PACKS[0].id.clone();
        assert_eq!(resolve_pack(Some("khong-co")), Some(first.clone()));
        assert_eq!(resolve_packs(&[]), [Some(first.clone())]);
        let second = PACKS[1].id.clone();
        let chosen = resolve_packs(&[second.clone(), second.clone(), "khong-co".into(), first.clone()]);
        assert_eq!(chosen, [Some(second), Some(first)]);
    }

    #[test]
    fn pet_tam_du_animation_ngu_choang_dung_yen() {
        let sprite = placeholder();
        assert_eq!(sprite.animation(PetState::Climb).len(), sprite.animation(PetState::Dragged).len());
        assert_eq!(sprite.animation(PetState::Sleep).len(), 2);
        assert!(sprite.animation(PetState::Idle).masks[0].has(8, 10));
        assert!(!sprite.animation(PetState::Idle).masks[0].has(0, 0));
    }
}
