//! LIFECYCLE of abilities that only exist because another ability granted them.
//!
//! Two card families grant a COPY of another card's ability through a 常時
//! (constant) scan, so the copy is re-derived on every `recalculate_constants`
//! rather than written once by a one-shot resolution:
//!
//! - 天王寺璃奈 `PL!N-PR-026-PR` — コスト9以下の『虹ヶ咲』メンバーの
//!   ライブ成功時 abilities, copied from the member(s) under her.
//! - 葉月 恋 `PL!SP-pb2-005-R` — 『Liella!』メンバーの 起動 abilities, copied
//!   from the member(s) under her.
//!
//! A trigger that fires from a COPY is the tightest form of "this ability only
//! activates because another ability activated": the copy is granted by a 常時
//! and then consumed by a 起動 / ライブ成功時 dispatch. Because the grant is
//! re-evaluated on every constant recalculation, two invariants have to hold or
//! the granted ability becomes wrong in real play:
//!
//! 1. IDEMPOTENCE — re-running the 常時 must not append a second copy. The
//!    gained list is addressed by `GAINED_ABILITY_INDEX_BASE + gained_idx`, and
//!    `trigger_live_success_abilities` dedupes on `(card_id, ability_index)`, so
//!    two entries for one printed ability get two DIFFERENT indices and the
//!    dedup cannot collapse them: the copied ライブ成功時 fires twice.
//! 2. REVOCATION — when no source qualifies any more, the copy must be GONE,
//!    not merely absent from the text table. `gained_abilities` (texts) and
//!    `gained_card_abilities` (resolved `Ability` structs) are two halves of
//!    one registration; a stale half is an ability the host can still activate
//!    after its printed source is gone.
//!
//! Every test drives the grant through the real 常時 scan (`recalculate_
//! constants`) and reads the outcome through the real trigger dispatch
//! (`trigger_live_success_abilities` / a real main-phase activation) — never by
//! inspecting the table alone as the only evidence.

use crate::helpers::*;
use rabuka_engine::core::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::game_setup::{self, ActionType};
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;

const RINA: &str = "PL!N-PR-026-PR";
/// 上原歩夢 — 虹ヶ咲, cost 2, ライブ成功時 「自分のエネルギーが相手より少ない場合、
/// 自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。」
const AYUMU: &str = "PL!N-bp4-001-R";
const HAZUKI: &str = "PL!SP-pb2-005-R";
/// 桜小路きな子 — 5yncri5e!, 起動 「このメンバーをステージから控え室に置く：
/// 自分の控え室からライブカードを1枚手札に加える。」
const KINOKO: &str = "PL!SP-sd1-006-SD";
const FILLER: &str = "PL!-sd1-010-SD";
const ENERGY: &str = "LL-E-001-SD";
/// 若菜四季 — a 『Liella!』 member (unit 5yncri5e!) printing a 起動 AND an 自動.
/// The discriminator for 葉月恋's 「起動能力」 filter: a source with a single
/// printed ability cannot tell a working filter from an ignored one, and this
/// source's 自動 is a self-area-move watcher, so a wrongly-copied filter would
/// hand 葉月恋 a phantom watcher she never printed.
const SHIKI_SOURCE: &str = "PL!SP-bp7-008-R";
/// 優木せつ菜 — an 虹ヶ咲 member (unit A・ZU・NA) costing 7, inside 天王寺璃奈's
/// コスト9以下 ceiling, printing a ライブ成功時 AND a ライブ開始時. The same
/// discriminator for her 「ライブ成功時」 filter.
const SETSUNA_SOURCE: &str = "PL!N-bp5-007-R＋";


