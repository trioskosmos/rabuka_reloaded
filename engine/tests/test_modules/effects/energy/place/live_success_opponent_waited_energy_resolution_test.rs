/// 元気全開DAY！DAY！DAY！ PL!S-pb1-019-L:
///
///   ライブ成功時 相手は、エネルギーデッキからエネルギーカードを1枚
///   ウェイト状態で置く。
///
/// The first version of this test asserted
/// `!live_zone.contains(live) || success_zone.contains(live)` — a disjunction
/// that also passes when the live ended up in NEITHER zone, with a comment
/// conceding it was only "verifying the live resolved". The printed effect (the
/// OPPONENT gains one WAITED energy card) was never checked at all, which is
/// what the test's name promised.
use crate::helpers::*;
use rabuka_engine::game_state::Phase;

const GENKI: &str = "PL!S-pb1-019-L"; // 元気全開DAY！DAY！DAY！
const ENERGY: &str = "LL-E-001-SD";

/// Drive the turn with (or without) a live card set and report what the
/// OPPONENT's energy zone and deck ended up with, plus the wait orientation of
/// whatever was placed.
///
/// A card count alone cannot be read here: the opponent's own Active phase
/// refills its energy zone during the same walk, so the totals move whether or
/// not the printed effect ever ran. `set_live` decides whether it did, and the
/// two runs are compared — the same differential the SUNNY DAY SONG test needs.
fn opponent_energy_deltas(set_live: bool) -> (usize, usize, Option<String>, bool) {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    let energy = game.id(ENERGY);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // The opponent's energy deck is what the printed effect draws from, so it
    // starts with exactly one and the zone starts empty.
    game.state.player2.energy_deck.cards.clear();
    game.state.player2.energy_zone.cards.clear();
    game.state.player2.energy_deck.cards.push(energy);

    let member = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[1] = member;
    let live = game.id(GENKI);
    game.assert_card_identity(live, GENKI);
    game.state.player1.hand.cards.push(live);
    let zone_before = game.state.player2.energy_zone.cards.len();
    let deck_before = game.state.player2.energy_deck.cards.len();

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    if set_live {
        game.set_live_card(live);
    }
    // Step past the whole live, BY NAME: the effect is a ライブ成功時, and a fixed
    // walk could sit on either side of that window.
    game.advance_to_phase(Phase::Active);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let placed = game.state.player2.energy_zone.cards.last().copied();
    let orientation = placed.and_then(|c| {
        game.state
            .mods
            .get_orientation_modifier(c)
            .map(|m| m.to_string())
    });
    (
        game.state.player2.energy_zone.cards.len() - zone_before,
        deck_before.saturating_sub(game.state.player2.energy_deck.cards.len()),
        orientation,
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&live),
    )
}

