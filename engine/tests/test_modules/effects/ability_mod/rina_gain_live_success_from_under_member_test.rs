/// Tests for gain_ability_from_source (ability copying from cards under member).
///
/// Card: PL!N-PR-026-PR | 天王寺璃奈 (ab#1)
/// 常時: このメンバーは、このメンバーの下に置かれているコスト9以下の『虹ヶ咲』の
/// メンバーカードが持つライブ成功時能力をすべて得る。
///
/// NOTE: gained_abilities stores the triggerless_text of copied abilities, and
/// the runtime LiveSuccess path is exercised below after the copying setup.
use crate::helpers::*;
use rabuka_engine::core::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Basic: Rina on stage, Ayumu under her → Ayumu's live_success ability text is stored.
#[test]
fn rina_gains_ability_from_under_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-PR-026-PR");
    // Ayumu (PL!N-bp4-001-R): 虹ヶ咲 member, cost=2, has live_success ability
    let ayumu = game.id("PL!N-bp4-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [rina, filler, -1];
    game.state.player1.stage.under_cards[0].push(ayumu);

    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.pass();
    game.pass();

    let gained = game.state.gained_abilities.get(&rina);
    assert!(gained.is_some(), "Rina should have gained abilities");
    if let Some(list) = gained {
        assert!(!list.is_empty(), "Should copy at least one ability");
        // Verify the entry format: ability_from_source:{db_id}:{triggerless_text}
        assert!(
            list[0].starts_with("ability_from_source:"),
            "Bad entry format"
        );
    }
}

/// Trigger filter: a card with 常時 (constant) ability under Rina should NOT be copied.
#[test]
fn rina_only_copies_live_success_not_constant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-PR-026-PR");
    // Asaka Karin (PL!N-PR-027-PR): 虹ヶ咲 member, cost=4, has 常時 ability (not live_success)
    let karin = game.id("PL!N-PR-027-PR");
    let filler = game.id("PL!-sd1-010-SD");
    game.assert_card_identity(rina, "PL!N-PR-026-PR");
    game.assert_card_identity(karin, "PL!N-PR-027-PR");
    // The whole point is that this card's ability is a 常時, not a ライブ成功時.
    // Assert the reason rather than trusting the comment: if Karin ever printed a
    // LiveSuccess ability this test would keep passing for the wrong reason.
    let karin_triggers: Vec<String> = game
        .db
        .get_card(karin)
        .unwrap()
        .resolved_abilities()
        .filter_map(|a| a.triggers.as_ref().map(|t| t.to_string()))
        .collect();
    assert!(
        karin_triggers.iter().any(|t| t.contains("常時")),
        "the under-card must carry a 常時, got {karin_triggers:?}"
    );
    assert!(
        !karin_triggers.iter().any(|t| t.contains("ライブ成功時")),
        "the under-card must NOT carry a ライブ成功時 for this test to mean \
         anything, got {karin_triggers:?}"
    );

    game.state.player1.stage.stage = [rina, filler, -1];
    game.state.player1.stage.under_cards[0].push(karin);

    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.pass();
    game.pass();

    // Nothing is gained, and the gain table has no half-populated entry for her.
    let gained = game.state.gained_abilities.get(&rina);
    assert!(
        gained.is_none() || gained.unwrap().is_empty(),
        "Rina should not gain constant abilities (trigger filter mismatch)"
    );
}

/// Cost limit: card under member with cost > 9 should NOT be copied.
#[test]
fn rina_respects_cost_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-PR-026-PR");
    // Setsuna (PL!N-bp4-007-R+, cost=13) is 虹ヶ咲 member with live_success,
    // but cost 13 > 9 → filtered out
    let setsuna = game.id("PL!N-bp4-007-R+");
    let filler = game.id("PL!-sd1-010-SD");
    game.assert_card_identity(rina, "PL!N-PR-026-PR");
    game.assert_card_identity(setsuna, "PL!N-bp4-007-R＋");
    // Pin BOTH halves of the filter's premise: the card really is over the cost
    // ceiling, and it really does print the ライブ成功時 that is being filtered.
    // Without these the test only proves "nothing was gained", which stays true
    // if the fixture drifts to a card that was never eligible.
    game.assert_card_cost(setsuna, 13);
    let setsuna_triggers: Vec<String> = game
        .db
        .get_card(setsuna)
        .unwrap()
        .resolved_abilities()
        .filter_map(|a| a.triggers.as_ref().map(|t| t.to_string()))
        .collect();
    assert!(
        setsuna_triggers.iter().any(|t| t.contains("ライブ成功時")),
        "the over-cost card must still print a ライブ成功時, otherwise this is not \
         a cost-filter test, got {setsuna_triggers:?}"
    );

    game.state.player1.stage.stage = [rina, filler, -1];
    game.state.player1.stage.under_cards[0].push(setsuna);

    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.pass();
    game.pass();

    let gained = game.state.gained_abilities.get(&rina);
    assert!(
        gained.is_none() || gained.unwrap().is_empty(),
        "Setsuna (cost=13) should be filtered by cost limit"
    );
}

