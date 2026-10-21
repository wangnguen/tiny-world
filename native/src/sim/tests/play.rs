//! `play.test.ts`: gặp nhau, nói, ngủ cạnh nhau, ban đêm, bubble, đang chat.

use super::*;
use crate::sim::pet::Goal;
use crate::sim::snapshot::PetSnapshot;
use crate::sim::tuning as t;

fn setup(xs: &[f64], seed: u64) -> World {
    group(xs, seed, BOUNDS)
}

/// Thử từng seed tới khi `scenario` trả về `true`; 30 seed không cái nào được thì test lỗi.
fn some_seed(scenario: impl Fn(u64) -> bool) {
    assert!((1..=30).any(scenario), "Không seed nào xảy ra.");
}

/// Cho pet `i` đi về phía `facing` ngay bây giờ.
fn walk(world: &mut World, i: usize, facing: i8) {
    world.pet(i).stroll(facing, 30.0);
    world.pets[i].goal = None;
}

fn asleep_at(id: &str, x: f64, since_interaction: f64) -> PetSnapshot {
    PetSnapshot { id: id.into(), x, facing: 1, asleep: true, since_interaction }
}

#[test]
fn di_toi_sat_con_khac_thi_ca_hai_dung_lai_quay_mat_vao_nhau() {
    let mut world = setup(&[300.0, 400.0], 1);
    world.pet(1).pause(1, 30.0);
    walk(&mut world, 0, 1);
    assert!(run_until(&mut world, 5.0, |w| w.pets[0].state() == PetState::Idle));
    let (a, b) = (&world.pets[0], &world.pets[1]);
    assert_eq!(a.facing, 1);
    assert_eq!(b.facing, -1);
    assert_eq!(b.state(), PetState::Idle);
    assert!(b.x - a.x < 60.0);
}

#[test]
fn vua_chao_nhau_xong_thi_mot_luc_sau_moi_chao_lai() {
    let mut world = setup(&[300.0, 400.0], 1);
    world.pet(1).pause(1, 30.0);
    walk(&mut world, 0, 1);
    run_until(&mut world, 5.0, |w| w.pets[0].state() == PetState::Idle);
    assert!(world.pets[0].since_meet < 1.0);
    // Đi tiếp ngay qua mặt con kia: không dừng lại chào nữa.
    walk(&mut world, 0, 1);
    run(&mut world, 2.0);
    assert_eq!(world.pets[0].state(), PetState::Walk);
}

#[test]
fn chao_xong_co_luc_ru_nhau_di_cung_mot_doan_cung_huong() {
    some_seed(|seed| {
        let mut world = setup(&[300.0, 400.0], seed);
        world.pet(1).pause(1, 30.0);
        walk(&mut world, 0, 1);
        run_until(&mut world, 5.0, |w| w.pets[0].state() == PetState::Idle);
        run_until(&mut world, 6.0, |w| {
            let (a, b) = (&w.pets[0], &w.pets[1]);
            a.goal == Some(Goal::Stroll) && b.goal == Some(Goal::Stroll) && a.facing == b.facing
        })
    });
}

#[test]
fn chao_nhau_co_luc_noi_mot_cau_nhung_ca_nhom_co_gioi_han_tan_suat() {
    some_seed(|seed| {
        let mut world = setup(&[300.0, 400.0], seed);
        run_until(&mut world, t::CHAT_GAP, |_| false);
        world.pet(1).pause(1, 30.0);
        walk(&mut world, 0, 1);
        run_until(&mut world, 5.0, |w| w.pets[0].state() == PetState::Idle);
        world.pets[0].speech.is_some()
    });
    let mut world = setup(&[300.0, 400.0], 1);
    assert!(!world.chat(0, "a"));
    run_until(&mut world, t::CHAT_GAP / 2.0 + 1.0, |_| false);
    // Chạy lâu thì cả nhóm đã ngủ: con đang ngủ không nói.
    assert!(!world.chat(0, "a"));
    world.wake_all();
    assert!(world.chat(0, "a"));
    assert!(!world.chat(1, "b"));
    run_until(&mut world, t::CHAT_GAP + 1.0, |_| false);
    world.wake_all();
    world.busy = true;
    assert!(!world.chat(1, "b"));
    world.busy = false;
    assert!(world.chat(1, "b"));
}

