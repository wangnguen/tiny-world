//! Thế giới pet sống (`world.ts`): địa hình, các pet, số ngẫu nhiên chung, con trỏ, ngày/đêm, giới hạn nói chuyện.

use super::math::clamp;
use super::pet::{Facing, Pet, PetMut, PetOptions, PetState};
use super::rng::Rng;
use super::snapshot::{WorldSnapshot, SNAPSHOT_VERSION};
use super::terrain::{Bounds, Terrain};
use super::tuning as t;
use super::{Lang, Point, Rect, Remap, WindowInfo};

/// Hai màn hình lệch nhau chừng này (CSS pixel, do làm tròn DPI) vẫn tính là giáp mép.
const EDGE_SLACK: f64 = 2.0;
/// Các con chạy theo sang màn hình mới: con ở gần mép sang trước, con ở xa sang sau. Xuất hiện ngoài mép cách mép
/// `FOLLOW_SCALE` lần khoảng cách cũ tới mép (tối đa `FOLLOW_MAX` px, chạy vào mất khoảng 3 giây), hai con cách nhau
/// ít nhất `FOLLOW_GAP` lần bề ngang pet để không chồng lên nhau.
const FOLLOW_SCALE: f64 = 0.15;
const FOLLOW_MAX: f64 = 320.0;
const FOLLOW_GAP: f64 = 0.8;

#[derive(Debug, Clone)]
pub struct World {
    pub pets: Vec<Pet>,
    pub rng: Rng,
    /// Hệ số tốc độ đi/chạy của mọi pet (Settings).
    pub speed: f64,
    pub terrain: Terrain,
    /// Vùng làm việc của các màn hình khác, toạ độ của overlay này (`ScreenInfo::neighbors`).
    pub neighbors: Vec<Rect>,
    /// Con trỏ chuột (CSS pixel của overlay), `None` khi chưa biết.
    pub cursor: Option<Point>,
    /// Đang là ban đêm (theo giờ máy hoặc thời tiết thật): đi chậm hơn, buồn ngủ sớm hơn.
    pub night: bool,
    /// Người dùng đang gõ phím: không nói câu cho vui, không làm gì nổi bật.
    pub busy: bool,
    /// Cho pet nói câu cho vui (Settings): tắt thì `chat` không nói gì.
    pub chatter: bool,
    /// Im lặng (tray): không nói câu cho vui, ít tự đi lại (`QUIET_CALM`).
    pub quiet: bool,
    /// Ngôn ngữ của câu pet tự nói (chào nhau).
    pub lang: Lang,
    /// Số giây (lúc vòng lặp chạy) kể từ câu nói cho vui gần nhất của cả nhóm. Mở app thì chờ nửa `CHAT_GAP`.
    since_chat: f64,
    next_uid: u32,
}

impl World {
    pub fn new(bounds: Bounds, seed: u64) -> Self {
        Self {
            pets: Vec::new(),
            rng: Rng::new(seed),
            speed: 1.0,
            terrain: Terrain::empty(bounds),
            neighbors: Vec::new(),
            cursor: None,
            night: false,
            busy: false,
            chatter: true,
            quiet: false,
            lang: Lang::Vi,
            since_chat: t::CHAT_GAP / 2.0,
            next_uid: 1,
        }
    }

    pub fn bounds(&self) -> Bounds {
        self.terrain.bounds
    }

