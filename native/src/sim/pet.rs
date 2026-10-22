//! Một pet: dữ liệu, máy trạng thái và mọi hành vi (`pet.ts`). Pet sống trong `World`; hành vi cần đọc hay đổi cả
//! thế giới (địa hình, số ngẫu nhiên, các con khác) nên nằm ở `PetMut` (`world.pet(i)`), còn `Pet` chỉ giữ dữ liệu
//! và những gì đọc được từ riêng nó. Thứ tự bốc số ngẫu nhiên giữ y như bản TS.

use super::lines;
use super::math::clamp;
use super::moves::{
    buried, climb_targets, ground_of, hidden, jump_targets, landing_ledge, nearest_open_x, open_wall, roomy, Ground,
    JumpTarget,
};
use super::snapshot::PetSnapshot;
use super::terrain::{Bounds, Ledge, Wall};
use super::tuning as t;
use super::world::World;
use super::{js_round, Point, Rect, Remap};

/// 1: nhìn sang phải, -1: nhìn sang trái.
pub type Facing = i8;

/// State của pet, trùng tên với animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PetState {
    Idle,
    Walk,
    Run,
    Sleep,
    Dragged,
    Fall,
    Land,
    React,
    Dizzy,
    /// Leo cạnh cửa sổ.
    Climb,
    /// Ngồi ở mép cửa sổ.
    Perch,
    /// Nhảy giữa cửa sổ.
    Jump,
}

impl PetState {
    /// Tên animation trong pet.json.
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Walk => "walk",
            Self::Run => "run",
            Self::Sleep => "sleep",
            Self::Dragged => "dragged",
            Self::Fall => "fall",
            Self::Land => "land",
            Self::React => "react",
            Self::Dizzy => "dizzy",
            Self::Climb => "climb",
            Self::Perch => "perch",
            Self::Jump => "jump",
        }
    }
}

/// State đang đi lại: con khác đang ở các state này thì con này hay đứng yên hơn (`GROUP_CALM`).
fn moving(state: PetState) -> bool {
    matches!(state, PetState::Walk | PetState::Run | PetState::Climb | PetState::Jump)
}

/// State pet đang tỉnh, đứng trên mặt đất hoặc mép cửa sổ và không bận việc gì: phản ứng được với cửa sổ.
fn calm(state: PetState) -> bool {
    matches!(state, PetState::Idle | PetState::Walk | PetState::Run | PetState::Perch | PetState::Land)
}

/// State đứng yên ngay được (`hold`): đang rảnh, đang ngủ hay đang choáng trên mặt đất hoặc mép cửa sổ.
fn holdable(state: PetState) -> bool {
    calm(state) || matches!(state, PetState::Sleep | PetState::Dizzy)
}

/// Pet đang đứng trên (hoặc leo) cửa sổ `id`: điểm chân cách góc trên trái cửa sổ (dx, dy). Cửa sổ di chuyển thì
/// pet đi theo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mount {
    pub id: isize,
    pub dx: f64,
    pub dy: f64,
}

/// Việc pet đang đi tới để làm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Goal {
    /// Đi tới chân tường rồi leo.
    Climb { id: isize, side: i8 },
    /// Ra chỗ không bị che, `dx` tính từ mép trái cửa sổ đang đứng.
    Peek { dx: f64 },
    /// Buồn ngủ: đi tới đầu mép rồi xuống taskbar.
    Down,
    /// Chạy trốn cửa sổ đang bị kéo tới: hết đường trên mép cửa sổ thì xuống luôn.
    Flee,
    /// Đi cùng con khác một đoạn sau khi chào nhau (không gặp lại, không sang màn hình khác giữa chừng).
    Stroll,
    /// Buồn ngủ: đi tới chỗ `x` dưới đất, nằm cạnh con đang ngủ.
    Nap { x: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossPhase {
    /// Đang ra khỏi màn hình này, chờ overlay sang màn hình kia.
    Out,
    /// Đã sang, đang đi vào.
    In,
}

/// Đang sang màn hình bên cạnh qua mép `dir`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossing {
    pub dir: Facing,
    pub phase: CrossPhase,
    /// Số giây ở phase hiện tại.
    pub elapsed: f64,
    /// Bị ném ra quá mép: vận tốc ngang lúc bị giữ lại ngay ngoài mép chờ overlay sang. Overlay sang lúc pet còn
    /// đang rơi thì bay tiếp vào màn hình mới với vận tốc này, không rơi thẳng xuống ngoài mép.
    pub vx: Option<f64>,
}

