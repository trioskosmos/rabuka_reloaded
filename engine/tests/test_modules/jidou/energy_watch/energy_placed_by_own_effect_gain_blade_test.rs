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
///
/// Renamed from `twice_per_turn_energy_watcher_own_effect_places_energy_gains_blade`:
/// this test places energy exactly ONCE, so the old name promised the ターン2回
/// second firing while exercising only the first. The real two-firing behaviour is
/// in `own_effect_energy_placed_blade_upper_bound_test` (asserted exactly: 1, then
/// 2, then refused) — see docs/JIDOU_COMBINATION_WORK.md §6, Bug B.
#[test]
fn first_own_effect_placement_gains_one_blade() {
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
fn once_per_turn_energy_watcher_own_effect_places_energy_gains_blade() {
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
fn once_per_turn_energy_watcher_no_energy_placed_no_blade() {
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

/// No energy placed → no blade (葉月 恋, ab#1 ターン2回).
#[test]
fn turn2_energy_watcher_no_energy_placed_no_blade() {
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
fn once_per_turn_energy_watcher_turn1_blocks_second_energy_placed() {
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
fn turn2_energy_watcher_opponent_effect_places_energy_no_blade() {
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

/// 葉月 恋 ab#1 is 自動 **ターン2回** — a second own-effect energy placement in the
/// same turn must grant again, and a third must be refused.
///
/// This is the Bug B regression guard on a REAL path: the two placements come from
/// two genuine debuts of `PL!SP-pb1-005-R` (its debut places one energy from the
/// energy deck into the zone), not from a poked `push_movement_event`. The poked
/// variant lives in `own_effect_energy_placed_blade_upper_bound_test`; driving it
/// through real card play proves the whole chain — debut → energy placement →
/// jidou fires — rather than just the event decoder.
///
/// Before the re-scan-guard fix the second debut was swallowed (the guard leaked
/// into the next action, and ブレード+1 is a non-movement effect), so this read 1.
#[test]
fn turn2_watcher_second_real_debut_placement_gains_again_and_third_is_refused() {
    let mut game = TestGame::new(load_real_database());

    let ren = game.id("PL!SP-bp7-005-R＋"); // ab#1: 自動 ターン2回
    game.assert_card_identity(ren, "PL!SP-bp7-005-R＋");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [ren, -1, -1];
    // Two energy-deck cards so each placer debut really has one to place, plus
    // enough ACTIVE zone energy that playing the second placer's cost succeeds
    // (the cost is paid from the energy zone; the deck cards are the jidou food).
    game.state
        .player1
        .energy_deck
        .cards
        .push(game.id("LL-E-001-SD"));
    game.state
        .player1
        .energy_deck
        .cards
        .push(game.id("LL-E-001-SD"));
    game.give_energy(12);

    let blade_before = blade(&game, ren);

    // --- 1st real placement: play the placer to the RIGHT ---
    let placer1 = game.id(ENERGY_PLACER);
    game.state.player1.hand.cards.push(placer1);
    game.play_to_stage(placer1, MemberArea::RightSide);
    drain_auto_choices(&mut game);
    assert_eq!(
        blade(&game, ren),
        blade_before + 1,
        "debut 1: the placer's energy landed by YOUR effect → 恋 gains ブレード"
    );

    // --- 2nd real placement: play a second copy to the LEFT ---
    // The LEFT slot is free, so this is a second genuine debut -> a second
    // own-effect energy placement in the same turn.
    let placer2 = game.id(ENERGY_PLACER);
    game.state.player1.hand.cards.push(placer2);
    game.play_to_stage(placer2, MemberArea::LeftSide);
    drain_auto_choices(&mut game);
    assert_eq!(
        blade(&game, ren),
        blade_before + 2,
        "debut 2: the SECOND own-effect placement in the same turn must grant again \
         (ターン2回). Before the re-scan-guard fix this stayed at 1 — the guard \
         leaked across the action and vetoed the second firing."
    );

    // --- The stage is now full, so a third placement is impossible this turn ---
    // 恋 + 2 placers occupy all three slots, so no further debut can happen: both
    // ターン2回 allowances are exactly what was just consumed. Center is the empty
    // slot because the placers took LEFT and RIGHT.
    assert_eq!(
        game.state.player1.stage.stage,
        [placer2, -1, placer1],
        "precondition: 恋 plus both placers now fill LEFT and RIGHT, so no third \
         debut is possible this turn — the two ターン2回 allowances are spent"
    );
}
