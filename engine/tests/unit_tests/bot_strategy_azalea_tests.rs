// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::bot::strategy_azalea::*;
use rabuka_engine::card::*;
use rabuka_engine::game_setup::*;
use rabuka_engine::game_state::*;

use rabuka_engine::player::Player;

fn card(db: &CardDatabase, name: &str) -> i16 {
    let id = db.get_card_id(name).unwrap();
    assert_eq!(db.get_card(id).unwrap().card_no, name);
    id
}

/// Live Card Set for the second attacker, with the live-slot limit
/// cut to one: the set must pick the single highest-priced passable
/// live, which is what makes the guide pricing observable.
fn live_set_fixture() -> (GameState, rabuka_engine::Arc<CardDatabase>) {
    let db = rabuka_engine::card_loader::test_card_db().clone();
    let p1 = Player::new("p1".into(), "P1".into(), true);
    let p2 = Player::new("p2".into(), "P2".into(), false);
    let mut gs = GameState::new(p1, p2, rabuka_engine::Arc::clone(&db));
    gs.current_phase = Phase::LiveCardSetSecondAttacker;
    gs.current_turn_phase = rabuka_engine::game_state::TurnPhase::Live;
    gs.player2.live_card_set_limit_reduction = 2;
    (gs, db)
}

/// The guides price トリコリコPLEASE!! at 6, or 8 once the stage
/// carries 10+ green hearts. The scorer is the deck knowledge; the
/// live-set pick that consumes it is v8's argmax over candidate
/// value, which folds in pass probability and contestation, so the
/// pricing is asserted on the scorer directly — at 9 green hearts
/// the bonus is off (6), at 10 it flips on (8).
#[test]
fn azalea_live_set_prices_torikorico_at_eight_with_ten_green_hearts() {
    let (mut gs, db) = live_set_fixture();
    let torikorico = card(&db, "PL!S-pb1-020-L");

    // 9 green hearts: 果南(4) + 曜(2) + ダイヤ(3). Below the
    // 10-heart bar the live-start bonus is off.
    gs.player2.stage.stage = [
        card(&db, "PL!S-pb1-003-P＋"),
        card(&db, "PL!S-bp2-005-P"),
        card(&db, "PL!S-bp7-004-P"),
    ];
    assert_eq!(
        azalea_life_score(&gs, 1, &db, torikorico),
        6,
        "at 9 green hearts トリコリコ prices at its printed 6"
    );

    // 10 green hearts: 花丸(5) + 曜(2) + ダイヤ(3). The bonus
    // fires and prices the live at 8.
    gs.player2.stage.stage = [
        card(&db, "PL!S-bp5-007-P"),
        card(&db, "PL!S-bp2-005-P"),
        card(&db, "PL!S-bp7-004-P"),
    ];
    assert_eq!(
        azalea_life_score(&gs, 1, &db, torikorico),
        8,
        "at 10+ green hearts トリコリコ prices at 8"
    );
}

/// The guides price アオクハルカ at 2, or 3 once three Three's
/// Bouquet lives sit in the waitroom. Same direct assertion: with
/// an empty waitroom the bonus is off (2), with three Bouquet
/// lives waiting it flips on (3).
#[test]
fn azalea_live_set_prices_aoqharuka_at_three_with_three_bouquet_lives_waiting() {
    let (mut gs, db) = live_set_fixture();
    let aoqharuka = card(&db, "PL!HS-bp2-022-L");
    // トリニティ(2) + 夏美(1) + 花丸(5) green hearts on stage.
    gs.player2.stage.stage = [
        card(&db, "LL-bp2-001-R＋"),
        card(&db, "PL!SP-pb1-009-P＋"),
        card(&db, "PL!S-bp5-007-P"),
    ];

    // Empty waitroom: the live-start bonus is off.
    assert_eq!(
        azalea_life_score(&gs, 1, &db, aoqharuka),
        2,
        "with an empty waitroom アオクハルカ prices at its printed 2"
    );

    // Three Three's Bouquet lives in the waitroom: the bonus fires.
    for name in ["PL!HS-bp2-021-L", "PL!HS-bp5-019-L", "PL!HS-bp6-026-L"] {
        gs.player2.waitroom.cards.push(card(&db, name));
    }
    assert_eq!(
        azalea_life_score(&gs, 1, &db, aoqharuka),
        3,
        "with three Bouquet lives waiting アオクハルカ prices at 3"
    );
}

/// The guides' opening rule: keep a hand that holds an engine piece
/// (三神 / 曜 / 夏美 / 瑠璃乃 / the cross-series finisher) plus a
/// live. 渡辺曜 + Snowhalation is exactly that, so the bot confirms
/// the mulligan without discarding anything.
#[test]
fn azalea_mulligan_keeps_engine_plus_live_opening() {
    let db = rabuka_engine::card_loader::test_card_db().clone();
    let p1 = Player::new("p1".into(), "P1".into(), true);
    let p2 = Player::new("p2".into(), "P2".into(), false);
    let mut gs = GameState::new(p1, p2, rabuka_engine::Arc::clone(&db));
    gs.current_phase = Phase::MulliganFirstAttacker;
    gs.player1.hand.cards.extend([
        card(&db, "PL!S-bp2-014-N"), // 渡辺曜 — engine piece
        card(&db, "PL!-bp3-020-L"),  // Snowhalation — a live
    ]);
    let actions = generate_possible_actions(&gs);
    let pick = choose_mulligan(&gs, &actions, &db);
    assert!(
        matches!(
            pick.action_type,
            ActionType::ConfirmMulligan | ActionType::SkipMulligan
        ),
        "an engine+live opening must be kept, got {:?}",
        pick.action_type
    );
}

/// An opening with no engine piece is the one the guides redraw:
/// the most expensive non-engine members go back (up to three),
/// lives and energy stay. Five Aqours members that are not engine
/// pieces plus a live — the two cost-15 members sort first, so the
/// bot must select 桜内梨子 (hand index 1) for replacement.
#[test]
fn azalea_mulligan_redraws_openings_without_engine_pieces() {
    let db = rabuka_engine::card_loader::test_card_db().clone();
    let p1 = Player::new("p1".into(), "P1".into(), true);
    let p2 = Player::new("p2".into(), "P2".into(), false);
    let mut gs = GameState::new(p1, p2, rabuka_engine::Arc::clone(&db));
    gs.current_phase = Phase::MulliganFirstAttacker;
    gs.player1.hand.cards.extend([
        card(&db, "PL!S-bp2-001-P"), // 高海千歌 — cost 9
        card(&db, "PL!S-PR-014-PR"), // 桜内梨子 — cost 15
        card(&db, "PL!S-PR-018-PR"), // 津島善子 — cost 4
        card(&db, "PL!S-PR-020-PR"), // 小原鞠莉 — cost 9
        card(&db, "PL!S-PR-041-PR"), // 黒澤ルビィ — cost 15
        card(&db, "PL!-bp3-020-L"),  // Snowhalation — a live
    ]);
    let actions = generate_possible_actions(&gs);
    let pick = choose_mulligan(&gs, &actions, &db);
    assert_eq!(pick.action_type, ActionType::SelectMulligan);
    assert_eq!(
        pick.parameters.as_ref().and_then(|p| p.card_index),
        Some(1),
        "a no-engine opening must redraw, starting with the most expensive non-engine member"
    );
}
