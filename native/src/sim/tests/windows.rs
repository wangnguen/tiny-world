//! `windows.test.ts`: đứng trên cửa sổ, leo, nhảy, ngồi mép, phản ứng với cửa sổ.

use super::*;
use crate::sim::pet::{Goal, JumpPhase};
use crate::sim::tuning as t;
use std::cell::RefCell;
use std::collections::BTreeSet;

fn setup(windows: Vec<WindowInfo>, seed: u64, x: f64) -> World {
    let mut world = World::new(BOUNDS, seed);
    world.spawn(options("p", x, 60.0));
    world.set_windows(windows, &[]);
    world
}

fn mount_id(world: &World) -> Option<isize> {
    world.pets[0].mount.map(|m| m.id)
}

#[test]
fn tha_phia_tren_mep_cua_so_thi_dap_len_mep() {
    let mut world = setup(vec![win(1, 300.0, 400.0, 400.0, 300.0)], 1, 500.0);
    drop_at(&mut world, 0, 500.0, 200.0, 0.0, 0.0);
    assert!(simulate(&mut world, 1.0, |_, _| {}).contains(&PetState::Land));
    assert_eq!(world.pets[0].y, 400.0);
    assert_eq!(mount_id(&world), Some(1));
}

#[test]
fn tha_ngay_cho_bi_cua_so_khac_che_thi_roi_qua_xuong_dat() {
    // Cửa sổ 2 nằm trên, che mép cửa sổ 1 ở chỗ thả; mép của chính nó ở cao hơn chỗ thả.
    let mut world = setup(vec![win(2, 450.0, 150.0, 100.0, 300.0), win(1, 300.0, 400.0, 400.0, 300.0)], 1, 500.0);
    drop_at(&mut world, 0, 500.0, 200.0, 0.0, 0.0);
    run(&mut world, 2.0);
    assert_eq!(mount_id(&world), None);
    assert_eq!(world.pets[0].y, 700.0);
}

#[test]
fn di_theo_khi_cua_so_bi_keo() {
    let mut world = setup(vec![win(1, 300.0, 400.0, 400.0, 300.0)], 1, 500.0);
    stand_on(&mut world, 0, 500.0, 400.0);
    let x = world.pets[0].x;
    world.set_windows(vec![win(1, 350.0, 300.0, 400.0, 300.0)], &[]);
    assert_eq!(world.pets[0].x, x + 50.0);
    assert_eq!(world.pets[0].y, 300.0);
}

#[test]
fn roi_khi_cua_so_thu_nho_hoac_dong() {
    let mut world = setup(vec![win(1, 300.0, 400.0, 400.0, 300.0)], 1, 500.0);
    stand_on(&mut world, 0, 500.0, 400.0);
    world.set_windows(Vec::new(), &[]);
    assert!(simulate(&mut world, 2.0, |_, _| {}).contains(&PetState::Fall));
    assert_eq!(world.pets[0].y, 700.0);
    assert_eq!(mount_id(&world), None);
}

#[test]
fn roi_khi_cua_so_bi_keo_len_sat_tran() {
    let mut world = setup(vec![win(1, 300.0, 400.0, 400.0, 300.0)], 1, 500.0);
    stand_on(&mut world, 0, 500.0, 400.0);
    world.set_windows(vec![win(1, 300.0, 20.0, 400.0, 300.0)], &[]);
    run(&mut world, 2.0);
    assert_eq!(mount_id(&world), None);
}

#[test]
fn cua_so_bi_keo_ra_ngoai_man_hinh_thi_pet_lui_vao_trong() {
    let mut world = setup(vec![win(1, 300.0, 400.0, 400.0, 300.0)], 1, 500.0);
    stand_on(&mut world, 0, 350.0, 400.0);
    world.set_windows(vec![win(1, -200.0, 400.0, 400.0, 300.0)], &[]);
    assert_eq!(world.pets[0].x, 30.0);
    run(&mut world, DT);
    assert_eq!(mount_id(&world), Some(1));
}

