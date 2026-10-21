//! Đồ diễn riêng của Long (`aura.ts`, `longAction.ts`, `longCombo.ts`): aura sau lưng, sprite cinematic (biến hình,
//! quật đuôi), vệt tốc biến, và combo quật đuôi (Shift + click vào Long khi có con khác). Chỉ đổi chỗ đứng và state
//! trong mô phỏng (`hold`, `blink_to`, `vanish`), không đụng tới cài đặt hay dữ liệu lưu. Ảnh effect nạp lúc tạo, bỏ
//! khi Long bị bỏ khỏi màn hình.

use crate::assets;
use crate::gfx::{mirrored, rect, Gfx, Layer};
use crate::random::random;
use crate::sim::pet::{Facing, PetState};
use crate::sim::Point;
use crate::sprites::premultiplied_bgra;
use anyhow::Result;
use windows::Win32::Graphics::Direct2D::{ID2D1Bitmap1, ID2D1StrokeStyle, D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC};
use windows_numerics::Matrix3x2;

/// Pack của Long.
pub const LONG_PACK: &str = "c-long";

/// Tỉ lệ ngang/dọc của mọi frame effect của Long (sheet nguồn 4 frame 543×724 xếp ngang).
const FRAME_ASPECT: f64 = 3.0 / 4.0;

/// Aura chạy chậm hơn sprite một chút để vẫn sáng rõ nhưng không tốn CPU.
const AURA_FRAME_MS: f64 = 100.0;
const AURA_MIN_MS: f64 = 5_000.0;
const AURA_MAX_MS: f64 = 7_000.0;
const ACTION_FRAME_MS: f64 = 1000.0 / 7.0;

/// Thời lượng từng đoạn của combo (ms).
const TRANSFORM_MS: f64 = 1_100.0;
const BLINK_MS: f64 = 180.0;
const SETTLE_MS: f64 = 100.0;
const SWIPE_MS: f64 = 650.0;
const STUN_MS: f64 = 550.0;
/// Mục tiêu đang bận (bị kéo, đang rơi, leo, nhảy, chat) thì chờ tối đa chừng này rồi bỏ qua nó.
const WAIT_MS: f64 = 2_000.0;
/// Bị quật trúng thì biến mất chừng này giây.
const VANISH_SECONDS: f64 = 5.0;
/// Long hiện ra cách mục tiêu chừng này lần tổng bề ngang hai con (giữa hai điểm chân).
const REACH: f64 = 0.42;
/// Mục tiêu đứng yên tới lúc trúng đòn, dư nửa giây cho vòng lặp vẽ trễ.
const TARGET_HOLD: f64 = (2.0 * BLINK_MS + SETTLE_MS + SWIPE_MS) / 1000.0 + 0.5;
/// Long đứng yên từ lúc hiện ra cạnh mục tiêu tới hết lúc mục tiêu choáng.
const LONG_HOLD: f64 = (BLINK_MS + SETTLE_MS + SWIPE_MS + STUN_MS) / 1000.0 + 0.5;

/// Ảnh frame của một effect (`assets/effects/<dir>/frame-*.webp`), theo tên.
fn frames(gfx: &Gfx, dir: &str) -> Vec<ID2D1Bitmap1> {
    assets::list(dir)
        .filter_map(|(name, bytes)| {
            let image = image::load_from_memory(bytes).map_err(|e| log!("Effect {name} lỗi: {e}")).ok()?.into_rgba8();
            let (width, height) = image.dimensions();
            gfx.bitmap(width, height, &premultiplied_bgra(image)).map_err(|e| log!("Effect {name} lỗi: {e}")).ok()
        })
        .collect()
}

/// Vẽ `bitmap` vừa khít lớp, lật ngang nếu `flip`.
fn paint(layer: &Layer, bitmap: &ID2D1Bitmap1, flip: bool) -> Result<()> {
    let (w, h) = (layer.width as f32, layer.height as f32);
    layer.draw(|canvas| {
        canvas.transform(&if flip { mirrored(0.0, 0.0, w) } else { Matrix3x2::identity() });
        unsafe {
            canvas.dc.DrawBitmap(bitmap, Some(&rect(0.0, 0.0, w, h)), 1.0, D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC, None, None)
        };
        Ok(())
    })
}

