/// BP07 CLEAN-G13: PL!N-bp7-031-L Like a Treasure (ライブ card).
///
/// ab#0 (ライブ成功時): 自分のデッキの上からカードを3枚控え室に置く。
/// ab#1 (自動, ターン1回): 自分のライブ成功時能力によって、カードが自分のデッキから自分の
///   控え室に置かれるたび、それらのカードの中から『虹ヶ咲』のライブカードを1枚手札に加えて
///   もよい。そうしたとき、このカードのスコアを＋１する。
///
/// ab#1 is a live-card each_time: when a card is placed deck→discard by one of
/// the owner's live-success abilities, MAY add 1 『虹ヶ咲』 live card among those
/// moved to hand; if you do, +1 score. ターン1回.
///
/// The each_time fires through the real TAS scan (scan live_card_zone for AUTO
/// abilities, captured trigger_moved_cards from recently_moved_cards), exactly
/// like the riko_bp6 flow — no live-phase scaffolding needed.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

const LIKE_A_TREASURE: &str = "PL!N-bp7-031-L";
const NIJI_LIVE: &str = "PL!N-bp1-026-L"; // 虹ヶ咲 live card (Poppin' Up!)
const NON_NIJI_LIVE: &str = "PL!SP-sd1-020-SD"; // non-虹ヶ咲 live card

/// Trigger Like a Treasure's ab#1 each_time by simulating the mill: the
/// `moved` cards went deck→discard by a live-success ability. Runs the real
/// TAS scan + standby processing, then answers the conditional_on_optional.
/// `accept`: true → 1 (do it), false → 0 (skip).
fn trigger_lat_ab1(game: &mut TestGame, lat: i16, moved: Vec<i16>, accept: bool) {
    // The real ab#0 mill moves deck→discard by a live-success ability, which
    // records a movement event per card. Do the same so the each_time
    // condition (destination == discard) can be evaluated.
    for &cid in &moved {
        game.state
            .push_movement_event(cid, "deck", "discard", Some(lat), "p1", true);
    }
    game.state
        .trigger_auto_abilities_for_player(&game.state.player1.id.clone());
    game.state
        .process_pending_auto_abilities(&game.state.player1.id.clone());

    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectTarget { target, options, .. }
                if target == "conditional_optional" =>
            {
                let pick = if accept { 1 } else { 0 };
                if let Some(ref opts) = options {
                    assert!(
                        pick < opts.len(),
                        "conditional_optional only offers {} options (want index {})",
                        opts.len(),
                        pick
                    );
                }
                game.select_choice_option(pick);
            }
            Choice::SelectCard { count, .. } => {
                // Follow-up "pick which card" choice (count=1 when multiple
                // 虹ヶ咲 live cards match). Select the first available index.
                if *count > 0 {
                    game.select_indices(&[0]);
                } else {
                    game.select_indices(&[]);
                }
            }
            _ => break,
        }
    }
}

fn setup_lat_live_zone(game: &mut TestGame) -> i16 {
    let lat = game.id(LIKE_A_TREASURE);
    game.state.player1.live_card_zone.cards.push(lat);
    lat
}

fn score(game: &TestGame, id: i16) -> i32 {
    game.state.mods.get_score_modifier(id)
}

/// 1. A 虹ヶ咲 live card among the moved cards; accept → it reaches hand, +1 score.
#[test]
fn like_a_treasure_accept_adds_niji_live_and_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji = game.id(NIJI_LIVE);
    let filler = game.id("PL!-sd1-010-SD");
    let lat = setup_lat_live_zone(&mut game);
    game.state.player1.waitroom.cards.push(niji);
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    let score_before = score(&game, lat);

    trigger_lat_ab1(&mut game, lat, vec![niji, filler, filler], true);

    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "the 虹ヶ咲 live card should be added to hand on accept"
    );
    assert_eq!(
        game.state.player1.hand.cards.iter().filter(|&&c| c == niji).count(),
        1,
        "exactly one copy of the 虹ヶ咲 live card is in hand"
    );
    assert_eq!(
        score(&game, lat),
        score_before + 1,
        "accepting grants exactly +1 score to Like a Treasure, got {} -> {}",
        score_before,
        score(&game, lat)
    );
    // The milled non-虹ヶ咲 filler must NOT reach hand.
    assert!(
        !game.state.player1.hand.cards.contains(&filler),
        "the non-虹ヶ咲 filler must not be added to hand"
    );
}

