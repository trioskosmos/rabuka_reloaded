/// BP07 CLEAN-G17: 葉月 恋 "energy placed into your energy zone by your card's
/// effect → gain blade ×1 until live end."
///
/// D25: PL!SP-bp7-005-R＋ 葉月 恋 ab#1 (自動, ターン2回)
/// D24: PL!SP-bp7-016-N 葉月 恋 ab#0 (自動, ターン1回)
///
/// 自分のカードの効果によって、自分のエネルギー置き場にエネルギーが置かれたとき、
/// ライブ終了時まで、ブレードを得る。
///
/// Real gameplay: PL!SP-pb1-005-R (葉月かほり) debut places 1 energy from the
/// energy deck into the energy zone. That effect-driven placement must fire the
/// auto trigger and grant blade ×1.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

/// PL!SP-pb1-005-R debut: place 1 energy from energy_deck → energy_zone (WAIT).
const ENERGY_PLACER: &str = "PL!SP-pb1-005-R";

fn drain_auto_choices(game: &mut TestGame) {
    while game.has_pending_choice() {
        match game.get_pending_choice().clone() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            other => panic!(
                "expected only auto-ability ordering choices, got {:?}",
                other
            ),
        }
    }
}

fn fill_deck_and_energy(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..15 {
        game.state.player1.energy_zone.cards.push(filler);
    }
    game.state.player1.energy_zone.set_active_count(15);
}

fn blade(game: &TestGame, cid: i16) -> i32 {
    game.state.mods.get_blade_modifier(cid)
}

/// 葉月 恋 is on stage; a card whose debut places energy into the zone is played.
/// The placed energy must trigger 葉月 恋's auto → gain blade ×1.
#[test]
fn ren_ab1_own_effect_places_energy_gains_blade() {
    let mut game = TestGame::new(load_real_database());

    let ren = game.id("PL!SP-bp7-005-R＋"); // ab#1: ターン2回
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [ren, -1, -1];
    game.state.player1.energy_deck.cards.push(game.id("PL!-sd1-010-SD"));

    let placer = game.id(ENERGY_PLACER);
    game.state.player1.hand.cards.push(placer);

    let energy_zone_before = game.state.player1.energy_zone.cards.len();
    let blade_before = blade(&game, ren);
    game.play_to_stage(placer, MemberArea::RightSide);
    drain_auto_choices(&mut game);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_zone_before + 1,
        "the placer card's debut should place 1 energy into the zone"
    );
    assert_eq!(
        blade(&game, ren),
        blade_before + 1,
        "own card effect placed energy into the zone → 葉月 恋 gains blade ×1"
    );
}

/// PL!SP-bp7-016-N ab#0 (ターン1回) — same trigger, different card.
#[test]
fn ren_016_ab0_own_effect_places_energy_gains_blade() {
    let mut game = TestGame::new(load_real_database());

    let ren = game.id("PL!SP-bp7-016-N");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [ren, -1, -1];
    game.state.player1.energy_deck.cards.push(game.id("PL!-sd1-010-SD"));

    let placer = game.id(ENERGY_PLACER);
    game.state.player1.hand.cards.push(placer);

    let blade_before = blade(&game, ren);
    game.play_to_stage(placer, MemberArea::RightSide);
    drain_auto_choices(&mut game);

    assert_eq!(
        blade(&game, ren),
        blade_before + 1,
        "own card effect placed energy → PL!SP-bp7-016-N gains blade ×1"
    );
}

/// No energy placed → no blade.
#[test]
fn ren_016_no_energy_placed_no_blade() {
    let mut game = TestGame::new(load_real_database());

    let ren = game.id("PL!SP-bp7-016-N");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [ren, -1, -1];

    let blade_before = blade(&game, ren);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    drain_auto_choices(&mut game);

    assert_eq!(
        blade(&game, ren),
        blade_before,
        "no effect placed energy → PL!SP-bp7-016-N must not gain blade"
    );
}

/// No energy placed → no blade.
#[test]
fn ren_ab1_no_energy_placed_no_blade() {
    let mut game = TestGame::new(load_real_database());

    let ren = game.id("PL!SP-bp7-005-R＋");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [ren, -1, -1];

    let blade_before = blade(&game, ren);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    drain_auto_choices(&mut game);

    assert_eq!(
        blade(&game, ren),
        blade_before,
        "no effect placed energy into the zone → no blade"
    );
}

/// Turn1 blocks second energy placement same turn.
#[test]
fn ren_016_turn1_blocks_second_energy_placed() {
    let mut game = TestGame::new(load_real_database());
    let ren = game.id("PL!SP-bp7-016-N");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [ren, -1, -1];
    game.state.player1.energy_deck.cards.push(game.id("PL!-sd1-010-SD"));
    game.state.player1.energy_deck.cards.push(game.id("PL!-sd1-010-SD"));
    let placer = game.id(ENERGY_PLACER);
    game.state.player1.hand.cards.push(placer);
    game.state.player1.hand.cards.push(game.id(ENERGY_PLACER));
    let blade_before = blade(&game, ren);
    game.play_to_stage(placer, MemberArea::RightSide);
    drain_auto_choices(&mut game);
    assert_eq!(blade(&game, ren), blade_before + 1);
    // Second placer same turn
    let placer2 = game.id(ENERGY_PLACER);
    game.state.player1.hand.cards.push(placer2);
    // Need to give energy for second placer? It also places energy, but turn limit should block second blade
    // We simulate second energy placement via direct movement event (own effect)
    game.state.push_movement_event(-1, "energy_deck", "energy", Some(placer2), "p1", true);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    drain_auto_choices(&mut game);
    assert_eq!(blade(&game, ren), blade_before + 1, "ターン1回 should block second blade");
}

/// An OPPONENT card's effect placing energy must NOT trigger ("自分のカードの効果").
#[test]
fn ren_ab1_opponent_effect_places_energy_no_blade() {
    let mut game = TestGame::new(load_real_database());

    let ren = game.id("PL!SP-bp7-005-R＋");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [ren, -1, -1];

    let blade_before = blade(&game, ren);
    // Opponent's effect places energy into player1's zone — must not fire.
    // cause_player_id "p2", effect-driven (last arg true).
    game.state
        .push_movement_event(-1, "energy_deck", "energy", None, "p2", true);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &pid,
    );
    game.state.process_pending_auto_abilities(&pid);
    drain_auto_choices(&mut game);

    assert_eq!(
        blade(&game, ren),
        blade_before,
        "opponent card's effect must not trigger (自分のカードの効果 only)"
    );
}
