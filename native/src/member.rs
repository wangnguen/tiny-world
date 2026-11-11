//! Một con trên màn hình: sprite pack, phần vẽ, thời tiết quanh nó. Phần mô phỏng nằm trong `World`, gọi theo
//! `uid`.

use crate::gfx::{Gfx, Group};
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
    group: Group,
}

impl Member {
    /// Tạo các lớp theo thứ tự từ dưới lên: sao choáng phía sau, pet, sao phía trước, thời tiết, bóng thoại. Gán
    /// `uid` sau khi đã thêm pet vào `World`.
    pub fn new(gfx: &Gfx, pack: Option<String>, prepared: Prepared) -> Result<Self> {
        let group = gfx.group()?;
        let layer = || gfx.layer(Some(&group));
        let stars_back = layer()?;
        let sprite_layer = layer()?;
        let stars_front = layer()?;
        let weather = layer()?;
        let bubble = layer()?;
        let layers = ViewLayers { stars_back, sprite: sprite_layer, stars_front, bubble };
        let view = PetView::new(gfx, layers, prepared)?;
        let effect = WeatherEffect::new(gfx, weather)?;
        Ok(Self { pack, uid: 0, view, effect, group })
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
