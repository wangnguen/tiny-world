//! `blink.test.ts`: những gì combo quật đuôi của Long cần ở mô phỏng: bắt đứng yên, tốc biến tới cạnh con khác, biến
//! mất.

use super::*;
use crate::sim::tuning as t;

fn setup(xs: &[f64], windows: Vec<WindowInfo>, seed: u64, area: Bounds) -> World {
    let mut world = group(xs, seed, area);
    world.set_windows(windows, &[]);
    world
}

/// Đang đứng (không nhảy nhót) thì chân phải chạm mép cửa sổ đang đứng hoặc mặt đất, không lơ lửng.
fn expect_standing(world: &World, i: usize) {
    let pet = &world.pets[i];
    if !pet.grounded() || pet.state() == PetState::React || pet.state() == PetState::Jump {
        return;
    }
    let ledge = pet.mount.and_then(|m| world.terrain.ledge(m.id));
    assert_eq!(pet.y, ledge.map_or(BOUNDS.floor, |l| l.y));
}

#[test]
fn hold_dang_ranh_thi_dung_yen_dung_chung_ay_giay() {
    let mut world = setup(&[500.0], Vec::new(), 1, BOUNDS);
    assert!(world.pet(0).hold(-1, 2.0));
    let x = world.pets[0].x;
    simulate(&mut world, 1.9, |w, _| {
        let pet = &w.pets[0];
        assert_eq!(pet.state(), PetState::Idle);
        assert_eq!(pet.x, x);
        assert_eq!(pet.facing, -1);
    });
}

#[test]
fn hold_dang_ngu_thi_day_dung_yen() {
    let mut world = setup(&[500.0], Vec::new(), 1, BOUNDS);
    world.pets[0].since_interaction = 1e6;
    run(&mut world, 30.0);
    assert_eq!(world.pets[0].state(), PetState::Sleep);
    assert!(world.pet(0).hold(1, 2.0));
    assert_eq!(world.pets[0].state(), PetState::Idle);
}

#[test]
fn hold_dang_bi_keo_hay_dang_roi_thi_khong_ep_dung_giua_khong_trung() {
    let mut world = setup(&[500.0], Vec::new(), 1, BOUNDS);
    world.pet(0).grab();
    assert!(!world.pet(0).hold(1, 2.0));
    assert_eq!(world.pets[0].state(), PetState::Dragged);
    world.pet(0).drag_to(500.0, 300.0);
    world.pet(0).release(0.0, 0.0);
    assert!(!world.pet(0).hold(1, 2.0));
    assert_eq!(world.pets[0].state(), PetState::Fall);
    run(&mut world, 2.0);
    assert_eq!(world.pets[0].y, 700.0);
}

#[test]
fn hold_dang_bien_mat_thi_khong_bat_duoc() {
    let mut world = setup(&[500.0], Vec::new(), 1, BOUNDS);
    world.pets[0].vanish(5.0);
    assert!(!world.pet(0).hold(1, 2.0));
}

#[test]
fn blink_tu_mat_dat_toi_canh_con_dung_tren_mep_cua_so() {
    let mut world = setup(&[100.0, 500.0], vec![win(1, 300.0, 400.0, 400.0, 300.0)], 1, BOUNDS);
    let (long, target) = (0, 1);
    stand_on(&mut world, target, 500.0, 400.0);
    world.pet(target).hold(-1, 5.0);
    let x = world.pets[target].x - 50.0;
    world.pet(long).blink_to(target, x, 2.0);
    assert_eq!(world.pets[long].mount.map(|m| m.id), Some(1));
    assert_eq!(world.pets[long].y, 400.0);
    assert_eq!(world.pets[long].x, world.pets[target].x - 50.0);
    assert_eq!(world.pets[long].facing, 1);
    simulate(&mut world, 1.5, |w, _| assert_eq!(w.pets[long].y, 400.0));
    world.set_windows(vec![win(1, 350.0, 300.0, 400.0, 300.0)], &[]);
    assert_eq!(world.pets[long].y, 300.0);
    simulate(&mut world, 30.0, |w, _| expect_standing(w, long));
}

