//! Những gì của một pet cần giữ lại khi tắt/mở app (`world.json`). Chỉ giữ thứ còn đúng khi mở lại: pet luôn được
//! đặt lại trên mặt đất, đang rơi hay đang bị kéo thì coi như đứng.

use super::pet::Facing;
use serde::Serialize;
use serde_json::Value;

/// Tăng khi đổi định dạng; file cũ khác version thì bỏ qua, bắt đầu lại từ đầu.
pub const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSnapshot {
    pub id: String,
    /// Điểm chân theo chiều ngang (CSS pixel của overlay).
    pub x: f64,
    pub facing: Facing,
    pub asleep: bool,
    /// Số giây chưa được click/kéo, để lâu không đụng tới thì mở lại vẫn buồn ngủ.
    pub since_interaction: f64,
}

/// Nội dung `world.json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorldSnapshot {
    pub version: u32,
    pub pets: Vec<PetSnapshot>,
}

/// Đọc `world.json`. File sai version hoặc sai kiểu thì trả `None`; pet nào sai thì bỏ riêng pet đó.
pub fn parse_world_snapshot(value: &Value) -> Option<WorldSnapshot> {
    let object = value.as_object()?;
    if object.get("version").and_then(Value::as_u64) != Some(u64::from(SNAPSHOT_VERSION)) {
        return None;
    }
    let pets = object.get("pets")?.as_array()?.iter().filter_map(parse_pet).collect();
    Some(WorldSnapshot { version: SNAPSHOT_VERSION, pets })
}

fn parse_pet(value: &Value) -> Option<PetSnapshot> {
    let object = value.as_object()?;
    let id = object.get("id")?.as_str()?.to_owned();
    let x = object.get("x")?.as_f64().filter(|x| x.is_finite())?;
    let facing = match object.get("facing")?.as_i64()? {
        1 => 1,
        -1 => -1,
        _ => return None,
    };
    let asleep = object.get("asleep")?.as_bool()?;
    let since_interaction = object.get("sinceInteraction")?.as_f64().filter(|s| s.is_finite() && *s >= 0.0)?;
    Some(PetSnapshot { id, x, facing, asleep, since_interaction })
}