    /// Pet thứ `i`, kèm cả thế giới để làm gì đó với nó.
    pub fn pet(&mut self, i: usize) -> PetMut<'_> {
        PetMut { w: self, i }
    }

    /// Chỗ của pet `uid` trong `pets`, `None` nếu đã bị bỏ.
    pub fn index_of(&self, uid: u32) -> Option<usize> {
        self.pets.iter().position(|pet| pet.uid() == uid)
    }

    /// Hệ số tốc độ đi, chạy, leo: Settings, ban đêm chậm hơn.
    pub fn pace(&self) -> f64 {
        self.speed * if self.night { t::NIGHT_PACE } else { 1.0 }
    }

    /// Thêm pet đứng trên mặt đất; `x` bị kẹp lại để cả con nằm trong màn hình. Trả về chỗ của nó trong `pets`.
    pub fn spawn(&mut self, options: PetOptions) -> usize {
        let b = self.bounds();
        let half = options.width / 2.0;
        let x = clamp(options.x, b.left + half, b.right - half);
        // State đầu tiên không chạy `enter`, nên tự chọn thời lượng đứng.
        let planned = self.rng.range(t::IDLE_TIME.0, t::IDLE_TIME.1);
        let mut pet = Pet::new(options, x, b.floor, self.next_uid, planned);
        self.next_uid += 1;
        // Con mới vào nhóm: tính chung đồng hồ "không đụng tới" với cả nhóm (`step`).
        if let Some(peer) = self.pets.first() {
            pet.since_interaction = peer.since_interaction;
        }
        self.pets.push(pet);
        self.pets.len() - 1
    }

    /// Bỏ pet `id` (bớt nhân vật trong Settings).
    pub fn remove(&mut self, id: &str) {
        if let Some(index) = self.pets.iter().position(|pet| pet.id == id) {
            self.pets.remove(index);
        }
    }

    /// Chỉ có một con thì được tự đi sang màn hình khác. Cả nhóm dùng chung một overlay, nên có từ 2 con trở lên thì
    /// chỉ sang khi bị kéo hoặc ném, các con còn lại chạy theo (`PetMut::join_from`).
    pub fn wander(&self) -> bool {
        self.pets.len() < 2
    }

    /// Người dùng click ở chỗ khác trên màn hình: các con đang ngủ đều thức dậy.
    pub fn wake_all(&mut self) {
        for i in 0..self.pets.len() {
            self.pet(i).wake();
        }
    }

    /// Pet thứ `i` nói một câu cho vui: cả nhóm `CHAT_GAP` giây mới nói một câu (tối đa 4 câu mỗi giờ), không nói lúc
    /// người dùng đang gõ phím, lúc pet đang ngủ hay đang biến mất, lúc tắt `chatter`, lúc Im lặng. Trả về `true` nếu
    /// đã nói.
    pub fn chat(&mut self, i: usize, text: &str) -> bool {
        if !self.chatter || self.quiet || self.busy || self.since_chat < t::CHAT_GAP {
            return false;
        }
        let pet = &mut self.pets[i];
        if pet.state() == PetState::Sleep || pet.vanished > 0.0 {
            return false;
        }
        self.since_chat = 0.0;
        pet.say(text, t::SPEECH_TIME);
        true
    }

    /// Cửa sổ trên màn hình vừa đổi (xếp từ trên xuống theo thứ tự chồng): dựng lại địa hình, pet đang đứng trên cửa
    /// sổ nào thì đi theo cửa sổ đó. Pet phản ứng với cửa sổ bị kéo lại gần và cửa sổ vừa bị đóng hẳn (`closed`, kèm
    /// khung lúc còn hiện).
    pub fn set_windows(&mut self, windows: Vec<WindowInfo>, closed: &[WindowInfo]) {
        let terrain = Terrain::new(self.bounds(), windows.clone());
        let before = std::mem::replace(&mut self.terrain, terrain);
        for i in 0..self.pets.len() {
            self.pet(i).follow();
            for w in &windows {
                // Lệch dưới 1 pixel là do làm tròn (đổi DPI), không phải bị kéo.
                if let Some(old) = before.window(w.id) {
                    if (old.x - w.rect.x).abs() >= 1.0 || (old.y - w.rect.y).abs() >= 1.0 {
                        self.pet(i).window_moved(w.id, &old, &w.rect);
                    }
                }
            }
            for w in closed {
                self.pet(i).window_closed(w.id, &w.rect);
            }
        }
    }

    /// Overlay vừa sang màn hình khác, hoặc màn hình đổi độ phân giải, DPI, taskbar. `remap` đổi toạ độ cũ (pet, cửa
    /// sổ, con trỏ) sang toạ độ mới; danh sách cửa sổ theo toạ độ mới sẽ tới ngay sau đó.
    pub fn set_screen(&mut self, bounds: Bounds, neighbors: Vec<Rect>, remap: Option<Remap>) {
        let map = |r: Rect| match remap {
            Some(m) => Rect { x: r.x * m.scale + m.x, y: r.y * m.scale + m.y, width: r.width * m.scale, height: r.height * m.scale },
            None => r,
        };
        let windows = self.terrain.windows.iter().map(|w| WindowInfo { id: w.id, rect: map(w.rect) }).collect();
        self.terrain = Terrain::new(bounds, windows);
        self.neighbors = neighbors;
        if let (Some(m), Some(cursor)) = (remap, self.cursor) {
            self.cursor = Some(Point { x: cursor.x * m.scale + m.x, y: cursor.y * m.scale + m.y });
        }
        if let Some(m) = remap {
            for i in 0..self.pets.len() {
                self.pet(i).remap(&m);
            }
        }
        // Overlay sang màn hình khác theo một con (bị kéo, ném sang): con nào còn nằm ngoài màn hình mới (ở màn hình
        // cũ) thì chạy theo, vào từ mép phía màn hình cũ. Màn hình đổi tại chỗ (DPI, taskbar) thì pet vẫn nằm trong
        // màn hình, chỉ đứng lên mặt đất mới.
        let mut followers: Vec<(usize, Facing, f64)> = Vec::new();
        for i in 0..self.pets.len() {
            let pet = &self.pets[i];
            let away = if remap.is_some() && pet.crossing.is_none() && pet.state() != PetState::Dragged {
                self.outside(pet)
            } else {
                None
            };
            match away {
                Some((side, distance)) => followers.push((i, side, distance)),
                None => self.pet(i).rebound(),
            }
        }
        followers.sort_by(|a, b| a.2.total_cmp(&b.2));
        let mut behind = [f64::NEG_INFINITY; 2];
        for (i, side, distance) in followers {
            let slot = usize::from(side > 0);
            let at = (distance * FOLLOW_SCALE).min(FOLLOW_MAX).max(behind[slot] + self.pets[i].width * FOLLOW_GAP);
            self.pet(i).join_from(side, at);
            behind[slot] = at;
        }
    }

    /// Pet nằm ngoài màn hình về phía nào (-1: bên trái, 1: bên phải) và cách mép đó bao xa; `None` nếu vẫn ở trong.
    fn outside(&self, pet: &Pet) -> Option<(Facing, f64)> {
        let b = self.bounds();
        if pet.x < b.left {
            return Some((-1, b.left - pet.x));
        }
        if pet.x > b.right {
            return Some((1, pet.x - b.right));
        }
        None
    }

    /// Mép `side` của màn hình có giáp màn hình khác ở độ cao `y` không: pet đi hoặc bay sang được.
    pub fn exit(&self, side: Facing, y: f64) -> bool {
        let b = self.bounds();
        self.neighbors.iter().any(|n| {
            let edge = if side > 0 { n.x - b.right } else { b.left - (n.x + n.width) };
            edge.abs() <= EDGE_SLACK && y >= n.y && y < n.y + n.height
        })
    }

    /// Con trỏ vừa di chuyển (CSS pixel của overlay).
    pub fn move_cursor(&mut self, x: f64, y: f64) {
        self.cursor = Some(Point { x, y });
    }

    /// Các cửa sổ đang che một phần pet thứ `i` (pet đứng trên cửa sổ nằm dưới chúng).
    pub fn occluders(&self, i: usize) -> Vec<Rect> {
        let pet = &self.pets[i];
        let Some(mount) = pet.mount else {
            return Vec::new();
        };
        let left = pet.x - pet.width;
        let right = pet.x + pet.width;
        let top = pet.y - pet.height * 1.5;
        let bottom = pet.y + 1.0;
        (self.terrain.above(mount.id).iter())
            .filter(|r| r.x < right && r.x + r.width > left && r.y < bottom && r.y + r.height > top)
            .copied()
            .collect()
    }

    /// Mọi pet đều đang ngủ, không con nào đang nói: không có gì chuyển động, overlay có thể dừng vòng lặp vẽ.
    pub fn resting(&self) -> bool {
        self.pets.iter().all(|pet| pet.state() == PetState::Sleep && pet.speech.is_none())
    }

    pub fn step(&mut self, dt: f64) {
        self.since_chat += dt;
        for i in 0..self.pets.len() {
            self.pet(i).step(dt);
        }
        // Đồng hồ "không đụng tới" tính chung cả nhóm: click, kéo con nào cũng tính, nên cả nhóm cùng buồn ngủ và lần
        // lượt đi ngủ. Cả nhóm ngủ thì overlay dừng hẳn vòng lặp vẽ (`resting`).
        if self.pets.len() < 2 {
            return;
        }
        let since = self.pets.iter().map(|pet| pet.since_interaction).fold(f64::INFINITY, f64::min);
        for pet in &mut self.pets {
            pet.since_interaction = since;
        }
    }

    pub fn snapshot(&self) -> WorldSnapshot {
        WorldSnapshot { version: SNAPSHOT_VERSION, pets: self.pets.iter().map(Pet::snapshot).collect() }
    }
}