/// 2. Decline (Skip) → nothing to hand, no score.
#[test]
fn like_a_treasure_skip_adds_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji = game.id(NIJI_LIVE);
    let filler = game.id("PL!-sd1-010-SD");
    let lat = setup_lat_live_zone(&mut game);
    game.state.player1.waitroom.cards.push(niji);
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);

    trigger_lat_ab1(&mut game, lat, vec![niji, filler, filler], false);

    assert!(
        game.state.player1.hand.cards.is_empty(),
        "skipping adds nothing to hand, got {:?}",
        game.state.player1.hand.cards
    );
    assert_eq!(score(&game, lat), 0, "skipping grants no score");
}

/// 3. No 虹ヶ咲 live card among the moved cards → nothing is added, no score.
#[test]
fn like_a_treasure_no_niji_live_adds_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let non = game.id(NON_NIJI_LIVE);
    let filler = game.id("PL!-sd1-010-SD");
    let lat = setup_lat_live_zone(&mut game);
    game.state.player1.waitroom.cards.push(non);
    game.state.player1.waitroom.cards.push(filler);

    trigger_lat_ab1(&mut game, lat, vec![non, filler], true);

    assert!(
        game.state.player1.hand.cards.is_empty(),
        "no 虹ヶ咲 live card moved → nothing added to hand, got {:?}",
        game.state.player1.hand.cards
    );
    assert_eq!(score(&game, lat), 0, "no card added → no score");
    // The non-虹ヶ咲 live card must remain in the waitroom.
    assert!(
        game.state.player1.waitroom.cards.contains(&non),
        "the non-虹ヶ咲 live card stays in the waitroom"
    );
}

/// 4. Multiple 虹ヶ咲 live cards among the moved cards → exactly ONE is added.
#[test]
fn like_a_treasure_multiple_niji_live_adds_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let n1 = game.id(NIJI_LIVE);
    let n2 = game.id(NIJI_LIVE);
    let filler = game.id("PL!-sd1-010-SD");
    let lat = setup_lat_live_zone(&mut game);
    game.state.player1.waitroom.cards.push(n1);
    game.state.player1.waitroom.cards.push(n2);
    game.state.player1.waitroom.cards.push(filler);
    let score_before = score(&game, lat);

    trigger_lat_ab1(&mut game, lat, vec![n1, n2, filler], true);

    let in_hand = [n1, n2]
        .iter()
        .filter(|&&x| game.state.player1.hand.cards.contains(&x))
        .count();
    assert_eq!(
        in_hand, 1,
        "exactly ONE of the 虹ヶ咲 live cards is in hand, got {}",
        in_hand
    );
    assert_eq!(
        score(&game, lat),
        score_before + 1,
        "adding one card grants exactly +1 score"
    );
    // The other 虹ヶ咲 live card must stay in the waitroom.
    let remaining = [n1, n2]
        .iter()
        .filter(|&&x| game.state.player1.waitroom.cards.contains(&x))
        .count();
    assert_eq!(
        remaining, 1,
        "the non-chosen 虹ヶ咲 live card stays in the waitroom"
    );
}