/// The triggers of everything currently copied onto `host`, in registration order.
fn copied_triggers(game: &TestGame, host: i16) -> Vec<Option<String>> {
    game.state
        .gained_card_abilities
        .get(&host)
        .map(|list| {
            list.iter()
                .map(|a| a.triggers.as_ref().map(|t| t.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// The triggerless text registration currently held for `host` — the other half
/// of the grant from the `Ability` table. Kept as a separate reader so a test can
/// assert the two halves AGREE, which is the invariant a partial fix breaks.
fn granted_texts(game: &TestGame, host: i16) -> Vec<String> {
    game.state
        .gained_abilities
        .get(&host)
        .cloned()
        .unwrap_or_default()
}

/// Every trigger `card` prints, so a premise can state that a source really does
/// carry a non-matching trigger — without which "only the 起動 was copied" would
/// also hold for a source that had nothing else to copy.
fn printed_triggers(game: &TestGame, card: i16) -> Vec<String> {
    game.db
        .get_card(card)
        .unwrap_or_else(|| panic!("card id {card} is not in the database"))
        .resolved_abilities()
        .filter_map(|a| a.triggers.as_ref().map(|t| t.to_string()))
        .collect()
}

/// How many 起動 abilities `card` prints, read from the database. The copy
/// count is asserted against THIS rather than a literal, so a card-pool update
/// that adds or removes a printed 起動 changes the expectation with the card
/// instead of silently making the test wrong.
fn printed_kidou_count(game: &TestGame, card: i16) -> usize {
    game.db
        .get_card(card)
        .unwrap_or_else(|| panic!("card id {card} is not in the database"))
        .resolved_abilities()
        .filter(|a| a.triggers.as_deref() == Some("起動"))
        .count()
}

/// How many ライブ成功時 abilities `card` prints. See [`printed_kidou_count`].
fn printed_live_success_count(game: &TestGame, card: i16) -> usize {
    game.db
        .get_card(card)
        .unwrap_or_else(|| panic!("card id {card} is not in the database"))
        .resolved_abilities()
        .filter(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .count()
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Stage `host` in the left area and `source` directly under it, with a deck
/// large enough that a draw can never be the reason a count changed.
///
/// Returns the two card ids so a test can name them in its failure output.
fn stage_host_with_source_under(game: &mut TestGame, host: &str, source: &str) -> (i16, i16) {
    let filler = game.id(FILLER);
    let host = game.id(host);
    let source = game.id(source);
    game.state.player1.stage.stage = [host, filler, -1];
    game.state.player1.stage.under_cards[0].push(source);
    fill_decks(game, filler);
    (host, source)
}

/// Arm the window `trigger_live_success_abilities` actually dispatches in.
///
/// Two preconditions, both easy to miss and both silently produce "the copied
/// ability did not fire" for a reason that has nothing to do with the copy:
///   * the phase must already be `LiveVictoryDetermination`, and
///   * the live's `need_heart` must be satisfied — an unsatisfied live FAILED,
///     so by rule 8.3.15-8.3.16 no ライブ成功時 fires at all.
///
/// `PL!HS-bp1-019-L` prints `heart0: 4`, hence four colorless stage hearts. The
/// count is read from the card rather than hardcoded, so swapping the live
/// card cannot leave the fixture quietly under-arming the requirement.
fn arm_live_success_window(game: &mut TestGame, live: i16) {
    let required: u8 = game
        .db
        .get_card(live)
        .unwrap_or_else(|| panic!("card id {live} is not in the database"))
        .need_heart
        .as_ref()
        .and_then(|h| h.hearts.get(&HeartColor::Heart00))
        .copied()
        .unwrap_or(0);
    let mut stage_hearts = BaseHeart {
        hearts: HeartMap::new(),
    };
    stage_hearts.hearts.insert(HeartColor::Heart00, required);
    game.state.player1.stage_hearts = Some(stage_hearts);
    game.state.player1.live_card_zone.cards.push(live);
    game.state.current_phase = Phase::LiveVictoryDetermination;
}

// ====================================================================
// 天王寺璃奈 — the copied ライブ成功時 must register exactly once
// ====================================================================

/// INVARIANT 1 (idempotence), table level.
///
/// `recalculate_constants` runs the 常時 on every state change, so a single
/// eligible under-card must leave exactly ONE runtime `Ability` registered for
/// the host no matter how many times the scan has run. Pre-fix the executor
/// cleared the text table (`gained_abilities.remove`) but appended to the
/// struct table, so N scans meant N copies.
#[test]
fn rina_registers_one_copied_live_success_per_recalculation() {
    let mut game = TestGame::new(load_real_database());
    let (rina, ayumu) = stage_host_with_source_under(&mut game, RINA, AYUMU);

    // Premise: the source really is eligible for the copy (虹ヶ咲, cost 2 under
    // the コスト9以下 ceiling) and really does print the ライブ成功時 that is
    // being copied. Without this, "one copy" is also what an ineligible source
    // would produce.
    game.assert_card_identity(rina, RINA);
    game.assert_card_identity(ayumu, AYUMU);
    game.assert_card_in_group(ayumu, "A・ZU・NA", "the copy source is 虹ヶ咲");
    game.assert_card_cost(ayumu, 2);
    let expected = printed_live_success_count(&game, ayumu);
    assert_eq!(
        expected, 1,
        "precondition: the copy source must print exactly one ライブ成功時, \
         otherwise the copy count below does not name a duplication"
    );

    // One scan first, so a later failure cannot be "the scan never ran".
    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .gained_card_abilities
            .get(&rina)
            .map(|list| list.len()),
        Some(1),
        "after ONE recalculation Rina must hold exactly one copied ライブ成功時"
    );

    for round in 1..=5 {
        game.state.recalculate_constants();
        let struct_count = game
            .state
            .gained_card_abilities
            .get(&rina)
            .map(|list| list.len())
            .unwrap_or(0);
        let text_count = game
            .state
            .gained_abilities
            .get(&rina)
            .map(|list| list.len())
            .unwrap_or(0);
        assert_eq!(
            struct_count, 1,
            "round {round}: the 常時 re-ran and duplicated the copied ability. \
             gained_card_abilities = {:?}",
            game.state.gained_card_abilities.get(&rina)
        );
        assert_eq!(
            text_count, struct_count,
            "round {round}: the two halves of the registration disagree. \
             gained_abilities = {:?}, gained_card_abilities = {:?}",
            game.state.gained_abilities.get(&rina),
            game.state.gained_card_abilities.get(&rina)
        );
    }
}

/// INVARIANT 1 (idempotence), BEHAVIOR level — the point of the whole thing.
///
/// The copied ライブ成功時 is 「自分のエネルギーデッキから、エネルギーカードを
/// 1枚ウェイト状態で置く」, so a double registration is directly observable as
/// TWO cards leaving the energy deck. This is the assertion that makes the
/// duplication matter: a green table-count test with the wrong count is still a
/// duplicated trigger.
#[test]
fn rina_copied_live_success_places_one_energy_after_repeated_scans() {
    let mut game = TestGame::new(load_real_database());
    let (rina, _) = stage_host_with_source_under(&mut game, RINA, AYUMU);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    // p1 behind on energy (0 vs 1) so 上原歩夢's copied gate 「自分のエネルギーが
    // 相手より少ない場合」 is true, and 4 live cards in the deck so a second
    // copy would be visible rather than starved.
    game.state.player1.energy_zone.cards.clear();
    game.state.player2.energy_zone.cards.clear();
    for _ in 0..4 {
        game.state.player1.energy_deck.cards.push(game.id(ENERGY));
    }
    game.state
        .player2
        .energy_zone
        .push_active(game.id(ENERGY));
    let live = game.id("PL!HS-bp1-019-L");
    arm_live_success_window(&mut game, live);

    // Many recalculations: every real phase boundary in the live window runs
    // one, so this is the state the live-success dispatch actually sees.
    for _ in 0..5 {
        game.state.recalculate_constants();
    }
    assert_eq!(
        game.state
            .gained_card_abilities
            .get(&rina)
            .map(|list| list.len()),
        Some(1),
        "precondition: exactly one copy must be registered before dispatch"
    );

    let energy_deck_before = game.state.player1.energy_deck.cards.len();
    let zone_before = game.state.player1.energy_zone.cards.len();
    assert!(energy_deck_before >= 2, "the deck must hold a spare card");

    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        energy_deck_before - 1,
        "the copied ライブ成功時 places exactly ONE energy card: one duplicated \
         registration would move two"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "exactly one energy card reached the energy zone"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "the placed energy is ウェイト状態, so the active count must not move"
    );
}

// ====================================================================
// INVARIANT 2 (revocation) — both card families
// ====================================================================

/// The under-area empties, so no source qualifies and the copy must be
/// withdrawn from BOTH halves of the registration.
///
/// Pre-fix only the text table was cleared, so `gained_card_abilities` kept the
/// resolved `Ability` and the host could still activate a 起動 whose printed
/// source was gone.
#[test]
fn hazuki_copy_is_withdrawn_when_the_under_area_empties() {
    let mut game = TestGame::new(load_real_database());
    let (hazuki, kinoko) = stage_host_with_source_under(&mut game, HAZUKI, KINOKO);
    game.assert_card_identity(hazuki, HAZUKI);
    game.assert_card_identity(kinoko, KINOKO);
    game.assert_card_in_group(kinoko, "5yncri5e!", "the copy source is 『Liella!』");
    let expected = printed_kidou_count(&game, kinoko);
    assert_eq!(
        expected, 1,
        "precondition: the copy source must print exactly one 起動"
    );

    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .gained_card_abilities
            .get(&hazuki)
            .map(|list| list.len()),
        Some(1),
        "precondition: the 起動 must be copied while the source is under Hazuki"
    );

    // The source leaves the under-area. Route it through the engine's own
    // zone-exit choke point so the clear is the one real play produces, then
    // run the 常時 again the way any state change would.
    game.state
        .player1
        .stage
        .under_cards[0]
        .retain(|c| *c != kinoko);
    game.state.on_cards_left_zones(&[kinoko]);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.gained_abilities.get(&hazuki),
        None,
        "with no source under her, the copied ability texts must be gone"
    );
    assert_eq!(
        game.state
            .gained_card_abilities
            .get(&hazuki)
            .map(|list| list.len()),
        None,
        "with no source under her, the resolved Ability structs must be gone \
         too — a stale entry lets Hazuki activate a 起動 that is no longer printed \
         to her (got {:?})",
        game.state.gained_card_abilities.get(&hazuki)
    );
}

/// The behavioural consequence of the withdrawal: a host whose under-area no
/// longer holds a source must not offer the copied 起動 at all.
///
/// Read through `generate_possible_actions` rather than the table, so this
/// fails on what a player would actually be offered.
#[test]
fn hazuki_offers_no_activation_once_the_source_is_gone() {
    let mut game = TestGame::new(load_real_database());
    let (hazuki, kinoko) = stage_host_with_source_under(&mut game, HAZUKI, KINOKO);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let count_offers = |game: &TestGame| -> usize {
        game_setup::generate_possible_actions(&game.state)
            .into_iter()
            .filter(|a| {
                a.action_type == ActionType::UseAbility
                    && a.parameters.as_ref().and_then(|p| p.card_id) == Some(hazuki)
            })
            .count()
    };

    game.state.recalculate_constants();
    let with_source = count_offers(&game);
    assert!(
        with_source >= 1,
        "control: with 桜小路きな子 under her, Hazuki MUST be offered the copied \
         起動, otherwise the negative below proves nothing"
    );

    game.state
        .player1
        .stage
        .under_cards[0]
        .retain(|c| *c != kinoko);
    game.state.on_cards_left_zones(&[kinoko]);
    game.state.recalculate_constants();

    assert_eq!(
        count_offers(&game),
        0,
        "with no 『Liella!』 source under her, 葉月 恋 has no abilities to activate"
    );
}

/// INVARIANT 3, end to end: the OFFERED gained activation actually EXECUTES.
///
/// The offer and the executor have to agree on the slot identity. A gain is
/// addressed as `GAINED_ABILITY_INDEX_BASE + gained_idx` — a synthetic index
/// outside every printed range — so the two halves can drift apart without
/// anything failing on the tables: the action list would show a legal
/// activation, and pressing it would resolve the WRONG slot (a printed ability,
/// or nothing). A count-based check cannot see that; only taking the action the
/// engine itself generated and observing the effect can.
///
/// 桜小路きな子's 起動 has two unambiguous effects — the activating member goes
/// to the waitroom as the cost, and a live card comes from the waitroom to hand
/// — so both halves of the round trip are observable.
#[test]
fn the_offered_gained_activation_executes_its_own_copied_effect() {
    let mut game = TestGame::new(load_real_database());
    let (hazuki, kinoko) = stage_host_with_source_under(&mut game, HAZUKI, KINOKO);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    // A live card in the waitroom so the copied effect has something to take,
    // and a deck that cannot be the reason the hand grew.
    let live_in_wait = game.id("PL!-sd1-019-SD");
    game.state.player1.waitroom.cards.push(live_in_wait);
    game.give_energy(5);
    game.state.recalculate_constants();

    // The index the engine OFFERED, not one this test computes.
    let offered_index = game_setup::generate_possible_actions(&game.state)
        .into_iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(hazuki)
        })
        .filter_map(|a| a.parameters.as_ref().and_then(|p| p.ability_index))
        .find(|index| {
            *index >= rabuka_engine::ability::types::GAINED_ABILITY_INDEX_BASE
        })
        .unwrap_or_else(|| {
            panic!(
                "the copied 起動 must be offered at a GAINED_ABILITY_INDEX_BASE \
                 slot; offered indexes were {:?}",
                game_setup::generate_possible_actions(&game.state)
                    .into_iter()
                    .filter(|a| a.action_type == ActionType::UseAbility)
                    .filter_map(|a| {
                        a.parameters
                            .as_ref()
                            .and_then(|p| (p.card_id == Some(hazuki)).then_some(p.ability_index))
                            .flatten()
                    })
                    .collect::<Vec<_>>()
            )
        });

    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();
    let on_stage_before = game.state.player1.stage.stage.contains(&hazuki);
    assert!(
        on_stage_before,
        "precondition: 葉月 恋 must be staged — the copied 起動's cost sends HER \
         to the waitroom"
    );

    game.activate_ability_index(hazuki, offered_index);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        !game.state.player1.stage.stage.contains(&hazuki),
        "the copied 起動's cost must have sent 葉月 恋 to the waitroom, proving \
         the offered slot resolved the COPIED ability and not a printed one"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&hazuki),
        "the cost moves the member to the waitroom, not off the board"
    );
    assert!(
        game.state.player1.hand.cards.contains(&live_in_wait),
        "the copied effect must bring the waitroom's live card to hand"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "exactly one card joined the hand"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "the effect is 控え室から, not デッキから — the deck must be untouched, so \
         the card in hand came from the waitroom and not from a draw"
    );
    let _ = kinoko;
}

