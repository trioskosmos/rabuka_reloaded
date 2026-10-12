// v17 — the draw-probability bot.
//
// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on
// the module does not re-export those, so they are imported here
// explicitly.

use rabuka_engine::bot::strategy_v17::*;
use rabuka_engine::bot::v17_model::*;
use rabuka_engine::game_setup::*;
use rabuka_engine::card::*;
use rabuka_engine::game_state::*;

fn db_real() -> rabuka_engine::Arc<CardDatabase> {
    rabuka_engine::card_loader::test_card_db().clone()
}

fn two_player_gs(db: &rabuka_engine::Arc<CardDatabase>) -> GameState {
    let p1 = rabuka_engine::player::Player::new("p1".into(), "P1".into(), true);
    let p2 = rabuka_engine::player::Player::new("p2".into(), "P2".into(), false);
    GameState::new(p1, p2, rabuka_engine::Arc::clone(db))
}

/// A live card from the real database, with its id.
fn a_live(db: &CardDatabase) -> i16 {
    db.cards
        .iter()
        .flatten()
        .find(|c| c.card_type == CardType::Live)
        .and_then(|c| db.get_card_id(c.card_no.as_ref()))
        .expect("database has a live card")
}

/// A member card carrying base hearts and a deployable
/// cost, for building a board that can actually pass a
/// check.
fn a_member_with_hearts(db: &CardDatabase) -> i16 {
    db.cards
        .iter()
        .flatten()
        .find(|c| {
            c.card_type == CardType::Member
                && c.cost.is_some_and(|cost| cost <= 8)
                && c.base_heart
                    .as_ref()
                    .is_some_and(|bh| bh.hearts.values_sum() > 0)
        })
        .and_then(|c| db.get_card_id(c.card_no.as_ref()))
        .expect("database has a member with hearts")
}

/// A plain member card with a low printed cost, so it is
/// deployable under the fixture's energy.
fn a_member(db: &CardDatabase) -> i16 {
    db.cards
        .iter()
        .flatten()
        .find(|c| {
            c.card_type == CardType::Member
                && c.cost.is_some_and(|cost| cost <= 4)
        })
        .and_then(|c| db.get_card_id(c.card_no.as_ref()))
        .expect("database has a cheap member card")
}

/// An energy card.
fn an_energy(db: &CardDatabase) -> i16 {
    db.cards
        .iter()
        .flatten()
        .find(|c| c.is_energy())
        .and_then(|c| db.get_card_id(c.card_no.as_ref()))
        .expect("database has an energy card")
}

// ── hypergeometric ────────────────────────────────────────────

#[test]
fn hypergeometric_tail_is_exact() {
    // P(draw >= 1 success in 1 draw from 60 cards with 30 successes)
    // = 30/60 = 0.5.
    assert!((hypergeometric_ge(1, 1, 30, 60) - 0.5).abs() < 1e-9);

    // P(draw >= 1 in 2 draws) = 1 - (30/60)(29/59) = 1 - 0.24576...
    let p = hypergeometric_ge(2, 1, 30, 60);
    assert!((p - (1.0 - (30.0 * 29.0) / (60.0 * 59.0))).abs() < 1e-9);

    // P(X >= 0) is always 1.
    assert_eq!(hypergeometric_ge(3, 0, 5, 60), 1.0);

    // No successes in the population: can never draw one.
    assert_eq!(hypergeometric_ge(3, 1, 0, 60), 0.0);

    // Asking for more successes than exist is impossible.
    assert_eq!(hypergeometric_ge(3, 2, 1, 60), 0.0);

    // Empty deck: nothing can be drawn.
    assert_eq!(hypergeometric_ge(1, 1, 30, 0), 0.0);

    // P(X >= 2) for X ~ Hypergeometric(5, 2, 3):
    // only X = 2 is possible at the tail, = C(2,2)C(3,1)/C(5,3) = 3/10.
    assert!((hypergeometric_ge(3, 2, 2, 5) - 0.3).abs() < 1e-9);
}

#[test]
fn hypergeometric_tail_is_monotone_in_draws() {
    // More draws can only raise P(>= k).
    for k in 1..=3 {
        let mut prev = 0.0;
        for n in 0..=10 {
            let p = hypergeometric_ge(n, k, 20, 60);
            assert!(
                p >= prev - 1e-12,
                "P(>={k}) must be non-decreasing in n: {prev} -> {p}"
            );
            prev = p;
        }
    }
}

