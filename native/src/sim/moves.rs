//! Chỗ pet đứng, chỗ nhảy tới, tường để leo, chỗ bị che: các phép đo trên địa hình cho một pet (`moves.ts`).

use super::math::clamp;
use super::pet::{Pet, PetState};
use super::terrain::{open_span_at, Ledge, Wall};
use super::tuning as t;
use super::world::World;

/// Chỗ pet đang đứng: mặt đất hoặc mép trên một cửa sổ.
#[derive(Debug, Clone, PartialEq)]
pub struct Ground {
    pub y: f64,
    /// `None`: mặt đất (mép trên taskbar).
    pub ledge: Option<Ledge>,
    /// Khoảng điểm chân đi lại được.
    pub from: f64,
    pub to: f64,
}

/// Chỗ nhảy tới: điểm chân lúc đáp, `id` là cửa sổ đáp lên (`None`: mặt đất).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpTarget {
    pub id: Option<isize>,
    pub x: f64,
    pub y: f64,
}

/// Chỗ đáp lên mép cửa sổ: `x` là điểm chân cắt ngang mép.
#[derive(Debug, Clone, PartialEq)]
pub struct Landing {
    pub ledge: Ledge,
    pub x: f64,
}

pub fn ground_of(w: &World, pet: &Pet) -> Ground {
    let bounds = w.terrain.bounds;
    let half = pet.width / 2.0;
    let ledge = match &pet.mount {
        Some(mount) if pet.state() != PetState::Climb => w.terrain.ledge(mount.id),
        _ => None,
    };
    let Some(ledge) = ledge else {
        return Ground { y: bounds.floor, ledge: None, from: bounds.left + half, to: bounds.right - half };
    };
    let margin = pet.width * t::LEDGE_MARGIN;
    Ground {
        y: ledge.y,
        from: (ledge.from + margin).max(bounds.left + half),
        to: (ledge.to - margin).min(bounds.right - half),
        ledge: Some(ledge.clone()),
    }
}

/// Mép đủ xa trần để pet đứng không bị cắt đầu.
pub fn roomy(w: &World, pet: &Pet, ledge: &Ledge) -> bool {
    ledge.y - w.terrain.bounds.top >= pet.height * t::HEADROOM
}

/// Mép cao nhất pet rơi qua trong bước vừa rồi (điểm chân từ (`prev_x`, `prev_y`) tới vị trí hiện tại) mà đáp được.
/// Xét đúng chỗ chân cắt ngang mép chứ không phải chỗ sau bước: bay nhanh thì mỗi bước lệch cả chục pixel, đáp sát
/// đầu mép sẽ trượt.
pub fn landing_ledge(w: &World, pet: &Pet, prev_x: f64, prev_y: f64) -> Option<Landing> {
    let mut best: Option<Landing> = None;
    for ledge in &w.terrain.ledges {
        if Some(ledge.id) == pet.skip_ledge || ledge.y < prev_y || ledge.y > pet.y {
            continue;
        }
        if !roomy(w, pet, ledge) {
            continue;
        }
        let k = if pet.y > prev_y { (ledge.y - prev_y) / (pet.y - prev_y) } else { 0.0 };
        let x = prev_x + k * (pet.x - prev_x);
        if open_span_at(&ledge.open, x).is_none() {
            continue;
        }
        if best.as_ref().is_none_or(|b| ledge.y < b.ledge.y) {
            best = Some(Landing { ledge: ledge.clone(), x });
        }
    }
    best
}

/// Pet đứng trên cửa sổ mà bị cửa sổ khác che gần hết.
pub fn hidden(w: &World, pet: &Pet) -> bool {
    let Some(mount) = &pet.mount else {
        return false;
    };
    w.terrain.covered(mount.id, pet.x, pet.y - 1.0) && w.terrain.covered(mount.id, pet.x, pet.y - pet.height / 2.0)
}

/// Chỗ gần nhất trên mép đang đứng mà không bị che, `None` nếu bị che hết.
pub fn nearest_open_x(pet: &Pet, ground: &Ground) -> Option<f64> {
    let ledge = ground.ledge.as_ref()?;
    let mut best: Option<f64> = None;
    for span in &ledge.open {
        let from = span.from.max(ground.from);
        let to = span.to.min(ground.to);
        if to - from < 8.0 {
            continue;
        }
        // Ra hẳn khỏi chỗ bị che nếu đoạn đủ rộng.
        let pad = (pet.width / 2.0).min((to - from) / 2.0);
        let x = clamp(pet.x, from + pad, to - pad);
        if best.is_none_or(|b| (x - pet.x).abs() < (b - pet.x).abs()) {
            best = Some(x);
        }
    }
    best
}