#[test]
fn di_lai_trong_mep_cua_so_khong_bao_gio_lo_lung_ngoai_mep() {
    for seed in 1..=5 {
        let mut world = setup(vec![win(1, 300.0, 400.0, 400.0, 300.0)], seed, 500.0);
        stand_on(&mut world, 0, 500.0, 400.0);
        simulate(&mut world, 60.0, |w, i| {
            let pet = &w.pets[i];
            let state = pet.state();
            if pet.mount.is_some_and(|m| m.id == 1) && state != PetState::Climb && state != PetState::Jump {
                assert!(pet.x >= 300.0 && pet.x <= 700.0);
            }
            if pet.grounded() && pet.mount.is_none() && state != PetState::React && state != PetState::Jump {
                assert_eq!(pet.y, 700.0);
            }
        });
    }
}

#[test]
fn cua_so_khac_de_len_thi_pet_bi_che_roi_di_ra_cho_khong_bi_che() {
    let mut world = setup(vec![win(1, 200.0, 400.0, 600.0, 300.0)], 3, 500.0);
    stand_on(&mut world, 0, 500.0, 400.0);
    world.set_windows(vec![win(2, 420.0, 250.0, 160.0, 300.0), win(1, 200.0, 400.0, 600.0, 300.0)], &[]);
    world.pets[0].planned = 0.0;
    assert_eq!(world.occluders(0), [Rect { x: 420.0, y: 250.0, width: 160.0, height: 300.0 }]);
    run(&mut world, 6.0);
    let pet = &world.pets[0];
    assert!(!world.terrain.covered(1, pet.x, pet.y - 1.0));
}

#[test]
fn cua_so_phong_to_len_truoc_che_het_mep_thi_roi_ngay_xuong_dat() {
    // Đúng cảnh trên máy thật: pet đứng trên cửa sổ lơ lửng, bấm vào trình duyệt phóng to nằm dưới nó.
    let mut world = setup(vec![win(1, 300.0, 150.0, 400.0, 400.0)], 1, 500.0);
    stand_on(&mut world, 0, 500.0, 150.0);
    world.set_windows(vec![win(2, 0.0, 0.0, 1000.0, 700.0), win(1, 300.0, 150.0, 400.0, 400.0)], &[]);
    world.step(DT);
    assert_eq!(world.pets[0].state(), PetState::Fall);
    assert_eq!(mount_id(&world), None);
    assert!(world.occluders(0).is_empty());
    simulate(&mut world, 3.0, |w, i| assert!(w.pets[i].mount.is_none()));
    assert_eq!(world.pets[0].y, 700.0);
}

#[test]
fn dung_duoi_dat_canh_cua_so_thi_co_luc_leo_len_mep_tren() {
    let mut climbed = false;
    for seed in 1..=20 {
        if climbed {
            break;
        }
        let mut world = setup(vec![win(1, 600.0, 450.0, 300.0, 250.0)], seed, 450.0);
        let mut last_y = world.pets[0].y;
        simulate(&mut world, 40.0, |w, i| {
            let pet = &w.pets[i];
            if pet.state() == PetState::Climb {
                // Mặt quay vào tường, điểm chân cách tường một khoảng cố định, chỉ đi lên.
                assert_eq!(pet.facing, 1);
                assert!(close(pet.x, 600.0 - pet.reach));
                assert!(pet.y <= last_y);
            }
            last_y = pet.y;
            if pet.mount.is_some_and(|m| m.id == 1) && pet.state() != PetState::Climb && pet.y == 450.0 {
                climbed = true;
            }
        });
    }
    assert!(climbed);
}