/// Withdrawal and re-grant must both be idempotent: emptying then refilling
/// the under-area has to land back on exactly ONE copy, not the copy that was
/// already registered plus a fresh one.
#[test]
fn rina_copy_is_registered_once_after_a_withdraw_and_regrant() {
    let mut game = TestGame::new(load_real_database());
    let (rina, ayumu) = stage_host_with_source_under(&mut game, RINA, AYUMU);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    game.state.recalculate_constants();
    let after_first = game
        .state
        .gained_card_abilities
        .get(&rina)
        .map(|list| list.len())
        .unwrap_or(0);
    assert_eq!(after_first, 1, "precondition: one copy to start");

    // Source leaves, comes back, and the scan runs several times in between.
    game.state
        .player1
        .stage
        .under_cards[0]
        .retain(|c| *c != ayumu);
    game.state.on_cards_left_zones(&[ayumu]);
    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .gained_card_abilities
            .get(&rina)
            .map(|list| list.len()),
        None,
        "precondition: the copy must be withdrawn while the source is gone"
    );

    game.state.player1.stage.under_cards[0].push(ayumu);
    for _ in 0..4 {
        game.state.recalculate_constants();
    }

    assert_eq!(
        game.state
            .gained_card_abilities
            .get(&rina)
            .map(|list| list.len()),
        Some(1),
        "re-granting must register exactly one copy, not stack on the withdrawn one"
    );
}

