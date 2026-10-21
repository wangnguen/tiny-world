//! `cursor.test.ts`: nhìn theo con trỏ, sang màn hình bên cạnh, màn hình đổi độ phân giải, DPI, taskbar.

use super::*;
use crate::sim::pet::CrossPhase;
use crate::sim::tuning as t;
use crate::sim::{Point, Remap};

fn setup(seed: u64, x: f64, width: f64) -> World {
    let mut world = World::new(BOUNDS, seed);
    world.spawn(options("p", x, width));
    world
}

/// Chạy `seconds` giây; mỗi bước hỏi `cursor` vị trí con trỏ (theo số giây đã chạy) rồi báo cho world. `until` đúng
/// thì dừng sớm, trả về `true`.
fn drive(
    world: &mut World,
    seconds: f64,
    mut cursor: impl FnMut(&World, f64) -> Option<Point>,
    mut until: impl FnMut(&World) -> bool,
) -> bool {
    let mut t = 0.0;
    while t < seconds {
        if let Some(at) = cursor(world, t) {
            world.move_cursor(at.x, at.y);
        }
        world.step(DT);
        if until(world) {
            return true;
        }
        t += DT;
    }
    false
}

fn no_cursor(_: &World, _: f64) -> Option<Point> {
    None
}

/// Thử từng seed tới khi `scenario` trả về thế giới; 20 seed không cái nào được thì test lỗi.
fn some_seed(scenario: impl Fn(u64) -> Option<World>) -> World {
    (1..=20).find_map(scenario).expect("Không seed nào xảy ra.")
}

/// Đợi pet đứng yên (`Idle`) rồi mới bắt đầu.
fn until_idle(world: &mut World) {
    drive(world, 10.0, no_cursor, |w| w.pets[0].state() == PetState::Idle && w.pets[0].state_time() < DT * 1.5);
}

fn leaving(world: &World) -> Option<Point> {
    world.pets[0].leaving(&world.bounds())
}

fn crossing_phase(world: &World) -> Option<CrossPhase> {
    world.pets[0].crossing.map(|c| c.phase)
}

#[test]
fn dung_yen_thi_quay_ve_phia_con_tro_o_gan() {
    let mut world = setup(1, 500.0, 60.0);
    until_idle(&mut world);
    let side = -world.pets[0].facing;
    drive(&mut world, 0.2, |w, _| Some(Point { x: w.pets[0].x + f64::from(side) * 120.0, y: 650.0 }), |_| false);
    assert_eq!(world.pets[0].facing, side);
}

#[test]
fn con_tro_ngay_tren_dau_hoac_o_xa_thi_khong_quay() {
    let mut world = setup(1, 500.0, 60.0);
    until_idle(&mut world);
    let facing = f64::from(world.pets[0].facing);
    drive(&mut world, 0.2, |w, _| Some(Point { x: w.pets[0].x - facing * 5.0, y: 600.0 }), |_| false);
    assert_eq!(f64::from(world.pets[0].facing), facing);
    drive(&mut world, 0.2, |w, _| Some(Point { x: w.pets[0].x - facing * 400.0, y: 650.0 }), |_| false);
    assert_eq!(f64::from(world.pets[0].facing), facing);
}

/// Màn hình bên phải cao bằng, cùng DPI.
const RIGHT: Rect = Rect { x: 1000.0, y: 0.0, width: 800.0, height: 700.0 };

fn leave(seed: u64, width: f64) -> Option<World> {
    let mut world = setup(seed, 900.0, width);
    world.set_screen(BOUNDS, vec![RIGHT], None);
    drive(&mut world, 40.0, no_cursor, |w| leaving(w).is_some()).then_some(world)
}

/// Overlay sang màn hình bên phải: gốc toạ độ dời sang phải 1000 px.
fn overlay_moves_right(world: &mut World) {
    world.set_screen(
        Bounds { left: 0.0, right: 800.0, top: 0.0, floor: 700.0 },
        vec![Rect { x: -1000.0, y: 0.0, width: 1000.0, height: 700.0 }],
        Some(Remap { scale: 1.0, x: -1000.0, y: 0.0 }),
    );
}

