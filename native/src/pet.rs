//! Hành vi tối thiểu cho PoC, chưa phải sim thật (packages/sim): đứng, đi, chạy, bị kéo, rơi, đáp. Đủ để đo chi phí
//! vẽ ở nhịp 30 lần/giây và nhịp màn hình. Số liệu lấy từ `TUNING` (packages/sim/src/tuning.ts), đơn vị CSS pixel
//! và giây; `World::px` đổi sang pixel vật lý.

const WALK_SPEED: f32 = 30.0;
const RUN_SPEED: f32 = 110.0;
const IDLE_TIME: (f32, f32) = (2.0, 5.0);
const WALK_TIME: (f32, f32) = (2.0, 6.0);
const RUN_TIME: (f32, f32) = (1.0, 2.5);
/// Hết lượt đứng thì đi (còn lại là chạy) với xác suất này.
const WALK_CHANCE: f32 = 0.7;
const TURN_CHANCE: f32 = 0.4;
const GRAVITY: f32 = 2000.0;
const BOUNCE_SPEED: f32 = 350.0;
const RESTITUTION: f32 = 0.4;
const GROUND_FRICTION: f32 = 0.6;
const WALL_BOUNCE: f32 = 0.5;
const MAX_THROW_SPEED: f32 = 3000.0;
const LAND_TIME: f32 = 0.25;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Idle,
    Walk,
    Run,
    Dragged,
    Fall,
    Land,
}

impl State {
    pub fn animation(self) -> &'static str {
        match self {
            State::Idle => "idle",
            State::Walk => "walk",
            State::Run => "run",
            State::Dragged => "dragged",
            State::Fall => "fall",
            State::Land => "land",
        }
    }

    /// Chuyển động nhanh: bước và vẽ theo nhịp màn hình thay vì 30 lần/giây (`FAST_STATES` trong
    /// desktop/src/main.ts, thêm `run`).
    pub fn fast(self) -> bool {
        matches!(self, State::Run | State::Dragged | State::Fall)
    }
}

/// `Auto`: tự đổi đứng/đi/chạy như pet thật. Còn lại giữ mọi pet ở một trạng thái, để đo từng nhịp vẽ.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Auto,
    Idle,
    Walk,
    Run,
}

impl Mode {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "auto" => Some(Mode::Auto),
            "idle" => Some(Mode::Idle),
            "walk" => Some(Mode::Walk),
            "run" => Some(Mode::Run),
            _ => None,
        }
    }

    fn forced(self) -> Option<State> {
        match self {
            Mode::Auto => None,
            Mode::Idle => Some(State::Idle),
            Mode::Walk => Some(State::Walk),
            Mode::Run => Some(State::Run),
        }
    }
}

/// Chỗ pet được đi, theo pixel của overlay: `left`..`right` cho điểm giữa chân, `ground` là mặt đất.
pub struct World {
    pub left: f32,
    pub right: f32,
    pub ground: f32,
    /// Pixel vật lý cho một CSS pixel.
    pub px: f32,
    /// Tốc độ đi trong Cài đặt.
    pub pace: f32,
}

pub struct Pet {
    /// Giữa chân, pixel của overlay.
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    pub facing_right: bool,
    pub state: State,
    /// Giây kể từ lúc vào trạng thái hiện tại, cũng là thời gian của animation.
    pub time: f32,
    until: f32,
    rng: u64,
}

impl Pet {
    pub fn new(x: f32, ground: f32, seed: u64) -> Self {
        let mut pet = Pet {
            x,
            y: ground,
            vx: 0.0,
            vy: 0.0,
            facing_right: seed.is_multiple_of(2),
            state: State::Idle,
            time: 0.0,
            until: 0.0,
            rng: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
        };
        pet.until = pet.between(IDLE_TIME);
        pet
    }

    pub fn step(&mut self, dt: f32, world: &World, mode: Mode) {
        self.time += dt;
        match self.state {
            State::Dragged => {}
            State::Fall => self.fall(dt, world),
            State::Land => {
                if self.time >= self.until {
                    self.choose(mode);
                }
            }
            State::Idle | State::Walk | State::Run => {
                if let Some(state) = mode.forced() {
                    if state != self.state {
                        self.enter(state, f32::INFINITY);
                    }
                }
                let speed = match self.state {
                    State::Walk => WALK_SPEED,
                    State::Run => RUN_SPEED,
                    _ => 0.0,
                } * world.px
                    * world.pace;
                if speed > 0.0 {
                    self.x += if self.facing_right { speed } else { -speed } * dt;
                    if self.x <= world.left {
                        self.x = world.left;
                        self.facing_right = true;
                    } else if self.x >= world.right {
                        self.x = world.right;
                        self.facing_right = false;
                    }
                }
                if self.time >= self.until {
                    self.choose(mode);
                }
            }
        }
    }

    pub fn grab(&mut self) {
        self.enter(State::Dragged, f32::INFINITY);
    }

    pub fn drag_to(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
    }

    /// Buông tay với vận tốc của con trỏ (pixel/giây).
    pub fn release(&mut self, vx: f32, vy: f32, world: &World) {
        let max = MAX_THROW_SPEED * world.px;
        let speed = vx.hypot(vy);
        let k = if speed > max { max / speed } else { 1.0 };
        self.vx = vx * k;
        self.vy = vy * k;
        self.enter(State::Fall, f32::INFINITY);
    }

    /// Thả rơi từ (`x`, `y`).
    pub fn drop_at(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        self.vx = 0.0;
        self.vy = 0.0;
        self.enter(State::Fall, f32::INFINITY);
    }

    fn fall(&mut self, dt: f32, world: &World) {
        self.vy += GRAVITY * world.px * dt;
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        if self.x < world.left {
            self.x = world.left;
            self.vx = -self.vx * WALL_BOUNCE;
        } else if self.x > world.right {
            self.x = world.right;
            self.vx = -self.vx * WALL_BOUNCE;
        }
        if self.y >= world.ground {
            self.y = world.ground;
            if self.vy > BOUNCE_SPEED * world.px {
                self.vy = -self.vy * RESTITUTION;
                self.vx *= GROUND_FRICTION;
            } else {
                self.vx = 0.0;
                self.vy = 0.0;
                self.enter(State::Land, LAND_TIME);
            }
        }
    }

    fn choose(&mut self, mode: Mode) {
        if let Some(state) = mode.forced() {
            self.enter(state, f32::INFINITY);
            return;
        }
        if matches!(self.state, State::Idle | State::Land) {
            let state = if self.random() < WALK_CHANCE { State::Walk } else { State::Run };
            if self.random() < TURN_CHANCE {
                self.facing_right = !self.facing_right;
            }
            let range = if state == State::Walk { WALK_TIME } else { RUN_TIME };
            let duration = self.between(range);
            self.enter(state, duration);
        } else {
            let duration = self.between(IDLE_TIME);
            self.enter(State::Idle, duration);
        }
    }

    fn enter(&mut self, state: State, duration: f32) {
        self.state = state;
        self.time = 0.0;
        self.until = duration;
    }

    fn between(&mut self, (min, max): (f32, f32)) -> f32 {
        min + (max - min) * self.random()
    }

    /// xorshift64*, đủ cho PoC.
    fn random(&mut self) -> f32 {
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        let value = self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (value >> 40) as f32 / (1u64 << 24) as f32
    }
}