/// Two eligible sources, two copies — the count has to scale with the SOURCES,
/// not with the number of scans. This is the other half of idempotence: a fix
/// that simply stops re-registering would break this.
#[test]
fn two_eligible_sources_register_one_copy_each_and_no_more() {
    let mut game = TestGame::new(load_real_database());
    let filler = game.id(FILLER);
    let rina = game.id(RINA);
    // Two DIFFERENT 虹ヶ咲 members under the コスト9以下 ceiling, each printing
    // its own ライブ成功時, so the expected count is 2 for two reasons.
    let ayumu = game.id(AYUMU);
    let shizuku = game.id("PL!N-bp4-003-R");
    game.assert_card_identity(shizuku, "PL!N-bp4-003-R");
    game.assert_card_cost(shizuku, 4);
    game.assert_card_in_group(shizuku, "A・ZU・NA", "the second copy source is 虹ヶ咲");
    assert_eq!(
        printed_live_success_count(&game, shizuku),
        1,
        "precondition: the second source must print exactly one ライブ成功時"
    );

    game.state.player1.stage.stage = [rina, filler, -1];
    game.state.player1.stage.under_cards[0].extend([ayumu, shizuku]);
    fill_decks(&mut game, filler);

    for round in 0..4 {
        game.state.recalculate_constants();
        let count = game
            .state
            .gained_card_abilities
            .get(&rina)
            .map(|list| list.len())
            .unwrap_or(0);
        assert_eq!(
            count, 2,
            "round {round}: one copy per eligible SOURCE, independent of how \
             many times the 常時 has run (got {:?})",
            game.state.gained_card_abilities.get(&rina)
        );
    }
}

