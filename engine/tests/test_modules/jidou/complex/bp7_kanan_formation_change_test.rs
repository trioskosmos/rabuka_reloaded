/// BP07 CLEAN-G9: PL!S-bp7-012-N 松浦果南 ab#0.
///
/// 登場：自分のステージに『Aqours』か『SaintSnow』のメンバーのみがいる場合、
/// フォーメーションチェンジしてもよい。この効果によって『SaintSnow』のメンバーが
/// 移動した場合、ライブ終了時まで、ブレード×2を得る。
///
/// (Debut) If your stage has only Aqours/SaintSnow members, you MAY formation-change.
/// If this effect moved a SaintSnow member, gain blade×2 until the live ends.
///
/// The parser previously DROPPED the formation-change action entirely (only the
/// conditional blade gain survived). These tests pin the real flow: debut offers a
/// formation change; moving a SaintSnow member grants the blade.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const KANAN: &str = "PL!S-bp7-012-N";
const SAINTSNOW: &str = "PL!S-bp5-111-R"; // SaintSnow member
const FILLER: &str = "PL!-sd1-010-SD";

/// Play 果南 onto a stage whose members are all Aqours/SaintSnow, driving the
/// debut formation-change flow. Returns 果南's id.
fn play_kanan_on_saintsnow_stage(game: &mut TestGame) -> i16 {
    let kanan = game.id(KANAN);
    let s1 = game.id(SAINTSNOW);
    let s2 = game.new_id(SAINTSNOW);
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [s1, s2, -1];
    game.state.player1.hand.cards.push(kanan);
    game.give_energy(30);
    game.play_to_stage(kanan, MemberArea::RightSide);
    kanan
}

fn total_blade(game: &TestGame) -> i32 {
    let mut sum = 0;
    for &cid in &game.state.player1.stage.stage {
        if cid != -1 {
            sum += game.state.mods.get_blade_modifier(cid);
        }
    }
    sum
}

/// 1. Debut with a SaintSnow member on stage → a formation-change choice appears;
/// moving a SaintSnow member grants blade×2 (live-end duration).
#[test]
fn kanan_debut_saintsnow_stage_formation_change_grants_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let _kanan = play_kanan_on_saintsnow_stage(&mut game);

    assert!(
        game.has_pending_choice(),
        "果南 debut should offer a formation-change choice, pending={:?}",
        game.pending_choice_type()
    );

    // Drive the formation-change moves (each member picks a destination).
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let t = game.pending_choice_type();
        // For position/destination selections choose a real move; otherwise skip.
        match t.as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            _ => game.select_option(1),
        }
    }

    assert!(
        total_blade(&game) >= 2,
        "moving a SaintSnow member should grant blade×2, total_blade={}",
        total_blade(&game)
    );
}

/// 2. A non-Aqours/SaintSnow member on stage → the debut effect does not fire.
#[test]
fn kanan_debut_non_group_member_no_formation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanan = game.id(KANAN);
    let outsider = game.id(FILLER); // not Aqours/SaintSnow
    let s1 = game.id(SAINTSNOW);
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [outsider, s1, -1];
    game.state.player1.hand.cards.push(kanan);
    game.give_energy(30);
    game.play_to_stage(kanan, MemberArea::RightSide);

    // Drain anything that appears.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        game.select_indices(&[]);
    }

    assert_eq!(
        total_blade(&game),
        0,
        "non-group member on stage → no formation change, no blade"
    );
}

/// 3. Formation change with NO member actually moving → no blade.
#[test]
fn kanan_formation_no_move_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let _kanan = play_kanan_on_saintsnow_stage(&mut game);

    assert!(
        game.has_pending_choice(),
        "formation-change choice should appear"
    );
    // Choose "stay" destinations for every member (no movement).
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let t = game.pending_choice_type();
        match t.as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            _ => game.select_option(0), // stay in place
        }
    }

    assert_eq!(
        total_blade(&game),
        0,
        "no SaintSnow member moved → no blade"
    );
}