/// Pet bị che mà không còn chỗ nào để ra: bị che gần hết trên mép bị che hết (bấm vào cửa sổ phóng to nằm dưới cửa
/// sổ đang đứng, kéo cửa sổ khác đè lên), hoặc chỗ đang bám trên cạnh cửa sổ bị che.
pub fn buried(w: &World, pet: &Pet) -> bool {
    let Some(mount) = &pet.mount else {
        return false;
    };
    if pet.state() == PetState::Climb {
        // Xét cột pixel ngoài cạnh như lúc chọn cạnh để leo (`open_wall`), không xét thân pet: cửa sổ nằm trên sát
        // cạnh mà không che cạnh thì pet leo được, không bám vào rồi buông ra mãi.
        let Some(wall) = pet.climbing.and_then(|c| w.terrain.wall(mount.id, c.side)) else {
            return false;
        };
        return open_span_at(&wall.open, clamp(pet.y - pet.height / 2.0, wall.from, wall.to)).is_none();
    }
    hidden(w, pet) && nearest_open_x(pet, &ground_of(w, pet)).is_none()
}

/// Cạnh `wall` không bị che từ mép trên xuống tới `bottom`: leo trên đó không bị khuất.
pub fn open_wall(wall: &Wall, bottom: f64) -> bool {
    let to = bottom.min(wall.to);
    wall.open.iter().any(|s| s.from <= wall.from + 1.0 && s.to >= to - 1.0)
}

/// Các chỗ nhảy tới được: mép cửa sổ khác trong tầm, và mặt đất nếu đang ở trên cửa sổ không quá cao.
pub fn jump_targets(w: &World, pet: &Pet) -> Vec<JumpTarget> {
    let bounds = w.terrain.bounds;
    let y0 = ground_of(w, pet).y;
    let here = pet.mount.as_ref().map(|m| m.id);
    let half = pet.width / 2.0;
    let margin = pet.width * t::LEDGE_MARGIN;
    let mut targets = Vec::new();
    for ledge in &w.terrain.ledges {
        if Some(ledge.id) == here || !roomy(w, pet, ledge) {
            continue;
        }
        let dy = ledge.y - y0;
        if dy < -t::MAX_JUMP_UP || dy > t::MAX_JUMP_DOWN {
            continue;
        }
        let lo = (ledge.from + margin).max(bounds.left + half);
        let hi = (ledge.to - margin).min(bounds.right - half);
        for span in &ledge.open {
            // Đáp hẳn vào phần nhìn thấy, không đáp sát chỗ bị che.
            let from = (span.from + 8.0).max(lo);
            let to = (span.to - 8.0).min(hi);
            if to < from {
                continue;
            }
            let x = clamp(pet.x, from, to);
            let dx = x - pet.x;
            if dx.abs() > t::MAX_JUMP_X || (dx.abs() < 16.0 && dy.abs() < 16.0) {
                continue;
            }
            targets.push(JumpTarget { id: Some(ledge.id), x, y: ledge.y });
        }
    }
    if here.is_some() && bounds.floor - y0 <= t::MAX_JUMP_DOWN {
        let x = clamp(pet.x + f64::from(pet.facing) * 60.0, bounds.left + half, bounds.right - half);
        targets.push(JumpTarget { id: None, x, y: bounds.floor });
    }
    targets
}

/// Cạnh cửa sổ leo lên được từ chỗ đang đứng: đủ cao, đáy chạm tới chân hoặc trong tầm nhảy lên bám, không bị che,
/// trên đỉnh đứng được.
pub fn climb_targets(w: &World, pet: &Pet, ground: &Ground) -> Vec<Wall> {
    let terrain = &w.terrain;
    let margin = pet.width * t::LEDGE_MARGIN;
    let here = pet.mount.as_ref().map(|m| m.id);
    terrain
        .walls()
        .into_iter()
        .filter(|wall| {
            if Some(wall.id) == here {
                return false;
            }
            let (Some(ledge), Some(rect)) = (terrain.ledge(wall.id), terrain.window(wall.id)) else {
                return false;
            };
            if !roomy(w, pet, ledge) || ground.y - ledge.y < t::MIN_CLIMB {
                return false;
            }
            // Đáy cửa sổ cao quá tầm nhảy lên bám thì chịu.
            if rect.y + rect.height + pet.height * t::GRAB_DEPTH < ground.y - t::MAX_GRAB_JUMP {
                return false;
            }
            let contact = wall.x + f64::from(wall.side) * pet.reach;
            if contact < ground.from || contact > ground.to || (contact - pet.x).abs() > t::CLIMB_SEARCH {
                return false;
            }
            if !open_wall(wall, ground.y) {
                return false;
            }
            let top = if wall.side < 0 { ledge.from + margin } else { ledge.to - margin };
            open_span_at(&ledge.open, top).is_some()
        })
        .cloned()
        .collect()
}
