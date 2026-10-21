//! `pet.test.ts`: xuất hiện, tự đi lại, click, kéo thả, ngủ, cài đặt, lưu trạng thái.

use super::*;
use crate::sim::snapshot::{parse_world_snapshot, PetSnapshot};
use crate::sim::tuning as t;
use serde_json::json;
use std::collections::BTreeSet;

fn setup(seed: u64, x: f64, area: Bounds) -> World {
    let mut world = World::new(area, seed);
    world.spawn(options("p", x, 60.0));
    world
}

fn one(seed: u64) -> World {
    setup(seed, 500.0, BOUNDS)
}

#[test]
fn xuat_hien_tren_mat_dat_nam_tron_trong_man_hinh() {
    let mut world = setup(1, 990.0, BOUNDS);
    assert_eq!(world.pets[0].y, 700.0);
    assert_eq!(world.pets[0].x, 970.0);
    let q = world.spawn(options("q", -50.0, 60.0));
    assert_eq!(world.pets[q].x, 30.0);
}

#[test]
fn dung_di_chay_xen_ke_luon_trong_man_hinh_va_quay_dau_o_mep() {
    // Màn hình hẹp 300 px để pet chắc chắn đi tới mép.
    let mut world = setup(7, 150.0, Bounds { right: 300.0, ..BOUNDS });
    let mut turned_at_edge = false;
    let mut last_facing = world.pets[0].facing;
    let states = simulate(&mut world, 150.0, |w, i| {
        let pet = &w.pets[i];
        assert!(pet.x >= 30.0 && pet.x <= 270.0);
        assert_eq!(pet.y, 700.0);
        let at_edge = pet.x == 30.0 || pet.x == 270.0;
        if at_edge && pet.facing != last_facing {
            turned_at_edge = true;
        }
        last_facing = pet.facing;
    });
    assert!(states.contains(&PetState::Walk));
    assert!(states.contains(&PetState::Run));
    assert!(!states.contains(&PetState::Sleep));
    assert!(turned_at_edge);
}

#[test]
fn cung_seed_thi_di_y_het_nhau() {
    let mut a = one(3);
    let mut b = one(3);
    run(&mut a, 30.0);
    run(&mut b, 30.0);
    assert_eq!(a.pets[0].x, b.pets[0].x);
    assert_eq!(a.pets[0].state(), b.pets[0].state());
}

#[test]
fn click_nhay_len_tai_cho_roi_di_hoac_chay_tiep() {
    let mut after = BTreeSet::new();
    for seed in 1..=10 {
        let mut world = one(seed);
        world.pet(0).poke();
        assert_eq!(world.pets[0].state(), PetState::React);
        world.step(t::REACT_TIME / 2.0);
        assert!(close(world.pets[0].y, 700.0 - t::HOP_HEIGHT));
        world.step(t::REACT_TIME / 2.0);
        assert_eq!(world.pets[0].y, 700.0);
        after.insert(world.pets[0].state().name());
    }
    assert_eq!(after.into_iter().collect::<Vec<_>>(), ["run", "walk"]);
}

#[test]
fn click_don_dap_chi_nhay_mot_lan() {
    let mut world = one(1);
    let mut hops = 0;
    let mut previous = world.pets[0].state();
    // Spam 5 click mỗi giây trong 3 giây.
    for step in 0..90 {
        if step % 6 == 0 {
            let pet = &world.pets[0];
            let hopping = (pet.state() == PetState::React).then(|| pet.state_time());
            world.pet(0).poke();
            if let Some(time) = hopping {
                assert_eq!(world.pets[0].state_time(), time);
            }
        }
        let state = world.pets[0].state();
        if state == PetState::React && previous != PetState::React {
            hops += 1;
        }
        previous = state;
        world.step(DT);
    }
    assert_eq!(hops, 1);
    assert!(world.pets[0].since_interaction < t::POKE_COOLDOWN);
}

#[test]
fn ngung_click_mot_luc_roi_click_tiep_thi_lai_nhay() {
    let mut world = one(1);
    world.pet(0).poke();
    run(&mut world, t::REACT_TIME + t::POKE_COOLDOWN);
    assert_ne!(world.pets[0].state(), PetState::React);
    world.pet(0).poke();
    assert_eq!(world.pets[0].state(), PetState::React);
}

