//! Jidou combination edge cases: multiple jidou on same card + jidou watching other triggers + effect-cause gating.
//! Covers TEST_COVERAGE.md C. cards pairing jidou with another ability — ensures BOTH fire, not just one.
//! Uses existing test idioms: fill_decks, fire_trigger, push_movement_event, recalculate_constants.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn jidou_watching_live_start_resolve_triggers() {
    // PL!-bp6-020-L (Dancing stars) watches muse LiveStart in center — copy idiom from bp6_020_dancing_stars_watchers_test.rs
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let watcher = game.id("PL!-bp6-020-L");
    let muse = game.id("PL!-bp6-001-R＋");
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(watcher);
    game.state.player1.stage.stage[1] = muse;
    fire_trigger(&mut game, muse, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice(), "watcher should trigger on muse LS");
    // Use same helper as existing test: pick generated action with stage_area == "left"
    let actions = game.generated_actions();
    let idx = actions.iter().position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("left")).unwrap_or(0);
    game.select_generated(idx);
    assert_eq!(game.state.player1.stage.stage[0], muse);
}

#[test]
fn jidou_effect_cause_both_sides() {
    // PL!SP-bp7-005-R＋ has two jidou: one on 登場/energy deck→energy, one on
    // energy placed by own effect.
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let jidou = game.id("PL!SP-bp7-005-R＋");
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    fill_energy_deck(&mut game, 0, 3);
    game.state.player1.stage.stage = [jidou, -1, -1];

    // The event that actually arms the energy-placed jidou: an energy card
    // placed by our own effect.
    let blades_before = game.state.mods.blade_modifiers.len();
    let energy_zone_before = game.state.player1.energy_zone.cards.len();
    game.state.push_movement_event(-1, "energy_deck", "energy", Some(jidou), "p1", true);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    // A DELTA, not a comparison. The previous assertion here was
    // `after_blades >= before_blades`, which a jidou that grants nothing — or
    // worse, one that is not wired at all — satisfies trivially. The sibling test
    // `jidou_paired_with_other_ability_both_fire` gets this right with
    // `len - before == 1`; this one did not, so its own doc claim ("effect-cause
    // gating") was never actually checked.
    assert_eq!(
        game.state.mods.blade_modifiers.len() - blades_before,
        1,
        "an own-effect energy placement must grant exactly this card's one blade"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_zone_before,
        "the blade jidou does not itself move energy into the zone"
    );
    assert!(game.state.player1.stage.stage.contains(&jidou));
}

#[test]
fn jidou_paired_with_other_ability_both_fire() {
    // PL!SP-bp7-005-R＋ carries TWO jidou: ab#0 is a sequential that places an
    // energy card from the energy deck in the wait state, ab#1 grants a blade.
    // They have to coexist, so this asserts each one's own effect — checking
    // only blade_modifiers saw just ab#1 and passed when ab#0 did nothing.
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let card = game.id("PL!SP-bp7-005-R＋");
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.stage.stage = [card, -1, -1];
    // ab#0 draws from the energy deck, and fill_decks only fills the MAIN deck —
    // without this the sequential silently did nothing and the test proved
    // nothing about it.
    fill_energy_deck(&mut game, 0, 3);
    let blades_before = game.state.mods.blade_modifiers.len();
    let energy_deck_before = game.state.player1.energy_deck.cards.len();
    // Trigger first jidou via登場 gate: push movement that counts as appeared
    game.state.push_movement_event(card, "hand", "stage", Some(card), "p1", true);
    game.state.record_card_appearance(card, "hand");
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    // Trigger second jidou via energy placed
    game.state.push_movement_event(-1, "energy_deck", "energy", Some(card), "p1", true);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert!(game.state.player1.stage.stage.contains(&card));
    // ab#1: exactly one blade granted. '>= before' passed when NEITHER jidou
    // fired, which is the case this test exists to rule out.
    assert_eq!(
        game.state.mods.blade_modifiers.len() - blades_before,
        1,
        "ab#1 granted its blade"
    );
    // ab#0: the sequential placed one energy card out of the energy deck.
    assert_eq!(
        energy_deck_before - game.state.player1.energy_deck.cards.len(),
        1,
        "ab#0 placed one energy card from the energy deck"
    );
}

#[test]
fn jidou_distinct_from_constant_and_activation() {
    // Use SP-bp7-005-R＋ (two jidou) plus a generic constant member to prove paths are distinct
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let m = game.id("PL!SP-bp7-005-R＋");
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.stage.stage = [m, -1, -1];
    let before = game.state.mods.blade_modifiers.clone();
    game.state.recalculate_constants();
    let after_const = game.state.mods.blade_modifiers.clone();
    // Constant recalc should be idempotent for pure jidou card (no constant on this card)
    assert_eq!(before.len(), after_const.len(), "constant recalc should be idempotent for jidou");

    // The point of the test is that the 自動 path is DISTINCT from the constant
    // path: recalculating constants must not absorb a jidou, and driving the
    // jidou must actually move the modifier set.
    //
    // The previous final assertion was
    // `after_jidou.len() >= after_const.len() || after_jidou.len() == after_const.len()`,
    // which is a tautology — `a >= b || a == b` holds for every pair — so this
    // test asserted nothing at all about the jidou path, while its own comment
    // conceded "may or may not add blade depending on trigger".
    game.state.push_movement_event(-1, "energy_deck", "energy", Some(m), "p1", true);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let after_jidou = game.state.mods.blade_modifiers.clone();
    assert_eq!(
        after_jidou.len() - after_const.len(),
        1,
        "driving the 自動 must add exactly this card's blade, and it must not be \
         folded into the constant path (recalculate_constants alone added nothing)"
    );
}

#[test]
fn jidou_both_on_same_card_coexist_and_fire_separately() {
    // PL!SP-bp7-005-R＋ has two jidou with different triggers (登場 vs energy placed).
    // Ensure they are distinct and can both be queued without shadowing.
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let card = game.id("PL!SP-bp7-005-R＋");
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    assert_eq!(db.get_card(card).unwrap().abilities.len(), 2, "SP-bp7-005 should have 2 jidou");
    game.state.player1.stage.stage = [card, -1, -1];
    // Fire first jidou via登場 (hand→stage)
    game.state.push_movement_event(card, "hand", "stage", Some(card), "p1", true);
    game.state.record_card_appearance(card, "hand");
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let after_first = game.state.player1.energy_zone.cards.len();
    // Baseline for the second trigger's blade delta.
    let blade_mods_before_second = game.state.mods.blade_modifiers.len();
    // Fire second jidou via energy placed by own effect
    game.state.push_movement_event(-1, "energy_deck", "energy", Some(card), "p1", true);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    // The turn1 jidou places a waited energy card into the energy deck; the
    // turn2 jidou only adds a blade — neither moves cards between energy
    // zones, so the energy zone count must be exactly unchanged.
    assert!(game.state.player1.stage.stage.contains(&card));
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        after_first,
        "neither jidou moves cards into the energy zone"
    );
    // A DELTA against the pre-second-trigger baseline. The previous check was
    // `!blade_modifiers.is_empty()`, which 葉月恋 satisfies from her OWN printed
    // blades whether or not the jidou ever fired — so the assertion could not
    // distinguish "granted" from "not wired at all". The sibling test in this
    // file already uses the correct delta form.
    assert_eq!(
        game.state.mods.blade_modifiers.len() - blade_mods_before_second,
        1,
        "the energy-placed jidou added exactly its one blade on the second trigger"
    );
}
