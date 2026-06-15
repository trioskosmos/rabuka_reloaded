/// Choice/Bullet-point cards — engine fix validation.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for p in [&mut game.state.player1, &mut game.state.player2] {
        p.main_deck.cards.clear();
        for _ in 0..50 {
            p.main_deck.cards.push(filler);
        }
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}
fn debut_play(game: &mut TestGame, card_id: i16, energy: usize, area: MemberArea) {
    fill_decks(game);
    game.state.player1.hand.cards.push(card_id);
    game.give_energy(energy);
    game.state.turn_number = 1;
    game.play_to_stage(card_id, area);
}

/// Dia (PL!S-bp5-004-R): Debut → choose 1 from {blade, position change}
/// Tests: execute_choice reads conditional_choice, executes selected option.
#[test]
fn dia_choose_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp5-004-R");
    let aqours = game.id("PL!S-bp2-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [aqours, -1, filler];
    debut_play(&mut game, dia, 15, MemberArea::Center);
    while game.state.has_pending_choice() {
        game.select_option(0);
    }
    while game.state.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.mods.get_blade_modifier(aqours) > 0,
        "Chika should have blade"
    );
}

/// Bouken (PL!S-bp6-020-L): LiveStart → choose 1 from 3
/// Tests: 3-option choice creation, option count verification.
#[test]
fn bouken_three_options() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let bouken = game.id("PL!S-bp6-020-L");
    let aqours = game.id("PL!S-bp2-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    game.state.player1.live_card_zone.cards.push(bouken);
    game.state.player1.stage.stage = [aqours, filler, -1];
    advance_to_live_card_set_p1(&mut game);

    while game.state.has_pending_choice() {
        if let Some(ref pc) = game.state.get_pending_choice_json() {
            if let Some(opts) = pc
                .as_object()
                .and_then(|j| j.get("options"))
                .and_then(|o| o.as_array())
            {
                assert_eq!(opts.len(), 3, "Should have exactly 3 options");
            }
        }
        game.select_option(0);
    }
}

/// Kotori (PL!-bp5-003-R+): 起動 — conditional_alternative
/// Discard non-μ's → retrieve 1 live card from discard.
/// Tests: conditional_alternative reads effect.condition fallback,
///        auto-evaluates without prompting, executes alt effect.
#[test]
fn kotori_discard_non_muse_retrieve_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    let kotori = game.id("PL!-bp5-003-R\u{ff0b}");
    let nm = game.id("PL!-sd1-003-SD");
    let live = game.id("PL!-sd1-019-SD");

    for p in [&mut game.state.player1, &mut game.state.player2] {
        p.main_deck.cards.clear();
        for _ in 0..40 {
            p.main_deck.cards.push(filler);
        }
    }
    game.state.player1.stage.stage = [kotori, filler, -1];
    game.state.player1.hand.cards.push(nm);
    game.state.player1.waitroom.cards.push(live);
    game.give_energy(10);
    game.state.turn_number = 1;

    game.activate_ability(kotori);
    while game.state.has_pending_choice() {
        game.select_indices(&[0]);
    }
    while game.state.has_pending_choice() {
        game.select_option(1);
    }

    assert!(
        game.state.player1.hand.cards.contains(&live),
        "Live card should be retrieved from discard"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "Retrieved card no longer in waitroom"
    );
}