#[test]
fn khong_phan_ung_khi_dang_bi_keo_hoac_dang_roi() {
    let mut world = one(1);
    world.pet(0).grab();
    world.pet(0).poke();
    assert_eq!(world.pets[0].state(), PetState::Dragged);
    world.pet(0).release(0.0, 0.0);
    world.pet(0).poke();
    assert_eq!(world.pets[0].state(), PetState::Fall);
}

#[test]
fn keo_di_theo_chuot_nhung_khong_ra_khoi_man_hinh() {
    let mut world = one(1);
    let mut pet = world.pet(0);
    pet.grab();
    pet.drag_to(200.0, 300.0);
    assert_eq!((world.pets[0].x, world.pets[0].y), (200.0, 300.0));
    world.pet(0).drag_to(-100.0, -100.0);
    assert_eq!((world.pets[0].x, world.pets[0].y), (30.0, 60.0));
    world.pet(0).drag_to(5000.0, 5000.0);
    assert_eq!((world.pets[0].x, world.pets[0].y), (970.0, 700.0));
}

#[test]
fn tha_thap_roi_nay_mot_chut_roi_tiep_dat() {
    let mut world = one(1);
    drop_at(&mut world, 0, 500.0, 600.0, 0.0, 0.0);
    let mut bounced = false;
    let states = simulate(&mut world, 1.5, |w, i| {
        let pet = &w.pets[i];
        if pet.state() == PetState::Fall && pet.vy < 0.0 {
            bounced = true;
        }
    });
    assert_eq!(states[..3], [PetState::Fall, PetState::Land, PetState::Idle]);
    assert!(bounced);
    assert_eq!(world.pets[0].y, 700.0);
}

#[test]
fn tha_tu_cao_choang_roi_tinh_lai() {
    let mut world = one(1);
    drop_at(&mut world, 0, 500.0, 100.0, 0.0, 0.0);
    let states = simulate(&mut world, 1.0 + t::DIZZY_TIME, |_, _| {});
    assert_eq!(states[..3], [PetState::Fall, PetState::Dizzy, PetState::Idle]);
}

#[test]
fn nem_manh_sang_trai_bay_theo_quan_tinh_dap_tuong_bat_lai() {
    let mut world = one(1);
    drop_at(&mut world, 0, 500.0, 400.0, -2500.0, -500.0);
    let mut hit_wall = false;
    simulate(&mut world, 3.0, |w, i| {
        let pet = &w.pets[i];
        assert!(pet.x >= 30.0);
        assert!(pet.y >= 60.0);
        if pet.x == 30.0 && pet.vx > 0.0 {
            hit_wall = true;
        }
    });
    assert!(hit_wall);
    assert!(world.pets[0].grounded());
}

#[test]
fn gioi_han_van_toc_nem() {
    let mut world = one(1);
    drop_at(&mut world, 0, 500.0, 400.0, 30000.0, 40000.0);
    let pet = &world.pets[0];
    assert!(close(pet.vx.hypot(pet.vy), t::MAX_THROW_SPEED));
}

#[test]
fn bat_duoc_pet_giua_luc_dang_roi() {
    let mut world = one(1);
    drop_at(&mut world, 0, 500.0, 100.0, 0.0, 0.0);
    run(&mut world, 0.2);
    world.pet(0).grab();
    assert_eq!(world.pets[0].state(), PetState::Dragged);
    assert_eq!((world.pets[0].vx, world.pets[0].vy), (0.0, 0.0));
}

#[test]
fn lau_khong_ai_dung_thi_ngu_ngu_yen_toi_khi_duoc_click() {
    let mut world = one(5);
    run(&mut world, t::SLEEP_AFTER + 10.0);
    assert_eq!(world.pets[0].state(), PetState::Sleep);
    assert!(world.resting());
    run(&mut world, 600.0);
    assert_eq!(world.pets[0].state(), PetState::Sleep);

    world.pet(0).poke();
    assert_eq!(world.pets[0].state(), PetState::React);
    assert_eq!(world.pets[0].since_interaction, 0.0);
    assert!(!world.resting());
    let states = simulate(&mut world, 60.0, |_, _| {});
    assert!(!states.contains(&PetState::Sleep));
}

#[test]
fn keo_cung_danh_thuc() {
    let mut world = one(5);
    run(&mut world, t::SLEEP_AFTER + 10.0);
    world.pet(0).grab();
    assert_eq!(world.pets[0].state(), PetState::Dragged);
    assert_eq!(world.pets[0].since_interaction, 0.0);
}