#[test]
fn blink_tu_mep_cua_so_toi_canh_con_duoi_dat() {
    let mut world = setup(&[500.0, 100.0], vec![win(1, 300.0, 400.0, 400.0, 300.0)], 1, BOUNDS);
    let (long, target) = (0, 1);
    stand_on(&mut world, long, 500.0, 400.0);
    world.pet(target).hold(1, 5.0);
    let x = world.pets[target].x + 50.0;
    world.pet(long).blink_to(target, x, 2.0);
    assert!(world.pets[long].mount.is_none());
    assert_eq!(world.pets[long].y, 700.0);
    assert_eq!(world.pets[long].facing, -1);
    world.set_windows(vec![win(1, 350.0, 300.0, 400.0, 300.0)], &[]);
    assert_eq!(world.pets[long].y, 700.0);
    simulate(&mut world, 30.0, |w, _| expect_standing(w, long));
}

#[test]
fn blink_con_kia_dung_sat_mep_man_hinh_thi_van_dung_trong_man_hinh() {
    let mut world = setup(&[500.0, 30.0], Vec::new(), 1, BOUNDS);
    let x = world.pets[1].x - 50.0;
    world.pet(0).blink_to(1, x, 2.0);
    assert_eq!(world.pets[0].x, 30.0);
    assert_eq!(world.pets[0].y, 700.0);
}

#[test]
fn blink_dang_roi_giua_chung_cung_dung_han_xuong_cho_moi() {
    let mut world = setup(&[800.0, 300.0], Vec::new(), 1, BOUNDS);
    let (long, target) = (0, 1);
    drop_at(&mut world, long, 800.0, 200.0, 0.0, 0.0);
    let x = world.pets[target].x + 50.0;
    world.pet(long).blink_to(target, x, 2.0);
    assert_eq!(world.pets[long].state(), PetState::Idle);
    simulate(&mut world, 1.5, |w, _| {
        let pet = &w.pets[long];
        assert_eq!(pet.state(), PetState::Idle);
        assert_eq!(pet.y, 700.0);
        assert_eq!(pet.x, 350.0);
    });
}

#[test]
fn vanish_dung_nguyen_nhu_luc_bien_mat_het_gio_thi_lam_tiep() {
    let mut world = setup(&[300.0], Vec::new(), 1, BOUNDS);
    world.pet(0).stroll(1, 10.0);
    run(&mut world, 0.5);
    let (x, state) = (world.pets[0].x, world.pets[0].state());
    world.pets[0].vanish(5.0);
    simulate(&mut world, 4.9, |w, _| {
        assert_eq!(w.pets[0].x, x);
        assert_eq!(w.pets[0].state(), state);
    });
    run(&mut world, 0.5);
    assert_eq!(world.pets[0].vanished, 0.0);
    assert!(world.pets[0].x > x);
}

#[test]
fn vanish_khong_giat_minh_khong_noi_cau_cho_vui_cau_dang_noi_de_danh() {
    let mut world = setup(&[300.0, 700.0], Vec::new(), 1, BOUNDS);
    let (pet, other) = (0, 1);
    // Chờ hết `CHAT_GAP` mà cả nhóm vẫn thức, không câu chào nào nói mất lượt.
    world.chatter = false;
    let mut t = 0.0;
    while t < t::CHAT_GAP {
        world.step(DT);
        world.pets[pet].since_interaction = 0.0;
        t += DT;
    }
    world.chatter = true;
    world.pets[pet].say("câu đang dở", 3.0);
    world.pets[pet].vanish(5.0);
    world.pet(pet).startle();
    assert_ne!(world.pets[pet].state(), PetState::React);
    assert!(!world.chat(pet, "chào"));
    assert!(world.chat(other, "chào"));
    run(&mut world, 4.0);
    assert_eq!(world.pets[pet].speech.as_ref().map(|s| s.text.as_str()), Some("câu đang dở"));
}

#[test]
fn vanish_con_khac_di_ngang_qua_khong_dung_lai_chao() {
    let narrow = Bounds { right: 300.0, ..BOUNDS };
    let met = |vanish: bool| {
        let mut world = setup(&[60.0, 240.0], Vec::new(), 3, narrow);
        if vanish {
            world.pets[1].vanish(90.0);
        }
        run(&mut world, 90.0);
        world.pets[0].since_meet.is_finite()
    };
    assert!(met(false));
    assert!(!met(true));
}
