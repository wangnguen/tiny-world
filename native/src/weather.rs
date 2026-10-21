//! Thời tiết chỉ quanh một pet (`weather.ts`): lớp nhỏ đi theo pet, vừa với thân thật của con đó; hạt mờ dần ra hai
//! bên và lên trên nên trông như chỉ có quanh pet. Mưa (giọt bắn lên lúc chạm đất), tuyết, cánh hoa, sương mù, sấm;
//! nắng (tia nắng, lấp lánh), trời quang ban đêm (sao, sao băng), nhiều mây (mây pixel trôi trên đầu); nóng thì hơi
//! nóng bốc lên hai bên thân, lạnh thì pet thở ra hơi nước từ miệng. Pet đang ngủ hay tắt hiệu ứng thì ẩn hẳn.

use crate::gfx::{scale, Canvas, Color, Gfx, Layer};
use crate::random::{between, random};
use crate::sim::weather::Warmth;
use crate::sim::{Point, Rect, Sky};
use anyhow::Result;
use std::f64::consts::PI;
use windows::Win32::Graphics::Direct2D::ID2D1StrokeStyle;
use windows_numerics::Matrix3x2;

/// Mưa, tuyết, cánh hoa rơi nhanh, khói thở bay nhanh: vẽ 12 lần mỗi giây. Nắng, sao, mây, sương, hơi nóng trôi
/// chậm: 8 lần là đủ, đỡ tốn CPU.
const FAST_FPS: f64 = 12.0;
const SLOW_FPS: f64 = 8.0;
/// Vùng có thời tiết quanh pet: rộng gấp chừng này lần cạnh lớn của thân, phía trên đầu chừa chừng này lần (cho mây,
/// sao). Mép dưới là mặt đất, không vẽ gì dưới đó.
const AREA_WIDTH: f64 = 3.0;
const ABOVE_HEAD: f64 = 1.0;
/// Chớp lúc có sấm kéo dài chừng này ms.
const FLASH_MS: f64 = 160.0;
/// Trời quang ban đêm: sao băng cách nhau ngẫu nhiên trong khoảng này (ms), bay qua trong chừng này ms.
const METEOR_GAP: (f64, f64) = (7_000.0, 16_000.0);
const METEOR_MS: f64 = 900.0;

/// Cách hạt di chuyển: rơi từ trên xuống, trôi ngang từ trái sang phải, đứng yên lấp lánh rồi hiện chỗ khác.
#[derive(Clone, Copy, PartialEq)]
enum Motion {
    Fall,
    Drift,
    Twinkle,
}

/// Chỗ hạt hiện: cả vùng, quanh chân (sương), trên đầu (sao, mây), quanh thân (lấp lánh lúc nắng).
#[derive(Clone, Copy, PartialEq)]
enum Zone {
    All,
    Feet,
    Sky,
    Around,
}

#[derive(Clone, Copy, Default)]
struct Particle {
    x: f64,
    y: f64,
    /// Tốc độ rơi hay trôi ngang (px/s).
    speed: f64,
    /// Cỡ: chiều dài hạt mưa, vệt sương, cạnh bông tuyết/cánh hoa, tia lấp lánh, ô pixel của mây (CSS pixel).
    size: f64,
    /// Pha đung đưa (tuyết, cánh hoa), chọn màu cánh hoa, hình đám mây.
    phase: f64,
    /// Hạt lấp lánh: đã hiện bao lâu, hiện tổng cộng bao lâu (giây).
    age: f64,
    life: f64,
}

struct Kind {
    motion: Motion,
    zone: Zone,
    count: usize,
    speed: (f64, f64),
    size: (f64, f64),
    life: (f64, f64),
}

fn kind(sky: Sky) -> Kind {
    let k = |motion, zone, count, speed, size, life| Kind { motion, zone, count, speed, size, life };
    match sky {
        Sky::Rain => k(Motion::Fall, Zone::All, 16, (380.0, 460.0), (7.0, 10.0), (0.0, 0.0)),
        Sky::Storm => k(Motion::Fall, Zone::All, 24, (460.0, 560.0), (8.0, 12.0), (0.0, 0.0)),
        Sky::Snow => k(Motion::Fall, Zone::All, 16, (22.0, 42.0), (1.5, 3.5), (0.0, 0.0)),
        Sky::Petals => k(Motion::Fall, Zone::All, 7, (22.0, 38.0), (3.0, 5.0), (0.0, 0.0)),
        Sky::Fog => k(Motion::Drift, Zone::Feet, 14, (8.0, 18.0), (8.0, 18.0), (0.0, 0.0)),
        Sky::Cloudy => k(Motion::Drift, Zone::Sky, 3, (5.0, 9.0), (0.0, 0.0), (0.0, 0.0)),
        Sky::Sunny => k(Motion::Twinkle, Zone::Around, 7, (0.0, 0.0), (2.0, 4.0), (1.2, 2.2)),
        Sky::Clear => k(Motion::Twinkle, Zone::Sky, 8, (0.0, 0.0), (1.0, 3.0), (2.5, 4.5)),
    }
}