#[test]
fn di_toi_mep_giap_man_hinh_khac_thi_co_luc_di_ra_khoi_mep() {
    let world = some_seed(|seed| leave(seed, 60.0));
    let pet = &world.pets[0];
    assert_eq!(leaving(&world), Some(Point { x: pet.x, y: 699.0 }));
    assert!(pet.x > 1000.0);
}

#[test]
fn overlay_sang_roi_thi_di_vao_han_man_hinh_moi() {
    let mut world = some_seed(|seed| leave(seed, 60.0));
    let x = world.pets[0].x;
    overlay_moves_right(&mut world);
    assert!(close(world.pets[0].x, x - 1000.0));
    assert_eq!(leaving(&world), None);
    drive(&mut world, 3.0, no_cursor, |w| w.pets[0].crossing.is_none());
    assert!(world.pets[0].crossing.is_none());
    assert!(world.pets[0].x >= 30.0);
    assert_eq!(world.pets[0].y, 700.0);
}

#[test]
fn pet_to_di_bo_cham_ra_mep_van_kip_ra_toi_cho_cho() {
    // Pet 96 px đi 30 px/s: từ đầu mặt đất tới lúc giữa thân qua mép mất 1,6 giây, lâu hơn CROSS_TIMEOUT.
    some_seed(|seed| leave(seed, 96.0));
}

#[test]
fn man_hinh_doi_tai_cho_luc_dang_di_ra_mep_thi_vao_lai_trong_man_hinh() {
    let mut world = some_seed(|seed| leave(seed, 60.0));
    world.set_screen(BOUNDS, vec![RIGHT], Some(Remap { scale: 1.0, x: 0.0, y: 0.0 }));
    run(&mut world, 5.0);
    assert!(world.pets[0].crossing.is_none());
    assert!(world.pets[0].x <= 970.0);
}

#[test]
fn overlay_khong_sang_duoc_thi_quay_lai() {
    let mut world = some_seed(|seed| leave(seed, 60.0));
    drive(&mut world, t::CROSS_TIMEOUT + 3.0, no_cursor, |w| w.pets[0].crossing.is_none());
    assert!(world.pets[0].crossing.is_none());
    assert!(world.pets[0].x <= 970.0);
}

#[test]
fn dang_vat_qua_mep_thi_cua_so_keo_toi_cung_khong_lam_pet_quay_dau_di_lac() {
    let mut world = some_seed(|seed| leave(seed, 60.0));
    world.set_windows(vec![win(9, 700.0, 560.0, 150.0, 100.0)], &[]);
    world.set_windows(vec![win(9, 800.0, 560.0, 150.0, 100.0)], &[]);
    assert!(world.pets[0].goal.is_none());
    // Không sang được: quay vào, cả con nằm lại trong màn hình.
    drive(&mut world, t::CROSS_TIMEOUT + 4.0, no_cursor, |w| w.pets[0].crossing.is_none());
    assert!(world.pets[0].crossing.is_none());
    assert!(world.pets[0].x <= 970.0);
    run(&mut world, 5.0);
    assert!(world.pets[0].x >= 30.0 && world.pets[0].x <= 970.0);
}

#[test]
fn da_sang_man_hinh_moi_ma_quay_dau_ra_lai_thi_vao_han_trong_man_hinh() {
    let mut world = some_seed(|seed| leave(seed, 60.0));
    overlay_moves_right(&mut world);
    world.pets[0].facing = -1;
    run(&mut world, DT);
    assert!(world.pets[0].crossing.is_none());
    assert_eq!(world.pets[0].x, 30.0);
}

#[test]
fn man_hinh_ben_canh_thap_hon_thi_khong_di_sang() {
    for seed in 1..=10 {
        let mut world = setup(seed, 900.0, 60.0);
        world.set_screen(BOUNDS, vec![Rect { height: 500.0, ..RIGHT }], None);
        assert!(!drive(&mut world, 40.0, no_cursor, |w| w.pets[0].crossing.is_some()));
    }
}

/// Ném pet về phía màn hình bên phải từ (x, y) với vận tốc (vx, vy).
fn throw(world: &mut World, x: f64, y: f64, vx: f64, vy: f64) {
    world.set_screen(BOUNDS, vec![RIGHT], None);
    let mut pet = world.pet(0);
    pet.grab();
    pet.drag_to(x, y);
    pet.release(vx, vy);
}