#[test]
fn cau_noi_het_gio_thi_tat_con_dang_noi_thi_vong_lap_chua_duoc_nghi() {
    let mut world = setup(&[300.0], 1);
    world.pet(0).restore(&asleep_at("p0", 300.0, 0.0));
    world.pets[0].say("Khuya rồi", 2.0);
    assert!(!world.resting());
    run_until(&mut world, 2.1, |_| false);
    assert!(world.pets[0].speech.is_none());
    assert!(world.resting());
}

#[test]
fn buon_ngu_ma_co_con_dang_ngu_gan_do_thi_di_toi_nam_canh() {
    let mut world = setup(&[300.0, 600.0], 1);
    let (sleeper, sleepy) = (0, 1);
    world.pet(sleeper).restore(&asleep_at("p0", 300.0, 1e6));
    world.pets[sleepy].since_interaction = 1e6;
    assert!(run_until(&mut world, 30.0, |w| w.pets[sleepy].state() == PetState::Sleep));
    // Nằm sát bên phải con đang ngủ, hơi chồng lên nhau.
    assert!(close_to(world.pets[sleepy].x, 300.0 + 60.0 * t::NAP_GAP, 0));
}

#[test]
fn con_dang_ngu_o_xa_thi_ngu_luon_tai_cho() {
    let mut world = setup(&[100.0, 900.0], 1);
    let (sleeper, sleepy) = (0, 1);
    world.pet(sleeper).restore(&asleep_at("p0", 100.0, 1e6));
    world.pets[sleepy].since_interaction = 1e6;
    let x = world.pets[sleepy].x;
    assert!(run_until(&mut world, 30.0, |w| w.pets[sleepy].state() == PetState::Sleep));
    assert!((world.pets[sleepy].x - x).abs() < t::RUN_SPEED * 3.0);
    assert!(world.pets[sleepy].x > 100.0 + t::NAP_RANGE);
}

#[test]
fn ban_dem_di_cham_hon_va_buon_ngu_som_hon() {
    let mut world = setup(&[500.0], 1);
    world.night = true;
    assert!(close(world.pace(), t::NIGHT_PACE));
    assert!(run_until(&mut world, t::SLEEP_AFTER_NIGHT + 30.0, |w| w.pets[0].state() == PetState::Sleep));
    // Ban ngày thì chưa ngủ sau chừng ấy thời gian.
    let mut day = setup(&[500.0], 1);
    assert!(!run_until(&mut day, t::SLEEP_AFTER_NIGHT + 30.0, |w| w.pets[0].state() == PetState::Sleep));
}

#[test]
fn bam_vao_con_dang_noi_thi_tat_bubble_ca_loi_nhac_dai() {
    let mut world = setup(&[300.0], 1);
    world.pets[0].say("Ngồi liền 50 phút rồi, đứng dậy nghỉ mắt chút đi :)))", 30.0);
    run_until(&mut world, 1.0, |_| false);
    assert!(world.pets[0].speech.is_some());
    world.pet(0).poke();
    assert!(world.pets[0].speech.is_none());
}

#[test]
fn dang_chat_thi_dung_yen_quay_ve_phia_khung_chat_dong_chat_thi_di_lai() {
    let mut world = setup(&[300.0, 400.0], 1);
    walk(&mut world, 0, 1);
    world.pet(0).listen(-1);
    let x = world.pets[0].x;
    // Con kia đi ngang qua cũng không dừng lại chào con đang chat.
    walk(&mut world, 1, -1);
    run_until(&mut world, t::SLEEP_AFTER + 30.0, |w| w.pets[0].state() != PetState::Idle);
    assert_eq!(world.pets[0].state(), PetState::Idle);
    assert_eq!(world.pets[0].facing, -1);
    assert_eq!(world.pets[0].x, x);
    world.pets[0].stop_listening();
    assert!(run_until(&mut world, 60.0, |w| w.pets[0].state() != PetState::Idle));
}

#[test]
fn dang_chat_ma_dang_ngu_thi_day() {
    let mut world = setup(&[300.0], 1);
    run_until(&mut world, t::SLEEP_AFTER + 30.0, |w| w.pets[0].state() == PetState::Sleep);
    assert_eq!(world.pets[0].state(), PetState::Sleep);
    world.pet(0).listen(1);
    run(&mut world, 5.0);
    assert_ne!(world.pets[0].state(), PetState::Sleep);
}