fn fast(sky: Sky) -> bool {
    matches!(sky, Sky::Rain | Sky::Storm | Sky::Snow | Sky::Petals)
}

/// Gió thổi mưa xiên: đi xuống 1 px thì sang phải chừng này px.
fn wind(sky: Sky) -> f64 {
    match sky {
        Sky::Rain => 0.08,
        Sky::Storm => 0.28,
        _ => 0.0,
    }
}

const PETAL_COLORS: [u32; 3] = [0xffb7d0, 0xff9ec2, 0xffd1e0];

/// Mây pixel: `X` thân, `S` bóng phía dưới, chấm là trong suốt; vẽ thêm viền quanh.
const CLOUD_SHAPES: [&[&str]; 3] = [
    &["....XXX.....", "..XXXXXXX.X.", ".XXXXXXXXXXX", "XXXXXXXXXXXX", ".SSSSSSSSSS."],
    &["...XX..XXX....", "..XXXXXXXXXX..", ".XXXXXXXXXXXX.", "XXXXXXXXXXXXXX", ".SSSSSSSSSSSS."],
    &["..XXX....", ".XXXXXXX.", "XXXXXXXXX", ".SSSSSSS."],
];
const CLOUD_BODY: u32 = 0xf4f7fd;
const CLOUD_SHADE: u32 = 0xc3cfe6;
const CLOUD_EDGE: u32 = 0x7b8db3;
/// Mỗi ô pixel của mây to bằng chừng này phần cạnh lớn của thân pet (ít nhất 2 px).
const CLOUD_CELL: f64 = 1.0 / 32.0;

/// Tia nắng xiên từ trên trái xuống: đi xuống 1 px thì sang phải chừng này px.
const SUN_SLANT: f64 = 0.45;

/// Hơi nóng bốc lên hai bên thân: số vệt, tốc độ bốc lên (px/s), dài (phần chiều cao thân).
const HEAT_COUNT: usize = 6;
const HEAT_SPEED: (f64, f64) = (16.0, 26.0);
const HEAT_SIZE: (f64, f64) = (0.17, 0.25);

/// Một giọt mưa bắn lên lúc chạm đất.
#[derive(Clone, Copy)]
struct Splash {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}
const SPLASH_GRAVITY: f64 = 420.0;

/// Hơi thở lúc lạnh: mỗi lần thở ra phả hơi trong chừng này ms; thở cách nhau chừng này ms lúc đứng yên, ngắn dần khi
/// đi, chạy (thở dồn).
const EXHALE_MS: f64 = 600.0;
const BREATH_REST_MS: f64 = 3_400.0;
const BREATH_RUN_MS: f64 = 1_400.0;
/// Mỗi giây thở ra chừng này hạt hơi nước.
const VAPOR_RATE: f64 = 24.0;

/// Một hạt hơi nước thở ra, toạ độ trong vùng vẽ.
#[derive(Clone, Copy)]
struct Vapor {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    age: f64,
    life: f64,
    /// Bán kính lúc mới thở ra và lúc tan hẳn (CSS pixel).
    from: f64,
    to: f64,
    seed: f64,
}

/// Miệng pet (chỗ thở ra khói), CSS pixel của overlay, và hướng mặt (1: sang phải màn hình).
pub type Mouth = (Point, i8);

pub struct WeatherEffect {
    layer: Layer,
    stroke: ID2D1StrokeStyle,
    sky: Option<Sky>,
    warmth: Option<Warmth>,
    particles: Vec<Particle>,
    splashes: Vec<Splash>,
    heat: Vec<Particle>,
    vapor: Vec<Vapor>,
    /// Cỡ vùng đang vẽ (CSS pixel).
    width: f64,
    height: f64,
    /// Góc trên trái vùng vẽ (CSS pixel của overlay).
    left: f64,
    top: f64,
    /// Thân pet trong vùng vẽ: đỉnh đầu, hai mép, chiều cao (CSS pixel).
    head_top: f64,
    body_left: f64,
    body_right: f64,
    body_height: f64,
    /// Cạnh lớn của thân pet (CSS pixel).
    unit: f64,
    last_draw: f64,
    flash_until: f64,
    /// Sao băng đang bay từ lúc này (ms), lần tới lúc nào; 0 là chưa hẹn.
    meteor_at: f64,
    meteor_next: f64,
    meteor_from: (f64, f64),
    meteor_dir: f64,
    /// Thở: lần thở ra tới lúc nào, lần đang thở ra từ lúc nào tới lúc nào (ms), số hạt hơi nước còn nợ.
    next_breath: f64,
    exhale_from: f64,
    exhale_until: f64,
    vapor_due: f64,
    shown: bool,
}