/// Aura sau lưng Long: bật 5–7 giây mỗi lần Shift + click.
pub struct Aura {
    layer: Layer,
    frames: Vec<ID2D1Bitmap1>,
    until: f64,
    shown: bool,
    last: Option<usize>,
}

impl Aura {
    fn new(gfx: &Gfx, mut layer: Layer) -> Result<Self> {
        layer.set_visible(false)?;
        Ok(Self { layer, frames: frames(gfx, "effects/long-aura"), until: 0.0, shown: false, last: None })
    }

    pub fn animating(&self, now: f64) -> bool {
        now < self.until
    }

    /// Vòng lặp phải vẽ lại trong vòng chừng này ms; vô cùng nếu aura đang tắt.
    pub fn frame_ms(&self, now: f64) -> f64 {
        if self.animating(now) {
            AURA_FRAME_MS
        } else {
            f64::INFINITY
        }
    }

    /// Bật aura trong một khoảng ngẫu nhiên 5–7 giây; kích lại thì bắt đầu một lượt mới.
    pub fn activate(&mut self, now: f64) {
        self.until = now + AURA_MIN_MS + random() * (AURA_MAX_MS - AURA_MIN_MS);
        self.last = None;
    }

    pub fn update(&mut self, gfx: &Gfx, foot: Point, size: (f64, f64), now: f64, visible: bool, px: f64) -> Result<()> {
        let show = self.animating(now) && visible && !self.frames.is_empty();
        if show != self.shown {
            self.shown = show;
            self.layer.set_visible(show)?;
        }
        if !show {
            return Ok(());
        }
        let height = (size.1 * 1.8).round();
        // Long có thể dùng frame không vuông sau khi biến hình; giữ aura ít nhất phủ rộng thân.
        let width = (height * FRAME_ASPECT).round().max((size.0 * 1.35).round());
        if self.layer.resize(gfx, (width * px).round() as u32, (height * px).round() as u32)? {
            self.last = None;
        }
        let left = (foot.x - width / 2.0).round();
        let top = (foot.y - height * 0.86).round();
        self.layer.move_to((left * px).round() as f32, (top * px).round() as f32)?;
        let frame = (now / AURA_FRAME_MS).floor() as usize % self.frames.len();
        if self.last != Some(frame) {
            self.last = Some(frame);
            paint(&self.layer, &self.frames[frame], false)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Transform,
    TailSwipe,
}

/// Sprite cinematic của Long, vẽ thay sprite thường trong lúc biến hình hoặc quật đuôi.
pub struct ActionEffect {
    layer: Layer,
    transform: Vec<ID2D1Bitmap1>,
    swipe: Vec<ID2D1Bitmap1>,
    action: Option<Action>,
    /// Hướng đã chốt lúc `play`: frame vẽ quay sang phải, -1 thì lật.
    facing: Facing,
    started: f64,
    last: Option<usize>,
}

impl ActionEffect {
    fn new(gfx: &Gfx, mut layer: Layer) -> Result<Self> {
        layer.set_visible(false)?;
        Ok(Self {
            layer,
            transform: frames(gfx, "effects/long-actions/transform"),
            swipe: frames(gfx, "effects/long-actions/tail-swipe"),
            action: None,
            facing: 1,
            started: 0.0,
            last: None,
        })
    }

    /// Diễn `action` quay về phía `facing`. Hướng chốt luôn lúc này: trong lúc diễn sim có thể quay đầu Long, đòn vẫn
    /// phải đánh về phía mục tiêu.
    pub fn play(&mut self, action: Action, now: f64, facing: Facing) -> Result<()> {
        self.action = Some(action);
        self.facing = facing;
        self.started = now;
        self.last = None;
        self.layer.set_visible(true)
    }

    pub fn stop(&mut self) -> Result<()> {
        self.action = None;
        self.layer.set_visible(false)
    }

    pub fn update(&mut self, gfx: &Gfx, foot: Point, size: (f64, f64), now: f64, px: f64) -> Result<()> {
        let Some(action) = self.action else {
            return Ok(());
        };
        let images = if action == Action::Transform { &self.transform } else { &self.swipe };
        if images.is_empty() {
            return Ok(());
        }
        let height = (size.1 * 2.0).max(size.0 * 1.7).round();
        let width = (height * FRAME_ASPECT).round();
        if self.layer.resize(gfx, (width * px).round() as u32, (height * px).round() as u32)? {
            self.last = None;
        }
        let left = (foot.x - width / 2.0).round();
        let top = (foot.y - height * 0.86).round();
        self.layer.move_to((left * px).round() as f32, (top * px).round() as f32)?;
        let frame = (((now - self.started) / ACTION_FRAME_MS).floor().max(0.0) as usize).min(images.len() - 1);
        if self.last != Some(frame) {
            self.last = Some(frame);
            paint(&self.layer, &images[frame], self.facing < 0)?;
        }
        Ok(())
    }
}

/// Vệt cam ngắn lúc Long tốc biến; `reverse` lật hướng vệt khi tái xuất.
pub struct TeleportStreak {
    layer: Layer,
    stroke: ID2D1StrokeStyle,
    until: f64,
    started: f64,
    direction: f64,
    /// Cỡ (CSS pixel).
    width: f64,
    height: f64,
}

impl TeleportStreak {
    fn new(gfx: &Gfx, mut layer: Layer) -> Result<Self> {
        layer.set_visible(false)?;
        let stroke = gfx.round_stroke()?;
        Ok(Self { layer, stroke, until: 0.0, started: 0.0, direction: 1.0, width: 0.0, height: 0.0 })
    }

    /// Vệt chạy hết trong `duration` ms, đầu vệt tại điểm chân `point`.
    #[allow(clippy::too_many_arguments)]
    pub fn play(
        &mut self,
        gfx: &Gfx,
        point: Point,
        size: (f64, f64),
        direction: Facing,
        reverse: bool,
        now: f64,
        duration: f64,
        px: f64,
    ) -> Result<()> {
        let direction = f64::from(direction);
        self.direction = if reverse { -direction } else { direction };
        self.width = (size.0 * 1.8).round();
        self.height = (size.1 * 1.15).round();
        self.layer.resize(gfx, (self.width * px).round() as u32, (self.height * px).round() as u32)?;
        let left = (point.x - self.width / 2.0).round();
        let top = (point.y - self.height * 0.8).round();
        self.layer.move_to((left * px).round() as f32, (top * px).round() as f32)?;
        self.started = now;
        self.until = now + duration;
        self.layer.set_visible(true)
    }

    pub fn animating(&self, now: f64) -> bool {
        now < self.until
    }

    pub fn update(&mut self, now: f64, px: f64) -> Result<()> {
        if now >= self.until {
            return self.layer.set_visible(false);
        }
        let t = (now - self.started) / (self.until - self.started);
        let (w, h, dir, stroke) = (self.width, self.height, self.direction, &self.stroke);
        self.layer.draw(|canvas| {
            canvas.transform(&crate::gfx::scale(px as f32, px as f32));
            for i in 0..6 {
                let i = f64::from(i);
                let y = h * (0.18 + i * 0.12);
                let length = w * (0.3 + i * 0.09);
                let head = w * 0.5 + dir * w * (0.14 + t * 0.2);
                let color = crate::gfx::Color::rgba(255, (150.0 + i * 12.0) as u8, 35, (1.0 - t) * (0.8 - i * 0.07));
                let brush = canvas.brush(color)?;
                let width = (h * (0.03 - i * 0.002)).max(1.0) as f32;
                canvas.styled_line(
                    ((head - dir * length) as f32, y as f32),
                    (head as f32, (y - h * 0.05) as f32),
                    &brush,
                    width,
                    stroke,
                );
            }
            Ok(())
        })
    }
}

/// Đồ diễn riêng của Long. Lớp aura nằm sau sprite, cinematic và vệt tốc biến nằm trên.
pub struct LongKit {
    pub aura: Aura,
    pub action: ActionEffect,
    pub teleport: TeleportStreak,
}

impl LongKit {
    pub fn new(gfx: &Gfx, aura: Layer, action: Layer, teleport: Layer) -> Result<Self> {
        Ok(Self {
            aura: Aura::new(gfx, aura)?,
            action: ActionEffect::new(gfx, action)?,
            teleport: TeleportStreak::new(gfx, teleport)?,
        })
    }

    /// Mỗi lần vẽ: cinematic đi theo chân Long, vệt tốc biến chạy tiếp. Aura do `Ambience` đặt cùng thời tiết.
    pub fn update(&mut self, gfx: &Gfx, foot: Point, size: (f64, f64), now: f64, px: f64) -> Result<()> {
        self.action.update(gfx, foot, size, now, px)?;
        self.teleport.update(now, px)
    }

    /// Vòng lặp phải vẽ lại trong vòng chừng này ms (aura, vệt tốc biến); vô cùng nếu không cần.
    pub fn frame_ms(&self, now: f64) -> f64 {
        let streak = if self.teleport.animating(now) { 0.0 } else { f64::INFINITY };
        self.aura.frame_ms(now).min(streak)
    }
}

/// Những gì combo cần ở overlay: pet theo `uid`, cỡ đang vẽ, ẩn sprite, đồ diễn của Long.
pub trait Stage {
    fn state(&self, pet: u32) -> Option<PetState>;
    fn position(&self, pet: u32) -> Option<Point>;
    fn facing(&self, pet: u32) -> Facing;
    fn set_facing(&mut self, pet: u32, facing: Facing);
    fn hold(&mut self, pet: u32, facing: Facing, seconds: f64) -> bool;
    fn blink_to(&mut self, pet: u32, target: u32, x: f64, seconds: f64);
    fn startle(&mut self, pet: u32);
    fn vanish(&mut self, pet: u32, seconds: f64);
    fn size(&self, pet: u32) -> (f64, f64);
    fn set_covered(&mut self, pet: u32, covered: bool);
    fn play(&mut self, action: Action, now: f64, facing: Facing);
    fn stop_action(&mut self);
    fn teleport(&mut self, at: Point, size: (f64, f64), direction: Facing, reverse: bool, now: f64, duration: f64);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Transform,
    Seek,
    BlinkOut,
    BlinkIn,
    Settle,
    Swipe,
    Stun,
    Done,
}

/// Combo của Long: biến hình, rồi lần lượt tốc biến tới cạnh từng con khác quật đuôi một cái; con bị trúng giật mình
/// rồi biến mất `VANISH_SECONDS` giây. Gọi `update` mỗi lần vẽ.
#[derive(Debug)]
pub struct LongCombo {
    long: u32,
    phase: Phase,
    until: f64,
    target: Option<u32>,
    /// Phía Long đánh tới mục tiêu hiện tại (1: mục tiêu ở bên phải).
    facing: Facing,
    queue: Vec<u32>,
}

impl LongCombo {
    pub fn start(stage: &mut impl Stage, long: u32, targets: Vec<u32>, now: f64) -> Self {
        let facing = stage.facing(long);
        // Đang rảnh thì đứng yên biến hình tại chỗ; đang rơi, leo, nhảy thì cứ thế, lúc tốc biến sẽ đứng hẳn lên.
        stage.hold(long, facing, TRANSFORM_MS / 1000.0 + 0.5);
        stage.set_covered(long, true);
        stage.play(Action::Transform, now, facing);
        Self { long, phase: Phase::Transform, until: now + TRANSFORM_MS, target: None, facing, queue: targets }
    }

    pub fn done(&self) -> bool {
        self.phase == Phase::Done
    }

    /// Con `pet` bị bỏ khỏi màn hình (đổi nhân vật trong Settings): bỏ chính Long thì dừng hẳn, bỏ mục tiêu thì bỏ qua.
    pub fn drop(&mut self, stage: &mut impl Stage, pet: u32) {
        if pet == self.long {
            self.stop(stage);
        } else {
            self.forget(pet);
        }
    }

    /// Dừng hẳn: Long hiện lại như thường.
    pub fn stop(&mut self, stage: &mut impl Stage) {
        if self.done() {
            return;
        }
        stage.stop_action();
        stage.set_covered(self.long, false);
        self.target = None;
        self.phase = Phase::Done;
    }

    pub fn update(&mut self, stage: &mut impl Stage, now: f64) {
        if self.done() {
            return;
        }
        let long = self.long;
        // Người dùng nhấc Long lên giữa chừng: thôi diễn, để Long bị kéo như thường.
        if stage.state(long) == Some(PetState::Dragged) {
            self.stop(stage);
            return;
        }
        let target = self.target;
        if self.phase != Phase::Transform && self.phase != Phase::Seek {
            // Mục tiêu bị bỏ khỏi màn hình hoặc bị nhấc lên giữa chừng: bỏ qua, sang con tiếp theo.
            let Some(target) = target.filter(|&t| stage.state(t).is_some_and(|s| s != PetState::Dragged)) else {
                self.miss(stage, now);
                return;
            };
            // Hai con luôn quay mặt vào nhau tới hết đòn: đứng yên thì sim quay đầu nhìn theo con trỏ.
            stage.set_facing(long, self.facing);
            stage.set_facing(target, -self.facing);
        }
        if self.phase == Phase::Seek {
            self.seek(stage, now);
            return;
        }
        if now < self.until {
            return;
        }
        if self.phase == Phase::Transform {
            stage.stop_action();
            stage.set_covered(long, false);
            self.wait(now);
            self.seek(stage, now);
            return;
        }
        let Some(target) = target else {
            return;
        };
        match self.phase {
            Phase::BlinkOut => {
                let reach = (stage.size(long).0 + stage.size(target).0) * REACH;
                let x = stage.position(target).map_or(0.0, |p| p.x) - f64::from(self.facing) * reach;
                stage.blink_to(long, target, x, LONG_HOLD);
                // Kẹp vào chỗ đứng (sát mép màn hình, mép cửa sổ hẹp) có khi phải đứng phía bên kia.
                self.facing = stage.facing(long);
                if let Some(at) = stage.position(long) {
                    stage.teleport(at, stage.size(long), self.facing, true, now, BLINK_MS);
                }
                self.next(Phase::BlinkIn, now + BLINK_MS);
            }
            Phase::BlinkIn => {
                // Vệt xuất hiện chạy hết rồi Long mới hiện rõ một chút trước khi quật.
                stage.set_covered(long, false);
                self.next(Phase::Settle, now + SETTLE_MS);
            }
            Phase::Settle => {
                stage.set_covered(long, true);
                stage.play(Action::TailSwipe, now, self.facing);
                self.next(Phase::Swipe, now + SWIPE_MS);
            }
            Phase::Swipe => {
                stage.stop_action();
                stage.set_covered(long, false);
                stage.startle(target);
                self.next(Phase::Stun, now + STUN_MS);
            }
            Phase::Stun => {
                stage.vanish(target, VANISH_SECONDS);
                self.forget(target);
                self.wait(now);
                self.seek(stage, now);
            }
            _ => {}
        }
    }

    /// Tìm mục tiêu: con đầu hàng đứng yên được thì tốc biến tới nó; đang bận thì chờ, chờ quá `WAIT_MS` thì bỏ qua
    /// con đó. Hết con thì xong.
    fn seek(&mut self, stage: &mut impl Stage, now: f64) {
        let long = self.long;
        loop {
            let Some(&target) = self.queue.first() else {
                self.stop(stage);
                return;
            };
            let (Some(at), Some(here)) = (stage.position(target), stage.position(long)) else {
                self.queue.remove(0);
                continue;
            };
            let facing: Facing = if at.x >= here.x { 1 } else { -1 };
            if stage.hold(target, -facing, TARGET_HOLD) {
                self.target = Some(target);
                self.facing = facing;
                // Đứng yên ở chỗ cũ trong lúc vệt biến đi chạy.
                stage.hold(long, facing, BLINK_MS / 1000.0 + 0.5);
                stage.set_covered(long, true);
                if let Some(at) = stage.position(long) {
                    stage.teleport(at, stage.size(long), facing, false, now, BLINK_MS);
                }
                self.next(Phase::BlinkOut, now + BLINK_MS);
                return;
            }
            if now < self.until {
                return;
            }
            self.queue.remove(0);
            self.until = now + WAIT_MS;
        }
    }

    /// Đòn đang dở mà mục tiêu không còn: Long hiện lại, tìm con tiếp theo.
    fn miss(&mut self, stage: &mut impl Stage, now: f64) {
        stage.stop_action();
        stage.set_covered(self.long, false);
        if let Some(target) = self.target {
            self.forget(target);
        }
        self.wait(now);
        self.seek(stage, now);
    }

    /// Bỏ `pet` khỏi hàng chờ, thôi nhắm vào nó.
    fn forget(&mut self, pet: u32) {
        self.queue.retain(|&p| p != pet);
        if self.target == Some(pet) {
            self.target = None;
        }
    }

    /// Bắt đầu chờ mục tiêu tiếp theo rảnh.
    fn wait(&mut self, now: f64) {
        self.phase = Phase::Seek;
        self.until = now + WAIT_MS;
    }

    fn next(&mut self, phase: Phase, until: f64) {
        self.phase = phase;
        self.until = until;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Một con đứng ở `x`; `busy` thì không chịu `hold` như con đang bị kéo, rơi, leo.
    #[derive(Debug, Clone)]
    struct Fighter {
        x: f64,
        y: f64,
        facing: Facing,
        state: PetState,
        vanished: f64,
        startled: u32,
        busy: bool,
        covered: bool,
    }

    #[derive(Default)]
    struct FakeStage {
        pets: HashMap<u32, Fighter>,
        log: Vec<String>,
        blinks: Vec<(bool, Facing, f64)>,
    }

    impl FakeStage {
        fn add(&mut self, uid: u32, x: f64, y: f64) {
            let fighter = Fighter {
                x,
                y,
                facing: 1,
                state: PetState::Idle,
                vanished: 0.0,
                startled: 0,
                busy: false,
                covered: false,
            };
            self.pets.insert(uid, fighter);
        }

        fn pet(&mut self, uid: u32) -> &mut Fighter {
            self.pets.get_mut(&uid).unwrap()
        }
    }

    impl Stage for FakeStage {
        fn state(&self, pet: u32) -> Option<PetState> {
            self.pets.get(&pet).map(|p| p.state)
        }
        fn position(&self, pet: u32) -> Option<Point> {
            self.pets.get(&pet).map(|p| Point { x: p.x, y: p.y })
        }
        fn facing(&self, pet: u32) -> Facing {
            self.pets.get(&pet).map_or(1, |p| p.facing)
        }
        fn set_facing(&mut self, pet: u32, facing: Facing) {
            if let Some(p) = self.pets.get_mut(&pet) {
                p.facing = facing;
            }
        }
        fn hold(&mut self, pet: u32, facing: Facing, _: f64) -> bool {
            let Some(p) = self.pets.get_mut(&pet) else { return false };
            if p.busy || p.state == PetState::Dragged {
                return false;
            }
            p.facing = facing;
            p.state = PetState::Idle;
            true
        }
        fn blink_to(&mut self, pet: u32, target: u32, x: f64, _: f64) {
            let (tx, ty) = (self.pets[&target].x, self.pets[&target].y);
            let p = self.pet(pet);
            p.x = x;
            p.y = ty;
            p.state = PetState::Idle;
            if (tx - x).abs() > 1.0 {
                p.facing = if tx > x { 1 } else { -1 };
            }
        }
        fn startle(&mut self, pet: u32) {
            let p = self.pet(pet);
            p.startled += 1;
            p.state = PetState::React;
        }
        fn vanish(&mut self, pet: u32, seconds: f64) {
            self.pet(pet).vanished = seconds;
        }
        fn size(&self, _: u32) -> (f64, f64) {
            (96.0, 96.0)
        }
        fn set_covered(&mut self, pet: u32, covered: bool) {
            if let Some(p) = self.pets.get_mut(&pet) {
                p.covered = covered;
            }
        }
        fn play(&mut self, action: Action, _: f64, facing: Facing) {
            self.log.push(format!("play {action:?} {facing}"));
        }
        fn stop_action(&mut self) {
            self.log.push("stop".into());
        }
        fn teleport(&mut self, at: Point, _: (f64, f64), direction: Facing, reverse: bool, _: f64, _: f64) {
            self.blinks.push((reverse, direction, at.x));
        }
    }

    /// Chạy combo mỗi 10 ms từ `from` tới `to` (hoặc tới lúc xong); `each(stage, t)` chạy trước mỗi lần.
    fn run(
        combo: &mut LongCombo,
        stage: &mut FakeStage,
        from: f64,
        to: f64,
        mut each: impl FnMut(&mut FakeStage, f64),
    ) -> f64 {
        let mut t = from;
        while t <= to && !combo.done() {
            each(stage, t);
            combo.update(stage, t);
            t += 10.0;
        }
        t
    }

    fn plays(stage: &FakeStage) -> Vec<&str> {
        stage.log.iter().filter(|l| l.starts_with("play")).map(String::as_str).collect()
    }

    #[test]
    fn bien_hinh_roi_lan_luot_toc_bien_toi_quat_tung_con() {
        let mut stage = FakeStage::default();
        stage.add(1, 100.0, 700.0);
        stage.add(2, 400.0, 700.0);
        stage.add(3, 900.0, 400.0);
        let mut combo = LongCombo::start(&mut stage, 1, vec![2, 3], 0.0);
        assert!(stage.pets[&1].covered);
        run(&mut combo, &mut stage, 0.0, TRANSFORM_MS - 10.0, |_, _| {});
        assert_eq!(stage.pets[&2].startled, 0);
        run(&mut combo, &mut stage, TRANSFORM_MS, 20_000.0, |_, _| {});
        assert!(combo.done());
        assert!(!stage.pets[&1].covered);
        for target in [2, 3] {
            assert_eq!(stage.pets[&target].startled, 1);
            assert_eq!(stage.pets[&target].vanished, VANISH_SECONDS);
        }
        assert_eq!(plays(&stage), ["play Transform 1", "play TailSwipe 1", "play TailSwipe 1"]);
        // Đi từ chỗ Long đứng, hiện ra cạnh mục tiêu, trên chỗ đứng của chính mục tiêu.
        let ways: Vec<bool> = stage.blinks.iter().map(|b| b.0).collect();
        assert_eq!(ways, [false, true, false, true]);
        assert_eq!(stage.blinks[0].2, 100.0);
        assert!((stage.blinks[1].2 - (400.0 - 192.0 * REACH)).abs() < 1e-9);
        assert_eq!(stage.pets[&1].y, 400.0);
    }

    #[test]
    fn don_quat_van_huong_ve_muc_tieu_du_sim_quay_long_theo_con_tro() {
        let mut stage = FakeStage::default();
        stage.add(1, 800.0, 700.0);
        stage.add(2, 300.0, 700.0);
        let mut combo = LongCombo::start(&mut stage, 1, vec![2], 0.0);
        // Pet đứng yên nhìn theo con trỏ; người dùng vừa bấm chỗ Long đứng cũ (bên phải).
        run(&mut combo, &mut stage, 0.0, 20_000.0, |stage, _| {
            stage.pet(1).facing = 1;
            stage.pet(2).facing = 1;
        });
        assert_eq!(plays(&stage)[1], "play TailSwipe -1");
        assert_eq!(stage.pets[&2].startled, 1);
    }

    #[test]
    fn muc_tieu_dang_ban_thi_cho_roi_bo_qua_con_sau_van_trung() {
        let mut stage = FakeStage::default();
        stage.add(1, 100.0, 700.0);
        stage.add(2, 400.0, 700.0);
        stage.add(3, 700.0, 700.0);
        stage.pet(2).busy = true;
        let mut combo = LongCombo::start(&mut stage, 1, vec![2, 3], 0.0);
        let end = run(&mut combo, &mut stage, 0.0, 20_000.0, |_, _| {});
        assert_eq!(stage.pets[&2].vanished, 0.0);
        assert_eq!(stage.pets[&2].startled, 0);
        assert_eq!(stage.pets[&3].vanished, VANISH_SECONDS);
        assert!(end > TRANSFORM_MS + WAIT_MS);
    }

    #[test]
    fn muc_tieu_ranh_ra_trong_luc_cho_thi_van_trung() {
        let mut stage = FakeStage::default();
        stage.add(1, 100.0, 700.0);
        stage.add(2, 400.0, 700.0);
        stage.pet(2).busy = true;
        let mut combo = LongCombo::start(&mut stage, 1, vec![2], 0.0);
        run(&mut combo, &mut stage, 0.0, 20_000.0, |stage, t| {
            if t == TRANSFORM_MS + 500.0 {
                stage.pet(2).busy = false;
            }
        });
        assert_eq!(stage.pets[&2].vanished, VANISH_SECONDS);
    }

    #[test]
    fn nhac_long_len_giua_chung_thi_dung_va_hien_lai_long() {
        let mut stage = FakeStage::default();
        stage.add(1, 100.0, 700.0);
        stage.add(2, 400.0, 700.0);
        let mut combo = LongCombo::start(&mut stage, 1, vec![2], 0.0);
        run(&mut combo, &mut stage, 0.0, TRANSFORM_MS + 300.0, |_, _| {});
        stage.pet(1).state = PetState::Dragged;
        combo.update(&mut stage, TRANSFORM_MS + 310.0);
        assert!(combo.done());
        assert!(!stage.pets[&1].covered);
        assert_eq!(stage.log.last().map(String::as_str), Some("stop"));
        run(&mut combo, &mut stage, TRANSFORM_MS + 320.0, 20_000.0, |_, _| {});
        assert_eq!(stage.pets[&2].vanished, 0.0);
    }

    #[test]
    fn bo_long_khoi_man_hinh_thi_dung_combo() {
        let mut stage = FakeStage::default();
        stage.add(1, 100.0, 700.0);
        stage.add(2, 400.0, 700.0);
        let mut combo = LongCombo::start(&mut stage, 1, vec![2], 0.0);
        combo.drop(&mut stage, 1);
        assert!(combo.done());
        assert!(!stage.pets[&1].covered);
    }

    #[test]
    fn muc_tieu_bi_bo_hay_bi_nhac_len_giua_don_thi_thoat_long_sang_con_tiep() {
        let mut stage = FakeStage::default();
        stage.add(1, 100.0, 700.0);
        stage.add(2, 400.0, 700.0);
        stage.add(3, 600.0, 700.0);
        stage.add(4, 800.0, 700.0);
        let mut combo = LongCombo::start(&mut stage, 1, vec![2, 3, 4], 0.0);
        // Bị bỏ khỏi màn hình trong lúc Long tốc biến tới.
        run(&mut combo, &mut stage, 0.0, TRANSFORM_MS + 100.0, |_, _| {});
        combo.drop(&mut stage, 2);
        stage.pets.remove(&2);
        // Bị người dùng nhấc lên lúc Long đang quật.
        run(&mut combo, &mut stage, TRANSFORM_MS + 110.0, 20_000.0, |stage, _| {
            let long = &stage.pets[&1];
            if long.covered && long.x > 400.0 && long.x < 600.0 {
                stage.pet(3).state = PetState::Dragged;
            }
        });
        assert_eq!(stage.pets[&3].vanished, 0.0);
        assert_eq!(stage.pets[&3].startled, 0);
        assert_eq!(stage.pets[&4].vanished, VANISH_SECONDS);
        assert!(!stage.pets[&1].covered);
    }

    #[test]
    fn khong_con_muc_tieu_thi_xong_va_hien_lai_long() {
        let mut stage = FakeStage::default();
        stage.add(1, 100.0, 700.0);
        stage.add(2, 400.0, 700.0);
        stage.pet(2).busy = true;
        let mut combo = LongCombo::start(&mut stage, 1, vec![2], 0.0);
        run(&mut combo, &mut stage, 0.0, 20_000.0, |_, _| {});
        assert!(combo.done());
        assert!(!stage.pets[&1].covered);
        assert_eq!(stage.pets[&2].startled, 0);
    }
}