#[test]
fn nem_ve_phia_man_hinh_ben_canh_thi_bay_qua_mep_khong_nay_lai() {
    let mut world = setup(1, 800.0, 60.0);
    throw(&mut world, 900.0, 300.0, 1500.0, -200.0);
    assert!(drive(&mut world, 1.0, no_cursor, |w| leaving(w).is_some()));
    assert_eq!(world.pets[0].state(), PetState::Fall);
}

#[test]
fn nem_qua_mep_ma_roi_cham_dat_van_cho_overlay_sang() {
    let mut world = setup(1, 800.0, 60.0);
    throw(&mut world, 900.0, 300.0, 1500.0, -200.0);
    drive(&mut world, 0.5, no_cursor, |w| leaving(w).is_some());
    assert_eq!(crossing_phase(&world), Some(CrossPhase::Out));
    // Chạy thêm 0,8 giây (đủ rơi chạm đất nhưng chưa hết CROSS_TIMEOUT 1,5 giây).
    run(&mut world, 0.8);
    assert_eq!(world.pets[0].y, 700.0);
    assert_eq!(crossing_phase(&world), Some(CrossPhase::Out));
    assert!(leaving(&world).is_some());
}

#[test]
fn nem_qua_mep_roi_cham_dat_roi_overlay_sang_thi_dap_binh_thuong() {
    let mut world = setup(1, 800.0, 60.0);
    throw(&mut world, 900.0, 300.0, 1500.0, -200.0);
    drive(&mut world, 0.5, no_cursor, |w| leaving(w).is_some());
    run(&mut world, 0.8);
    assert_eq!(crossing_phase(&world), Some(CrossPhase::Out));
    overlay_moves_right(&mut world);
    assert_eq!(crossing_phase(&world), Some(CrossPhase::In));
    drive(&mut world, 2.0, no_cursor, |w| w.pets[0].crossing.is_none());
    assert!(world.pets[0].crossing.is_none());
    assert_eq!(world.pets[0].y, 700.0);
}

#[test]
fn nem_thap_qua_mep_cham_dat_luc_moi_qua_mot_phan_roi_overlay_sang_thi_di_bo_vao() {
    let mut world = setup(1, 800.0, 60.0);
    throw(&mut world, 970.0, 680.0, 300.0, 0.0);
    drive(&mut world, 0.5, no_cursor, |w| leaving(w).is_some());
    run(&mut world, 0.3);
    // Đứng chờ trên đất, giữa thân đã qua mép nhưng một phần thân còn bên này.
    assert_eq!(crossing_phase(&world), Some(CrossPhase::Out));
    assert_eq!(world.pets[0].y, 700.0);
    assert!(world.pets[0].x > 1000.0 && world.pets[0].x < 1030.0);
    overlay_moves_right(&mut world);
    assert_eq!(crossing_phase(&world), Some(CrossPhase::In));
    // Mỗi bước chỉ nhích một chút (đi bộ vào), không nhảy vọt cả bề ngang pet vào trong màn hình.
    let mut x = world.pets[0].x;
    let mut jumps: Vec<f64> = Vec::new();
    drive(&mut world, 4.0, no_cursor, |w| {
        jumps.push((w.pets[0].x - x).abs());
        x = w.pets[0].x;
        w.pets[0].crossing.is_none()
    });
    let pet = &world.pets[0];
    assert!(pet.crossing.is_none());
    assert_eq!(pet.state(), PetState::Walk);
    assert!(pet.x >= 30.0);
    assert_eq!(pet.y, 700.0);
    assert!(jumps.iter().copied().fold(f64::NEG_INFINITY, f64::max) < 5.0);
}

#[test]
fn nem_manh_qua_mep_overlay_sang_luc_pet_con_dang_bay_thi_bay_tiep_vao() {
    let mut world = setup(1, 800.0, 60.0);
    throw(&mut world, 900.0, 300.0, 3000.0, -200.0);
    // Bay ra hẳn ngoài mép, bị giữ lại chờ overlay sang.
    assert!(drive(&mut world, 0.5, no_cursor, |w| w.pets[0].x == 1030.0));
    overlay_moves_right(&mut world);
    run(&mut world, 0.1);
    let pet = &world.pets[0];
    assert_eq!(pet.state(), PetState::Fall);
    assert!(pet.y < 700.0);
    assert!(pet.x > 30.0);
    assert!(pet.crossing.is_none());
}