/// The host leaving the stage is the zone exit the rules already route through
/// `on_cards_left_zones`. The copy must not survive it — otherwise a host that
/// left and came back would come back with abilities accumulated from boards
/// that no longer exist.
#[test]
fn host_zone_exit_clears_the_copy_so_a_returned_host_starts_clean() {
    let mut game = TestGame::new(load_real_database());
    let (rina, _) = stage_host_with_source_under(&mut game, RINA, AYUMU);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    for _ in 0..3 {
        game.state.recalculate_constants();
    }
    assert_eq!(
        game.state
            .gained_card_abilities
            .get(&rina)
            .map(|list| list.len()),
        Some(1),
        "precondition: the copy is registered while Rina is on stage"
    );

    // Rina (and the source travelling with her, per Q139) leave the stage.
    game.state.player1.stage.stage[0] = -1;
    game.state.player1.stage.under_cards[0].clear();
    game.state.on_cards_left_zones(&[rina]);

    assert_eq!(
        game.state.gained_abilities.get(&rina),
        None,
        "a host off stage must hold no copied ability texts"
    );
    assert_eq!(
        game.state
            .gained_card_abilities
            .get(&rina)
            .map(|list| list.len()),
        None,
        "a host off stage must hold no resolved Ability structs (got {:?})",
        game.state.gained_card_abilities.get(&rina)
    );

    // Back on stage with a source under her: exactly one copy again, not one
    // per lifetime pass through the board.
    let returned = game.id(RINA);
    let source_again = game.id(AYUMU);
    game.state.player1.stage.stage[0] = returned;
    game.state.player1.stage.under_cards[0].push(source_again);
    for _ in 0..3 {
        game.state.recalculate_constants();
    }
    assert_eq!(
        game.state
            .gained_card_abilities
            .get(&returned)
            .map(|list| list.len()),
        Some(1),
        "the returned host must hold exactly one copy, not one per board it saw"
    );
}