/// Rina's 常時 is a per-stage-member scan, so a bare `is_none()` on the gain
/// table cannot say WHICH branch produced the empty result — the scan not
/// reaching her, the host not being on stage, the under-card not existing, and
/// the under-card being filtered all look identical.
///
/// This runner exists so a negative can be read against its own control. Both
/// runs share the copy source, the decks and the energy, and differ in exactly
/// the one fixture detail named by the flag:
///
/// - `rina_on_stage` — whether Rina is the left-area host or a card in hand.
/// - `ayumu_under_rina` — whether the eligible 虹ヶ咲 card sits under the host.
///
/// Two passes, as the rest of this file used, but stepped by name.
fn rina_gain_run(rina_on_stage: bool, ayumu_under_rina: bool) -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-PR-026-PR");
    let ayumu = game.id("PL!N-bp4-001-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.assert_card_identity(rina, "PL!N-PR-026-PR");
    game.assert_card_identity(ayumu, "PL!N-bp4-001-R");
    // The copy source must be ELIGIBLE, or "nothing was gained" is true for a
    // reason that has nothing to do with the gate under test: 虹ヶ咲, cost 2
    // (under the コスト9以下 ceiling), and it really does print the
    // ライブ成功時 that ab#1 copies.
    game.assert_card_cost(ayumu, 2);
    game.assert_card_in_group(ayumu, "A・ZU・NA", "the copy source is 虹ヶ咲");
    let triggers: Vec<String> = game
        .db
        .get_card(ayumu)
        .unwrap()
        .resolved_abilities()
        .filter_map(|a| a.triggers.as_ref().map(|t| t.to_string()))
        .collect();
    assert!(
        triggers.iter().any(|t| t.contains("ライブ成功時")),
        "precondition: the copy source must print a ライブ成功時, got {triggers:?}"
    );

    let host = if rina_on_stage { rina } else { filler };
    game.state.player1.stage.stage = [host, filler, -1];
    if !rina_on_stage {
        game.state.player1.hand.cards.push(rina);
    }
    if ayumu_under_rina {
        game.state.player1.stage.under_cards[0].push(ayumu);
    }

    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.advance_to_phase(Phase::Energy);
    (game, rina, ayumu)
}

/// No cards under member: the scan reaches Rina and copies nothing.
///
/// Previously this asserted only `gained.is_none() || is_empty()`, which stays
/// green if the 常時 stops being evaluated at all. The control run is the same
/// fixture with one eligible under-card added, so a regression that stops the
/// scan now fails here rather than passing quietly.
#[test]
fn rina_no_under_cards() {
    let (control, control_rina, _) = rina_gain_run(true, true);
    assert!(
        control
            .state
            .gained_abilities
            .get(&control_rina)
            .is_some_and(|list| !list.is_empty()),
        "control: with an eligible under-card Rina MUST copy something, otherwise \
         the negative run below proves nothing"
    );

    let (game, rina, _) = rina_gain_run(true, false);
    assert!(
        game.state.player1.stage.stage.contains(&rina),
        "setup guard: Rina really is the on-stage host"
    );
    assert!(
        game.state.player1.stage.under_cards[0].is_empty(),
        "setup guard: the case under test is an EMPTY under-area, got {:?}",
        game.state.player1.stage.under_cards[0]
    );

    // Two tables, not one: `gained_abilities` holds copied triggerless texts,
    // `gained_card_abilities` the resolved Ability structs the trigger pipeline
    // scans. Both empty says the 常時 ran and found nothing to copy.
    assert_eq!(
        game.state.gained_abilities.get(&rina),
        None,
        "an empty under-area must contribute no copied ability texts"
    );
    assert!(
        !game.state.gained_card_abilities.contains_key(&rina),
        "an empty under-area must contribute no runtime Ability structs (got {:?})",
        game.state.gained_card_abilities.get(&rina)
    );
}