#[test]
fn tha_sat_mep_hoi_hat_ra_cham_dat_luc_giua_than_chua_qua_mep_thi_dap_luon() {
    let mut world = setup(1, 800.0, 60.0);
    throw(&mut world, 965.0, 690.0, 100.0, 0.0);
    assert!(drive(&mut world, 0.5, no_cursor, |w| w.pets[0].state() == PetState::Land));
    assert!(world.pets[0].crossing.is_none());
    assert!(world.pets[0].x <= 970.0);
}

#[test]
fn khong_co_man_hinh_ben_canh_thi_nem_vao_mep_van_nay_lai() {
    let mut world = setup(1, 800.0, 60.0);
    let mut pet = world.pet(0);
    pet.grab();
    pet.drag_to(900.0, 300.0);
    pet.release(1500.0, -200.0);
    run(&mut world, 1.0);
    assert!(world.pets[0].crossing.is_none());
    assert!(world.pets[0].x <= 970.0);
}

#[test]
fn taskbar_cao_len_thi_dung_len_mat_dat_moi_thap_xuong_thi_roi_xuong() {
    let mut world = setup(1, 500.0, 60.0);
    until_idle(&mut world);
    world.set_screen(Bounds { floor: 650.0, ..BOUNDS }, Vec::new(), None);
    assert_eq!(world.pets[0].y, 650.0);
    world.set_screen(Bounds { floor: 750.0, ..BOUNDS }, Vec::new(), None);
    assert_eq!(world.pets[0].state(), PetState::Fall);
    run(&mut world, 1.0);
    assert_eq!(world.pets[0].y, 750.0);
}

#[test]
fn dang_nhay_vi_bi_click_thi_nhay_not_tren_mat_dat_moi() {
    let mut world = setup(1, 500.0, 60.0);
    until_idle(&mut world);
    world.pet(0).poke();
    run(&mut world, 0.1);
    world.set_screen(Bounds { floor: 720.0, ..BOUNDS }, Vec::new(), None);
    assert_eq!(world.pets[0].state(), PetState::React);
    run(&mut world, t::REACT_TIME);
    assert_eq!(world.pets[0].y, 720.0);
}

#[test]
fn dang_ngu_thi_nam_yen_tren_mat_dat_moi() {
    let mut world = setup(1, 500.0, 60.0);
    world.pets[0].since_interaction = t::SLEEP_AFTER;
    drive(&mut world, 10.0, no_cursor, |w| w.pets[0].state() == PetState::Sleep);
    world.set_screen(Bounds { floor: 750.0, ..BOUNDS }, Vec::new(), None);
    assert_eq!(world.pets[0].state(), PetState::Sleep);
    assert_eq!(world.pets[0].y, 750.0);
}

#[test]
fn man_hinh_hep_lai_thi_pet_lui_vao_trong() {
    let mut world = setup(1, 950.0, 60.0);
    until_idle(&mut world);
    world.set_screen(Bounds { right: 800.0, ..BOUNDS }, Vec::new(), None);
    assert!(world.pets[0].x <= 770.0);
}

#[test]
fn doi_dpi_thi_pet_dung_tren_cua_so_van_dung_dung_mep_cua_so_do() {
    let mut world = setup(1, 500.0, 60.0);
    world.set_windows(vec![win(1, 300.0, 400.0, 400.0, 300.0)], &[]);
    drop_at(&mut world, 0, 500.0, 350.0, 0.0, 0.0);
    run(&mut world, 1.0);
    assert_eq!(world.pets[0].mount.map(|m| m.id), Some(1));
    // 100% → 125%: toạ độ CSS nhỏ lại còn 0,8.
    world.set_screen(
        Bounds { left: 0.0, right: 800.0, top: 0.0, floor: 560.0 },
        Vec::new(),
        Some(Remap { scale: 0.8, x: 0.0, y: 0.0 }),
    );
    assert_eq!(world.pets[0].mount.map(|m| m.id), Some(1));
    assert!(close(world.pets[0].y, 320.0));
    assert_eq!(world.terrain.window(1), Some(Rect { x: 240.0, y: 320.0, width: 320.0, height: 240.0 }));
    run(&mut world, 1.0);
    assert_eq!(world.pets[0].mount.map(|m| m.id), Some(1));
}