#[test]
fn cua_so_lo_lung_nhay_thang_len_bam_canh_roi_leo_tiep() {
    // Đáy cửa sổ cao hơn mặt đất 150 px, mép trên cao hơn 350 px: không leo thẳng, không nhảy lên mép được.
    let grabs = RefCell::new(Vec::new());
    let world = first_world(
        |seed| {
            grabs.borrow_mut().clear();
            setup(vec![win(1, 600.0, 350.0, 300.0, 200.0)], seed, 450.0)
        },
        |w| {
            let pet = &w.pets[0];
            let grabbing = pet.jumping.is_some_and(|j| j.grab == Some(-1) && j.phase == JumpPhase::Air);
            if pet.state() == PetState::Jump && grabbing {
                grabs.borrow_mut().push(pet.y);
            }
            if pet.state() == PetState::Climb {
                // Bám lúc chân thấp hơn đáy cửa sổ nửa thân, rồi chỉ đi lên.
                assert!(!grabs.borrow().is_empty());
                assert!(pet.y <= 550.0 + 30.0 + 1.0);
                assert!(close(pet.x, 600.0 - pet.reach));
            }
            pet.mount.is_some_and(|m| m.id == 1) && pet.state() != PetState::Climb && pet.y == 350.0
        },
    );
    assert!(grabs.borrow().iter().copied().fold(f64::INFINITY, f64::min) >= 579.0);
    assert_eq!(mount_id(&world), Some(1));
}

#[test]
fn cua_so_lo_lung_cao_qua_tam_nhay_thi_khong_len_duoc() {
    for seed in 1..=10 {
        let mut world = setup(vec![win(1, 600.0, 100.0, 300.0, 200.0)], seed, 450.0);
        simulate(&mut world, 40.0, |w, i| assert!(w.pets[i].mount.is_none()));
    }
}

#[test]
fn nhay_sang_cua_so_ben_canh_ma_khong_cham_dat() {
    let mut jumped = false;
    for seed in 1..=20 {
        if jumped {
            break;
        }
        let mut world = setup(vec![win(1, 100.0, 400.0, 300.0, 300.0), win(2, 500.0, 350.0, 300.0, 350.0)], seed, 500.0);
        stand_on(&mut world, 0, 250.0, 400.0);
        // Rời cửa sổ 1 rồi tới cửa sổ 2 mà giữa chừng không đứng dưới đất.
        let mut from_one = false;
        simulate(&mut world, 40.0, |w, i| {
            let pet = &w.pets[i];
            let id = pet.mount.map(|m| m.id);
            if id == Some(1) {
                from_one = true;
            }
            if id.is_none() && pet.y == 700.0 {
                from_one = false;
            }
            if id == Some(2) && pet.y == 350.0 && from_one {
                jumped = true;
            }
        });
    }
    assert!(jumped);
}

#[test]
fn nhay_sang_cua_so_nao_thi_dap_dung_cua_so_do() {
    // Đúng bố cục thử trên máy thật: B nằm bên trái, cao hơn A 80 px, cách 119 px.
    let mut jumps = 0;
    for seed in 1..=20 {
        let mut world = World::new(Bounds { left: 0.0, right: 1920.0, top: 0.0, floor: 1040.0 }, seed);
        world.speed = 2.0;
        world.spawn(PetOptions { reach: Some(30.5), ..options("p", 1300.0, 96.0) });
        world.set_windows(vec![win(2, 707.0, 560.0, 366.0, 293.0), win(1, 1192.0, 640.0, 506.0, 401.0)], &[]);
        stand_on(&mut world, 0, 1300.0 + seed as f64 * 10.0, 640.0);
        let mut target: Option<Option<isize>> = None;
        simulate(&mut world, 60.0, |w, i| {
            let pet = &w.pets[i];
            match pet.jumping.filter(|j| pet.state() == PetState::Jump && j.phase == JumpPhase::Air) {
                Some(j) => target = Some(j.id),
                None => {
                    if let Some(id) = target.take() {
                        assert_eq!(pet.mount.map(|m| m.id), id);
                        jumps += 1;
                    }
                }
            }
        });
    }
    assert!(jumps > 5);
}

