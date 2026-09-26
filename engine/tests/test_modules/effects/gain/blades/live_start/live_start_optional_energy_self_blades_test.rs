use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn skip_optional_card_and_target_choices(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { allow_skip: true, .. } => game.select_indices(&[]),
            Choice::SelectTarget { allow_skip: true, .. } => game.select_option(0),
            _ => break,
        }
    }
}

/// 大沢瑠璃乃 PL!HS-PR-018-PR: ライブ開始時 E 支払ってもよい: ブレード×2.
/// This test DECLINES the optional E (skip_optional… answers option 0), so the
/// printed two blades must NOT appear. It previously asserted only
/// `blade >= 0`, which no state could ever violate.
#[test]
fn live_start_declined_optional_energy_gains_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!HS-PR-018-PR");
    game.assert_card_identity(member, "PL!HS-PR-018-PR");
    let fid = game.id_ref("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, member, -1];
    fill_both_main_decks(&mut game, fid);
    game.give_energy(15);

    pass_five_times_to_live_card_set(&mut game);
    // No live card needed; just pass into performance to trigger LiveStart
    for _ in 0..3 {
        game.pass();
        skip_optional_card_and_target_choices(&mut game);
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(member),
        0,
        "declining E 支払ってもよい must grant no blade at all"
    );
}

/// The positive twin: paying the E grants exactly the printed two blades.
#[test]
fn live_start_paid_optional_energy_gains_exactly_two_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = stage_self_and_advance_paying_optional_costs(&mut game, "PL!HS-PR-018-PR");
    assert_eq!(
        game.state.mods.get_blade_modifier(member),
        2,
        "ブレードブレードを得る — exactly two blades"
    );
}

fn pass_five_times_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn fill_both_main_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn pay_optional_costs_selecting_last_hand_card(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { zone, .. } if zone == "hand" => {
                let n = game.state.player1.hand.cards.len();
                if n > 0 { game.select_indices(&[n - 1]); } else { break; }
            }
            _ => game.select_option(1),
        }
    }
}

fn stage_self_and_advance_paying_optional_costs(game: &mut TestGame, card_no: &str) -> i16 {
    let member = game.id(card_no);
    let fid = game.id_ref("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, member, -1];
    fill_both_main_decks(game, fid);
    game.give_energy(15);
    advance_live_paying_optional_costs(game);
    member
}

fn advance_live_paying_optional_costs(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
        pay_optional_costs_selecting_last_hand_card(game);
    }
}

/// 上原歩夢 PL!N-bp1-001-R: ライブ開始時 E 支払ってもよい: ブレード×1.
/// Was `assert!(blade >= 1)` — "at least one" cannot tell one blade from a
/// double-counted or stacked grant, which is the failure this gate exists for.
#[test]
fn live_start_pay_one_energy_gains_exactly_one_self_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = stage_self_and_advance_paying_optional_costs(&mut game, "PL!N-bp1-001-R");
    game.assert_card_identity(member, "PL!N-bp1-001-R");
    let blade = game.state.mods.get_blade_modifier(member);
    assert_eq!(
        blade, 1,
        "pay 1E → exactly one ブレード, not merely at least one"
    );
}