// ── expected placements ───────────────────────────────────────

#[test]
fn expected_placements_prices_a_terminal_win() {
    let db = db_real();
    let mut gs = two_player_gs(&db);
    gs.player1.is_first_attacker = true;
    gs.game_result = GameResult::FirstAttackerWins;
    let win = expected_placements(&gs, 0, &db);
    let loss = expected_placements(&gs, 1, &db);
    assert_eq!(win, WIN);
    assert_eq!(loss, LOSS);
}

#[test]
fn expected_placements_rewards_holding_a_life() {
    let db = db_real();
    let live = a_live(&db);
    let member = a_member_with_hearts(&db);

    // Two identical boards; one holds a life in hand, the other
    // does not. Holding the life is the only difference, so the
    // forecast must be at least as high — the t=0 check is
    // placeable only when a life is in hand.
    let mut with_life = two_player_gs(&db);
    with_life.current_phase = Phase::Main;
    with_life.player1.hand.cards.push(live);
    with_life.player1.hand.cards.push(member);
    with_life.player1.stage.stage = [member, -1, -1];
    with_life.player1.main_deck.cards.extend([member; 20]);

    let mut without_life = two_player_gs(&db);
    without_life.current_phase = Phase::Main;
    without_life.player1.hand.cards.push(member);
    without_life.player1.hand.cards.push(member);
    without_life.player1.stage.stage = [member, -1, -1];
    without_life.player1.main_deck.cards.extend([member; 20]);

    let a = expected_placements(&with_life, 0, &db);
    let b = expected_placements(&without_life, 0, &db);
    assert!(
        a >= b,
        "holding a life must not lower the placement forecast: {a} < {b}"
    );
}

// ── live set ──────────────────────────────────────────────────

/// Second-attacker live-set state with a controllable hand.
fn live_set_gs(
    db: &rabuka_engine::Arc<CardDatabase>,
    hand: &[i16],
    stage: [i16; 3],
    deck: &[i16],
) -> GameState {
    let mut gs = two_player_gs(db);
    gs.current_phase = Phase::LiveCardSetSecondAttacker;
    gs.current_turn_phase = rabuka_engine::game_state::TurnPhase::Live;
    gs.player2.hand.cards.clear();
    for &cid in hand {
        gs.player2.hand.cards.push(cid);
    }
    gs.player2.stage.stage = stage;
    gs.player2.main_deck.cards.clear();
    for &cid in deck {
        gs.player2.main_deck.cards.push(cid);
    }
    gs.player1.main_deck.cards.clear();
    for &cid in deck {
        gs.player1.main_deck.cards.push(cid);
    }
    gs
}

#[test]
fn live_set_sets_a_passable_life() {
    let db = db_real();
    // The same full-coverage board the v7 live tests use:
    // three members whose hearts cover the live's whole
    // requirement, so the check passes and the set must
    // select the live card.
    let live = db
        .get_card_id("PL!-sd1-019-SD")
        .expect("live card exists");
    let m005 = db
        .get_card_id("PL!-sd1-005-SD")
        .expect("member 005 exists");
    let m010 = db
        .get_card_id("PL!-sd1-010-SD")
        .expect("member 010 exists");
    let m002 = db
        .get_card_id("PL!-sd1-002-SD")
        .expect("member 002 exists");
    let gs = live_set_gs(&db, &[live], [m005, m010, m002], &[m005; 12]);
    let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
    let chosen = choose_live_set(&gs, &actions, &db);
    assert_eq!(
        chosen.action_type,
        ActionType::SelectLiveCard,
        "a passable life on a full-coverage board must be selected, got {:?}",
        chosen.action_type
    );
}

#[test]
fn live_set_folds_when_no_life_is_in_hand() {
    let db = db_real();
    let member = a_member_with_hearts(&db);
    // No life in hand: the only candidate is the fold, which
    // confirms an empty set.
    let gs = live_set_gs(&db, &[member, member], [member, -1, -1], &[member; 20]);
    let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
    let chosen = choose_live_set(&gs, &actions, &db);
    assert_ne!(
        chosen.action_type,
        ActionType::SelectLiveCard,
        "no life in hand: must not select a card to set"
    );
}

// ── main phase / ability usage ────────────────────────────────

