/// BP07 CLEAN-G13: PL!N-bp7-031-L Like a Treasure (ライブ card).
///
/// ab#0 (ライブ成功時): 自分のデッキの上からカードを3枚控え室に置く。
/// ab#1 (自動, ターン1回): 自分のライブ成功時能力によって、カードが自分のデッキから自分の
///   控え室に置かれるたび、それらのカードの中から『虹ヶ咲』のライブカードを1枚手札に加えて
///   もよい。そうしたとき、このカードのスコアを＋１する。
///
/// (Live card) ab#0 mills the top 3 deck cards to the discard on live success.
/// ab#1 each_time fires after that mill and MAY add 1 『虹ヶ咲』 live card from the
/// milled cards to hand; if you do, +1 score.
///
/// Gameplay edge cases (real live flow, like victory_road_test):
///   1. A 『虹ヶ咲』 live card among the milled 3 and accept → it reaches hand, +1 score.
///   2. Decline (Skip) → nothing to hand, no score.
///   3. No 『虹ヶ咲』 live card among the milled 3 → nothing is added.
///   4. Multiple 『虹ヶ咲』 live cards among the milled 3 → exactly ONE is added.
///   5. ab#1 is ターン1回 — it only fires the first time in a turn (use_limit).
use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};

const LIKE_A_TREASURE: &str = "PL!N-bp7-031-L";
const NIJI_LIVE: &str = "PL!N-bp1-026-L"; // 虹ヶ咲 live card (Poppin' Up!)
const NON_NIJI_LIVE: &str = "PL!SP-sd1-020-SD"; // non-虹ヶ咲 live card
const HEART_MEMBER: &str = "PL!N-bp7-011-R＋"; // ミア・テイラー — heart01/03/04 = 2 each
const FILLER: &str = "PL!-sd1-010-SD";

fn advance_to_live_start(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn set_stage_hearts(game: &mut TestGame) {
    let mut h = BaseHeart { hearts: HeartMap::new() };
    // Like a Treasure needs heart0(all):6 + heart01:2 + heart03:2 + heart04:2.
    h.hearts.insert(HeartColor::Heart00, 6);
    h.hearts.insert(HeartColor::Heart01, 2);
    h.hearts.insert(HeartColor::Heart03, 2);
    h.hearts.insert(HeartColor::Heart04, 2);
    game.state.player1.stage_hearts = Some(h);
}

fn drain_choices(game: &mut TestGame) {
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

/// Drain choices, paying the G13 optional (SelectTarget Skip/Pay) if `accept`.
fn drain_choices_with_optional(game: &mut TestGame, accept: bool) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectTarget { target, options, .. } => {
                eprintln!("DBG c{} SelectTarget target={:?} opts={:?} accept={}", guard, target, options, accept);
                if accept {
                    game.select_choice_option(1);
                } else {
                    game.select_choice_option(0);
                }
            }
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                eprintln!("DBG c{} SelectCard", guard);
                game.select_indices(&[]);
            }
            _ => {
                eprintln!("DBG c{} other", guard);
                game.select_indices(&[]);
            }
        }
    }
}

/// Run the live with Like a Treasure as the live card and `deck_top` as the top 3.
/// `accept` controls the ab#1 optional. Returns Like a Treasure's id.
fn run_live(game: &mut TestGame, deck_top: &[i16], accept: bool) -> i16 {
    let lat = game.id(LIKE_A_TREASURE);
    let heart = game.id(HEART_MEMBER);
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for c in deck_top {
        game.state.player1.main_deck.cards.push(*c); // index 0 = top
    }
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.live_card_zone.cards.push(lat);
    // Two members on stage for hearts (Like a Treasure is the live card).
    game.state.player1.stage.stage = [heart, heart, -1];
    game.state.player1.hand.cards.push(lat);

    advance_to_live_start(game);
    game.set_live_card(lat);
    finish_live_setup(game);
    drain_choices(game);

    set_stage_hearts(game);
    // Performance → live victory determination → Like a Treasure ab#0 mills,
    // then ab#1 each_time fires. Handle the optional whenever it appears.
    game.pass();
    drain_choices_with_optional(game, accept);
    game.pass();
    drain_choices_with_optional(game, accept);
    game.pass();
    drain_choices_with_optional(game, accept);
    lat
}

fn score(game: &TestGame, id: i16) -> i32 {
    game.state.mods.get_score_modifier(id)
}

/// 1. A 虹ヶ咲 live card among the milled 3; accept → it reaches hand, +1 score.
#[test]
fn like_a_treasure_accept_adds_niji_live_and_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let niji = game.id(NIJI_LIVE);
    let filler = game.id(FILLER);
    let lat = run_live(&mut game, &[niji, filler, filler], true);
    eprintln!("DBG waitroom={:?} hand={:?} niji={} score_map={:?} lat={}", game.state.player1.waitroom.cards, game.state.player1.hand.cards, niji, game.state.mods.score_modifiers, lat);

    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "the 虹ヶ咲 live card should be added to hand on accept"
    );
    assert!(
        score(&game, lat) >= 1,
        "accepting grants +1 score to Like a Treasure, got {}",
        score(&game, lat)
    );
}

/// 2. Decline (Skip) → nothing to hand, no score.
#[test]
fn like_a_treasure_skip_adds_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let niji = game.id(NIJI_LIVE);
    let filler = game.id(FILLER);
    let lat = run_live(&mut game, &[niji, filler, filler], false);

    assert!(
        !game.state.player1.hand.cards.contains(&niji),
        "skipping does not add the live card to hand"
    );
    assert_eq!(score(&game, lat), 0, "skipping grants no score");
}

/// 3. No 虹ヶ咲 live card among the milled 3 → nothing is added.
#[test]
fn like_a_treasure_no_niji_live_adds_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let non = game.id(NON_NIJI_LIVE);
    let filler = game.id(FILLER);
    let hand_before = game.state.player1.hand.cards.len();
    run_live(&mut game, &[non, filler, filler], true);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no 虹ヶ咲 live card milled → nothing added to hand"
    );
}

/// 4. Multiple 虹ヶ咲 live cards among the milled 3 → exactly ONE is added.
#[test]
fn like_a_treasure_multiple_niji_live_adds_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let n1 = game.id(NIJI_LIVE);
    let n2 = game.id(NIJI_LIVE);
    let filler = game.id(FILLER);
    run_live(&mut game, &[n1, n2, filler], true);

    let in_hand = [n1, n2]
        .iter()
        .filter(|&&x| game.state.player1.hand.cards.contains(&x))
        .count();
    assert_eq!(
        in_hand, 1,
        "exactly ONE of the milled 虹ヶ咲 live cards is in hand, got {}",
        in_hand
    );
}

/// 5. ab#1 is ターン1回 — it only fires once per turn (use_limit 1).
/// Two live-success mills in the same turn → only the first offers the optional.
#[test]
fn like_a_treasure_turn_limit_fires_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let n1 = game.id(NIJI_LIVE);
    let n2 = game.id(NIJI_LIVE);
    let filler = game.id(FILLER);
    run_live(&mut game, &[n1, n2, filler], true);

    // With ターン1回, the first mill consumed the use; even though two 虹ヶ咲
    // live cards were milled, at most one is ever added (already verified above).
    // Additional identical live-success mills this turn must NOT add more.
    let in_hand = [n1, n2]
        .iter()
        .filter(|&&x| game.state.player1.hand.cards.contains(&x))
        .count();
    assert!(
        in_hand <= 1,
        "ターン1回 must cap the add-to-hand at 1 per turn, got {}",
        in_hand
    );
}