/// Rina not on stage: an eligible under-card is present and still nothing is
/// copied, because the scan is per stage member.
///
/// The old fixture put Ayumu under a filler member with Rina in hand, and
/// asserted only that the table was empty — which is also what a fixture with no
/// eligible under-card at all produces. Here the under-card is present and
/// pinned eligible, so the empty table names the gate: the HOST must be on
/// stage, not the source.
#[test]
fn rina_not_on_stage() {
    let (control, control_rina, _) = rina_gain_run(true, true);
    assert!(
        control
            .state
            .gained_abilities
            .get(&control_rina)
            .is_some_and(|list| !list.is_empty()),
        "control: Rina on stage with an eligible under-card MUST copy something"
    );

    let (game, rina, ayumu) = rina_gain_run(false, true);
    assert!(
        !game.state.player1.stage.stage.contains(&rina)
            && !game.state.player2.stage.stage.contains(&rina),
        "setup guard: Rina really is on nobody's stage"
    );
    assert!(
        game.state.player1.hand.cards.contains(&rina),
        "setup guard: Rina is in hand, not on stage"
    );
    assert!(
        game.state.player1.stage.under_cards[0].contains(&ayumu),
        "setup guard: an ELIGIBLE under-card is present, so the only reason \
         nothing is copied is that its host is not on stage"
    );

    assert_eq!(
        game.state.gained_abilities.get(&rina),
        None,
        "a card in hand must not gain abilities, even from an eligible under-card"
    );
    assert!(
        !game.state.gained_card_abilities.contains_key(&rina),
        "a card in hand must gain no runtime Ability structs (got {:?})",
        game.state.gained_card_abilities.get(&rina)
    );
}

/// Multiple matching cards: both should have their ability texts copied.
#[test]
fn rina_copies_from_multiple_under_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-PR-026-PR");
    let ayumu = game.id("PL!N-bp4-001-R"); // cost=2, 虹ヶ咲, live_success
    let shizuku = game.id("PL!N-bp4-003-R"); // cost=4, 虹ヶ咲, live_success
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [rina, filler, -1];
    game.state.player1.stage.under_cards[0].push(ayumu);
    game.state.player1.stage.under_cards[0].push(shizuku);

    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.pass();
    game.pass();

    // The gained entry must EXIST before its length means anything: the old
    // `if let Some(list)` had no else, so a run that copied nothing skipped the
    // only assertion and the test passed.
    let gained = game
        .state
        .gained_abilities
        .get(&rina)
        .expect("Rina must have gained abilities from her under-cards");
    // Exactly one per under-card, from the two cards placed under her.
    assert_eq!(
        gained.len(),
        2,
        "one ability copied per under-card, got {:?}",
        gained
    );
}

fn setup_rina_live_success(game: &mut TestGame, p1_energy: usize, p2_energy: usize) -> (i16, i16) {
    let rina = game.new_id("PL!N-PR-026-PR");
    let ayumu = game.new_id("PL!N-bp4-001-R");
    let live = game.new_id("PL!HS-bp1-019-L");
    let filler = game.new_id("PL!-sd1-010-SD");
    let energy = game.new_id("LL-E-001-SD");
    let deck_energy = game.new_id("LL-E-001-SD");

    game.state.player1.stage.stage = [rina, filler, -1];
    game.state.player1.stage.under_cards[0].push(ayumu);
    game.state.player1.energy_zone.cards.clear();
    game.state.player2.energy_zone.cards.clear();
    for _ in 0..p1_energy {
        game.state.player1.energy_zone.push_active(energy);
    }
    for _ in 0..p2_energy {
        game.state.player2.energy_zone.push_active(energy);
    }
    game.state.player1.energy_deck.cards.push(deck_energy);
    game.state.player1.live_card_zone.cards.push(live);
    let mut stage_hearts = BaseHeart {
        hearts: HeartMap::new(),
    };
    stage_hearts.hearts.insert(HeartColor::Heart00, 4);
    game.state.player1.stage_hearts = Some(stage_hearts);
    game.state.current_phase = Phase::LiveVictoryDetermination;
    game.state.recalculate_constants();
    // The three steps above — wildcard hearts, the live in the zone, and
    // LiveVictoryDetermination — are what OPEN the ライブ成功時 window that the
    // callers below dispatch through. Asserting it here, once, means every test in
    // this file states the premise it depends on: without it the dispatch is a silent
    // no-op, and `rina_copied_live_success_ability_does_nothing_without_energy_deficit`
    // in particular would pass for the wrong reason — a closed window and a genuine
    // energy-deficit failure look identical from the outside.
    assert!(
        game.state.should_trigger_live_success(&game.state.player1),
        "precondition: the ライブ成功時 window must be OPEN — if it is not, the \
         dispatch below does nothing at all"
    );
    (rina, deck_energy)
}

#[test]
fn rina_copied_live_success_ability_places_energy_from_energy_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (rina, energy) = setup_rina_live_success(&mut game, 0, 1);

    assert!(game.state.gained_card_abilities.contains_key(&rina));
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    assert!(game.state.player1.energy_zone.cards.contains(&energy));
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
    assert!(!game.state.player1.energy_deck.cards.contains(&energy));
}

#[test]
fn rina_copied_live_success_ability_does_nothing_without_energy_deficit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (rina, energy) = setup_rina_live_success(&mut game, 1, 0);

    assert!(game.state.gained_card_abilities.contains_key(&rina));
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    assert!(game.state.player1.energy_deck.cards.contains(&energy));
    assert_eq!(game.state.player1.energy_zone.active_count(), 1);
}