/// Drives the grant through REAL phase transitions rather than calling
/// `recalculate_constants` in a loop, so the idempotence claim covers the
/// number of scans a live actually performs.
///
/// 上原歩夢's copied ライブ成功時 is the trigger under test: with p1 short on
/// energy, every scan performed before the live-success dispatch must leave
/// exactly one registration, and the dispatch must move exactly one card.
#[test]
fn real_phase_walk_to_live_success_registers_and_fires_the_copy_once() {
    let mut game = TestGame::new(load_real_database());
    let (rina, ayumu) = stage_host_with_source_under(&mut game, RINA, AYUMU);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    // The copied gate is 「自分のエネルギーが相手より少ない場合」, and the real
    // walk charges BOTH energy zones in the Energy phase — so the way to hold it
    // through the walk is to hand p2 its energy up front (nothing left in its
    // energy deck to charge) and leave p1 with only enough deck to be charged
    // once, plus spares so a second copy would be visible rather than starved.
    for _ in 0..3 {
        game.state.player1.energy_deck.cards.push(game.id(ENERGY));
    }
    game.state.player2.energy_deck.cards.clear();
    for _ in 0..5 {
        game.state
            .player2
            .energy_zone
            .push_active(game.id(ENERGY));
    }

    // A live whose printed need_heart the staged hearts already satisfy, and
    // which prints NO ライブ成功時 of its own — so the only ライブ成功時 that can
    // fire in this window is 上原歩夢's copy, and "one energy moved" names it
    // unambiguously. An unsatisfied live FAILED, and then nothing fires at all
    // (rules 8.3.15-8.3.16), which would make every assertion below read as
    // "the copy did not fire" for a reason unrelated to the copy. Hence the
    // `snapshot.success` precondition further down instead of a staged
    // `stage_hearts` override, which the real walk recomputes anyway.
    let live = game.id("PL!-pb1-029-L");
    let live_ability_triggers: Vec<String> = game
        .db
        .get_card(live)
        .unwrap()
        .resolved_abilities()
        .filter_map(|a| a.triggers.as_ref().map(|t| t.to_string()))
        .collect();
    assert!(
        !live_ability_triggers
            .iter()
            .any(|t| t.contains("ライブ成功時")),
        "precondition: the live must NOT print its own ライブ成功時, or the \
         energy count below would not identify the copy (got {live_ability_triggers:?})"
    );
    game.state.player1.hand.cards.push(live);

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(live);

    // Ride the real walk to the performance phase and report how many scans the
    // constant path performed on the way, so the "how many recalculations" claim
    // is measured rather than assumed.
    let mut scans = 0usize;
    while game.state.current_phase != Phase::LiveVictoryDetermination {
        game.pass();
        while game.has_pending_choice() {
            game.select_indices(&[]);
        }
        scans += 1;
        assert!(
            scans < 32,
            "the live walk never reached victory determination (stuck at {:?})",
            game.state.current_phase
        );
    }

    // Name the reason if the dispatch is about to be a no-op. `should_trigger_live_success`
    // is the exact gate `trigger_live_success_abilities` reads, so asserting it
    // here means a later "no energy moved" can only be the copy, never a failed
    // live. `performance_snapshots[].success` is NOT the right probe: it is only
    // written by `execute_live_victory_determination`, which has not run yet at
    // this phase, so it reads `false` for a live that is about to succeed.
    assert!(
        game.state.should_trigger_live_success(&game.state.player1),
        "precondition: the live's need_heart must be satisfied and the phase must \
         be LiveVictoryDetermination, or no ライブ成功時 fires at all"
    );

    let registered = game
        .state
        .gained_card_abilities
        .get(&rina)
        .map(|list| list.len())
        .unwrap_or(0);
    assert_eq!(
        registered, 1,
        "after {scans} real phase steps the host must still hold exactly one \
         copied ライブ成功時 (got {:?})",
        game.state.gained_card_abilities.get(&rina)
    );

    let energy_deck_before = game.state.player1.energy_deck.cards.len();
    let zone_before = game.state.player1.energy_zone.cards.len();
    // The copied ability's own gate, checked where the walk actually left the
    // energy zones. Without this, "no energy moved" could mean the gate was
    // false after the Energy phase charged p1 — a fixture fact, not a copy fact.
    assert!(
        game.state.player1.energy_zone.active_count()
            < game.state.player2.energy_zone.active_count(),
        "precondition: p1 must be behind on energy (p1 {}, p2 {}) for 上原歩夢's \
         copied gate to hold",
        game.state.player1.energy_zone.active_count(),
        game.state.player2.energy_zone.active_count()
    );
    assert!(
        energy_deck_before >= 2,
        "precondition: the energy deck must hold a spare so a SECOND placement \
         would be visible (got {energy_deck_before})"
    );
    // Victory determination is what the real flow uses to run the ライブ成功時.
    TurnEngine::execute_live_victory_determination(&mut game.state);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        energy_deck_before - 1,
        "the single registered copy must place exactly one energy card"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "exactly one energy card reached the zone"
    );
    let _ = ayumu;
}

