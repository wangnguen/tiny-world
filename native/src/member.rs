//! Một con trên màn hình: sprite pack, phần vẽ, thời tiết quanh nó, đồ diễn của Long. Phần mô phỏng nằm trong
//! `World`, gọi theo `uid`.

use crate::gfx::{Gfx, Group};
use crate::long::{LongKit, LONG_PACK};
use crate::view::{PetView, Prepared, ViewLayers};
use crate::weather::WeatherEffect;
use anyhow::Result;

pub struct Member {
    /// Tên thư mục pack, `None` là pet tạm.
    pub pack: Option<String>,
    /// `Pet::uid` trong `World`.
    pub uid: u32,
    pub view: PetView,
    pub effect: WeatherEffect,
    /// Đồ diễn riêng của Long; con khác không có.
    pub kit: Option<LongKit>,
    group: Group,
}

impl Member {
    /// Tạo các lớp theo thứ tự từ dưới lên: sao choáng phía sau, aura (Long), pet, sao phía trước, thời tiết,
    /// cinematic và vệt tốc biến (Long), bóng thoại. Gán `uid` sau khi đã thêm pet vào `World`.
    pub fn new(gfx: &Gfx, pack: Option<String>, prepared: Prepared) -> Result<Self> {
        let group = gfx.group()?;
        let layer = || gfx.layer(Some(&group));
        let long = pack.as_deref() == Some(LONG_PACK);
        let stars_back = layer()?;
        let aura = long.then(layer).transpose()?;
        let sprite_layer = layer()?;
        let stars_front = layer()?;
        let weather = layer()?;
        let action = long.then(layer).transpose()?;
        let teleport = long.then(layer).transpose()?;
        let bubble = layer()?;
        let layers = ViewLayers { stars_back, sprite: sprite_layer, stars_front, bubble };
        let view = PetView::new(gfx, layers, prepared)?;
        let effect = WeatherEffect::new(gfx, weather)?;
        let kit = match (aura, action, teleport) {
            (Some(aura), Some(action), Some(teleport)) => Some(LongKit::new(gfx, aura, action, teleport)?),
            _ => None,
        };
        Ok(Self { pack, uid: 0, view, effect, kit, group })
    }

    /// Vẽ con này lên trên các con khác (vừa bị bấm, bị kéo).
    pub fn raise(&self, gfx: &Gfx) -> Result<()> {
        gfx.raise(&self.group)
    }

    /// Bỏ khỏi màn hình (bớt nhân vật trong Settings).
    pub fn destroy(&self, gfx: &Gfx) {
        gfx.remove(&self.group);
    }
}
