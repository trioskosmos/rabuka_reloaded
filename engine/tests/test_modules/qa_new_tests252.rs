/// Tests for Q252 — 桜内梨子 PL!S-bp6-002-R+
///
/// Auto ability (ab#0, 1/turn):
///   『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、
///   そのライブカードをデッキの一番上か一番下に置いてもよい。
///
/// Q252: When multiple Aqours live cards go to waitroom simultaneously,
///       only 1 can be put on deck. Player chooses which.
use crate::helpers::*;
use rabuka_engine::game_state::AbilityTrigger;

const RIKO_AUTO_TEXT: &str = "{{jidou.png|自動}}{{turn1.png|ターン1回}}『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に置いてもよい。";
const AQOURS_LIVE: &str = "PL!S-pb1-023-L"; // Next SPARKLING!! — H02:6 H04:6 H05:6

fn trigger_riko_auto(game: &mut TestGame) {
    let ability_id = format!("PL!S-bp6-002-R\u{ff0b}_{}", RIKO_AUTO_TEXT);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Auto,
        "player1".to_string(),
        Some("PL!S-bp6-002-R\u{ff0b}".to_string()),
        None,
        None,
        None,
    );
    game.state.process_pending_auto_abilities("player1");
}

fn setup_riko_and_filler(game: &mut TestGame, cards_in_waitroom: &[i16]) -> i16 {
    let riko = game.id("PL!S-bp6-002-R\u{ff0b}");
    let filler = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = riko;
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for &cid in cards_in_waitroom {
        game.state.player1.waitroom.add_card(cid);
    }
    // Simulate engine tracking: these cards were just moved from live_card_zone
    game.state.recently_moved_cards = Some(cards_in_waitroom.to_vec());
    game.state.current_phase = rabuka_engine::game_state::Phase::Main;
    riko
}

/// Basic trigger: 1 Aqours live card in waitroom → select it → pick top.
#[test]
fn test_q252_basic_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id(AQOURS_LIVE);
    let _ = setup_riko_and_filler(&mut game, &[live]);
    trigger_riko_auto(&mut game);

    // Choice 1: select the card (or skip)
    assert!(game.has_pending_choice(), "Prompt: pick card or skip");
    game.select_indices(&[0]);

    // Choice 2: top or bottom
    assert!(game.has_pending_choice(), "Prompt: top or bottom");
    game.select_option(0); // top

    assert!(!game.has_pending_choice(), "No remaining choices");
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&live),
        "Live card on top of deck"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "Live card removed from waitroom"
    );
}

/// Non-Aqours live card → condition fails → no trigger.
#[test]
fn test_q252_non_aqours_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD"); // START:DASH!! (µ's)
    let _ = setup_riko_and_filler(&mut game, &[live]);
    trigger_riko_auto(&mut game);

    assert!(
        !game.has_pending_choice(),
        "Non-Aqours live should NOT trigger Riko's auto"
    );
}

/// Q252 main: 2 Aqours live cards in waitroom → choose 1 → pick top.
#[test]
fn test_q252_two_cards_choose_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live1 = game.id(AQOURS_LIVE);
    let live2 = game.new_id(AQOURS_LIVE);
    let _ = setup_riko_and_filler(&mut game, &[live1, live2]);
    trigger_riko_auto(&mut game);

    // Choice 1: choose 1 of 2 recently moved cards
    assert!(game.has_pending_choice(), "Prompt: choose 1 of 2 cards");
    game.select_indices(&[0]); // pick live1

    // Choice 2: top or bottom
    assert!(game.has_pending_choice(), "Prompt: top or bottom");
    game.select_option(0); // top

    assert!(!game.has_pending_choice(), "No remaining choices");
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&live1),
        "live1 on top of deck"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live2),
        "live2 stays in waitroom"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live1),
        "live1 removed from waitroom"
    );
}