#[test]
fn co_luc_ngoi_o_mep_cua_so() {
    let mut seen = BTreeSet::new();
    for seed in 1..=10 {
        let mut world = setup(vec![win(1, 300.0, 200.0, 250.0, 500.0)], seed, 500.0);
        stand_on(&mut world, 0, 400.0, 200.0);
        seen.extend(simulate(&mut world, 60.0, |_, _| {}).into_iter().map(PetState::name));
    }
    assert!(seen.contains("perch"));
}

#[test]
fn click_luc_dang_leo_thi_tuot_tay_roi_xuong() {
    let mut world = first_world(
        |seed| setup(vec![win(1, 600.0, 300.0, 300.0, 400.0)], seed, 450.0),
        |w| w.pets[0].state() == PetState::Climb,
    );
    world.pet(0).poke();
    assert_eq!(world.pets[0].state(), PetState::Fall);
    assert_eq!(mount_id(&world), None);
}

#[test]
fn dang_leo_ma_bi_cua_so_khac_che_het_thi_buong_tay() {
    let mut world = first_world(
        |seed| setup(vec![win(1, 600.0, 300.0, 300.0, 400.0)], seed, 450.0),
        |w| w.pets[0].state() == PetState::Climb && w.pets[0].y < 600.0,
    );
    world.set_windows(vec![win(2, 0.0, 0.0, 1000.0, 700.0), win(1, 600.0, 300.0, 300.0, 400.0)], &[]);
    world.step(DT);
    assert_eq!(world.pets[0].state(), PetState::Fall);
    assert_eq!(mount_id(&world), None);
    simulate(&mut world, 2.0, |w, i| assert!(w.pets[i].mount.is_none()));
    assert_eq!(world.pets[0].y, 700.0);
}

#[test]
fn cua_so_nam_tren_dat_sat_canh_ma_khong_che_canh_thi_van_leo_len_toi_mep() {
    // Cửa sổ 2 nằm trên, cách cạnh trái cửa sổ 1 có 10 px: che chỗ chân pet lúc bám, không che cạnh.
    let mut climbed = false;
    first_world(
        |seed| setup(vec![win(2, 400.0, 500.0, 190.0, 200.0), win(1, 600.0, 450.0, 300.0, 250.0)], seed, 450.0),
        |w| {
            let pet = &w.pets[0];
            let on_one = pet.mount.is_some_and(|m| m.id == 1);
            if pet.state() == PetState::Climb && on_one {
                climbed = true;
            } else if pet.state() != PetState::Jump {
                climbed = false;
            }
            // Leo tới đỉnh thì nhún qua mép (`Jump`) rồi đáp lên cửa sổ 1.
            climbed && pet.state() == PetState::Jump && on_one
        },
    );
}

#[test]
fn xuong_khoi_mep_ma_canh_ben_bi_che_thi_nhay_khoi_mep() {
    // Mép cửa sổ 1 cao hơn đất 500 px, quá tầm nhảy xuống; cửa sổ 2 nằm trên, che cạnh phải cửa sổ 1.
    let mut world = setup(vec![win(2, 690.0, 150.0, 200.0, 450.0), win(1, 300.0, 200.0, 400.0, 500.0)], 1, 500.0);
    stand_on(&mut world, 0, 600.0, 200.0);
    world.pets[0].since_interaction = t::SLEEP_AFTER;
    world.pets[0].planned = 0.0;
    assert!(!simulate(&mut world, 30.0, |_, _| {}).contains(&PetState::Climb));
    assert_eq!(world.pets[0].state(), PetState::Sleep);
    assert_eq!(world.pets[0].y, 700.0);
}

#[test]
fn buon_ngu_tren_cua_so_thi_xuong_taskbar_roi_moi_ngu() {
    for y in [300.0, 100.0] {
        let mut world = setup(vec![win(1, 300.0, y, 400.0, 700.0 - y)], 1, 500.0);
        stand_on(&mut world, 0, 500.0, y);
        world.pets[0].since_interaction = t::SLEEP_AFTER;
        run(&mut world, 60.0);
        assert_eq!(world.pets[0].state(), PetState::Sleep);
        assert_eq!(world.pets[0].y, 700.0);
        assert_eq!(mount_id(&world), None);
    }
}