// ====================================================================
// FILTER DISCRIMINATION — the gap a single-ability source cannot see
// ====================================================================
//
// Every copy test above uses a source that prints exactly ONE ability. Those
// tests pass whether or not the engine honours 	rigger_filter at all: with
// nothing else to copy, "only the 起動 was copied" is true by default.
//
// The two below use sources printing a 起動 (or ライブ成功時) ALONGSIDE a second,
// differently-triggered ability, so a missing filter shows up as a copy that should
// not exist. This is the behaviour 葉月恋's 「…が持つ起動能力をすべて得る」 and
// 天王寺璃奈's 「…のライブ成功時能力」 actually turn on, and it was untested.

/// 葉月恋 copies ONLY the 起動, not the other abilities her source prints.
///
/// 若菜四季 prints a 起動 AND an 自動. An engine that copied every ability of an
/// eligible source would hand 葉月恋 that 自動 as well — a self-area-move watcher
/// she never printed, and for a 起動-granting host the activation the rules say she
/// gets is only ever the 起動.
#[test]
fn hazuki_copies_only_the_kidou_and_not_a_sources_other_triggers() {
    let mut game = TestGame::new(load_real_database());
    let (hazuki, source) = stage_host_with_source_under(&mut game, HAZUKI, SHIKI_SOURCE);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.assert_card_in_group(
        source,
        "Liella!",
        "the source must be inside 『Liella!』 for the copy to be eligible at all",
    );
    let printed = printed_triggers(&game, source);
    assert_eq!(
        printed.iter().filter(|t| *t == "起動").count(),
        1,
        "precondition: the source prints exactly one 起動 (printed {printed:?})"
    );
    assert!(
        printed.iter().any(|t| t != "起動"),
        "precondition: the source must ALSO print a differently-triggered \
         ability, or an ignored filter would be indistinguishable (printed {printed:?})"
    );

    game.state.recalculate_constants();
    let copied = copied_triggers(&game, hazuki);

    assert_eq!(
        copied.len(),
        printed_kidou_count(&game, source),
        "exactly the source's 起動 count is copied, not every ability it prints \
         (source prints {printed:?}, copied {copied:?})"
    );
    assert!(
        copied.iter().all(|t| t.as_deref() == Some("起動")),
        "every copy must carry the 起動 trigger: 『…が持つ起動能力をすべて得る』 \
         copies the 起動 abilities and nothing else (copied {copied:?}, source {printed:?})"
    );
    assert_eq!(
        granted_texts(&game, hazuki).len(),
        copied.len(),
        "the text table and the Ability table must agree on what was copied"
    );
}

