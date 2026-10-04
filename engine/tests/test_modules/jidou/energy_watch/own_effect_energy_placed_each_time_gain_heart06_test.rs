use crate::helpers::*;
use rabuka_engine::card::HeartColor;

// ═══════════════════════════════════════════════════════════════
// #16 — Hazuki (PL!SP-bp4-016-N): on_energy_placed_each_time
//
// カードの効果によって自分のエネルギー置き場にエネルギーカードが
// 置かれるたび、ライブ終了時まで、heart06を得る。
// (相手のカードの効果でも発動する。)
//
// Stage card (member). each_time auto ability.
//
// # The condition used to be a tautology, and this file used to confess it
//
// The parse was `comparison_condition { location: energy_zone, resource_type: energy,
// card_type: energy_card }` with **no operator and no count**. `compare_counts` reads a
// missing operator as `>=`, so it evaluated as `count >= 0` — true on every board. The
// header of this file used to say exactly that ("Condition: compares count of energy
// cards >= 0") and told every reader to seed an energy card in the fixture "so the
// default comparison passes", because a positive cannot tell a working condition from a
// vacuous one.
//
// What actually gated her was never the condition: it was the `each_time` movement-event
// scan guard at `core/game_state/abilities.rs`, whose clause is keyed on the condition
// being `Condition::Comparison` **specifically** — so it stopped applying the moment the
// parse became honest.
//
// The parse is now a `movement_condition` with `trigger_event { type: energy_placed,
// tense: past, energy_placed: true }`, identical in shape to its sibling
// `PL!SP-bp5-004-P` (`〜置き場にエネルギーが置かれたとき`). Two details were needed, both in
// `parser.py::_try_movement`:
//   * the rule accepted only past-tense 「置**かれた**」, and 「たび」 is habitual-present, so
//     「置**かれる**たび」 — the same event in the other tense — was excluded and fell through
//     to the generic extractor. The exclusion was deliberate and is now reversed, with the
//     reason recorded at the rule;
//   * the `energy_placed: true` flag was set by a regex requiring 「エネルギーが」, while this
//     card writes 「エネルギーカード**が**」.
//
// The flag is load-bearing, not decorative: without it the engine reads a `moves`
// condition with no energy check as an AREA-MOVE watcher and demands an area move that
// never happens, so she under-fires instead of over-firing. The four state tests below
// caught precisely that — which is the argument for changing a parse on the strength of a
// real test rather than a shape argument.
// ═══════════════════════════════════════════════════════════════

fn setup_hazuki(game: &mut TestGame) -> i16 {
    let hazuki = game.id("PL!SP-bp4-016-N");
    game.state.player1.stage.stage = [-1, hazuki, -1];
    let energy_card = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(energy_card);
    hazuki
}

fn heart06_mod(game: &TestGame, card_id: i16) -> i16 {
    game.state
        .mods
        .get_heart_modifier(card_id, HeartColor::Heart06)
}

fn trigger_auto(v: &mut TestGame) {
    let pid = v.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut v.state, &pid);
    v.state.process_pending_auto_abilities(&pid);
}

/// Energy placed by own effect → triggers heart06 gain.
#[test]
fn hazuki_energy_by_own_effect_triggers() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hazuki = setup_hazuki(&mut v);
    v.state
        .push_movement_event(-1, "energy_deck", "energy", None, "p1", true);

    trigger_auto(&mut v);

    assert_eq!(
        heart06_mod(&v, hazuki),
        1,
        "Hazuki gains heart06 when energy placed by own effect"
    );
}

/// Energy placed by opponent's effect → STILL triggers (card text says
/// "相手のカードの効果でも発動する" — also activates with opponent's card effects).
#[test]
fn hazuki_energy_by_opponent_effect_triggers() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hazuki = setup_hazuki(&mut v);
    v.state
        .push_movement_event(-1, "energy_deck", "energy", None, "p2", true);

    trigger_auto(&mut v);

    assert_eq!(
        heart06_mod(&v, hazuki),
        1,
        "Hazuki gains heart06 even when energy placed by opponent per card text"
    );
}