impl WeatherEffect {
    pub fn new(gfx: &Gfx, mut layer: Layer) -> Result<Self> {
        layer.set_visible(false)?;
        Ok(Self {
            layer,
            stroke: gfx.round_stroke()?,
            sky: None,
            warmth: None,
            particles: Vec::new(),
            splashes: Vec::new(),
            heat: Vec::new(),
            vapor: Vec::new(),
            width: 0.0,
            height: 0.0,
            left: 0.0,
            top: 0.0,
            head_top: 0.0,
            body_left: 0.0,
            body_right: 0.0,
            body_height: 0.0,
            unit: 0.0,
            last_draw: 0.0,
            flash_until: 0.0,
            meteor_at: f64::NEG_INFINITY,
            meteor_next: 0.0,
            meteor_from: (0.0, 0.0),
            meteor_dir: 1.0,
            next_breath: 0.0,
            exhale_from: 0.0,
            exhale_until: 0.0,
            vapor_due: 0.0,
            shown: false,
        })
    }

    /// Có đang vẽ (vòng lặp phải thức dậy đủ `frame_ms`).
    pub fn animating(&self) -> bool {
        self.shown
    }

    /// Vòng lặp phải vẽ lại trong vòng chừng này ms; vô cùng nếu đang ẩn.
    pub fn frame_ms(&self) -> f64 {
        if !self.shown {
            return f64::INFINITY;
        }
        let fast = self.sky.is_some_and(fast) || self.warmth == Some(Warmth::Cold);
        1000.0 / if fast { FAST_FPS } else { SLOW_FPS }
    }

    /// Đổi thời tiết; `None` là tắt hiệu ứng.
    pub fn set_sky(&mut self, sky: Option<Sky>) {
        if sky == self.sky {
            return;
        }
        self.sky = sky;
        self.particles.clear();
        self.splashes.clear();
        self.meteor_next = 0.0;
    }

    /// Nóng (hơi nóng bốc lên), lạnh (thở ra hơi nước) hay bình thường.
    pub fn set_warmth(&mut self, warmth: Option<Warmth>) {
        if warmth == self.warmth {
            return;
        }
        self.warmth = warmth;
        self.heat.clear();
        self.vapor.clear();
        self.next_breath = 0.0;
        self.exhale_until = 0.0;
    }

    /// Sấm: chớp sáng một cái.
    pub fn flash(&mut self, now: f64) {
        self.flash_until = now + FLASH_MS;
    }