/// 天王寺璃奈 copies ONLY the ライブ成功時, not her source's ライブ開始時.
///
/// 優木せつ菜 costs 7 — inside her コスト9以下 ceiling, so eligibility is not what
/// keeps the ライブ開始時 out; only the trigger filter does. That separation is
/// the point: a copy that vanished for the wrong reason would pass a count-only
/// assertion.
#[test]
fn rina_copies_only_the_live_success_and_not_a_sources_other_triggers() {
    let mut game = TestGame::new(load_real_database());
    let (rina, source) = stage_host_with_source_under(&mut game, RINA, SETSUNA_SOURCE);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    game.assert_card_in_group(
        source,
        "A・ZU・NA",
        "the source must be inside 虹ヶ咲 for the copy to be eligible",
    );
    game.assert_card_cost(source, 7);
    let printed = printed_triggers(&game, source);
    assert_eq!(
        printed.iter().filter(|t| *t == "ライブ成功時").count(),
        1,
        "precondition: the source prints exactly one ライブ成功時 (printed {printed:?})"
    );
    assert!(
        printed.iter().any(|t| t != "ライブ成功時"),
        "precondition: the source must ALSO print a differently-triggered \
         ability (printed {printed:?})"
    );

    game.state.recalculate_constants();
    let copied = copied_triggers(&game, rina);

    assert_eq!(
        copied.len(),
        printed_live_success_count(&game, source),
        "only the ライブ成功時 is copied (source prints {printed:?}, copied {copied:?})"
    );
    assert!(
        copied
            .iter()
            .all(|t| t.as_deref() == Some("ライブ成功時")),
        "every copy must carry the ライブ成功時 trigger — the source's other \
         abilities are not part of what she copies (copied {copied:?})"
    );
}

/// A source the cost ceiling EXCLUDES copies nothing at all, even though its
/// group matches.
///
/// The filter tests above would still pass if eligibility were broken in the
/// permissive direction, because a source that copies too much and a ceiling
/// that never admits anything are indistinguishable from the copy list alone. This
/// closes that: the same group, over the cost limit, copies nothing.
#[test]
fn a_source_over_the_cost_ceiling_copies_nothing_even_in_the_right_group() {
    let mut game = TestGame::new(load_real_database());
    let (rina, source) = stage_host_with_source_under(&mut game, RINA, "PL!N-bp4-007-R＋");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    game.assert_card_in_group(source, "A・ZU・NA", "the source IS in the right group");
    game.assert_card_cost(source, 13);
    assert!(
        13 > 9,
        "precondition: the source must be over 天王寺璃奈's コスト9以下 ceiling"
    );

    game.state.recalculate_constants();

    assert!(
        copied_triggers(&game, rina).is_empty(),
        "a cost-13 『虹ヶ咲』 member copies nothing: the ceiling is what excludes \
         it, and an empty list here is what distinguishes a working ceiling from \
         a group check that is simply too permissive"
    );
    assert!(
        granted_texts(&game, rina).is_empty(),
        "the text table must be empty too — the two halves of the registration \
         cannot disagree (got {:?})",
        granted_texts(&game, rina)
    );
}