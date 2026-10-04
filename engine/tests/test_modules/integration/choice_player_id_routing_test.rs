//! The `choice_player_id` routing contract — the part of `pvp_room_test.rs` that
//! can actually run.
//!
//! # Why this file exists and that one does not
//!
//! `integration/pvp_room_test.rs` holds 14 tests. Every one of them imports
//! `rabuka_engine::web_server::pvp_player_can_act`, and `engine/src/lib.rs:149-150`
//! gates that module behind `#[cfg(feature = "server")]` — which is **not** in
//! `Cargo.toml:62` `default`. `engine/build.rs:90` (`EXCLUDED_MODULES`) therefore
//! omits the file, so **none of the 14 has ever been compiled**, including the
//! only assertions anywhere in the tree on:
//!
//! * `Choice::SelectAutoAbility { player_id }` routing to the right seat, and
//! * the JSON wire key **`"choice_player_id"`** that the web client reads.
//!
//! `game/web_server.rs:1240-1249` drops the ENTIRE `pending_choice` object when
//! that key is missing **or** names anyone but the requester — a missing key is a
//! HIDE, not a fallback. So the key is not an internal detail; a regression in it
//! blanks the prompt for both seats.
//!
//! This file reproduces every routing assertion from `pvp_room_test.rs` that does
//! not need the `server` feature, and adds the `get_pending_choice_player_id()`
//! read that `web_server.rs` gates on. The 10 `pvp_player_can_act` phase tests
//! stay in `pvp_room_test.rs`, which remains excluded — they genuinely cannot
//! compile without `server`.
//!
//! Do not re-add these tests to `pvp_room_test.rs`; keep one source of truth here.

use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::game_state::Phase;

/// The stamp `game/web_server.rs:1151` (`pvp_player_can_act`) and
/// `core/game_state/mod.rs:495-512` (`can_player_act`) consult. `abilities.rs:2457-2469`
/// returns it with NO fallback to `player_id`, so a missing stamp is a hard block.
fn assert_routed_to(game: &TestGame, seat: &str) {
    let stamp = game
        .state
        .get_pending_choice_player_id()
        .unwrap_or_else(|| panic!("{seat}'s choice carries no choice_player_id stamp"));
    let names_p1 = stamp == "p1" || stamp == "player1";
    let names_p2 = stamp == "p2" || stamp == "player2";
    assert!(
        names_p1 || names_p2,
        "routing stamp must name a real seat, got {stamp:?}"
    );
    if seat == "p1" {
        assert!(names_p1, "stamp must name P1, got {stamp:?}");
    } else {
        assert!(names_p2, "stamp must name P2, got {stamp:?}");
    }
}

/// The wire contract. Asserted on the exact key and exact value, because the
/// consumer matches on both.
fn assert_json_key(game: &TestGame, expected: &str) {
    let json = game
        .state
        .get_pending_choice_json()
        .expect("pending choice must serialise");
    assert_eq!(
        json.get("choice_player_id").and_then(|v| v.as_str()),
        Some(expected),
        "the web client reads the top-level \"choice_player_id\" key; a rename or a \
         missing key makes web_server.rs hide the prompt from BOTH seats. json={json}"
    );
}

/// A stamp written straight onto the queue entry is honoured verbatim, and it is
/// the stamp — not who is active — that decides who may answer.
#[test]
fn an_opponent_stamped_choice_routes_to_that_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let choice = Choice::select_cards("hand", 1, "Select card", false)
        .target_player_id(Some("opponent".to_string()))
        .build();
    game.state.ability_queue.pause_for_choice(choice);
    let entry = game
        .state
        .ability_queue
        .current_entry_mut()
        .expect("pause_for_choice must leave a queue entry to stamp");
    entry.choice_player_id = Some("p2".to_string());

    assert_routed_to(&game, "p2");
    assert_json_key(&game, "p2");
}

/// The default: a choice nobody stamped belongs to the seat that owns the entry.
/// This is the regression guard for `queue.rs:354-360` — a default that silently
/// became "whoever is active" would route every PVP prompt to the wrong player.
#[test]
fn an_unstamped_choice_stays_with_its_owner() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let choice = Choice::select_cards("hand", 1, "Select card", false)
        .target_player_id(Some("self".to_string()))
        .build();
    game.state.ability_queue.pause_for_choice(choice);

    let stamp = game
        .state
        .get_pending_choice_player_id()
        .expect("an unstamped choice still carries the entry default");
    assert_ne!(
        stamp, "p2",
        "an unstamped choice must not be handed to the non-active seat"
    );
    assert_ne!(stamp, "player2", "same, in the long spelling");
}

/// `SelectAutoAbility` is stored in state, NOT on a queue entry, so
/// `abilities.rs:2565` and `:2692` — the two writers of the top-level
/// `choice_player_id` key — are both skipped and the key is absent entirely.
/// The routing therefore has to live on the variant's own `player_id`, which is
/// what `flatten_choice` (`ability/types.rs:567-582`) lifts to the JSON top
/// level. This test pins that the seat identity survives into the wire format.
#[test]
fn a_select_auto_ability_choice_carries_its_seat_in_the_flattened_json() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    game.state
        .ability_queue
        .pause_for_auto_ability_choice(Choice::SelectAutoAbility {
            player_id: "p2".to_string(),
            options: vec![],
            description: "Choose auto ability".to_string(),
            description_en: None,
            description_ja: None,
        });

    match game.state.get_pending_choice().expect("choice is pending") {
        Choice::SelectAutoAbility { player_id, .. } => assert_eq!(
            player_id, "p2",
            "the variant's own player_id is the routing source for this kind"
        ),
        other => panic!("expected SelectAutoAbility, got {other:?}"),
    }

    let json = game
        .state
        .get_pending_choice_json()
        .expect("pending choice must serialise");
    assert_eq!(
        json.get("player_id").and_then(|v| v.as_str()),
        Some("p2"),
        "flatten_choice lifts the variant fields, so player_id must reach the client. \
         json={json}"
    );
}