impl Crossing {
    fn new(dir: Facing, phase: CrossPhase) -> Self {
        Self { dir, phase, elapsed: 0.0, vx: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Climbing {
    /// Cạnh cửa sổ đang bám: -1 trái, 1 phải.
    pub side: i8,
    /// -1: leo lên, 1: leo xuống.
    pub dir: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpPhase {
    Crouch,
    Air,
    Touchdown,
}

/// Lúc bật nhảy: thời điểm (giây trong state `Jump`), điểm chân, vận tốc. Lúc bay tính vị trí thẳng theo đường
/// parabol chứ không cộng dồn từng bước, để rơi đúng chỗ đã ngắm.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Launch {
    pub at: f64,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Jumping {
    /// Cửa sổ đáp lên hoặc bám vào (`None`: mặt đất).
    pub id: Option<isize>,
    /// Nhảy thẳng lên bám cạnh này của cửa sổ `id` (cửa sổ lơ lửng) rồi leo tiếp; `None`: nhảy để đáp.
    pub grab: Option<i8>,
    /// Điểm chân lúc đáp, tính từ mép trái cửa sổ (mặt đất: toạ độ overlay).
    pub offset: f64,
    pub phase: JumpPhase,
    pub launch: Launch,
    /// Thời điểm chạm chân (giây trong state `Jump`).
    pub landed_at: f64,
}

impl Jumping {
    fn new(id: Option<isize>, grab: Option<i8>, offset: f64, phase: JumpPhase) -> Self {
        Self { id, grab, offset, phase, launch: Launch::default(), landed_at: 0.0 }
    }
}

/// Câu pet đang nói, hiện thành speech bubble trên đầu.
#[derive(Debug, Clone, PartialEq)]
pub struct Speech {
    pub text: String,
    /// Số giây còn hiện.
    pub left: f64,
}

#[derive(Debug, Clone)]
pub struct PetOptions {
    pub id: String,
    pub x: f64,
    pub width: f64,
    pub height: f64,
    /// Khoảng từ điểm chân tới tường lúc leo (CSS pixel), mặc định theo bề ngang.
    pub reach: Option<f64>,
}

/// Máy trạng thái của pet: state hiện tại và số giây đã ở đó.
#[derive(Debug, Clone, Copy)]
struct Brain {
    state: PetState,
    time: f64,
}

#[derive(Debug, Clone)]
pub struct Pet {
    pub id: String,
    /// Số riêng của con này trong `World`, không đổi kể cả khi con khác bị bỏ (renderer, `buddy`).
    uid: u32,
    /// Kích thước khi vẽ (CSS pixel), dùng để giữ cả con nằm trong màn hình. Đổi bằng `resize`.
    pub width: f64,
    pub height: f64,
    /// Khoảng từ điểm chân tới tường lúc leo (CSS pixel).
    pub reach: f64,
    /// Điểm chân (giữa mép dưới) theo CSS pixel của overlay.
    pub x: f64,
    pub y: f64,
    /// Vận tốc khi đang rơi, bị ném hoặc đang nhảy (px/s).
    pub vx: f64,
    pub vy: f64,
    pub facing: Facing,
    /// Số giây kể từ lần cuối người dùng click hoặc kéo pet.
    pub since_interaction: f64,
    /// Số giây kể từ lần click trước, kể cả click bị bỏ qua.
    since_poke: f64,
    /// Thời lượng đã chọn cho lượt đứng / đi / chạy / ngồi mép hiện tại.
    pub planned: f64,
    /// Cửa sổ đang đứng hoặc leo; `None` là đang ở mặt đất hoặc trên không.
    pub mount: Option<Mount>,
    pub goal: Option<Goal>,
    pub climbing: Option<Climbing>,
    pub jumping: Option<Jumping>,
    /// Rơi qua mép cửa sổ này mà không đáp (vừa bước hoặc nhảy khỏi nó).
    pub skip_ledge: Option<isize>,
    /// Nhảy thêm chừng này cái nữa sau cú nhảy `React` đang dở (ăn mừng).
    pub hops: u32,
    /// Số giây kể từ lần ăn mừng trước.
    since_cheer: f64,
    /// Số giây kể từ lần quay đầu theo con trỏ.
    since_turn: f64,
    pub crossing: Option<Crossing>,
    /// Câu đang nói (speech bubble), `None` là không nói gì.
    pub speech: Option<Speech>,
    /// Số giây kể từ lần dừng lại chào con khác.
    pub since_meet: f64,
    /// Vừa chào con này (`uid`): đứng chào xong thì rủ nó đi cùng một đoạn.
    pub buddy: Option<u32>,
    /// Đang chat (khung chat mở cạnh con này, ở phía này): đứng yên quay về phía khung chat, không ngủ, không đi
    /// theo con khác. `None` là không chat.
    pub listening: Option<Facing>,
    /// Còn biến mất chừng này giây (bị Long quật bay, `vanish`): đứng nguyên như lúc biến mất, không làm gì, con
    /// khác không thấy; hết giờ thì hiện lại đúng chỗ cũ và làm tiếp việc đang dở. 0 là đang hiện.
    pub vanished: f64,
    brain: Brain,
}

impl Pet {
    /// `planned`: thời lượng đứng đầu tiên. State đầu tiên không chạy `enter`, nên `World` tự chọn.
    pub(super) fn new(options: PetOptions, x: f64, y: f64, uid: u32, planned: f64) -> Self {
        Self {
            reach: options.reach.unwrap_or(options.width * t::REACH),
            id: options.id,
            uid,
            width: options.width,
            height: options.height,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            facing: 1,
            since_interaction: 0.0,
            since_poke: f64::INFINITY,
            planned,
            mount: None,
            goal: None,
            climbing: None,
            jumping: None,
            skip_ledge: None,
            hops: 0,
            since_cheer: f64::INFINITY,
            since_turn: f64::INFINITY,
            crossing: None,
            speech: None,
            since_meet: f64::INFINITY,
            buddy: None,
            listening: None,
            vanished: 0.0,
            brain: Brain { state: PetState::Idle, time: 0.0 },
        }
    }

    pub fn uid(&self) -> u32 {
        self.uid
    }

    pub fn state(&self) -> PetState {
        self.brain.state
    }

    /// Số giây đã ở state hiện tại, renderer dùng để chọn frame.
    pub fn state_time(&self) -> f64 {
        self.brain.time
    }

    /// Frame bắt buộc của animation hiện tại (nhảy: lấy đà, bay, tiếp đất); `None` là chạy theo thời gian.
    pub fn pose(&self) -> Option<usize> {
        let jumping = self.jumping.as_ref().filter(|_| self.state() == PetState::Jump)?;
        Some(match jumping.phase {
            JumpPhase::Crouch => 0,
            JumpPhase::Air => 1,
            JumpPhase::Touchdown if self.state_time() - jumping.landed_at < t::JUMP_TOUCHDOWN / 2.0 => 2,
            JumpPhase::Touchdown => 3,
        })
    }

    /// Nói `text` trong `seconds` giây (speech bubble). Không qua giới hạn tần suất: gọi `World::chat` cho câu nói
    /// cho vui.
    pub fn say(&mut self, text: impl Into<String>, seconds: f64) {
        self.speech = Some(Speech { text: text.into(), left: seconds });
    }

    /// Biến mất `seconds` giây (`vanished`); câu đang nói để dành tới lúc hiện lại.
    pub fn vanish(&mut self, seconds: f64) {
        self.vanished = self.vanished.max(seconds);
    }

    /// Khung chat đã đóng hoặc đổi sang con khác: lại đi lại như thường.
    pub fn stop_listening(&mut self) {
        self.listening = None;
    }

    /// Đang đứng trên mặt đất hoặc mép cửa sổ: không bị kéo, không đang rơi, leo hay bay.
    pub fn grounded(&self) -> bool {
        match self.state() {
            PetState::Dragged | PetState::Fall | PetState::Climb => false,
            PetState::Jump => self.jumping.is_none_or(|j| j.phase != JumpPhase::Air),
            _ => true,
        }
    }

    /// Đang ra khỏi mép giáp màn hình khác, giữa thân đã qua mép: điểm (CSS pixel của overlay, nằm trên màn hình
    /// kia) mà overlay cần sang màn hình chứa nó. `None` là không đi đâu.
    pub fn leaving(&self, bounds: &Bounds) -> Option<Point> {
        let crossing = self.crossing.filter(|c| c.phase == CrossPhase::Out)?;
        let inside = if crossing.dir > 0 { self.x <= bounds.right } else { self.x >= bounds.left };
        if inside {
            return None;
        }
        let y = if self.state() == PetState::Fall { self.y - self.height / 2.0 } else { self.y - 1.0 };
        Some(Point { x: self.x, y })
    }

    pub fn snapshot(&self) -> PetSnapshot {
        PetSnapshot {
            id: self.id.clone(),
            x: js_round(self.x),
            facing: self.facing,
            asleep: self.state() == PetState::Sleep,
            since_interaction: self.since_interaction.floor(),
        }
    }
}

/// Hai con đứng trên cùng mặt đất (taskbar hoặc cùng mép cửa sổ), cách nhau dưới `reach` lần nửa bề ngang cộng lại.
fn neighbors(pet: &Pet, other: &Pet, reach: f64) -> bool {
    if !pet.grounded() || !other.grounded() || pet.mount.map(|m| m.id) != other.mount.map(|m| m.id) {
        return false;
    }
    if (pet.y - other.y).abs() > 2.0 {
        return false;
    }
    (other.x - pet.x).abs() < ((pet.width + other.width) / 2.0) * reach
}

/// `pet` đang đi vừa tới sát `other` ở phía trước, cả hai đều rảnh: dừng lại chào được.
fn meetable(pet: &Pet, other: &Pet) -> bool {
    if other.since_meet < t::MEET_COOLDOWN || other.crossing.is_some() || other.goal.is_some() || other.listening.is_some()
    {
        return false;
    }
    // Con đang biến mất thì không thấy để mà chào.
    if other.vanished > 0.0 {
        return false;
    }
    if !matches!(other.state(), PetState::Idle | PetState::Walk | PetState::Run) {
        return false;
    }
    (other.x - pet.x) * f64::from(pet.facing) > 0.0 && neighbors(pet, other, t::MEET_REACH)
}

/// Khoảng cách từ thân pet (khung một frame, đứng trên điểm chân) tới hình chữ nhật `rect`.
fn gap(pet: &Pet, rect: &Rect) -> f64 {
    let half = pet.width / 2.0;
    let dx = 0f64.max(rect.x - (pet.x + half)).max(pet.x - half - (rect.x + rect.width));
    let dy = 0f64.max(rect.y - pet.y).max(pet.y - pet.height - (rect.y + rect.height));
    dx.hypot(dy)
}

fn sign(value: f64) -> Facing {
    if value > 0.0 {
        1
    } else {
        -1
    }
}

/// Hệ số `GROUP_CALM`; test đổi được để so cả nhóm có và không có nó.
#[cfg(not(test))]
fn group_calm() -> f64 {
    t::GROUP_CALM
}

#[cfg(test)]
thread_local! {
    pub(super) static GROUP_CALM: std::cell::Cell<f64> = const { std::cell::Cell::new(t::GROUP_CALM) };
}

#[cfg(test)]
fn group_calm() -> f64 {
    GROUP_CALM.with(std::cell::Cell::get)
}

/// Pet thứ `i` của `world`, kèm cả thế giới để đọc địa hình, bốc số ngẫu nhiên, đụng tới con khác.
pub struct PetMut<'a> {
    pub(super) w: &'a mut World,
    pub(super) i: usize,
}

impl PetMut<'_> {
    fn me(&self) -> &Pet {
        &self.w.pets[self.i]
    }

    fn p(&mut self) -> &mut Pet {
        &mut self.w.pets[self.i]
    }

    /// Con thứ `j` trong cùng thế giới.
    fn other(&mut self, j: usize) -> PetMut<'_> {
        PetMut { w: self.w, i: j }
    }

    fn ground(&self) -> Ground {
        ground_of(self.w, self.me())
    }

    fn bounds(&self) -> Bounds {
        self.w.terrain.bounds
    }

    fn window(&self, id: isize) -> Option<Rect> {
        self.w.terrain.window(id)
    }

    fn mount_rect(&self) -> Option<Rect> {
        self.me().mount.and_then(|m| self.window(m.id))
    }

    /// Dừng lại đứng yên `seconds` giây, quay về phía `facing` (gặp con khác).
    pub fn pause(&mut self, facing: Facing, seconds: f64) {
        self.p().goal = None;
        self.p().facing = facing;
        self.go(PetState::Idle);
        self.p().planned = seconds;
    }

    /// Đi về phía `dir` trong `seconds` giây, cùng một con khác (`Goal::Stroll`).
    pub fn stroll(&mut self, dir: Facing, seconds: f64) {
        self.p().facing = dir;
        self.set_off(PetState::Walk, Goal::Stroll);
        self.p().planned = seconds;
    }

    /// Đứng yên `seconds` giây quay về phía `facing`, nếu đang rảnh trên mặt đất hoặc mép cửa sổ (đang ngủ, đang
    /// choáng thì dậy luôn). Đang bị kéo, rơi, leo, nhảy, chat, sang màn hình khác hay đang biến mất thì thôi, trả
    /// về `false`: ép đứng yên lúc đó thì pet đứng khựng giữa không trung.
    pub fn hold(&mut self, facing: Facing, seconds: f64) -> bool {
        let me = self.me();
        if !holdable(me.state()) || me.crossing.is_some() || me.listening.is_some() || me.vanished > 0.0 {
            return false;
        }
        self.pause(facing, seconds);
        true
    }

    /// Tốc biến tới đứng cạnh con thứ `other` (đang đứng trên mặt đất hoặc mép cửa sổ): cùng chỗ đứng với nó, điểm
    /// chân ở `x` (kẹp trong chỗ đứng), quay mặt về phía nó rồi đứng yên `seconds` giây. Đang rơi, leo, nhảy cũng
    /// được, đứng hẳn lên chỗ mới chứ không lơ lửng. Không gọi lúc đang bị kéo.
    pub fn blink_to(&mut self, other: usize, x: f64, seconds: f64) {
        let facing = self.me().facing;
        self.pause(facing, seconds);
        let target = &self.w.pets[other];
        let mount = if target.grounded() { target.mount } else { None };
        let pet = self.p();
        pet.vx = 0.0;
        pet.vy = 0.0;
        pet.crossing = None;
        pet.skip_ledge = None;
        pet.mount = mount.map(|m| Mount { id: m.id, dx: 0.0, dy: m.dy });
        let ground = self.ground();
        // Cửa sổ vừa đóng thì đứng xuống mặt đất.
        if ground.ledge.is_none() {
            self.p().mount = None;
        }
        // Mép hẹp hơn chỗ đứng: đứng giữa mép, bước sau sẽ rơi (`mount_holds`) như mọi lúc khác.
        self.p().x =
            if ground.to < ground.from { (ground.from + ground.to) / 2.0 } else { clamp(x, ground.from, ground.to) };
        self.p().y = ground.y;
        self.sync_mount();
        let other_x = self.w.pets[other].x;
        if (other_x - self.me().x).abs() > 1.0 {
            self.p().facing = sign(other_x - self.me().x);
        }
    }

    /// Người dùng click vào pet: phản ứng, đang ngủ thì thức dậy, đang leo thì tuột tay. Đang nhảy hoặc click dồn
    /// dập thì chỉ nhảy một lần: cú nhảy không bị bắt đầu lại, phải ngừng click một lúc mới nhảy tiếp.
    pub fn poke(&mut self) {
        // Bấm vào con đang nói thì tắt bubble (cả lời nhắc nghỉ, nhắc khuya).
        self.p().speech = None;
        if self.me().state() == PetState::Climb {
            self.p().since_poke = 0.0;
            self.p().since_interaction = 0.0;
            self.let_go();
            return;
        }
        if !self.me().grounded() {
            return;
        }
        let spam = self.me().since_poke < t::POKE_COOLDOWN;
        self.p().since_poke = 0.0;
        self.p().since_interaction = 0.0;
        if spam || self.me().state() == PetState::React {
            return;
        }
        self.p().hops = 0;
        self.go(PetState::React);
    }

    /// Cửa sổ `id` vừa di chuyển từ `from` tới `to`. Giữ nguyên cỡ (đang bị kéo, không phải phóng to hay đổi cỡ)
    /// mà lại gần sát pet thì chạy trốn; hết đường chạy dưới đất thì giật mình nhảy lên.
    pub fn window_moved(&mut self, id: isize, from: &Rect, to: &Rect) {
        let me = self.me();
        // Đang vắt qua mép sang màn hình khác: đi nốt, không quay đầu giữa chừng.
        if me.mount.is_some_and(|m| m.id == id) || !calm(me.state()) || me.crossing.is_some() || me.vanished > 0.0 {
            return;
        }
        if (from.width - to.width).abs() >= 1.0 || (from.height - to.height).abs() >= 1.0 {
            return;
        }
        let after = gap(me, to);
        if after >= t::FLEE_RANGE || after >= gap(me, from) {
            return;
        }
        let away = if me.x < to.x + to.width / 2.0 { -1 } else { 1 };
        self.run_from(away);
    }

    /// Cửa sổ `id` (khung cuối cùng `rect`) vừa bị đóng: ở gần thì quay về phía đó nhảy cẫng lên ăn mừng.
    pub fn window_closed(&mut self, id: isize, rect: &Rect) {
        let me = self.me();
        if me.mount.is_some_and(|m| m.id == id) || !calm(me.state()) || me.vanished > 0.0 {
            return;
        }
        if me.since_cheer < t::CHEER_COOLDOWN || gap(me, rect) > t::CHEER_RANGE {
            return;
        }
        if !self.w.rng.chance(t::CHEER_CHANCE) {
            return;
        }
        let pet = self.p();
        pet.since_cheer = 0.0;
        let center = rect.x + rect.width / 2.0;
        if (center - pet.x).abs() > 1.0 {
            pet.facing = sign(center - pet.x);
        }
        pet.hops = t::CHEER_HOPS - 1;
        self.go(PetState::React);
    }

    /// Giật mình (sấm, sự kiện hiếm): đang rảnh đứng trên mặt đất hoặc mép cửa sổ thì nhảy dựng lên một cái. Không
    /// tính là người dùng đụng vào, nên không làm cả nhóm tỉnh ngủ lâu hơn.
    pub fn startle(&mut self) {
        let me = self.me();
        if !calm(me.state()) || !me.grounded() || me.crossing.is_some() || me.vanished > 0.0 {
            return;
        }
        self.p().hops = 0;
        self.go(PetState::React);
    }

    /// Người dùng mở khung chat với con này, khung chat ở phía `side`: đang đi thì dừng lại, đang ngủ thì dậy, rồi
    /// đứng yên quay về phía đó tới khi `stop_listening`. Đang leo, nhảy, rơi thì xong rồi mới đứng yên.
    pub fn listen(&mut self, side: Facing) {
        self.p().listening = Some(side);
        self.p().since_interaction = 0.0;
        self.wake();
        if matches!(self.me().state(), PetState::Walk | PetState::Run | PetState::Idle) {
            self.pause(side, t::IDLE_TIME.0);
        }
    }

    /// Người dùng click ở chỗ khác trên màn hình: đang ngủ thì giật mình thức dậy như bị click.
    pub fn wake(&mut self) {
        if self.me().state() == PetState::Sleep {
            self.poke();
        }
    }

    /// Người dùng bắt đầu kéo pet, bắt được cả khi pet đang rơi, leo hay nhảy.
    pub fn grab(&mut self) {
        let pet = self.p();
        pet.since_interaction = 0.0;
        pet.vx = 0.0;
        pet.vy = 0.0;
        pet.mount = None;
        pet.skip_ledge = None;
        pet.crossing = None;
        self.go(PetState::Dragged);
    }

    /// Đặt điểm chân của pet theo chuột trong lúc kéo, luôn giữ trong vùng cho phép.
    pub fn drag_to(&mut self, x: f64, y: f64) {
        if self.me().state() != PetState::Dragged {
            return;
        }
        let b = self.bounds();
        let pet = self.p();
        let half = pet.width / 2.0;
        pet.x = clamp(x, b.left + half, b.right - half);
        pet.y = clamp(y, b.top + pet.height, b.floor);
    }

    /// Thả pet với vận tốc chuột lúc buông (px/s): rơi xuống, ném mạnh thì bay theo quán tính.
    pub fn release(&mut self, vx: f64, vy: f64) {
        if self.me().state() != PetState::Dragged {
            return;
        }
        let speed = vx.hypot(vy);
        let k = if speed > t::MAX_THROW_SPEED { t::MAX_THROW_SPEED / speed } else { 1.0 };
        self.p().vx = vx * k;
        self.p().vy = vy * k;
        self.go(PetState::Fall);
    }

    pub fn step(&mut self, dt: f64) {
        let pet = self.p();
        pet.since_interaction += dt;
        pet.since_poke += dt;
        pet.since_cheer += dt;
        pet.since_turn += dt;
        pet.since_meet += dt;
        // Đang biến mất: đứng nguyên như lúc biến mất (câu đang nói để dành), hết giờ thì làm tiếp.
        if pet.vanished > 0.0 {
            pet.vanished = (pet.vanished - dt).max(0.0);
            return;
        }
        if let Some(speech) = &mut pet.speech {
            speech.left -= dt;
            if speech.left <= 0.0 {
                pet.speech = None;
            }
        }
        if self.me().crossing.is_some() {
            // Chỉ tính giờ chờ overlay sang từ lúc giữa thân qua mép: đi bộ từ đầu mặt đất ra tới đó mất hơn một giây.
            let bounds = self.bounds();
            if self.me().leaving(&bounds).is_some() {
                if let Some(crossing) = &mut self.p().crossing {
                    crossing.elapsed += dt;
                }
            }
            // Bị click, bị kéo giữa chừng: thôi sang màn hình kia.
            if !matches!(self.me().state(), PetState::Walk | PetState::Run | PetState::Fall) {
                self.stay_inside();
            }
        }
        if let Some(mount) = self.me().mount {
            // Cửa sổ đang đứng vừa đóng, thu nhỏ, bị kéo tới chỗ không đứng được: rơi.
            if !self.mount_holds() {
                self.drop_off();
            }
            // Bị che hết: rơi ra trước mọi cửa sổ (đang leo thì buông tay), không đáp lại mép đó. Trước đó pet đi ra
            // đầu mép rồi leo xuống, khuất sau cửa sổ cả chục giây như đã biến mất.
            else if buried(self.w, self.me()) {
                self.p().skip_ledge = Some(mount.id);
                if self.me().state() == PetState::Climb {
                    self.let_go();
                } else {
                    self.drop_off();
                }
            }
        }
        self.watch_cursor();
        self.update(dt);
    }

    /// Overlay vừa sang màn hình khác theo một con khác (bị kéo hoặc ném sang) mà con này còn ở màn hình cũ: xuống
    /// đất ngay ngoài mép `side` của màn hình mới (-1: mép trái), cách mép thêm `behind` px, rồi chạy vào. Đang ngủ
    /// thì nằm luôn ở sát mép đó.
    pub fn join_from(&mut self, side: Facing, behind: f64) {
        let b = self.bounds();
        let pet = self.p();
        let half = pet.width / 2.0;
        pet.mount = None;
        pet.goal = None;
        pet.skip_ledge = None;
        pet.vx = 0.0;
        pet.vy = 0.0;
        pet.y = b.floor;
        pet.facing = if side < 0 { 1 } else { -1 };
        if pet.state() == PetState::Sleep {
            pet.crossing = None;
            pet.x = if side < 0 { b.left + half } else { b.right - half };
            return;
        }
        pet.x = if side < 0 { b.left - half - behind } else { b.right + half + behind };
        self.go_with(PetState::Run, |pet| {
            pet.crossing = Some(Crossing::new(pet.facing, CrossPhase::In));
        });
    }

    /// Thôi sang màn hình bên cạnh, về hẳn trong màn hình này.
    pub fn stay_inside(&mut self) {
        let b = self.bounds();
        let pet = self.p();
        pet.crossing = None;
        pet.x = clamp(pet.x, b.left + pet.width / 2.0, b.right - pet.width / 2.0);
    }

    /// Overlay vừa đổi chỗ hoặc đổi cỡ (sang màn hình khác, đổi DPI, taskbar): toạ độ cũ `p` thành
    /// `p * scale + (x, y)`. Gọi `rebound` sau khi đã có vùng màn hình mới.
    pub fn remap(&mut self, remap: &Remap) {
        let Remap { scale, x, y } = *remap;
        let pet = self.p();
        pet.x = pet.x * scale + x;
        pet.y = pet.y * scale + y;
        pet.vx *= scale;
        pet.vy *= scale;
        if let Some(mount) = &mut pet.mount {
            mount.dx *= scale;
            mount.dy *= scale;
        }
        // Đường leo, đường nhảy tính theo toạ độ cũ: buông ra rơi cho chắc (hiếm khi xảy ra đúng lúc này).
        if matches!(self.me().state(), PetState::Climb | PetState::Jump) {
            self.drop_off();
        }
        // Overlay đã sang màn hình pet đang đi tới. Bị giữ ngay ngoài mép lúc đang bay thì bay tiếp vào.
        if let Some(crossing) = self.me().crossing.filter(|c| c.phase == CrossPhase::Out) {
            let pet = self.p();
            if let (PetState::Fall, Some(vx)) = (pet.state(), crossing.vx) {
                pet.vx = vx * scale;
            }
            pet.crossing = Some(Crossing::new(crossing.dir, CrossPhase::In));
        }
    }

    /// Vùng màn hình vừa đổi: giữ pet trong vùng mới, đang ở dưới đất thì đứng lên (hoặc rơi xuống) mặt đất mới.
    pub fn rebound(&mut self) {
        let b = self.bounds();
        if self.me().mount.is_some() {
            self.follow();
            return;
        }
        let pace = self.w.pace();
        let pet = self.p();
        let half = pet.width / 2.0;
        let state = pet.state();
        if state == PetState::Fall {
            return;
        }
        if pet.crossing.is_none() {
            pet.x = clamp(pet.x, b.left + half, b.right - half);
        }
        if state == PetState::Dragged {
            pet.y = clamp(pet.y, b.top + pet.height, b.floor);
            return;
        }
        // Đang nhảy lên vì bị click: mỗi bước tự tính độ cao theo mặt đất mới.
        if state == PetState::React {
            return;
        }
        // Đang ngủ thì nằm yên trên mặt đất mới, không bị đánh thức.
        if pet.y < b.floor - 1.0 && state != PetState::Sleep {
            pet.vx = if pet.crossing.is_some() { f64::from(pet.facing) * t::WALK_SPEED * pace } else { 0.0 };
            pet.vy = 0.0;
            self.go(PetState::Fall);
            return;
        }
        pet.y = b.floor;
    }

    /// Địa hình vừa đổi: pet đang đứng hoặc leo trên cửa sổ nào thì đi theo cửa sổ đó.
    pub fn follow(&mut self) {
        let (Some(mount), Some(rect)) = (self.me().mount, self.mount_rect()) else {
            return;
        };
        let b = self.bounds();
        let pet = self.p();
        pet.x = rect.x + mount.dx;
        pet.y = rect.y + mount.dy;
        if pet.state() == PetState::Climb {
            return;
        }
        // Cửa sổ bị kéo ra ngoài màn hình: pet lùi vào trong chứ không ra theo.
        let x = clamp(pet.x, b.left + pet.width / 2.0, b.right - pet.width / 2.0);
        if x != pet.x {
            pet.x = x;
            if let Some(mount) = &mut pet.mount {
                mount.dx = x - rect.x;
            }
        }
    }

    /// Đổi cỡ pet (Settings), kẹp lại để cả con vẫn nằm trong màn hình. `reach` mặc định theo bề ngang.
    pub fn resize(&mut self, width: f64, height: f64, reach: Option<f64>) {
        let b = self.bounds();
        let pet = self.p();
        pet.width = width;
        pet.height = height;
        pet.reach = reach.unwrap_or(width * t::REACH);
        // Đang đứng trên cửa sổ thì vẫn đứng đúng mép; to quá không còn chỗ thì bước sau sẽ rơi.
        if pet.mount.is_some() {
            self.follow();
            return;
        }
        pet.x = clamp(pet.x, b.left + width / 2.0, b.right - width / 2.0);
        pet.y = clamp(pet.y, b.top + height, b.floor);
    }

    /// Đặt lại theo trạng thái đã lưu: đứng trên mặt đất ở chỗ cũ, đang ngủ thì ngủ tiếp.
    pub fn restore(&mut self, saved: &PetSnapshot) {
        let b = self.bounds();
        let pet = self.p();
        pet.x = clamp(saved.x, b.left + pet.width / 2.0, b.right - pet.width / 2.0);
        pet.y = b.floor;
        pet.vx = 0.0;
        pet.vy = 0.0;
        pet.mount = None;
        pet.facing = saved.facing;
        pet.since_interaction = saved.since_interaction;
        self.go(if saved.asleep { PetState::Sleep } else { PetState::Idle });
    }

    fn mount_holds(&self) -> bool {
        let me = self.me();
        let terrain = &self.w.terrain;
        let (Some(mount), Some(rect)) = (me.mount, self.mount_rect()) else {
            return false;
        };
        if me.state() == PetState::Climb {
            let Some(side) = me.climbing.map(|c| c.side) else {
                return false;
            };
            if terrain.wall(mount.id, side).is_none() {
                return false;
            }
            return me.y >= rect.y - 1.0 && me.y <= rect.y + rect.height + me.height;
        }
        terrain
            .ledge(mount.id)
            .is_some_and(|ledge| roomy(self.w, me, ledge) && me.x >= ledge.from - 1.0 && me.x <= ledge.to + 1.0)
    }

    fn drop_off(&mut self) {
        let pet = self.p();
        pet.mount = None;
        pet.goal = None;
        pet.vx = 0.0;
        pet.vy = 0.0;
        self.go(PetState::Fall);
    }

    /// Chạy về phía `away`; dưới đất mà sát mép màn hình, không còn đường thì giật mình nhảy lên.
    fn run_from(&mut self, away: Facing) {
        if self.me().state() == PetState::Run && self.me().goal == Some(Goal::Flee) {
            self.p().facing = away;
            return;
        }
        let ground = self.ground();
        let x = self.me().x;
        let room = if away > 0 { ground.to - x } else { x - ground.from };
        if ground.ledge.is_none() && room < self.me().width {
            self.p().hops = 0;
            self.go(PetState::React);
            return;
        }
        self.p().facing = away;
        self.set_off(PetState::Run, Goal::Flee);
    }

    /// Đi/chạy để làm `goal`. `exit` của walk/run xoá việc đang làm, nên đặt việc mới sau đó.
    fn set_off(&mut self, state: PetState, goal: Goal) {
        self.go_with(state, |pet| pet.goal = Some(goal));
    }

    /// Con trỏ ở gần thì pet quay về phía con trỏ.
    fn watch_cursor(&mut self) {
        let Some(cursor) = self.w.cursor else {
            return;
        };
        let pet = self.p();
        if pet.state() != PetState::Idle || !pet.grounded() || pet.crossing.is_some() {
            return;
        }
        let dx = cursor.x - pet.x;
        if dx.abs() >= t::LOOK_DEAD_ZONE
            && dx.hypot(cursor.y - (pet.y - pet.height / 2.0)) <= t::LOOK_RANGE
            && pet.since_turn >= t::LOOK_TURN
        {
            let side = sign(dx);
            if side != pet.facing {
                pet.facing = side;
                pet.since_turn = 0.0;
            }
        }
    }

    /// Đang leo thì buông tay, bật nhẹ ra khỏi tường.
    fn let_go(&mut self) {
        let pet = self.p();
        let side = pet.climbing.map_or(-pet.facing, |c| c.side);
        pet.mount = None;
        pet.vx = f64::from(side) * 60.0;
        pet.vy = 0.0;
        self.go(PetState::Fall);
    }

    // Máy trạng thái (`fsm.ts`).

    /// Chuyển state ngay. Chuyển sang chính state đang ở thì chạy lại state đó từ đầu.
    fn go(&mut self, next: PetState) {
        self.go_with(next, |_| {});
    }

    /// Như `go`; `prepare` chạy sau `exit` của state cũ, trước `enter` của state mới: đặt dữ liệu mà `exit` sẽ xoá
    /// còn `enter` cần đọc.
    fn go_with(&mut self, next: PetState, prepare: impl FnOnce(&mut Pet)) {
        self.exit(self.me().state());
        prepare(self.p());
        self.p().brain = Brain { state: next, time: 0.0 };
        self.enter(next);
    }

    fn update(&mut self, dt: f64) {
        self.p().brain.time += dt;
        let Brain { state, time } = self.me().brain;
        if let Some(next) = self.update_state(state, time, dt) {
            self.go(next);
        }
    }

    fn enter(&mut self, state: PetState) {
        match state {
            PetState::Idle => self.plan(t::IDLE_TIME),
            PetState::Walk => self.start_moving(t::WALK_TIME),
            PetState::Run => self.start_moving(t::RUN_TIME),
            PetState::Perch => self.plan(t::PERCH_TIME),
            _ => {}
        }
    }

    fn exit(&mut self, state: PetState) {
        let pet = self.p();
        match state {
            // Chưa kịp rủ đi cùng thì thôi (bị click, bị kéo, cửa sổ kéo tới...).
            PetState::Idle => pet.buddy = None,
            PetState::Walk | PetState::Run => pet.goal = None,
            PetState::Climb => pet.climbing = None,
            PetState::Jump => pet.jumping = None,
            _ => {}
        }
    }

    /// Một bước của state `state`; trả về state tiếp theo để chuyển, `None` để ở lại.
    fn update_state(&mut self, state: PetState, time: f64, dt: f64) -> Option<PetState> {
        match state {
            PetState::Idle => (time >= self.me().planned).then(|| self.next_activity()).flatten(),
            PetState::Walk => self.walk(t::WALK_SPEED, time, dt),
            PetState::Run => self.walk(t::RUN_SPEED, time, dt),
            // Chỉ thức dậy khi người dùng click hoặc kéo. Lúc bị kéo, vị trí do chuột điều khiển qua `drag_to`.
            PetState::Sleep | PetState::Dragged => None,
            PetState::React => self.hop(time),
            PetState::Fall => self.fall(dt),
            PetState::Land => (time >= t::LAND_TIME).then_some(PetState::Idle),
            PetState::Dizzy => (time >= t::DIZZY_TIME).then_some(PetState::Idle),
            PetState::Climb => self.climb(dt),
            PetState::Perch => (time >= self.me().planned).then(|| self.leave_perch()),
            PetState::Jump => self.jump(time),
        }
    }

    // Các state (`pet.ts`).

    fn plan(&mut self, (min, max): (f64, f64)) {
        let planned = self.w.rng.range(min, max);
        self.p().planned = planned;
    }

    /// Việc tiếp theo sau lượt đứng yên; `None` là đã tự chuyển state (rủ con khác đi cùng).
    fn next_activity(&mut self) -> Option<PetState> {
        if let Some(side) = self.me().listening {
            // Đang chat: đứng yên nhìn về phía khung chat, không tính là bỏ mặc nên không buồn ngủ.
            self.p().facing = side;
            self.p().since_interaction = 0.0;
            return Some(PetState::Idle);
        }
        let ground = self.ground();
        // Chỉ ngủ trên taskbar: đang ở trên cửa sổ thì xuống trước. Có con đang ngủ gần đó thì ra nằm cạnh.
        let sleep_after = if self.w.night { t::SLEEP_AFTER_NIGHT } else { t::SLEEP_AFTER };
        if self.me().since_interaction >= sleep_after {
            return Some(if ground.ledge.is_some() { self.go_down(&ground) } else { self.nap() });
        }
        // Bị che hết thì `step` đã cho rơi: ở đây còn chỗ để ra.
        if hidden(self.w, self.me()) {
            if let Some(x) = nearest_open_x(self.me(), &ground) {
                return Some(self.walk_to(x));
            }
        }
        // Vừa chào con khác xong, nó vẫn đứng cạnh: rủ đi cùng một đoạn (Im lặng thì thôi).
        let buddy = self.p().buddy.take().and_then(|uid| self.w.index_of(uid));
        if let Some(b) = buddy {
            let other = &self.w.pets[b];
            if !self.w.quiet && other.state() == PetState::Idle && neighbors(self.me(), other, t::MEET_REACH * 1.5) {
                let dir = if self.w.rng.chance(0.5) { 1 } else { -1 };
                let time = self.w.rng.range(t::STROLL_TIME.0, t::STROLL_TIME.1);
                self.other(b).stroll(dir, time);
                self.stroll(dir, time);
                return None;
            }
        }
        // Con khác đang đi lại thì con này hay đứng yên hơn: cả nhóm ít khi cùng chạy nhảy một lúc, đỡ rối mắt và
        // vòng lặp vẽ được nghỉ nhiều hơn. Chỉ đổi xác suất, không bốc thêm số ngẫu nhiên, nên một con thì sống y
        // như trước.
        let movers = (self.w.pets.iter().enumerate())
            .filter(|&(j, other)| j != self.i && other.vanished == 0.0 && moving(other.state()))
            .count();
        let quiet = if self.w.quiet { t::QUIET_CALM } else { 1.0 };
        let keep = (1.0 - group_calm() * movers as f64).max(0.0) * quiet;
        // Chỉ bốc số ngẫu nhiên khi có chỗ để nhảy / leo, để không có cửa sổ thì pet sống y như Phase 1.
        let jumps = jump_targets(self.w, self.me());
        if !jumps.is_empty() && self.w.rng.chance(t::JUMP_CHANCE * keep) {
            let target = *self.w.rng.pick(&jumps);
            return Some(self.start_jump(target));
        }
        let walls = climb_targets(self.w, self.me(), &ground);
        if !walls.is_empty() && self.w.rng.chance(t::CLIMB_CHANCE * keep) {
            let wall = self.w.rng.pick(&walls).clone();
            return Some(self.approach(&wall));
        }
        // Mép hẹp hơn chỗ đứng thì không đi được.
        if ground.to - ground.from < 1.0 {
            return Some(PetState::Idle);
        }
        let roll = self.w.rng.next();
        Some(if roll < 0.45 * keep {
            PetState::Walk
        } else if roll < 0.6 * keep {
            PetState::Run
        } else {
            PetState::Idle
        })
    }

    fn start_moving(&mut self, time: (f64, f64)) {
        match self.me().goal {
            Some(Goal::Flee) => return self.plan(t::FLEE_TIME),
            Some(_) => {
                self.p().planned = t::GOAL_TIME;
                return;
            }
            None => {}
        }
        self.plan(time);
        // Đang đi vào từ mép màn hình mới (bị ném sang, chạm đất ngoài mép) thì không quay đầu.
        if self.me().crossing.is_none() && self.w.rng.chance(t::TURN_CHANCE) {
            self.p().facing = -self.me().facing;
        }
    }

    /// Một bước đi/chạy với tốc độ `speed`.
    fn walk(&mut self, speed: f64, time: f64, dt: f64) -> Option<PetState> {
        let ground = self.ground();
        if ground.to - ground.from < 1.0 {
            return Some(PetState::Idle);
        }
        let pace = self.w.pace();
        let pet = self.p();
        pet.x += f64::from(pet.facing) * speed * pace * dt;
        if let Some(crossing) = pet.crossing {
            return self.cross(crossing, time);
        }
        let goal = self.me().goal;
        // Đi tới sát con khác đang đứng hoặc đi trên cùng mặt đất: dừng lại chào nhau.
        if goal.is_none() && self.me().since_meet >= t::MEET_COOLDOWN {
            let friend = (0..self.w.pets.len()).find(|&j| j != self.i && meetable(self.me(), &self.w.pets[j]));
            if let Some(friend) = friend {
                self.meet(friend);
                return None;
            }
        }
        if let Some(goal @ (Goal::Climb { .. } | Goal::Peek { .. } | Goal::Nap { .. })) = goal {
            match self.goal_x(goal, &ground) {
                None => self.p().goal = None,
                Some(target) if (target - self.me().x) * f64::from(self.me().facing) <= 0.0 => {
                    self.p().x = target;
                    self.sync_mount();
                    return Some(self.arrive(goal));
                }
                Some(_) => {}
            }
        }
        if self.me().x >= ground.to {
            self.p().x = ground.to;
            self.sync_mount();
            return self.at_end(&ground, 1);
        }
        if self.me().x <= ground.from {
            self.p().x = ground.from;
            self.sync_mount();
            return self.at_end(&ground, -1);
        }
        self.sync_mount();
        (time >= self.me().planned).then_some(PetState::Idle)
    }

    /// Điểm chân cần đi tới cho `goal`, `None` nếu không còn tới được.
    fn goal_x(&self, goal: Goal, ground: &Ground) -> Option<f64> {
        match goal {
            Goal::Climb { id, side } => {
                let wall = self.w.terrain.wall(id, side)?;
                let x = wall.x + f64::from(side) * self.me().reach;
                (x >= ground.from && x <= ground.to).then_some(x)
            }
            Goal::Peek { dx } => self.mount_rect().map(|rect| clamp(rect.x + dx, ground.from, ground.to)),
            Goal::Nap { x } => ground.ledge.is_none().then(|| clamp(x, ground.from, ground.to)),
            _ => None,
        }
    }

    fn arrive(&mut self, goal: Goal) -> PetState {
        match goal {
            Goal::Climb { id, side } => match self.w.terrain.wall(id, side).cloned() {
                Some(wall) => self.climb_or_grab(&wall),
                None => PetState::Idle,
            },
            Goal::Nap { .. } => PetState::Sleep,
            _ => PetState::Idle,
        }
    }

    /// Hai con gặp nhau: đứng lại quay mặt vào nhau, có khi chào một câu, có khi rủ nhau đi cùng một đoạn.
    fn meet(&mut self, other: usize) {
        let time = self.w.rng.range(t::MEET_TIME.0, t::MEET_TIME.1);
        let facing = if self.w.pets[other].x > self.me().x { 1 } else { -1 };
        self.p().since_meet = 0.0;
        self.w.pets[other].since_meet = 0.0;
        self.pause(facing, time);
        // Con kia đứng lâu hơn một chút để còn kịp được rủ đi cùng (`next_activity` của con này).
        self.other(other).pause(-facing, time + 1.0);
        if self.w.rng.chance(t::MEET_CHAT_CHANCE) {
            let greetings = lines::greetings(self.w.lang);
            let text = self.w.rng.pick(&greetings).clone();
            self.w.chat(self.i, &text);
        }
        if self.w.rng.chance(t::STROLL_CHANCE) {
            let uid = self.w.pets[other].uid;
            self.p().buddy = Some(uid);
        }
    }

    /// Buồn ngủ dưới đất: có con đang ngủ dưới đất trong `NAP_RANGE` thì đi tới nằm cạnh nó (bên gần hơn, còn chỗ
    /// trong màn hình, không đè lên con ngủ khác), không thì ngủ luôn tại chỗ.
    fn nap(&mut self) -> PetState {
        let b = self.bounds();
        let me = self.me();
        let half = me.width / 2.0;
        let sleepers: Vec<&Pet> = (self.w.pets.iter().enumerate())
            .filter(|&(j, other)| j != self.i && other.state() == PetState::Sleep && other.mount.is_none())
            .map(|(_, other)| other)
            .collect();
        let mut best: Option<f64> = None;
        for other in &sleepers {
            let gap = ((me.width + other.width) / 2.0) * t::NAP_GAP;
            for x in [other.x - gap, other.x + gap] {
                if x < b.left + half || x > b.right - half || (x - me.x).abs() > t::NAP_RANGE {
                    continue;
                }
                if sleepers.iter().any(|s| (s.x - x).abs() < gap * 0.9) {
                    continue;
                }
                if best.is_none_or(|best| (x - me.x).abs() < (best - me.x).abs()) {
                    best = Some(x);
                }
            }
        }
        let Some(best) = best.filter(|best| (best - me.x).abs() >= 2.0) else {
            return PetState::Sleep;
        };
        let pet = self.p();
        pet.facing = sign(best - pet.x);
        pet.goal = Some(Goal::Nap { x: best });
        PetState::Walk
    }

    /// Đang sang màn hình bên cạnh. Ra hẳn khỏi mép thì chờ overlay sang bên kia (`Pet::leaving`), lâu quá thì quay
    /// lại; đã sang thì đi vào tới khi cả con nằm trong màn hình mới.
    fn cross(&mut self, crossing: Crossing, time: f64) -> Option<PetState> {
        let b = self.bounds();
        let pet = self.p();
        let half = pet.width / 2.0;
        // Không dừng lại giữa chừng lúc đang nằm vắt qua mép.
        pet.planned = pet.planned.max(time + 1.0);
        if crossing.phase == CrossPhase::In {
            // Vào hẳn rồi thì thôi; lỡ quay đầu ra lại thì vào luôn trong màn hình, không đi lạc ra ngoài. Màn hình đổi
            // tại chỗ (taskbar, DPI) lúc đang đi ra thì cũng thành `In` mà pet đang ở mép bên kia: vào luôn.
            if !self.entering(&crossing) || self.me().facing != crossing.dir {
                self.stay_inside();
            }
            return None;
        }
        if pet.facing != crossing.dir {
            pet.crossing = Some(Crossing::new(pet.facing, CrossPhase::In));
            return None;
        }
        let out = if crossing.dir > 0 { b.right + half } else { b.left - half };
        if (pet.x - out) * f64::from(crossing.dir) > 0.0 {
            pet.x = out;
        }
        if crossing.elapsed >= t::CROSS_TIMEOUT {
            pet.facing = if crossing.dir > 0 { -1 } else { 1 };
            pet.crossing = Some(Crossing::new(pet.facing, CrossPhase::In));
        }
        None
    }

    /// Đã sang màn hình mới mà còn nằm ngoài (hoặc vắt qua) mép phía đi vào.
    fn entering(&self, crossing: &Crossing) -> bool {
        let b = self.bounds();
        let me = self.me();
        let half = me.width / 2.0;
        if crossing.dir > 0 {
            me.x < b.left + half
        } else {
            me.x > b.right - half
        }
    }

    /// Đi tới đầu mép. Mặt đất, hoặc cửa sổ chạm cạnh màn hình: quay đầu như chạm tường. Mép cửa sổ: quay lại, ngồi
    /// mép hoặc xuống.
    fn at_end(&mut self, ground: &Ground, dir: Facing) -> Option<PetState> {
        let ledge = ground.ledge.as_ref();
        let b = self.bounds();
        let half = self.me().width / 2.0;
        let screen_edge = if dir > 0 { ground.to >= b.right - half } else { ground.from <= b.left + half };
        let goal = self.me().goal;
        if let (Some(ledge), Some(Goal::Down | Goal::Flee)) = (ledge, goal) {
            return Some(self.step_off(ledge.clone(), dir));
        }
        // Chạy trốn mà bị dồn vào mép màn hình: đứng lại, không quay đầu chạy về phía cửa sổ.
        if goal == Some(Goal::Flee) {
            return Some(PetState::Idle);
        }
        // Mép màn hình giáp màn hình khác: có khi đi sang bên đó (chỉ khi có một con, xem `World::wander`).
        if ledge.is_none()
            && goal.is_none()
            && self.w.wander()
            && self.w.exit(dir, self.me().y - 1.0)
            && self.w.rng.chance(t::CROSS_CHANCE)
        {
            self.p().crossing = Some(Crossing::new(dir, CrossPhase::Out));
            return None;
        }
        let ledge = match ledge {
            Some(ledge) if !screen_edge && goal.is_none() => ledge,
            _ => {
                self.p().goal = None;
                self.p().facing = -dir;
                return None;
            }
        };
        let roll = self.w.rng.next();
        if roll < t::EDGE_TURN {
            self.p().facing = -dir;
            return None;
        }
        if roll < t::EDGE_TURN + t::EDGE_PERCH {
            return Some(PetState::Perch);
        }
        Some(self.step_off(ledge.clone(), dir))
    }

    fn leave_perch(&mut self) -> PetState {
        let Some(ledge) = self.ground().ledge else {
            return PetState::Idle;
        };
        if self.w.rng.chance(t::PERCH_LEAVE_TURN) {
            self.p().facing = -self.me().facing;
            return PetState::Walk;
        }
        let facing = self.me().facing;
        self.step_off(ledge, facing)
    }

    /// Xuống khỏi mép cửa sổ về phía `dir`: nhảy xuống chỗ thấp hơn nếu với tới, không thì leo xuống (cạnh không bị
    /// che), không nữa thì nhảy khỏi mép.
    fn step_off(&mut self, ledge: Ledge, dir: Facing) -> PetState {
        self.p().goal = None;
        self.p().facing = dir;
        let x = self.me().x;
        let side = f64::from(dir);
        let below = jump_targets(self.w, self.me())
            .into_iter()
            .filter(|t| t.y > ledge.y + 8.0 && (t.x - x) * side > 0.0)
            .reduce(|a, b| if (b.x - x).abs() < (a.x - x).abs() { b } else { a });
        if let Some(nearest) = below {
            return self.start_jump(nearest);
        }
        let b = self.bounds();
        if let Some(wall) = self.w.terrain.wall(ledge.id, dir).filter(|wall| open_wall(wall, wall.to)).cloned() {
            let x = wall.x + side * self.me().reach;
            let half = self.me().width / 2.0;
            if x >= b.left + half && x <= b.right - half {
                return self.start_climb(&wall, 1);
            }
        }
        let pet = self.p();
        pet.skip_ledge = Some(ledge.id);
        pet.mount = None;
        pet.vx = side * t::STEP_OFF_SPEED;
        pet.vy = -t::STEP_OFF_HOP;
        PetState::Fall
    }

    /// Buồn ngủ trên cửa sổ: nhảy thẳng xuống taskbar nếu không quá cao, không thì đi ra đầu mép gần hơn.
    fn go_down(&mut self, ground: &Ground) -> PetState {
        if let Some(floor) = jump_targets(self.w, self.me()).into_iter().find(|t| t.id.is_none()) {
            return self.start_jump(floor);
        }
        let pet = self.p();
        pet.facing = if pet.x - ground.from <= ground.to - pet.x { -1 } else { 1 };
        pet.goal = Some(Goal::Down);
        PetState::Walk
    }

    fn walk_to(&mut self, x: f64) -> PetState {
        let Some(rect) = self.mount_rect().filter(|_| (x - self.me().x).abs() >= 1.0) else {
            return PetState::Idle;
        };
        let pet = self.p();
        pet.facing = sign(x - pet.x);
        pet.goal = Some(Goal::Peek { dx: x - rect.x });
        PetState::Walk
    }

    fn approach(&mut self, wall: &Wall) -> PetState {
        let x = wall.x + f64::from(wall.side) * self.me().reach;
        if (x - self.me().x).abs() < 1.0 {
            return self.climb_or_grab(wall);
        }
        let pet = self.p();
        pet.facing = sign(x - pet.x);
        pet.goal = Some(Goal::Climb { id: wall.id, side: wall.side });
        PetState::Walk
    }

    /// Đứng ở chân tường: cửa sổ chạm tới thì leo luôn, cửa sổ lơ lửng thì lấy đà nhảy lên bám.
    fn climb_or_grab(&mut self, wall: &Wall) -> PetState {
        let Some(rect) = self.window(wall.id) else {
            return PetState::Idle;
        };
        let me = self.me();
        if rect.y + rect.height + me.height * t::GRAB_DEPTH >= me.y - 1.0 {
            return self.start_climb(wall, -1);
        }
        let pet = self.p();
        pet.goal = None;
        pet.facing = if wall.side == 1 { -1 } else { 1 };
        pet.x = wall.x + f64::from(wall.side) * pet.reach;
        pet.jumping = Some(Jumping::new(Some(wall.id), Some(wall.side), 0.0, JumpPhase::Crouch));
        PetState::Jump
    }

    /// Bám vào cạnh `wall`, mặt quay vào tường, rồi leo lên (`dir` -1) hoặc xuống (1).
    fn start_climb(&mut self, wall: &Wall, dir: i8) -> PetState {
        let Some(rect) = self.window(wall.id) else {
            return PetState::Idle;
        };
        let pet = self.p();
        pet.goal = None;
        pet.facing = if wall.side == 1 { -1 } else { 1 };
        pet.x = wall.x + f64::from(wall.side) * pet.reach;
        if dir > 0 {
            pet.y = pet.y.max(rect.y + 1.0);
        }
        pet.climbing = Some(Climbing { side: wall.side, dir });
        pet.mount = Some(Mount { id: wall.id, dx: pet.x - rect.x, dy: pet.y - rect.y });
        PetState::Climb
    }

    fn climb(&mut self, dt: f64) -> Option<PetState> {
        let me = self.me();
        let (Some(climbing), Some(mount), Some(rect)) = (me.climbing, me.mount, self.mount_rect()) else {
            return Some(PetState::Fall);
        };
        let b = self.bounds();
        let pace = self.w.pace();
        self.p().y += f64::from(climbing.dir) * t::CLIMB_SPEED * pace * dt;
        let y = self.me().y;
        if climbing.dir < 0 {
            if y > rect.y {
                self.set_mount_dy(y - rect.y);
                return None;
            }
            // Lên tới đỉnh: nhún một cái qua mép, đáp lên cửa sổ, mặt quay vào trong. Đặt thẳng lên mép thì pet giật
            // ngang cả chục pixel (từ chỗ bám tường tới chỗ đứng).
            let Some(ledge) = self.w.terrain.ledge(mount.id).filter(|ledge| roomy(self.w, self.me(), ledge)).cloned()
            else {
                self.p().mount = None;
                return Some(PetState::Fall);
            };
            let pet = self.p();
            let margin = pet.width * t::LEDGE_MARGIN;
            let half = pet.width / 2.0;
            let x = clamp(
                if climbing.side < 0 { ledge.from + margin } else { ledge.to - margin },
                b.left + half,
                b.right - half,
            );
            pet.y = ledge.y;
            pet.jumping = Some(Jumping::new(Some(mount.id), None, x - rect.x, JumpPhase::Air));
            if !self.launch(0.0, Some(t::MANTLE_ARC)) {
                return Some(PetState::Fall);
            }
            return Some(PetState::Jump);
        }
        if y >= b.floor {
            self.p().y = b.floor;
            self.p().mount = None;
            return Some(PetState::Land);
        }
        // Hết cạnh cửa sổ mà chưa tới đất: buông tay.
        if y >= rect.y + rect.height {
            let pet = self.p();
            pet.mount = None;
            pet.vx = f64::from(climbing.side) * 30.0;
            pet.vy = 0.0;
            return Some(PetState::Fall);
        }
        self.set_mount_dy(y - rect.y);
        None
    }

    fn set_mount_dy(&mut self, dy: f64) {
        if let Some(mount) = &mut self.p().mount {
            mount.dy = dy;
        }
    }

    fn start_jump(&mut self, target: JumpTarget) -> PetState {
        let rect = target.id.and_then(|id| self.window(id));
        let pet = self.p();
        pet.goal = None;
        let offset = rect.map_or(target.x, |rect| target.x - rect.x);
        pet.jumping = Some(Jumping::new(target.id, None, offset, JumpPhase::Crouch));
        if (target.x - pet.x).abs() > 1.0 {
            pet.facing = sign(target.x - pet.x);
        }
        PetState::Jump
    }

    fn jump(&mut self, time: f64) -> Option<PetState> {
        let Some(jumping) = self.me().jumping else {
            return Some(PetState::Idle);
        };
        match jumping.phase {
            JumpPhase::Crouch => {
                if time < t::JUMP_CROUCH {
                    return None;
                }
                let launched = if jumping.grab.is_none() { self.launch(time, None) } else { self.leap(time) };
                if !launched {
                    return Some(PetState::Idle);
                }
                if let Some(jumping) = &mut self.p().jumping {
                    jumping.phase = JumpPhase::Air;
                }
                return None;
            }
            JumpPhase::Touchdown => {
                return (time - jumping.landed_at >= t::JUMP_TOUCHDOWN).then_some(PetState::Idle);
            }
            JumpPhase::Air => {}
        }
        let b = self.bounds();
        let from = jumping.launch;
        let dt = time - from.at;
        let pet = self.p();
        let half = pet.width / 2.0;
        let (prev_x, prev_y) = (pet.x, pet.y);
        pet.x = clamp(from.x + from.vx * dt, b.left + half, b.right - half);
        pet.y = from.y + from.vy * dt + (t::GRAVITY * dt * dt) / 2.0;
        pet.vx = from.vx;
        pet.vy = from.vy + t::GRAVITY * dt;
        if pet.vy <= 0.0 {
            return None;
        }
        if let (Some(grab), Some(id)) = (jumping.grab, jumping.id) {
            // Lên tới đỉnh cú nhảy: bám vào cạnh rồi leo. Cửa sổ biến mất giữa chừng thì rơi.
            if let Some(wall) = self.w.terrain.wall(id, grab).cloned() {
                return Some(self.start_climb(&wall, -1));
            }
            self.p().vx = 0.0;
            return Some(PetState::Fall);
        }
        let landing = landing_ledge(self.w, self.me(), prev_x, prev_y);
        let ground = landing.as_ref().map_or(b.floor, |l| l.ledge.y);
        if self.me().y < ground {
            return None;
        }
        let pet = self.p();
        if let Some(landing) = &landing {
            pet.x = landing.x;
        }
        pet.y = ground;
        let impact = pet.vy;
        self.settle(landing.as_ref().map(|l| &l.ledge));
        // Cửa sổ định đáp biến mất giữa chừng, rơi từ quá cao: choáng.
        if impact >= t::DIZZY_SPEED {
            return Some(PetState::Dizzy);
        }
        if let Some(jumping) = &mut self.p().jumping {
            jumping.phase = JumpPhase::Touchdown;
            jumping.landed_at = time;
        }
        None
    }

    /// Nhảy thẳng lên, đỉnh cú nhảy đúng chỗ bám cạnh cửa sổ lơ lửng.
    fn leap(&mut self, time: f64) -> bool {
        let Some(rect) = self.me().jumping.and_then(|j| j.id).and_then(|id| self.window(id)) else {
            return false;
        };
        let pet = self.p();
        let y = rect.y + rect.height + pet.height * t::GRAB_DEPTH;
        // Cửa sổ vừa bị kéo xuống, không cần nhảy nữa: lần sau leo thẳng.
        if y >= pet.y {
            return false;
        }
        let up = (2.0 * t::GRAVITY * (pet.y - y)).sqrt();
        pet.vx = 0.0;
        pet.vy = -up;
        let launch = Launch { at: time, x: pet.x, y: pet.y, vx: 0.0, vy: -up };
        if let Some(jumping) = &mut pet.jumping {
            jumping.launch = launch;
        }
        pet.skip_ledge = pet.mount.map(|m| m.id);
        pet.mount = None;
        true
    }

    /// Tính vận tốc lúc bật nhảy để rơi đúng chỗ đáp (cửa sổ có thể đã di chuyển từ lúc chọn). `arc`: đỉnh cao hơn
    /// điểm cao hơn giữa chỗ đứng và chỗ đáp, mặc định theo khoảng cách ngang.
    fn launch(&mut self, time: f64, arc: Option<f64>) -> bool {
        let Some(jumping) = self.me().jumping else {
            return false;
        };
        let b = self.bounds();
        let mut x = jumping.offset;
        let mut y = b.floor;
        if let Some(id) = jumping.id {
            let (Some(rect), Some(ledge)) = (self.window(id), self.w.terrain.ledge(id)) else {
                return false;
            };
            x += rect.x;
            y = ledge.y;
        }
        let pet = self.p();
        let dx = x - pet.x;
        let highest = pet.y.min(y);
        // Không nhảy cao quá trần, nhưng đỉnh luôn cao hơn cả chỗ đứng lẫn chỗ đáp.
        let peak = highest - arc.unwrap_or(t::JUMP_ARC + t::JUMP_ARC_PER_PX * dx.abs());
        let apex = highest.min(peak.max(b.top + pet.height * t::HEADROOM));
        let g = t::GRAVITY;
        let up = (2.0 * g * (pet.y - apex)).sqrt();
        let flight = up / g + ((2.0 * (y - apex)) / g).sqrt();
        pet.vx = if flight > 0.0 { dx / flight } else { 0.0 };
        pet.vy = -up;
        let launch = Launch { at: time, x: pet.x, y: pet.y, vx: pet.vx, vy: pet.vy };
        if dx.abs() > 1.0 {
            pet.facing = sign(dx);
        }
        // Không đáp lại chỗ vừa rời, trừ khi nhảy lên chính cửa sổ đang leo.
        let here = pet.mount.map(|m| m.id);
        pet.skip_ledge = if here == jumping.id { None } else { here };
        pet.mount = None;
        if let Some(jumping) = &mut pet.jumping {
            jumping.launch = launch;
        }
        true
    }

    fn hop(&mut self, time: f64) -> Option<PetState> {
        let y = self.ground().y;
        let k = (time / t::REACT_TIME).min(1.0);
        let pet = self.p();
        pet.y = y - t::HOP_HEIGHT * (std::f64::consts::PI * k).sin();
        if time < t::REACT_TIME {
            return None;
        }
        pet.y = y;
        if pet.hops > 0 {
            pet.hops -= 1;
            return Some(PetState::React);
        }
        // Nhảy xong thì đi hoặc chạy tiếp luôn: đứng lại ngay sau khi bị click trông như bị đơ.
        Some(if self.w.rng.chance(t::RUN_AFTER_POKE) { PetState::Run } else { PetState::Walk })
    }

    fn fall(&mut self, dt: f64) -> Option<PetState> {
        let b = self.bounds();
        let pet = self.p();
        let half = pet.width / 2.0;
        let (prev_x, prev_y) = (pet.x, pet.y);
        pet.vy += t::GRAVITY * dt;
        pet.x += pet.vx * dt;
        pet.y += pet.vy * dt;

        if pet.x < b.left + half {
            if !self.through_edge(-1) {
                let pet = self.p();
                pet.x = b.left + half;
                pet.vx = pet.vx.abs() * t::WALL_BOUNCE;
            }
        } else if pet.x > b.right - half {
            if !self.through_edge(1) {
                let pet = self.p();
                pet.x = b.right - half;
                pet.vx = -pet.vx.abs() * t::WALL_BOUNCE;
            }
        } else if pet.crossing.is_some_and(|c| c.phase == CrossPhase::In) {
            pet.crossing = None;
        }
        let pet = self.p();
        if pet.y - pet.height < b.top {
            pet.y = b.top + pet.height;
            pet.vy = pet.vy.abs() * t::WALL_BOUNCE;
        }
        if pet.vx.abs() > 20.0 {
            pet.facing = sign(pet.vx);
        }
        // Chỉ đáp lên mép cửa sổ khi đang rơi xuống qua nó, không đáp khi đang bay lên.
        let landing = if pet.vy > 0.0 { landing_ledge(self.w, self.me(), prev_x, prev_y) } else { None };
        let ground = landing.as_ref().map_or(b.floor, |l| l.ledge.y);
        if self.me().y < ground {
            return None;
        }

        // Bay qua mép sang màn hình khác, giữa thân đã qua mép mà chạm đất: đứng lại chờ overlay sang bên kia, không
        // kéo về. Chưa qua tới mép thì overlay không sang: đáp luôn trong màn hình này.
        let crossing = self.me().crossing;
        if crossing.is_some() && self.me().leaving(&b).is_some() {
            let pet = self.p();
            pet.y = ground;
            pet.vx = 0.0;
            pet.vy = 0.0;
            // Đã đứng trên đất: overlay sang thì đi bộ vào, không bay tiếp.
            if let Some(crossing) = &mut pet.crossing {
                crossing.vx = None;
            }
            return None;
        }
        // Đã sang màn hình mới mà chạm đất lúc còn ở ngoài mép: đi bộ vào, không hiện ra đột ngột trong màn hình.
        if let Some(crossing) = crossing.filter(|c| c.phase == CrossPhase::In) {
            if landing.is_none() && self.entering(&crossing) {
                let pet = self.p();
                pet.y = ground;
                pet.vx = 0.0;
                pet.vy = 0.0;
                pet.facing = crossing.dir;
                return Some(PetState::Walk);
            }
        }
        // Chạm đất hoặc mép cửa sổ. Đang bay vào từ mép sang màn hình mới thì đáp luôn trong màn hình này.
        if let Some(landing) = &landing {
            self.p().x = landing.x;
        }
        if self.me().crossing.is_some() {
            self.stay_inside();
        }
        let pet = self.p();
        pet.y = ground;
        let impact = pet.vy;
        let ledge = landing.as_ref().map(|l| &l.ledge);
        if impact >= t::DIZZY_SPEED {
            self.settle(ledge);
            return Some(PetState::Dizzy);
        }
        if impact >= t::BOUNCE_SPEED {
            pet.vy = -impact * t::RESTITUTION;
            pet.vx *= t::GROUND_FRICTION;
            return None;
        }
        self.settle(ledge);
        Some(PetState::Land)
    }

    /// Bị ném vào mép `side` của màn hình: mép đó giáp màn hình khác thì bay qua luôn, không nảy lại. Bay ra hẳn mà
    /// overlay chưa sang bên kia thì dừng ngay ngoài mép, rơi thẳng xuống chờ, lâu quá thì nảy lại vào trong. Overlay
    /// sang lúc pet còn đang rơi thì bay tiếp vào với vận tốc lúc bị giữ lại (`Crossing::vx`).
    fn through_edge(&mut self, side: Facing) -> bool {
        let me = self.me();
        match me.crossing {
            // Vừa sang màn hình mới, đang bay vào từ mép bên kia.
            Some(crossing) if crossing.phase == CrossPhase::In => return crossing.dir == -side,
            None => {
                if me.vx * f64::from(side) <= 0.0 || !self.w.exit(side, me.y - me.height / 2.0) {
                    return false;
                }
                self.p().crossing = Some(Crossing::new(side, CrossPhase::Out));
            }
            Some(crossing) if crossing.dir != side || crossing.elapsed >= t::CROSS_TIMEOUT => {
                self.stay_inside();
                return false;
            }
            Some(_) => {}
        }
        let b = self.bounds();
        let pet = self.p();
        let out = if side > 0 { b.right + pet.width / 2.0 } else { b.left - pet.width / 2.0 };
        if (pet.x - out) * f64::from(side) > 0.0 {
            pet.x = out;
            let vx = pet.vx;
            if let Some(crossing) = &mut pet.crossing {
                crossing.vx.get_or_insert(vx);
            }
            pet.vx = 0.0;
            pet.vy = 0.0;
        }
        true
    }

    /// Dừng hẳn trên `ledge` (hoặc mặt đất nếu `None`).
    fn settle(&mut self, ledge: Option<&Ledge>) {
        let rect = ledge.and_then(|ledge| self.window(ledge.id));
        let pet = self.p();
        pet.vx = 0.0;
        pet.vy = 0.0;
        pet.skip_ledge = None;
        pet.mount = match (ledge, rect) {
            (Some(ledge), Some(rect)) => Some(Mount { id: ledge.id, dx: pet.x - rect.x, dy: 0.0 }),
            _ => None,
        };
    }

    /// Pet vừa đi trên mép cửa sổ: ghi lại chỗ đứng so với cửa sổ để lần sau cửa sổ di chuyển thì đi theo.
    fn sync_mount(&mut self) {
        let Some(rect) = self.mount_rect() else {
            return;
        };
        let pet = self.p();
        if let Some(mount) = &mut pet.mount {
            mount.dx = pet.x - rect.x;
        }
    }
}