/// 5. ab#1 is ターン1回 — it only fires once per turn (use_limit 1).
/// Two identical mills in the same turn → only the first offers the optional.
#[test]
fn like_a_treasure_turn_limit_fires_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let n1 = game.id(NIJI_LIVE);
    let n2 = game.id(NIJI_LIVE);
    let filler = game.id("PL!-sd1-010-SD");
    let lat = setup_lat_live_zone(&mut game);
    game.state.player1.waitroom.cards.push(n1);
    game.state.player1.waitroom.cards.push(n2);
    game.state.player1.waitroom.cards.push(filler);

    // First mill: consume the use limit, accept one.
    trigger_lat_ab1(&mut game, lat, vec![n1, n2, filler], true);
    let score_after_first = score(&game, lat);

    // Second identical mill this turn: ab#1 must NOT fire again (use_limit 1).
    let n3 = game.id(NIJI_LIVE);
    game.state.player1.waitroom.cards.push(n3);
    trigger_lat_ab1(&mut game, lat, vec![n3], true);

    assert!(
        !game.state.player1.hand.cards.contains(&n3),
        "ターン1回: second mill's card must not reach hand"
    );
    assert_eq!(
        score(&game, lat),
        score_after_first,
        "ターン1回: second mill must not grant more score"
    );
}

/// 6. "those cards" — only cards actually MOVED by the live-success mill count.
/// A 虹ヶ咲 live card that is ALREADY in the waitroom (not part of this mill)
/// must NOT be added to hand, even though it qualifies by type/group.
#[test]
fn like_a_treasure_preexisting_waitroom_niji_not_added() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji_preexisting = game.id(NIJI_LIVE); // in waitroom, NOT moved this mill
    let non = game.id(NON_NIJI_LIVE); // the card actually moved to discard
    let lat = setup_lat_live_zone(&mut game);
    game.state.player1.waitroom.cards.push(niji_preexisting);
    game.state.player1.waitroom.cards.push(non);

    trigger_lat_ab1(&mut game, lat, vec![non], true);

    assert!(
        !game.state.player1.hand.cards.contains(&niji_preexisting),
        "the pre-existing (non-moved) 虹ヶ咲 live card must NOT be added"
    );
    assert_eq!(
        score(&game, lat),
        0,
        "no eligible moved card → no score"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&niji_preexisting),
        "the pre-existing 虹ヶ咲 live card stays in the waitroom"
    );
}

/// 7. The card added must be a LIVE card, not just any 虹ヶ咲 card.
/// A 虹ヶ咲 MEMBER card among the moved cards must NOT be added (card_type filter).
#[test]
fn like_a_treasure_niji_member_not_added() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji_member = game.id("PL!N-sd2-011-SD2"); // ミア・テイラー (虹ヶ咲 member, not live)
    let lat = setup_lat_live_zone(&mut game);
    game.state.player1.waitroom.cards.push(niji_member);

    trigger_lat_ab1(&mut game, lat, vec![niji_member], true);

    assert!(
        !game.state.player1.hand.cards.contains(&niji_member),
        "a 虹ヶ咲 MEMBER card must not be added (only 虹ヶ咲 LIVE cards)"
    );
    assert_eq!(score(&game, lat), 0, "no live card added → no score");
}

/// 8. No cards moved at all → the each_time does not offer the optional.
#[test]
fn like_a_treasure_no_movement_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji = game.id(NIJI_LIVE);
    let lat = setup_lat_live_zone(&mut game);
    game.state.player1.waitroom.cards.push(niji);

    trigger_lat_ab1(&mut game, lat, Vec::new(), true);

    assert!(
        game.state.player1.hand.cards.is_empty(),
        "no movement → nothing added to hand"
    );
    assert_eq!(score(&game, lat), 0, "no movement → no score");
}

/// 9. Duplicate copies of the same 虹ヶ咲 live card id among the moved cards →
/// still at most one is added, and only one copy.
#[test]
fn like_a_treasure_duplicate_niji_copies_add_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji = game.id(NIJI_LIVE);
    let filler = game.id("PL!-sd1-010-SD");
    let lat = setup_lat_live_zone(&mut game);
    game.state.player1.waitroom.cards.push(niji);
    game.state.player1.waitroom.cards.push(niji);
    game.state.player1.waitroom.cards.push(filler);

    trigger_lat_ab1(&mut game, lat, vec![niji, niji, filler], true);

    let in_hand = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&c| c == niji)
        .count();
    assert_eq!(
        in_hand, 1,
        "duplicate copies still add at most one, got {}",
        in_hand
    );
    assert_eq!(
        score(&game, lat),
        1,
        "adding one copy grants exactly +1 score"
    );
}