#[test]
fn frame_cua_cu_nhay_lay_da_bay_tiep_dat() {
    let poses = RefCell::new(BTreeSet::new());
    first_world(
        |seed| {
            let mut world =
                setup(vec![win(1, 100.0, 400.0, 300.0, 300.0), win(2, 500.0, 350.0, 300.0, 350.0)], seed, 500.0);
            stand_on(&mut world, 0, 250.0, 400.0);
            poses.borrow_mut().clear();
            world
        },
        |w| {
            let pet = &w.pets[0];
            if let Some(pose) = pet.pose() {
                poses.borrow_mut().insert(pose);
            }
            pet.state() == PetState::Idle && poses.borrow().contains(&3)
        },
    );
    assert_eq!(poses.into_inner().into_iter().collect::<Vec<_>>(), [0, 1, 2, 3]);
}

/// Đếm số cú nhảy `React` bắt đầu trong `seconds` giây (state chạy lại từ đầu cũng tính).
fn count_hops(world: &mut World, seconds: f64) -> u32 {
    let pet = &world.pets[0];
    let mut hops = u32::from(pet.state() == PetState::React);
    let mut prev = (pet.state(), pet.state_time());
    let mut t = 0.0;
    while t < seconds {
        world.step(DT);
        let pet = &world.pets[0];
        if pet.state() == PetState::React && (prev.0 != PetState::React || pet.state_time() < prev.1) {
            hops += 1;
        }
        prev = (pet.state(), pet.state_time());
        t += DT;
    }
    hops
}

#[test]
fn cua_so_bi_keo_lai_gan_thi_chay_tron_ve_phia_nguoc_lai() {
    let mut world = setup(vec![win(1, 700.0, 560.0, 200.0, 100.0)], 1, 500.0);
    world.set_windows(vec![win(1, 600.0, 560.0, 200.0, 100.0)], &[]);
    assert_eq!(world.pets[0].state(), PetState::Run);
    assert_eq!(world.pets[0].facing, -1);
    run(&mut world, 0.8);
    assert!(world.pets[0].x < 450.0);
}

#[test]
fn dang_di_hay_dang_chay_ma_bi_cua_so_keo_toi_thi_van_la_chay_tron() {
    for moving in [PetState::Walk, PetState::Run] {
        let mut world = first_world(
            |seed| setup(vec![win(1, 700.0, 560.0, 200.0, 100.0)], seed, 500.0),
            |w| {
                let pet = &w.pets[0];
                pet.state() == moving && pet.facing == 1 && pet.x < 560.0
            },
        );
        let x = world.pets[0].x;
        world.set_windows(vec![win(1, x + 100.0, 560.0, 200.0, 100.0)], &[]);
        assert_eq!(world.pets[0].state(), PetState::Run);
        assert_eq!(world.pets[0].goal, Some(Goal::Flee));
        assert_eq!(world.pets[0].facing, -1);
    }
}

#[test]
fn khong_chay_khi_cua_so_di_xa_ra_bi_phong_to_pet_dung_tren_no_hoac_dang_ngu() {
    let mut away = setup(vec![win(1, 600.0, 560.0, 200.0, 100.0)], 1, 500.0);
    away.set_windows(vec![win(1, 700.0, 560.0, 200.0, 100.0)], &[]);
    assert_eq!(away.pets[0].state(), PetState::Idle);

    let mut maximized = setup(vec![win(1, 700.0, 560.0, 200.0, 100.0)], 1, 500.0);
    maximized.set_windows(vec![win(1, 0.0, 0.0, 1000.0, 700.0)], &[]);
    assert_eq!(maximized.pets[0].state(), PetState::Idle);

    let mut riding = setup(vec![win(1, 300.0, 400.0, 400.0, 300.0)], 1, 500.0);
    stand_on(&mut riding, 0, 500.0, 400.0);
    let state = riding.pets[0].state();
    riding.set_windows(vec![win(1, 350.0, 400.0, 400.0, 300.0)], &[]);
    assert_eq!(riding.pets[0].state(), state);

    let mut asleep = setup(vec![win(1, 700.0, 560.0, 200.0, 100.0)], 1, 500.0);
    let saved = crate::sim::snapshot::PetSnapshot {
        id: "p".into(),
        x: 500.0,
        facing: 1,
        asleep: true,
        since_interaction: 0.0,
    };
    asleep.pet(0).restore(&saved);
    asleep.set_windows(vec![win(1, 600.0, 560.0, 200.0, 100.0)], &[]);
    assert_eq!(asleep.pets[0].state(), PetState::Sleep);
}