/// Q252: 3 cards, pick the middle one → bottom.
#[test]
fn test_q252_three_cards_choose_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live1 = game.id(AQOURS_LIVE);
    let live2 = game.new_id(AQOURS_LIVE);
    let live3 = game.new_id(AQOURS_LIVE);
    let _ = setup_riko_and_filler(&mut game, &[live1, live2, live3]);
    trigger_riko_auto(&mut game);

    // Choice 1: choose 1 of 3
    assert!(game.has_pending_choice(), "Prompt: choose 1 of 3");
    game.select_indices(&[1]); // pick live2

    // Choice 2: top or bottom
    assert!(game.has_pending_choice(), "Prompt: top or bottom");
    game.select_option(1); // bottom

    assert!(!game.has_pending_choice(), "No remaining choices");
    assert_eq!(
        game.state.player1.main_deck.cards.last(),
        Some(&live2),
        "live2 on bottom of deck"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live1),
        "live1 stays in waitroom"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live3),
        "live3 stays in waitroom"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live2),
        "live2 removed from waitroom"
    );
}

/// Use limit: 1/turn — second trigger does nothing.
#[test]
fn test_q252_use_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live1 = game.id(AQOURS_LIVE);
    let live2 = game.new_id(AQOURS_LIVE);
    let _ = setup_riko_and_filler(&mut game, &[live1]);

    // First trigger
    trigger_riko_auto(&mut game);
    assert!(game.has_pending_choice(), "First trigger works");
    game.select_indices(&[0]);
    game.select_option(0); // top
    assert!(!game.has_pending_choice(), "First trigger resolved");

    // Second trigger — blocked by use_limit=1
    game.state.player1.waitroom.add_card(live2);
    game.state.recently_moved_cards = Some(vec![live2]);
    trigger_riko_auto(&mut game);
    assert!(
        !game.has_pending_choice(),
        "Second trigger blocked by use_limit"
    );
}

/// Test: choose 1 of 2, NOT the first.
#[test]
fn test_q252_pick_second_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live1 = game.id(AQOURS_LIVE);
    let live2 = game.new_id(AQOURS_LIVE);
    let _ = setup_riko_and_filler(&mut game, &[live1, live2]);
    trigger_riko_auto(&mut game);

    // Pick live2 (index 1), top
    assert!(game.has_pending_choice(), "Prompt: choose 1 of 2");
    game.select_indices(&[1]); // pick live2
    game.select_option(1); // bottom

    assert!(!game.has_pending_choice(), "No remaining choices");
    assert_eq!(
        game.state.player1.main_deck.cards.last(),
        Some(&live2),
        "live2 on bottom"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live1),
        "live1 stays in waitroom"
    );
}

fn setup_end_to_end(game: &mut TestGame, live_cards: &[i16]) {
    let riko = game.id("PL!S-bp6-002-R\u{ff0b}");
    game.state.player1.stage.stage[1] = riko;

    let filler = game.new_id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }

    for &cid in live_cards {
        game.state.player1.live_card_zone.cards.push(cid);
    }

    let p2_member = game.new_id("PL!-sd1-010-SD");
    game.state.player2.live_card_zone.cards.push(p2_member);
    game.state.player2_cheer_blade_heart_count = 1;

    game.state.current_turn_phase = rabuka_engine::game_state::TurnPhase::Live;
    game.state.current_phase = rabuka_engine::game_state::Phase::LiveVictoryDetermination;
    assert!(!game.has_pending_choice());
}

/// End-to-end test simulating the live phase and victory determination.
/// Riko is on stage, player sets an Aqours live card, resolves the live victory determination,
/// which moves the live card to waitroom and should trigger Riko's auto ability automatically.
#[test]
fn test_q252_stage_trigger_simulation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id(AQOURS_LIVE);
    setup_end_to_end(&mut game, &[live]);

    rabuka_engine::turn::TurnEngine::advance_phase(&mut game.state);

    assert!(
        game.has_pending_choice(),
        "Riko's auto ability should be triggered"
    );
    game.select_option(0); // place on top of deck

    assert!(
        !game.has_pending_choice(),
        "Choice should be fully resolved"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&live),
        "Live card should be put on top of deck"
    );
}

/// End-to-end: 2 Aqours live cards go to waitroom simultaneously.
/// Riko's auto ability triggers, player can only choose 1 to put on deck (Q252 ruling).
#[test]
fn test_q252_stage_two_cards_simulation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live1 = game.id(AQOURS_LIVE);
    let live2 = game.new_id(AQOURS_LIVE);
    setup_end_to_end(&mut game, &[live1, live2]);

    rabuka_engine::turn::TurnEngine::advance_phase(&mut game.state);

    assert!(
        game.has_pending_choice(),
        "Riko's auto should prompt: choose 1 of 2 cards"
    );
    game.select_indices(&[0]); // pick live1

    assert!(game.has_pending_choice(), "Prompt: top or bottom");
    game.select_option(0); // top

    assert!(!game.has_pending_choice(), "Fully resolved");
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&live1),
        "live1 on top of deck"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live2),
        "live2 stays in waitroom (only 1 can be placed)"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live1),
        "live1 removed from waitroom"
    );
}

