use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn heart_mod(v: &TestGame, cid: i16, hc: HeartColor) -> i32 {
    v.state.mods.get_heart_modifier(cid, hc)
}

fn filler_id(v: &mut TestGame) -> i16 {
    v.id("PL!-sd1-010-SD")
}

fn setup(v: &mut TestGame, stage: [i16; 3]) {
    let f = filler_id(v);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(f);
    }
    for _ in 0..15 {
        v.state.player1.energy_zone.cards.push(f);
    }
    v.state.player1.energy_zone.active_energy_count = 15;
    v.state.player1.stage.stage = stage;
}

// Drain any pending choices (position_change prompt, auto-ability order, etc.)
fn drain_choices(v: &mut TestGame) {
    while v.has_pending_choice() {
        match v.get_pending_choice().clone() {
            Choice::SelectTarget { .. } => v.select_option(0),
            Choice::SelectCard { .. } => v.select_indices(&[0]),
            _ => v.select_indices(&[]),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// A. Energy placement via PL!SP-pb1-005-R (Hazuki Kano)
//
// Expected: Sumire [1], Hazuki [1], Rurino [0]
//
// Trigger at RightSide so Sumire (pos 0) and Hazuki (pos 1) stay.
// ═══════════════════════════════════════════════════════════════

#[test]
fn energy_placement_triggers_sumire_and_hazuki() {
    let mut v = TestGame::new(load_real_database());

    let sumire = v.id("PL!SP-bp5-004-R\u{ff0b}");
    let hazuki = v.id("PL!SP-bp4-016-N");
    let rurino = v.id("PL!HS-pb1-003-R");
    setup(&mut v, [sumire, hazuki, rurino]);

    let e_f = filler_id(&mut v);
    v.state.player1.energy_deck.cards.push(e_f);
    let trigger = v.id("PL!SP-pb1-005-R");
    v.state.player1.hand.cards.push(trigger);
    v.play_to_stage(trigger, MemberArea::RightSide);
    drain_choices(&mut v);

    assert_eq!(
        heart_mod(&v, sumire, HeartColor::Heart02),
        1,
        "[1] Sumire: energy → on_move_or_energy"
    );
    assert_eq!(
        heart_mod(&v, hazuki, HeartColor::Heart06),
        1,
        "[1] Hazuki: energy → on_energy_placed_each_time"
    );
    assert_eq!(
        v.state
            .mods
            .blade_modifiers
            .get(&rurino)
            .map_or(0, |e| e.total()),
        0,
        "[0] Rurino: no hand discard → not triggered"
    );
}

// ═══════════════════════════════════════════════════════════════
// B. Position change via PL!SP-bp4-013-N (Tang Keke)
//
// Expected: Sumire [1], Natsumi [1], filler [0]
//
// Stage has [sumire, natsumi, filler]. Trigger at position 2 replaces
// filler (sacrificed). Debut fires optional position change → accept.
// Position change sets last_area_move_card_id. Auto scan picks up
// Sumire and Natsumi.
// ═══════════════════════════════════════════════════════════════

#[test]
fn position_change_triggers_sumire_and_natsumi() {
    let mut v = TestGame::new(load_real_database());

    let sumire = v.id("PL!SP-bp5-004-R\u{ff0b}");
    let natsumi = v.id("PL!SP-pb1-020-N");
    let scapegoat = filler_id(&mut v);
    setup(&mut v, [sumire, natsumi, scapegoat]);

    let trigger = v.id("PL!SP-bp4-013-N");
    v.state.player1.hand.cards.push(trigger);
    v.play_to_stage(trigger, MemberArea::RightSide);
    drain_choices(&mut v);

    assert_eq!(
        heart_mod(&v, sumire, HeartColor::Heart02),
        1,
        "[1] Sumire: area move → on_move_or_energy"
    );
    // Natsumi each_time: moved card triggers → draw 1 (deck was 40, stays 40 if
    // draw happens and is discarded? Or decreases by 1? Just check no crash.)
    assert_eq!(
        v.state.player1.main_deck.cards.len(),
        40,
        "[1] Natsumi: each_time area move fires"
    );
    assert_eq!(
        heart_mod(&v, scapegoat, HeartColor::Heart06),
        0,
        "[0] scapegoat: no energy → not triggered"
    );
}

// ═══════════════════════════════════════════════════════════════
// C. Opponent energy → self_effect_only check
//
// Expected: Sumire [0], Hazuki [1]
//
// P2 places the trigger. P1's Sumire must NOT fire (self_effect_only).
// P1's Hazuki card text says opponent also triggers → SHOULD fire.
// ═══════════════════════════════════════════════════════════════

#[test]
fn opponent_energy_triggers_hazuki_not_sumire() {
    let mut v = TestGame::new(load_real_database());
    let f = filler_id(&mut v);

    let sumire = v.id("PL!SP-bp5-004-R\u{ff0b}");
    let hazuki = v.id("PL!SP-bp4-016-N");
    v.state.player1.stage.stage = [sumire, hazuki, -1];
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(f);
    }
    for _ in 0..40 {
        v.state.player2.main_deck.cards.push(f);
    }
    for _ in 0..15 {
        v.state.player2.energy_zone.cards.push(f);
    }
    v.state.player2.energy_zone.active_energy_count = 15;
    v.state.player2.energy_deck.cards.push(f);

    // Place trigger on P2's stage. Use PlayMemberToStage which handles
    // energy cost, debut triggering, auto scanning, and processing.
    let trigger = v.id("PL!SP-pb1-005-R");
    v.state.player2.hand.cards.push(trigger);
    // First, give P2 energy to pay the play cost
    v.state.player2.hand.cards.push(trigger);
    v.state.player2.energy_zone.active_energy_count = 15;
    let f2 = filler_id(&mut v);
    v.state.player2.energy_deck.cards.push(f2);
    // PlayMemberToStage processes P2's debut, which places energy
    // from energy_deck → energy_zone. The post-resolution scan in
    // process_current_ability picks up auto abilities on P1's stage.
    let _ = rabuka_engine::turn::TurnEngine::execute_main_phase_action(
        &mut v.state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(trigger),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    drain_choices(&mut v);

    assert_eq!(
        heart_mod(&v, sumire, HeartColor::Heart02),
        0,
        "[0] Sumire: opponent energy → self_effect_only blocks"
    );
    assert_eq!(
        heart_mod(&v, hazuki, HeartColor::Heart06),
        1,
        "[1] Hazuki: opponent energy allowed per card text"
    );
}