#[test]
fn bi_don_vao_mep_man_hinh_thi_giat_minh_nhay_len() {
    let mut world = setup(vec![win(1, 200.0, 560.0, 200.0, 100.0)], 1, 40.0);
    world.set_windows(vec![win(1, 100.0, 560.0, 200.0, 100.0)], &[]);
    assert_eq!(world.pets[0].state(), PetState::React);
}

#[test]
fn dang_chay_tron_tren_mep_cua_so_ma_het_duong_thi_xuong_luon() {
    // Pet đứng cách đầu trái mép 88 px, cửa sổ 2 bị kéo tới từ bên phải.
    let mut world = setup(vec![win(1, 300.0, 400.0, 400.0, 300.0), win(2, 560.0, 250.0, 150.0, 100.0)], 1, 500.0);
    stand_on(&mut world, 0, 400.0, 400.0);
    world.set_windows(vec![win(1, 300.0, 400.0, 400.0, 300.0), win(2, 460.0, 250.0, 150.0, 100.0)], &[]);
    assert_eq!(world.pets[0].goal, Some(Goal::Flee));
    run(&mut world, 4.0);
    assert_ne!(mount_id(&world), Some(1));
}

#[test]
fn cua_so_gan_do_bi_dong_thi_nhay_cang_len_hai_cai() {
    let mut cheered = 0;
    for seed in 1..=10 {
        let mut world = setup(vec![win(1, 700.0, 400.0, 200.0, 200.0)], seed, 500.0);
        world.pets[0].facing = -1;
        world.set_windows(Vec::new(), &[win(1, 700.0, 400.0, 200.0, 200.0)]);
        if world.pets[0].state() != PetState::React {
            continue;
        }
        cheered += 1;
        assert_eq!(world.pets[0].facing, 1);
        assert_eq!(count_hops(&mut world, 1.5), 2);
    }
    // Xác suất 0,8 mỗi lần.
    assert!(cheered > 5);
}

#[test]
fn khong_an_mung_khi_cua_so_o_xa_chi_thu_nho_hoac_vua_an_mung_xong() {
    let mut far = setup(vec![win(1, 0.0, 100.0, 50.0, 50.0)], 1, 950.0);
    far.set_windows(Vec::new(), &[win(1, 0.0, 100.0, 50.0, 50.0)]);
    assert_eq!(far.pets[0].state(), PetState::Idle);

    let mut minimized = setup(vec![win(1, 700.0, 400.0, 200.0, 200.0)], 1, 500.0);
    minimized.set_windows(Vec::new(), &[]);
    assert_eq!(minimized.pets[0].state(), PetState::Idle);

    for seed in 1..=10 {
        let mut world = setup(vec![win(1, 700.0, 400.0, 200.0, 200.0), win(2, 200.0, 400.0, 200.0, 200.0)], seed, 500.0);
        world.set_windows(vec![win(2, 200.0, 400.0, 200.0, 200.0)], &[win(1, 700.0, 400.0, 200.0, 200.0)]);
        if world.pets[0].state() != PetState::React {
            continue;
        }
        run(&mut world, 3.0);
        let state = world.pets[0].state();
        world.set_windows(Vec::new(), &[win(2, 200.0, 400.0, 200.0, 200.0)]);
        assert_eq!(world.pets[0].state(), state);
        return;
    }
}