#[test]
fn click_o_cho_khac_thi_pet_dang_ngu_thuc_day() {
    let mut world = one(5);
    run(&mut world, t::SLEEP_AFTER + 10.0);
    world.pet(0).wake();
    assert_eq!(world.pets[0].state(), PetState::React);
    assert_eq!(world.pets[0].since_interaction, 0.0);
    assert!(!world.resting());
}

#[test]
fn click_o_cho_khac_khong_lam_gi_pet_dang_thuc() {
    let mut world = one(5);
    run(&mut world, 5.0);
    let before = world.pets[0].state();
    let since = world.pets[0].since_interaction;
    world.pet(0).wake();
    assert_eq!(world.pets[0].state(), before);
    assert_eq!(world.pets[0].since_interaction, since);
}

/// Quãng đường đi được trong một lượt walk đầu tiên, với hệ số tốc độ `speed`.
fn walk_distance(speed: f64) -> f64 {
    let mut world = one(2);
    world.speed = speed;
    while world.pets[0].state() != PetState::Walk {
        world.step(DT);
    }
    let start = world.pets[0].x;
    world.step(DT);
    (world.pets[0].x - start).abs()
}

#[test]
fn toc_do_nhan_vao_van_toc_di() {
    assert!(close(walk_distance(1.0), t::WALK_SPEED * DT));
    assert!(close(walk_distance(2.0), t::WALK_SPEED * DT * 2.0));
}

#[test]
fn doi_co_thi_van_nam_tron_trong_man_hinh() {
    let mut world = setup(1, 970.0, BOUNDS);
    world.pet(0).resize(120.0, 120.0, None);
    assert_eq!(world.pets[0].x, 940.0);
    assert_eq!(world.pets[0].width, 120.0);
}

#[test]
fn dang_ngu_thi_mo_lai_van_ngu_o_cho_cu() {
    let mut world = setup(5, 300.0, BOUNDS);
    run(&mut world, t::SLEEP_AFTER + 10.0);
    let json = serde_json::to_value(world.snapshot()).unwrap();
    let saved = parse_world_snapshot(&json).unwrap();

    let mut next = setup(9, 800.0, BOUNDS);
    next.pet(0).restore(&saved.pets[0]);
    assert_eq!(next.pets[0].state(), PetState::Sleep);
    assert_eq!(next.pets[0].x, crate::sim::js_round(world.pets[0].x));
    assert_eq!(next.pets[0].facing, world.pets[0].facing);
    assert!(next.resting());
}

#[test]
fn dang_bi_keo_tren_cao_thi_mo_lai_dung_tren_mat_dat() {
    let mut world = one(1);
    drop_at(&mut world, 0, 200.0, 100.0, 0.0, 0.0);
    world.pet(0).grab();
    let mut next = one(1);
    let saved = world.snapshot();
    next.pet(0).restore(&saved.pets[0]);
    assert_eq!(next.pets[0].state(), PetState::Idle);
    assert_eq!(next.pets[0].y, 700.0);
    assert_eq!(next.pets[0].x, 200.0);
}

#[test]
fn vi_tri_cu_ngoai_man_hinh_thi_kep_lai() {
    let mut world = one(1);
    world.pet(0).restore(&PetSnapshot { id: "p".into(), x: 5000.0, facing: -1, asleep: false, since_interaction: 12.0 });
    assert_eq!(world.pets[0].x, 970.0);
    assert_eq!(world.pets[0].since_interaction, 12.0);
}

#[test]
fn file_sai_version_sai_kieu_thi_bo_qua() {
    assert!(parse_world_snapshot(&json!(null)).is_none());
    assert!(parse_world_snapshot(&json!({ "version": 2, "pets": [] })).is_none());
    assert!(parse_world_snapshot(&json!({ "version": 1, "pets": {} })).is_none());
    let good = json!({ "id": "p", "x": 1, "facing": 1, "asleep": false, "sinceInteraction": 0 });
    let with = |key: &str, value: serde_json::Value| {
        let mut pet = good.clone();
        pet[key] = value;
        pet
    };
    let parsed = parse_world_snapshot(&json!({
        "version": 1,
        "pets": [good.clone(), with("x", json!("1")), with("facing", json!(0)), with("sinceInteraction", json!(-1))],
    }))
    .unwrap();
    assert_eq!(
        parsed.pets,
        [PetSnapshot { id: "p".into(), x: 1.0, facing: 1, asleep: false, since_interaction: 0.0 }]
    );
}