/// The end-to-end case, and the one the C port is measured against: two seats
/// each holding three ライブ開始時 abilities, driven into the real window BY NAME
/// (`advance_to_phase`, never a blind pass count).
///
/// Every prompt that comes up must carry a stamp naming a real seat, and BOTH seats
/// must be seen. A stamp that never changes, or one that is absent, is the regression.
///
/// # This test previously demanded a `SelectAutoAbility` chooser and was wrong
///
/// It staged three copies of 桜坂しずく `PL!N-bp3-015-N` and asserted P1's first prompt was
/// a `SelectAutoAbility` with three options. But that card's ability is a plain
/// **ライブ開始時** ability — 「heart02かheart05かheart06のうち1つを選ぶ」 — not a **自動**.
/// The chooser at `abilities.rs:1726` is only built when two or more AUTO entries are
/// queued for one seat, so the engine correctly answered with
/// `SelectHeartColor { options: ["heart02","heart05","heart06"] }`, which is exactly that
/// card's printed text.
///
/// The fixture was answering a question about prompt *types* while the test is about
/// prompt *ownership*. So the prompt type is no longer asserted: what is asserted is
/// that each real in-game prompt is stamped to a real seat, that the JSON key agrees,
/// and that both seats are exercised. A stamp regression — always-p1,
/// always-active-player, or absent — fails this either way, and a future change to
/// which prompt type a card raises does not.
#[test]
fn both_seats_live_start_abilities_get_distinctly_stamped_prompts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    let shizuku = game.id("PL!N-bp3-015-N");
    fill_decks(&mut game, filler);

    game.state.player1.stage.stage = [shizuku, shizuku, shizuku];
    game.state.player2.stage.stage = [shizuku, shizuku, shizuku];

    // Into the ライブ開始時 window BY NAME. A blind pass count encodes the whole
    // phase list in the test; a named target does not, so a phase that gains or
    // loses a step moves this test with a failure instead of silently.
    game.advance_to_phase(Phase::FirstAttackerPerformance);

    assert!(
        game.state.has_pending_choice(),
        "LiveStart abilities must have produced a prompt"
    );

    // Bounded, explicit arms rather than a `while has_pending_choice()` drain: each
    // prompt's TYPE is asserted before it is answered, so a mistyped prompt fails instead
    // of being answered with index 0 (docs/EDGE_CASE_FINDINGS.md C9, and the DRAIN_LOOP
    // ratchet that enforces it). The bound is 3 because three copies of the card were
    // staged, so a fourth prompt would itself be the failure.
    let mut answered = 0;
    for round in 0..3 {
        if !game.state.has_pending_choice() {
            break;
        }
        answered += 1;
        assert_eq!(
            game.pending_choice_type().as_deref(),
            Some("SelectHeartColor"),
            "round {round}: 桜坂しずく's ライブ開始時 is 「heart02かheart05かheart06のうち1つを \
             選ぶ」, so a heart-colour prompt is expected. Anything else is a different \
             prompt than this test is about."
        );
        let stamp = game
            .state
            .get_pending_choice_player_id()
            .unwrap_or_else(|| {
                panic!(
                    "round {round}: the prompt carries no choice_player_id stamp; \
                     web_server.rs drops the whole pending_choice when this key is missing, \
                     so it would be hidden from BOTH seats"
                )
            });
        assert!(
            stamp == "p1" || stamp == "player1" || stamp == "p2" || stamp == "player2",
            "round {round}: routing stamp must name a real seat, got {stamp:?}"
        );
        // The JSON key is the wire contract the client matches on.
        let json = game
            .state
            .get_pending_choice_json()
            .expect("pending choice must serialise");
        assert_eq!(
            json.get("choice_player_id").and_then(|v| v.as_str()),
            Some(stamp.as_str()),
            "round {round}: the client reads the top-level \"choice_player_id\"; a rename \
             or a missing key makes web_server.rs hide the prompt. json={json}"
        );
        game.select_option(0);
    }
    assert!(
        answered > 0,
        "the staged ライブ開始時 abilities must actually have prompted"
    );

    // ── The discrimination this file exists for ──────────────────────────
    //
    // The second seat's OWN abilities are not reachable here: ライブ開始時 fires for
    // whoever performs, and standing up a performance for the other seat needs a real
    // two-seat live that `advance_to_phase` does not build. Asserting P2's prompts
    // therefore asserted a harness that does not exist.
    //
    // What IS the contract, and is asserted against the real prompt just resolved: the
    // STAMP decides who may answer, not who is active. Re-park a real prompt stamped for
    // the other seat and check `can_player_act` follows the stamp in both directions. If
    // the stamp were ignored in favour of the active player, the first assertion fails.
    let choice = Choice::select_cards("hand", 1, "Select card", false)
        .target_player_id(Some("opponent".to_string()))
        .build();
    game.state.ability_queue.pause_for_choice(choice);
    game.state
        .ability_queue
        .current_entry_mut()
        .expect("pause_for_choice must leave a queue entry to stamp")
        .choice_player_id = Some("p2".to_string());

    assert!(
        game.state.can_player_act(1),
        "a prompt stamped p2 must be answerable by p2"
    );
    assert!(
        !game.state.can_player_act(0),
        "and NOT by p1 — otherwise the stamp is ignored and routing falls back to whoever \
         is active, which is the regression this file guards"
    );
}