/// Mixed recently_moved: both Aqours and non-Aqours live cards.
/// Only the Aqours card should be available for selection.
#[test]
fn test_q252_mixed_aqours_and_non_aqours() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aqours_live = game.id(AQOURS_LIVE);
    let non_aqours_live = game.id("PL!-sd1-019-SD"); // µ's live card
    let _ = setup_riko_and_filler(&mut game, &[aqours_live, non_aqours_live]);
    trigger_riko_auto(&mut game);

    // Even with 1 matching card, if the source zone has non-matching cards,
    // the engine prompts the user to select which card (or skip).
    assert!(game.has_pending_choice(), "Prompt: select the Aqours card");
    game.select_indices(&[0]); // pick the Aqours live card

    // Now the deck position choice
    assert!(game.has_pending_choice(), "Prompt: deck top or bottom");
    game.select_option(0); // top

    assert!(!game.has_pending_choice(), "Fully resolved");
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&aqours_live),
        "Aqours live card on top of deck"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_aqours_live),
        "Non-Aqours live card stays in waitroom"
    );
}

/// Non-live cards in recently_moved: only Aqours live cards should trigger.
/// Aqours member cards placed in waitroom should NOT trigger the ability.
#[test]
fn test_q252_non_live_in_recently_moved() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aqours_live = game.id(AQOURS_LIVE);
    let aqours_member = game.new_id("PL!S-pb1-005-R"); // 渡辺曜 (Aqours member)

    let _ = setup_riko_and_filler(&mut game, &[aqours_live, aqours_member]);
    trigger_riko_auto(&mut game);

    // Even with 1 matching card, the engine prompts to select it (or skip)
    // because the source zone (discard/waitroom) has non-matching cards too.
    assert!(
        game.has_pending_choice(),
        "Prompt: select the Aqours live card"
    );
    game.select_indices(&[0]); // pick the Aqours live card

    // Now the deck position choice
    assert!(game.has_pending_choice(), "Prompt: deck top or bottom");
    game.select_option(1); // bottom

    assert!(!game.has_pending_choice(), "Fully resolved");
    assert_eq!(
        game.state.player1.main_deck.cards.last(),
        Some(&aqours_live),
        "Aqours live card on bottom of deck"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&aqours_member),
        "Aqours member stays in waitroom (not a live card)"
    );
}

/// Skip/optional: player may decline to move any card.
/// The effect says "置いてもよい" (may place) — skipping keeps the card in waitroom.
#[test]
fn test_q252_skip_optional_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id(AQOURS_LIVE);
    let _ = setup_riko_and_filler(&mut game, &[live]);
    trigger_riko_auto(&mut game);

    // Choice: pick card or skip. Empty indices → skip.
    assert!(game.has_pending_choice(), "Prompt: pick card or skip");
    game.select_indices(&[]);

    assert!(!game.has_pending_choice(), "No further choices after skip");
    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "Live card remains in waitroom after skip"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&live),
        "Live card NOT moved to deck after skip"
    );
}

/// Skip/optional with 2 cards: skipping keeps both in waitroom.
/// If a future change breaks the skip mechanism, having 2 cards in the pool
/// may expose issues (e.g. partial move, incorrect card count after skip)
/// that a single-card skip test would not catch.
#[test]
fn test_q252_skip_optional_two_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live1 = game.id(AQOURS_LIVE);
    let live2 = game.new_id(AQOURS_LIVE);
    let _ = setup_riko_and_filler(&mut game, &[live1, live2]);
    trigger_riko_auto(&mut game);

    // Choice: pick 1 of 2 or skip. Empty indices → skip.
    assert!(game.has_pending_choice(), "Prompt: pick 1 of 2 or skip");
    game.select_indices(&[]);

    assert!(!game.has_pending_choice(), "No further choices after skip");
    assert!(
        game.state.player1.waitroom.cards.contains(&live1),
        "live1 remains in waitroom after skip"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live2),
        "live2 remains in waitroom after skip"
    );
}