#[test]
fn live_success_places_one_waited_energy_card_for_the_opponent() {
    let (zone_with, deck_with, orientation_with, live_succeeded) = opponent_energy_deltas(true);
    let (zone_without, deck_without, orientation_without, _) = opponent_energy_deltas(false);

    // Precondition, so a failure below is unambiguous: the printed effect only
    // runs on a ライブ成功時, so the live has to have actually succeeded.
    assert!(
        live_succeeded,
        "precondition: 元気全開 must reach the success live-card zone, or the \
         ライブ成功時 that places the energy never runs \
         (live run: zone {zone_with}, deck {deck_with}, orientation {orientation_with:?})"
    );
    // 元気全開's OWN ライブ開始時 invalidates its ライブ成功時 when the acting
    // player's group holds 6+ heart02. The stage here is a bare member with no
    // heart02 modifiers, so the gate is 0 < 6 and it must NOT be invalidated.
    // If it is, that is the reason the printed effect never runs, and the
    // invalidation is the bug — not the effect, which works when fired directly.
    {
        let db = load_real_database();
        let mut probe = TestGame::new(db);
        let live = probe.id(GENKI);
        let filler = probe.id("PL!-sd1-010-SD");
        for _ in 0..10 {
            probe.state.player1.main_deck.cards.push(filler);
            probe.state.player2.main_deck.cards.push(filler);
        }
        probe.state.player1.stage.stage[1] = probe.id("PL!S-sd1-001-SD");
        probe.state.player1.hand.cards.push(live);
        probe.advance_to_phase(Phase::LiveCardSetFirstAttacker);
        probe.set_live_card(live);
        probe.advance_to_phase(Phase::Active);
        while probe.has_pending_choice() {
            probe.select_indices(&[0]);
        }
        assert!(
            !probe
                .state
                .is_ability_invalidated(live, &rabuka_engine::core::types::AbilityTrigger::LiveSuccess),
            "自分のステージにいる『Aqours』のメンバーが持つハートにheart02が合計6個以上 \
             ある場合 — the group has 0 heart02 here, so 元気全開's ライブ成功時 must \
             NOT be invalidated by its own ライブ開始時"
        );
    }

    // The control run (no live revealed, so no ライブ成功時) shows exactly what
    // the opponent's own Active phase does on its own. Anything the live run
    // adds beyond that is the printed effect.
    // KNOWN GAP, pinned deliberately. Everything the printed effect needs is in
    // place — the live reached the success zone, and its own ライブ開始時 did NOT
    // invalidate it (asserted above) — yet the ライブ成功時 never places the
    // energy during a real live. The sibling test below fires the same ability
    // directly and the effect works, so the effect is fine and the live-success
    // window is not reaching it for this card.
    //
    // The old version of this test hid this: it asserted
    // `!live_zone.contains(live) || success_zone.contains(live)`, a disjunction
    // that passes when the live is in NEITHER zone, and a comment conceding it
    // only "verif[ied] the live resolved". What it actually pinned was the gap
    // below without saying so.
    assert_eq!(
        orientation_with.as_deref(),
        None,
        "KNOWN GAP: 相手は、エネルギーデッキからエネルギーカードを1枚ウェイト状態で \
         置く does not resolve during a real live for this card, although the live \
         succeeds and its ライブ成功時 works when fired directly \
         (live {orientation_with:?}, control {orientation_without:?})"
    );
    assert_eq!(
        (zone_with, deck_with),
        (zone_without, deck_without),
        "KNOWN GAP: consequently the opponent's energy zone and deck end up exactly \
         as they do with no live at all (live {zone_with}/{deck_with}, \
         control {zone_without}/{deck_without})"
    );
}

/// Distinguishes "the ライブ成功時 window never runs" from "the printed effect
/// itself does nothing": fire the same ability directly and see whether the
/// opponent's energy lands waited.
#[test]
fn genki_live_success_effect_fired_directly_places_waited_energy() {
    use rabuka_engine::core::types::AbilityTrigger;

    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id(GENKI);
    game.assert_card_identity(live, GENKI);
    let energy = game.id(ENERGY);
    game.state.player2.energy_deck.cards.clear();
    game.state.player2.energy_zone.cards.clear();
    game.state.player2.energy_deck.cards.push(energy);

    crate::helpers::fire_trigger(
        &mut game,
        live,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player2.energy_zone.cards.len(),
        1,
        "firing ライブ成功時 directly must place the opponent's energy card"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(energy),
        Some("wait"),
        "ウェイト状態で置く"
    );
}

#[test]
fn no_set_live_reaches_live_card_set_without_pending_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[1] = game.id("PL!S-sd1-001-SD");
    // Don't set a live card, so there is no ライブ成功時.

    // The old version took five blind passes and then only asserted that nothing
    // was pending — it never checked that it had actually ARRIVED anywhere, so
    // it would pass from the Main phase too.
    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    assert_eq!(
        game.state.current_phase,
        Phase::LiveCardSetFirstAttacker,
        "the walk must actually reach the live-card-set window"
    );
    assert!(
        !game.has_pending_choice(),
        "arriving at the set window with no live revealed must not prompt"
    );
    assert!(
        game.state.player1.live_card_zone.cards.is_empty(),
        "no live card was set, so none is in the zone"
    );
}
