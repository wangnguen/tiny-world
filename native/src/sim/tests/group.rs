//! `group.test.ts`: nhiều pet sống chung, ném một con sang màn hình khác thì các con còn lại chạy theo.

use super::*;
use crate::sim::pet::{CrossPhase, Crossing, GROUP_CALM};
use crate::sim::snapshot::PetSnapshot;
use crate::sim::tuning as t;
use crate::sim::Remap;

/// Màn hình bên phải cao bằng, cùng DPI.
const RIGHT: Rect = Rect { x: 1000.0, y: 0.0, width: 800.0, height: 700.0 };

/// Một nhóm pet 60 px đứng ở các chỗ `xs`, màn hình bên phải giáp mép.
fn setup(xs: &[f64], seed: u64) -> World {
    let mut world = World::new(BOUNDS, seed);
    world.set_screen(BOUNDS, vec![RIGHT], None);
    for (i, &x) in xs.iter().enumerate() {
        world.spawn(options(&format!("p{i}"), x, 60.0));
    }
    world
}

/// Overlay sang màn hình bên phải: gốc toạ độ dời sang phải 1000 px.
fn overlay_moves_right(world: &mut World) {
    world.set_screen(
        Bounds { left: 0.0, right: 800.0, top: 0.0, floor: 700.0 },
        vec![Rect { x: -1000.0, y: 0.0, width: 1000.0, height: 700.0 }],
        Some(Remap { scale: 1.0, x: -1000.0, y: 0.0 }),
    );
}

/// Ném pet `i` sang màn hình bên phải, chạy tới lúc overlay cần sang.
fn throw_right(world: &mut World, i: usize) {
    let mut pet = world.pet(i);
    pet.grab();
    pet.drag_to(900.0, 300.0);
    pet.release(1500.0, -200.0);
    assert!(run_until(world, 1.0, |w| w.pets[i].leaving(&w.bounds()).is_some()));
}

fn walking(state: PetState) -> bool {
    matches!(state, PetState::Walk | PetState::Run)
}

#[test]
fn bo_mot_con_thi_cac_con_con_lai_van_song_tiep() {
    let mut world = setup(&[200.0, 500.0, 800.0], 1);
    world.remove("p1");
    assert_eq!(world.pets.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["p0", "p2"]);
    run(&mut world, 5.0);
    assert_eq!(world.pets[0].y, 700.0);
}

#[test]
fn dong_ho_khong_dung_toi_tinh_chung_ca_nhom() {
    let mut world = setup(&[200.0, 500.0, 800.0], 1);
    run_until(&mut world, t::SLEEP_AFTER - 30.0, |_| false);
    world.pet(1).poke();
    run_until(&mut world, 1.0, |_| false);
    for pet in &world.pets {
        assert!(pet.since_interaction < 2.0);
    }
    assert!(run_until(&mut world, t::SLEEP_AFTER + 20.0, World::resting));
    for pet in &world.pets {
        assert_eq!(pet.state(), PetState::Sleep);
    }
}

#[test]
fn con_moi_them_vao_tinh_chung_dong_ho_voi_ca_nhom() {
    let mut world = setup(&[200.0], 1);
    run_until(&mut world, 60.0, |_| false);
    let late = world.spawn(options("late", 600.0, 60.0));
    assert_eq!(world.pets[late].since_interaction, world.pets[0].since_interaction);
}

#[test]
fn click_o_dau_cung_danh_thuc_ca_nhom() {
    let mut world = setup(&[200.0, 500.0], 1);
    for pet in &mut world.pets {
        pet.since_interaction = 1e6;
    }
    assert!(run_until(&mut world, 60.0, World::resting));
    world.wake_all();
    for pet in &world.pets {
        assert_ne!(pet.state(), PetState::Sleep);
    }
}

#[test]
fn con_khac_dang_di_lai_thi_con_nay_hay_dung_yen_hon() {
    let together = |calm: f64| {
        GROUP_CALM.with(|c| c.set(calm));
        let mut all = 0;
        for seed in 1..=5 {
            let mut world = setup(&[150.0, 500.0, 850.0], seed);
            let mut t = 0.0;
            while t < 120.0 {
                world.step(DT);
                if world.pets.iter().all(|p| walking(p.state())) {
                    all += 1;
                }
                t += DT;
            }
        }
        all
    };
    let calm = together(t::GROUP_CALM);
    let free = together(0.0);
    GROUP_CALM.with(|c| c.set(t::GROUP_CALM));
    assert!(f64::from(calm) < f64::from(free) * 0.6);
}