    /// Đặt vùng thời tiết quanh thân pet `body` (`PetView::body`), vẽ lại nếu tới lượt. `visible`: pet đang thức (ngủ
    /// thì không vẽ, vòng lặp được dừng). `mouth`: miệng ở frame đang vẽ. `effort`: 0 đứng yên, 1 chạy (thở dồn hơn).
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        gfx: &Gfx,
        body: Rect,
        now: f64,
        visible: bool,
        mouth: Option<Mouth>,
        effort: f64,
        px: f64,
    ) -> Result<()> {
        let show = visible && (self.sky.is_some() || self.warmth.is_some());
        if show != self.shown {
            self.shown = show;
            self.layer.set_visible(show)?;
        }
        if !show {
            return Ok(());
        }
        let unit = body.width.max(body.height);
        let width = (unit * AREA_WIDTH).round();
        let height = (body.height + unit * ABOVE_HEAD).round();
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.particles.clear();
            self.splashes.clear();
            self.heat.clear();
            self.vapor.clear();
        }
        let resized = self.layer.resize(gfx, (width * px).round() as u32, (height * px).round() as u32)?;
        self.unit = unit;
        self.body_height = body.height;
        self.left = (body.x + body.width / 2.0 - width / 2.0).round();
        // Mép dưới vùng vẽ là mặt đất (chân pet).
        self.top = (body.y + body.height).round() - height;
        self.head_top = body.y - self.top;
        self.body_left = body.x - self.left;
        self.body_right = self.body_left + body.width;
        self.layer.move_to((self.left * px).round() as f32, (self.top * px).round() as f32)?;
        if !resized && now - self.last_draw < self.frame_ms() {
            return Ok(());
        }
        let dt = ((now - self.last_draw) / 1000.0).min(0.2);
        self.last_draw = now;
        self.step(dt, now, mouth, effort);
        self.draw(gfx, now, px)
    }

    /// Khoảng [trên, dưới] theo chiều dọc của chỗ hạt hiện.
    fn zone(&self, zone: Zone) -> (f64, f64) {
        let head = self.head_top;
        match zone {
            Zone::Feet => (self.height - self.body_height * 0.5, self.height - 2.0),
            Zone::Sky => (head * 0.06, head * 0.8),
            Zone::Around => (head * 0.6, self.height - 4.0),
            Zone::All => (0.0, self.height),
        }
    }

    /// Hạt mới; `anywhere`: ở chỗ bất kỳ (lúc mới bật), không thì ở chỗ bắt đầu (trên cùng, mép trái).
    fn spawn(&self, sky: Sky, anywhere: bool) -> Particle {
        let kind = kind(sky);
        let life = between(kind.life);
        let mut p = Particle {
            x: random() * self.width,
            y: 0.0,
            speed: between(kind.speed),
            size: if sky == Sky::Cloudy { (self.unit * CLOUD_CELL).round().max(2.0) } else { between(kind.size) },
            phase: random() * PI * 2.0,
            age: if anywhere { random() * life } else { 0.0 },
            life,
        };
        let (top, bottom) = self.zone(kind.zone);
        // Mây nằm hẳn trên đầu, không đè xuống đầu pet.
        let lowest =
            if sky == Sky::Cloudy { top.max(self.head_top - self.drift_size(sky, &p).1 - 2.0) } else { bottom };
        p.y = top + random() * (lowest - top);
        if kind.motion == Motion::Fall {
            if !anywhere {
                p.y = -random() * 20.0;
            }
            // Mưa xiên: hạt bắt đầu lệch sang trái để rơi xiên vào vùng vẽ.
            p.x -= wind(sky) * self.height * random();
        }
        if kind.motion == Motion::Drift && !anywhere {
            p.x = -self.drift_size(sky, &p).0 - random() * 20.0;
        }
        p
    }

    /// Cỡ của hạt trôi ngang: đám mây thì theo hình mây, vệt sương thì dài `size`.
    fn drift_size(&self, sky: Sky, p: &Particle) -> (f64, f64) {
        if sky != Sky::Cloudy {
            return (p.size, 2.0);
        }
        let rows = CLOUD_SHAPES[p.phase.floor() as usize % CLOUD_SHAPES.len()];
        ((rows[0].len() as f64 + 2.0) * p.size, (rows.len() as f64 + 2.0) * p.size)
    }

    fn step(&mut self, dt: f64, now: f64, mouth: Option<Mouth>, effort: f64) {
        if let Some(sky) = self.sky {
            let kind = kind(sky);
            while self.particles.len() < kind.count {
                let p = self.spawn(sky, true);
                self.particles.push(p);
            }
            let sway = match sky {
                Sky::Snow => 10.0,
                Sky::Petals => 18.0,
                _ => 0.0,
            };
            let wind = wind(sky);
            for i in 0..self.particles.len() {
                let mut p = self.particles[i];
                match kind.motion {
                    Motion::Fall => {
                        p.y += p.speed * dt;
                        p.x += p.speed * wind * dt;
                        if sway != 0.0 {
                            p.x += (now / 700.0 + p.phase).sin() * sway * dt;
                        }
                        // Chạm mặt đất thì hết (mưa thì bắn lên vài giọt), rơi lại từ trên.
                        if p.y >= self.height {
                            if wind > 0.0 && self.fade(p.x, self.height, true) > 0.2 {
                                self.splash(p.x, if sky == Sky::Storm { 3 } else { 2 });
                            }
                            p = self.spawn(sky, false);
                        }
                    }
                    Motion::Drift => {
                        p.x += p.speed * dt;
                        // Trôi hết sang mép phải thì hiện lại bên trái.
                        if p.x >= self.width {
                            p = self.spawn(sky, false);
                        }
                    }
                    Motion::Twinkle => {
                        p.age += dt;
                        // Lấp lánh xong thì hiện ở chỗ khác.
                        if p.age >= p.life {
                            p = self.spawn(sky, false);
                        }
                    }
                }
                self.particles[i] = p;
            }
            for s in &mut self.splashes {
                s.x += s.vx * dt;
                s.y += s.vy * dt;
                s.vy += SPLASH_GRAVITY * dt;
            }
            let height = self.height;
            self.splashes.retain(|s| s.y < height);
            if sky == Sky::Clear {
                if self.meteor_next == 0.0 {
                    self.meteor_next = now + between(METEOR_GAP);
                } else if now >= self.meteor_next {
                    self.meteor_next = now + between(METEOR_GAP);
                    self.meteor_at = now;
                    self.meteor_dir = if random() < 0.5 { 1.0 } else { -1.0 };
                    let x = if self.meteor_dir > 0.0 { 0.1 + random() * 0.3 } else { 0.6 + random() * 0.3 };
                    self.meteor_from = (self.width * x, self.head_top * (0.05 + random() * 0.25));
                }
            }
        }

        if self.warmth == Some(Warmth::Hot) {
            while self.heat.len() < HEAT_COUNT {
                let p = self.spawn_heat(true);
                self.heat.push(p);
            }
            for i in 0..self.heat.len() {
                self.heat[i].y -= self.heat[i].speed * dt;
                // Bốc lên tới ngang đầu pet thì tan, bốc lại từ mặt đất.
                if self.heat[i].y < self.head_top {
                    self.heat[i] = self.spawn_heat(false);
                }
            }
        }

        if self.warmth == Some(Warmth::Cold) {
            self.breathe(dt, now, mouth, effort);
        }
    }

    /// Giọt mưa chạm đất ở `x`: bắn lên `count` giọt nhỏ rồi rơi lại.
    fn splash(&mut self, x: f64, count: usize) {
        for i in 0..count {
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            self.splashes.push(Splash {
                x,
                y: self.height - 1.0,
                vx: side * (15.0 + random() * 25.0),
                vy: -(45.0 + random() * 40.0),
            });
        }
    }

    /// Thở ra hơi nước như người lúc trời lạnh: mỗi lần thở phả ra một luồng hạt hơi nước từ miệng, bay về phía trước
    /// rồi chậm dần, bốc lên, nở to và tan. Đi, chạy thì thở dồn hơn.
    fn breathe(&mut self, dt: f64, now: f64, mouth: Option<Mouth>, effort: f64) {
        // Vóc to thì hơi thở to, bay xa.
        let scale = self.body_height / 90.0;
        for v in &mut self.vapor {
            v.age += dt;
            let drag = (-1.9 * dt).exp();
            v.vx *= drag;
            v.vy = v.vy * drag - 7.0 * scale * dt;
            v.x += (v.vx + (v.age * 5.0 + v.seed).sin() * 4.0 * scale) * dt;
            v.y += v.vy * dt;
        }
        self.vapor.retain(|v| v.age < v.life);
        let Some((mouth, dir)) = mouth else {
            return;
        };
        if now >= self.next_breath {
            let gap = BREATH_REST_MS - (BREATH_REST_MS - BREATH_RUN_MS) * effort;
            self.next_breath = now + gap * (0.88 + random() * 0.24);
            self.exhale_from = now;
            self.exhale_until = now + EXHALE_MS * (1.0 - 0.35 * effort);
        }
        if now >= self.exhale_until {
            self.vapor_due = 0.0;
            return;
        }
        // Đầu hơi thở mạnh, cuối hơi thở yếu dần.
        let progress = (now - self.exhale_from) / (self.exhale_until - self.exhale_from);
        self.vapor_due += VAPOR_RATE * dt;
        let (x, y) = (mouth.x - self.left, mouth.y - self.top);
        while self.vapor_due >= 1.0 {
            self.vapor_due -= 1.0;
            let angle = (-8.0 + random() * 30.0) * PI / 180.0;
            let speed = (52.0 + random() * 20.0) * scale * (1.0 - 0.5 * progress);
            self.vapor.push(Vapor {
                x: x + (random() - 0.5) * 2.0,
                y: y + (random() - 0.5) * 2.0,
                vx: f64::from(dir) * speed * angle.cos(),
                vy: -speed * angle.sin() - 3.0 * scale,
                age: 0.0,
                life: 1.3 + random() * 0.6,
                from: 1.0 + 0.015 * self.body_height,
                to: (0.1 + random() * 0.05) * self.body_height,
                seed: random() * PI * 2.0,
            });
        }
    }

    /// Vệt hơi nóng mới: hai bên thân pet (không đè lên thân), từ mặt đất bốc lên.
    fn spawn_heat(&self, anywhere: bool) -> Particle {
        let reach = (self.body_right - self.body_left) * 0.5 + 4.0;
        let x = if random() < 0.5 {
            self.body_left - 3.0 - random() * reach
        } else {
            self.body_right + 3.0 + random() * reach
        };
        Particle {
            x,
            y: if anywhere {
                self.head_top + random() * (self.height - self.head_top)
            } else {
                self.height + random() * 12.0
            },
            speed: between(HEAT_SPEED),
            size: (between(HEAT_SIZE) * self.body_height).max(10.0),
            phase: random() * PI * 2.0,
            age: 0.0,
            life: 0.0,
        }
    }

    /// Hạt ở giữa vùng (quanh pet) rõ nhất, mờ dần ra hai bên; `from_top`: mờ dần cả lên trên.
    fn fade(&self, x: f64, y: f64, from_top: bool) -> f64 {
        let dx = (x - self.width / 2.0) / (self.width / 2.0);
        let across = (1.0 - dx * dx).max(0.0);
        let down = if from_top { (y / self.height * 2.5).min(1.0) } else { 1.0 };
        across * down
    }

    fn draw(&self, gfx: &Gfx, now: f64, px: f64) -> Result<()> {
        self.layer.draw(|canvas| {
            let base = scale(px as f32, px as f32);
            canvas.transform(&base);
            let sky = self.sky;
            if sky == Some(Sky::Sunny) {
                self.draw_rays(canvas, now)?;
            }
            if sky == Some(Sky::Clear) {
                self.draw_meteor(canvas, now)?;
            }
            if let Some(sky) = sky {
                for p in &self.particles {
                    self.draw_particle(canvas, &base, sky, p, now)?;
                }
            }
            for s in &self.splashes {
                let alpha = 0.8 * self.fade(s.x, self.height, true);
                if alpha <= 0.02 {
                    continue;
                }
                let brush = canvas.brush(Color::rgba(110, 160, 235, alpha))?;
                canvas.fill_rect(s.x.round() as f32, s.y.round() as f32, 1.5, 1.5, &brush);
            }
            if sky == Some(Sky::Storm) && now < self.flash_until {
                let left = ((self.flash_until - now) / FLASH_MS) as f32;
                let cy = (self.height - self.body_height * 0.5) as f32;
                let (w, h) = (self.width as f32, self.height as f32);
                let glow = canvas.radial(
                    w / 2.0,
                    cy,
                    w / 2.0,
                    &[(0.0, Color::rgba(255, 255, 235, f64::from(0.55 * left))), (1.0, Color::rgba(255, 255, 235, 0.0))],
                )?;
                canvas.fill_rect(0.0, 0.0, w, h, &glow);
            }
            if self.warmth == Some(Warmth::Hot) {
                self.draw_heat(gfx, canvas, now)?;
            }
            if self.warmth == Some(Warmth::Cold) {
                self.draw_vapor(canvas, &base)?;
            }
            Ok(())
        })
    }

    fn draw_particle(&self, canvas: &Canvas, base: &Matrix3x2, sky: Sky, p: &Particle, now: f64) -> Result<()> {
        let alpha = self.fade(p.x, p.y, sky != Sky::Cloudy && sky != Sky::Clear);
        if alpha <= 0.02 {
            return Ok(());
        }
        let (x, y) = (p.x.round(), p.y.round());
        // Phần chạm xuống dưới mặt đất thì cắt bỏ.
        let room = self.height - y;
        if room <= 0.0 {
            return Ok(());
        }
        match sky {
            Sky::Rain | Sky::Storm => {
                // Xanh vừa phải để thấy được trên cả hình nền sáng lẫn tối; xiên theo gió, đuôi ở phía trên.
                let length = p.size.min(room);
                let slant = wind(sky) * length;
                let brush = canvas.brush(Color::rgba(110, 160, 235, 0.85 * alpha))?;
                canvas.line(((p.x - slant) as f32, y as f32), (p.x as f32, (y + length) as f32), &brush, 1.5);
            }
            Sky::Snow => {
                let brush = canvas.brush(Color::rgba(255, 255, 255, 0.9 * alpha))?;
                let size = p.size.round();
                canvas.fill_rect(x as f32, y as f32, size as f32, size.min(room) as f32, &brush);
            }
            Sky::Petals => {
                // Cánh hoa lật qua lật lại khi rơi: bề ngang co giãn.
                let turn = 0.3 + 0.7 * (now / 280.0 + p.phase * 3.0).cos().abs();
                let color = PETAL_COLORS[p.phase.floor() as usize % PETAL_COLORS.len()];
                let brush = canvas.brush(Color::hex(color, alpha as f32))?;
                let width = (p.size * turn).round().max(1.0);
                canvas.fill_rect(x as f32, y as f32, width as f32, (p.size * 0.6).round().min(room) as f32, &brush);
            }
            Sky::Fog => {
                // Vệt sương hơi nhấp nhô.
                let bob = ((now / 900.0 + p.phase).sin() * 1.5).round();
                let brush = canvas.brush(Color::rgba(200, 210, 228, 0.8 * alpha))?;
                canvas.fill_rect(x as f32, (y + bob) as f32, p.size.round() as f32, room.min(2.0) as f32, &brush);
            }
            Sky::Cloudy => {
                let shape = CLOUD_SHAPES[p.phase.floor() as usize % CLOUD_SHAPES.len()];
                let bob = (now / 1500.0 + p.phase).sin().round();
                self.draw_cloud(canvas, base, shape, p.size, x, y + bob, 0.95 * alpha)?;
            }
            Sky::Sunny | Sky::Clear => {
                // Lấp lánh: tia dài ra rồi ngắn lại, sáng lên rồi tắt.
                let glow = (p.age / p.life * PI).sin();
                let arm = (p.size * glow).round();
                let color = if sky == Sky::Sunny {
                    Color::rgba(255, 196, 70, alpha * glow)
                } else {
                    Color::rgba(255, 244, 196, alpha * (0.35 + 0.65 * glow))
                };
                let brush = canvas.brush(color)?;
                canvas.fill_rect((x - arm) as f32, y as f32, (2.0 * arm + 1.0) as f32, 1.0, &brush);
                canvas.fill_rect(x as f32, (y - arm) as f32, 1.0, (2.0 * arm + 1.0) as f32, &brush);
            }
        }
        Ok(())
    }

    /// Đám mây pixel hình `shape`, mỗi ô `cell` CSS pixel, có viền, góc trên trái ở (`x`, `y`).
    #[allow(clippy::too_many_arguments)]
    fn draw_cloud(
        &self,
        canvas: &Canvas,
        base: &Matrix3x2,
        shape: &[&str],
        cell: f64,
        x: f64,
        y: f64,
        alpha: f64,
    ) -> Result<()> {
        canvas.transform(&(Matrix3x2::translation(x as f32, y as f32) * *base));
        let cells: Vec<(i32, i32, u32)> = (shape.iter().enumerate())
            .flat_map(|(r, row)| {
                row.bytes().enumerate().filter_map(move |(c, ch)| match ch {
                    b'X' => Some((c as i32 + 1, r as i32 + 1, CLOUD_BODY)),
                    b'S' => Some((c as i32 + 1, r as i32 + 1, CLOUD_SHADE)),
                    _ => None,
                })
            })
            .collect();
        let cell = cell as f32;
        // Viền: tô các ô sát bên mỗi ô có hình, rồi tô thân mây đè lên. Ô viền tô trước rồi mới tô thân, không chồng
        // độ mờ lên nhau: vẽ riêng các ô viền chưa có hình.
        let filled: std::collections::HashSet<(i32, i32)> = cells.iter().map(|&(c, r, _)| (c, r)).collect();
        let mut edges = std::collections::HashSet::new();
        for &(c, r, _) in &cells {
            for (dc, dr) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                if !filled.contains(&(c + dc, r + dr)) {
                    edges.insert((c + dc, r + dr));
                }
            }
        }
        let edge = canvas.brush(Color::hex(CLOUD_EDGE, alpha as f32))?;
        for (c, r) in edges {
            canvas.fill_rect(c as f32 * cell, r as f32 * cell, cell, cell, &edge);
        }
        let body = canvas.brush(Color::hex(CLOUD_BODY, alpha as f32))?;
        let shade = canvas.brush(Color::hex(CLOUD_SHADE, alpha as f32))?;
        for (c, r, color) in cells {
            let brush = if color == CLOUD_BODY { &body } else { &shade };
            canvas.fill_rect(c as f32 * cell, r as f32 * cell, cell, cell, brush);
        }
        canvas.transform(base);
        Ok(())
    }

    /// Nắng: vài tia nắng vàng nhạt xiên từ trên trái xuống rọi vào pet, sáng lên tối đi chậm.
    fn draw_rays(&self, canvas: &Canvas, now: f64) -> Result<()> {
        let body_width = self.body_right - self.body_left;
        for i in 0..3 {
            // Chân tia ở quanh thân pet, đầu tia ở mép trên.
            let x1 = self.width / 2.0 + (f64::from(i) - 1.0) * body_width * 0.6;
            let x0 = x1 - self.height * SUN_SLANT;
            let pulse = 0.5 + 0.5 * (now / 1800.0 + f64::from(i) * 2.1).sin();
            let alpha = 0.12 + 0.16 * pulse;
            let ray = canvas.linear(
                (x0 as f32, 0.0),
                (x1 as f32, self.height as f32),
                &[
                    (0.0, Color::rgba(255, 206, 96, 0.0)),
                    (0.45, Color::rgba(255, 206, 96, alpha)),
                    (1.0, Color::rgba(255, 206, 96, 0.0)),
                ],
            )?;
            canvas.styled_line((x0 as f32, 0.0), (x1 as f32, self.height as f32), &ray, 3.0 + i as f32, &self.stroke);
        }
        Ok(())
    }

    /// Sao băng: một vệt sáng ngắn bay chéo xuống phía trên đầu pet, đuôi mờ dần.
    fn draw_meteor(&self, canvas: &Canvas, now: f64) -> Result<()> {
        let t = (now - self.meteor_at) / METEOR_MS;
        if !(0.0..1.0).contains(&t) {
            return Ok(());
        }
        let head = (
            self.meteor_from.0 + self.meteor_dir * self.width * 0.35 * t,
            self.meteor_from.1 + self.head_top * 0.4 * t,
        );
        let tail = 22.0;
        let from = (head.0 - self.meteor_dir * tail * 0.87, head.1 - tail * 0.5);
        let alpha = (t * PI).sin();
        let trail = canvas.linear(
            (from.0 as f32, from.1 as f32),
            (head.0 as f32, head.1 as f32),
            &[(0.0, Color::rgba(255, 244, 196, 0.0)), (1.0, Color::rgba(255, 248, 220, 0.9 * alpha))],
        )?;
        canvas.styled_line((from.0 as f32, from.1 as f32), (head.0 as f32, head.1 as f32), &trail, 1.5, &self.stroke);
        Ok(())
    }

    /// Nóng: vệt hơi nóng lượn sóng màu cam bốc lên hai bên thân pet, nhạt dần khi lên cao.
    fn draw_heat(&self, gfx: &Gfx, canvas: &Canvas, now: f64) -> Result<()> {
        let rise = self.height - self.head_top;
        for p in &self.heat {
            let left = ((p.y - self.head_top) / (rise * 0.5)).min(1.0);
            let alpha = 0.9 * left * self.fade(p.x, self.height, true);
            if alpha <= 0.02 {
                continue;
            }
            let mut points = Vec::new();
            let mut k = 0.0;
            while k <= p.size {
                let y = p.y + k;
                if y > self.height {
                    break;
                }
                let x = p.x + (y / 4.0 + p.phase + now / 260.0).sin() * 2.6;
                points.push((x as f32, y as f32));
                k += 1.5;
            }
            if points.len() < 2 {
                continue;
            }
            let path = gfx.polyline(&points)?;
            let brush = canvas.brush(Color::rgba(255, 122, 48, alpha))?;
            canvas.stroke(&path, &brush, 2.0, &self.stroke);
        }
        Ok(())
    }

    /// Lạnh: hơi nước thở ra, mỗi hạt là một đốm mờ nở to dần rồi tan, chồng lên nhau thành luồng hơi.
    fn draw_vapor(&self, canvas: &Canvas, base: &Matrix3x2) -> Result<()> {
        // Một đốm hơi nước mềm (trắng giữa, tan dần ra mép), bán kính 1 rồi phóng to theo từng hạt.
        let puff = canvas.radial(
            0.0,
            0.0,
            1.0,
            &[
                (0.0, Color::rgba(240, 244, 251, 1.0)),
                (0.45, Color::rgba(212, 222, 238, 0.65)),
                (1.0, Color::rgba(200, 212, 232, 0.0)),
            ],
        )?;
        for v in &self.vapor {
            let t = v.age / v.life;
            let r = v.from + (v.to - v.from) * t.sqrt();
            // Hiện nhanh rồi tan dần.
            let alpha = 0.4 * (t * 6.0).min(1.0) * (1.0 - t).powf(1.4);
            if alpha <= 0.01 {
                continue;
            }
            let place = scale(r as f32, r as f32) * Matrix3x2::translation(v.x as f32, v.y as f32);
            canvas.transform(&(place * *base));
            unsafe { puff.SetOpacity(alpha as f32) };
            canvas.fill_circle(0.0, 0.0, 1.0, &puff);
        }
        canvas.transform(base);
        Ok(())
    }
}