/// Energy phase draw — no push_movement_event for energy.
/// The each_time guard at abilities.rs blocks enqueue when
/// batch_movements is clear, even though the
/// comparison_condition would pass (energy card in zone).
#[test]
fn two_hazuki_copies_gain_once_per_opponent_effect_energy_placement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let left = game.id("PL!SP-bp4-016-N");
    let right = game.new_id("PL!SP-bp4-016-N");
    game.state.player1.stage.stage = [left, right, -1];

    for _ in 0..2 {
        let energy = game.id("LL-E-001-SD");
        game.state.player1.energy_zone.cards.push(energy);
        game.state
            .push_movement_event(-1, "energy_deck", "energy", None, "p2", true);
        trigger_auto(&mut game);
        game.drain_choices_strict(&["SelectAutoAbility"], &[0]);
    }

    assert_eq!(heart06_mod(&game, left), 2);
    assert_eq!(heart06_mod(&game, right), 2);
    assert_ne!(left, right);
    assert_eq!(game.state.player1.energy_zone.cards.len(), 2);
}

#[test]
fn hazuki_energy_phase_no_effect_flag() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hazuki = setup_hazuki(&mut v);
    v.state.batch_movements.clear();

    trigger_auto(&mut v);

    // Energy placed by phase action (not a card effect) — Hazuki should NOT trigger
    // because her text says "カードの効果によって" (by a card effect).
    // The each_time guard checks for a push_movement_event energy for comparison conditions
    // on energy_zone.
    assert_eq!(
        heart06_mod(&v, hazuki),
        0,
        "Hazuki should NOT trigger when energy placed by phase action, not a card effect"
    );
}

/// The parse must stay an EVENT, not a zone state — the tautology guard.
///
/// The clause's whole content is 「カードの効果によって自分のエネルギー置き場に
/// エネルギーカードが**置かれるたび**」. Parsed as a zone-occupancy check it cannot tell
/// 「energy is in my zone」 — which the energy phase satisfies on its own — from 「each time
/// a card effect puts energy there」, and the operator-less form of that check is true on
/// every board. All four state tests above pass identically under either reading, so only
/// the shape separates them.
#[test]
fn the_energy_clause_parses_as_an_event_not_a_zone_state() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let hazuki = game.id("PL!SP-bp4-016-N");
    let eff = game
        .db
        .get_card(hazuki)
        .unwrap()
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("自動"))
        .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
        .expect("自動 ability has an effect");
    let cond = eff.condition.as_deref().expect("the gate is a condition");

    assert!(
        matches!(cond, rabuka_engine::card::Condition::Movement { .. }),
        "the clause must be a Movement condition. It used to be a Comparison with no \
         operator and no count, which `compare_counts` reads as `count >= 0` — true on \
         every board — and which the `each_time` scan guard at \
         `core/game_state/abilities.rs` keys on by type. That gate was the only thing \
         deciding her fate; the condition was decoration."
    );
    assert_eq!(
        cond.get_movement(),
        Some("moves"),
        "「〜が置かれるたび」 is an event the trigger scan consumes, not a count over a zone"
    );
    assert_eq!(
        cond.get_self_effect_only(),
        None,
        "「カードの効果によって」 — any card, not 「自分のカード」. Her parenthetical \
         「(相手のカードの効果でも発動する)」 is the printed form of that, and the opponent \
         positive above is what holds the engine to it"
    );
    assert_eq!(
        cond.get_location(),
        Some("energy_zone"),
        "the placement is into the ENERGY ZONE — which the old comparison read as \
         「do I have energy?」, something the energy phase alone satisfies. Note that the \
         `energy_placed: true` marker rides on `trigger_event`, which is where the engine \
         reads it from (`condition.get_energy_placed().or_else(trigger_event)`); it is not \
         readable off the condition, so the type assertion above is the guard that matters."
    );
}