#[test]
fn main_phase_deploys_a_member_over_pass() {
    let db = db_real();
    // The compatible trio the v7 live tests use: the live
    // "PL!-sd1-019-SD" needs three heart colours, and the
    // three members "PL!-sd1-005/010/002-SD" supply them
    // between them.
    let live = db
        .get_card_id("PL!-sd1-019-SD")
        .expect("live card exists");
    let m005 = db
        .get_card_id("PL!-sd1-005-SD")
        .expect("member 005 exists");
    let m010 = db
        .get_card_id("PL!-sd1-010-SD")
        .expect("member 010 exists");

    let mut gs = two_player_gs(&db);
    gs.current_phase = Phase::Main;
    let energy = an_energy(&db);
    for _ in 0..20 {
        gs.player1.energy_zone.cards.push(energy);
    }
    gs.player1.energy_zone.active_energy_count = 20;
    // A partially-covered board: one member staged, a second
    // compatible member in hand to deploy, and the life the
    // deployment is for. Deploying the second member grows
    // the board, which raises P(pass) for the life, so the
    // deploy must beat Pass.
    gs.player1.stage.stage = [m005, -1, -1];
    gs.player1.hand.cards.push(m010);
    gs.player1.hand.cards.push(live);
    gs.player1.main_deck.cards.extend([m005; 5]);
    gs.player1.main_deck.cards.extend([m010; 5]);
    gs.player1.main_deck.cards.extend([live; 10]);
    gs.player2.main_deck.cards.extend([m005; 20]);

    let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
    assert!(
        actions.len() >= 2,
        "fixture must offer a real choice, got {}",
        actions.len()
    );
    let chosen = choose_action(&gs, &actions, 0);
    assert_ne!(
        chosen.action_type,
        ActionType::Pass,
        "an affordable deploy must beat Pass"
    );
}

// ── mulligan ──────────────────────────────────────────────────

#[test]
fn mulligan_keeps_an_opening_that_holds_a_life() {
    let db = db_real();
    let live = a_live(&db);
    let member = a_member(&db);
    let mut gs = two_player_gs(&db);
    gs.current_phase = Phase::MulliganFirstAttacker;
    // A life plus a member, against an all-member deck: discarding
    // the life can only lose the only ammo in the game, so the
    // opening must be kept.
    gs.player1.hand.cards.push(live);
    gs.player1.hand.cards.push(member);
    gs.player1.main_deck.cards.extend([member; 20]);
    gs.player2.main_deck.cards.extend([member; 20]);

    let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
    let chosen = choose_mulligan(&gs, &actions, &db);
    assert_ne!(
        chosen.action_type,
        ActionType::SelectMulligan,
        "an opening holding a life against a life-less deck must be kept"
    );
}

#[test]
fn mulligan_discards_an_opening_with_no_life_when_the_deck_has_them() {
    let db = db_real();
    // The compatible trio the v7 live tests use: the live
    // "PL!-sd1-019-SD" needs three heart colours, and the
    // three members "PL!-sd1-005/010/002-SD" supply them
    // between them, so the full-coverage board passes the
    // check (p_pass > 0).
    let live = db
        .get_card_id("PL!-sd1-019-SD")
        .expect("live card exists");
    let m005 = db
        .get_card_id("PL!-sd1-005-SD")
        .expect("member 005 exists");
    let m010 = db
        .get_card_id("PL!-sd1-010-SD")
        .expect("member 010 exists");
    let m002 = db
        .get_card_id("PL!-sd1-002-SD")
        .expect("member 002 exists");

    let mut gs = two_player_gs(&db);
    gs.current_phase = Phase::MulliganFirstAttacker;
    // No life in hand, but the deck is full of them and the
    // board fully covers the live's requirement. Keeping
    // holds zero lives, so P(ammo at t=0) is 0 and every
    // future term is discounted; discarding both members
    // draws replacements from the all-life deck, putting
    // lives in hand for an undiscounted t=0 placement. The
    // discard must win.
    gs.player1.hand.cards.push(m005);
    gs.player1.hand.cards.push(m010);
    gs.player1.stage.stage = [m005, m010, m002];
    gs.player1.main_deck.cards.extend([live; 20]);
    gs.player2.main_deck.cards.extend([live; 20]);

    let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
    let chosen = choose_mulligan(&gs, &actions, &db);
    assert_eq!(
        chosen.action_type,
        ActionType::SelectMulligan,
        "a life-less opening against a life-dense deck must discard"
    );
}