#[test]
fn im_lang_it_tu_di_lai_hon_han_khong_noi_cau_cho_vui() {
    let moving = |quiet: bool| {
        let mut frames = 0;
        for seed in 1..=5 {
            let mut world = setup(&[150.0, 500.0, 850.0], seed);
            world.quiet = quiet;
            let mut t = 0.0;
            while t < 120.0 {
                world.step(DT);
                frames += world.pets.iter().filter(|p| walking(p.state())).count();
                t += DT;
            }
        }
        frames as f64
    };
    assert!(moving(true) < moving(false) * 0.5);
    let mut world = setup(&[200.0, 500.0], 1);
    world.quiet = true;
    run_until(&mut world, t::CHAT_GAP + 1.0, |_| false);
    // Chờ lâu thì cả nhóm đã ngủ: đánh thức để chỉ còn Im lặng chặn.
    world.wake_all();
    assert!(!world.chat(0, "Chào"));
    world.quiet = false;
    assert!(world.chat(0, "Chào"));
}

#[test]
fn tu_2_con_tro_len_thi_khong_tu_di_sang_man_hinh_khac() {
    for seed in 1..=10 {
        let mut world = setup(&[850.0, 950.0], seed);
        assert!(!run_until(&mut world, 60.0, |w| w.pets.iter().any(|p| p.crossing.is_some())));
    }
    // Chỉ còn một con thì lại đi sang như trước.
    let crossed = (1..=5).any(|seed| {
        let mut world = setup(&[950.0], seed);
        run_until(&mut world, 60.0, |w| w.pets[0].leaving(&w.bounds()).is_some())
    });
    assert!(crossed);
}

#[test]
fn nem_mot_con_sang_man_hinh_khac_cac_con_con_lai_chay_theo() {
    let mut world = setup(&[100.0, 400.0, 800.0], 1);
    let (far, near, thrown) = (0, 1, 2);
    throw_right(&mut world, thrown);
    overlay_moves_right(&mut world);
    // Con bị ném đang bay vào; hai con kia ở ngoài mép trái của màn hình mới, chạy vào.
    assert_eq!(world.pets[thrown].crossing.map(|c| c.phase), Some(CrossPhase::In));
    for i in [far, near] {
        let pet = &world.pets[i];
        assert_eq!(pet.state(), PetState::Run);
        assert_eq!(pet.facing, 1);
        assert_eq!(pet.crossing, Some(Crossing { dir: 1, phase: CrossPhase::In, elapsed: 0.0, vx: None }));
        assert!(pet.x < -30.0);
        assert_eq!(pet.y, 700.0);
    }
    assert!(world.pets[near].x > world.pets[far].x);
    assert!(world.pets[near].x - world.pets[far].x >= 60.0 * 0.8);
    // Cả nhóm vào hẳn màn hình mới trong vài giây.
    assert!(run_until(&mut world, 8.0, |w| w.pets.iter().all(|p| p.crossing.is_none())));
    for pet in &world.pets {
        assert!(pet.x >= 30.0 && pet.x <= 770.0);
    }
}

#[test]
fn con_o_xa_cung_khong_bat_cho_qua_lau() {
    let mut world = setup(&[40.0, 900.0], 1);
    throw_right(&mut world, 1);
    overlay_moves_right(&mut world);
    // Cách mép cũ gần 1000 px nhưng chỉ xuất hiện cách mép mới tối đa 320 px.
    assert!(world.pets[0].x >= -30.0 - 320.0);
}

#[test]
fn con_dang_dung_tren_cua_so_o_man_hinh_cu_thi_xuong_dat_roi_chay_theo() {
    let mut world = setup(&[300.0, 800.0], 1);
    let (mounted, thrown) = (0, 1);
    world.set_windows(vec![win(7, 200.0, 400.0, 300.0, 300.0)], &[]);
    drop_at(&mut world, mounted, 300.0, 380.0, 0.0, 0.0);
    run_until(&mut world, 1.0, |w| w.pets[mounted].mount.is_some() && w.pets[mounted].state() == PetState::Idle);
    assert_eq!(world.pets[mounted].mount.map(|m| m.id), Some(7));
    throw_right(&mut world, thrown);
    overlay_moves_right(&mut world);
    let pet = &world.pets[mounted];
    assert!(pet.mount.is_none());
    assert_eq!(pet.y, 700.0);
    assert_eq!(pet.state(), PetState::Run);
}

#[test]
fn con_dang_ngu_thi_nam_luon_o_sat_mep_man_hinh_moi() {
    let mut world = setup(&[200.0, 800.0], 1);
    let (sleeper, thrown) = (0, 1);
    world.step(DT);
    let saved = PetSnapshot { id: "p0".into(), x: 200.0, facing: 1, asleep: true, since_interaction: 0.0 };
    world.pet(sleeper).restore(&saved);
    throw_right(&mut world, thrown);
    overlay_moves_right(&mut world);
    assert_eq!(world.pets[sleeper].state(), PetState::Sleep);
    assert_eq!(world.pets[sleeper].x, 30.0);
}

#[test]
fn man_hinh_doi_tai_cho_thi_khong_con_nao_chay_theo_ai() {
    let mut world = setup(&[200.0, 500.0, 800.0], 1);
    world.set_screen(Bounds { floor: 680.0, ..BOUNDS }, vec![RIGHT], Some(Remap { scale: 1.0, x: 0.0, y: 0.0 }));
    for pet in &world.pets {
        assert!(pet.crossing.is_none());
    }
}
