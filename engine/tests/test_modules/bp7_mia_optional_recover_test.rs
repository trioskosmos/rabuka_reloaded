/// BP07 CLEAN-G7: PL!N-bp7-011-R＋ ミア・テイラー ab#0.
///
/// 自動：このカードがデッキから控え室に置かれたとき、手札を1枚控え室に置いてもよい。
/// そうしたとき、控え室からこのカードを手札に加える。
///
/// (Auto) When this card is placed from your deck to your discard, you MAY discard
/// 1 card from your hand. If you do, add this card from your discard to your hand.
///
/// The each_time fires through the real TAS scan (zone_change deck→discard).
/// Accepting the optional discards 1 from hand AND recovers ミア; skipping does
/// neither. そうしたとき — the recover is contingent on the discard actually
/// happening (empty hand ⇒ no discard ⇒ no recover).
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

const MIA: &str = "PL!N-bp7-011-R\u{ff0b}";
const FILLER: &str = "PL!-sd1-010-SD";

/// Trigger ミア's ab#0 each_time by simulating her moving deck→discard (the real
/// live-success/effect path records a movement event per card). Runs the TAS scan
/// + standby processing, then answers the conditional_on_optional.
/// `accept`: true → option 1 (pay/do it), false → option 0 (skip).
fn trigger_mia(game: &mut TestGame, mia: i16, moved: Vec<i16>, accept: bool) {
    for &cid in &moved {
        game.state
            .push_movement_event(cid, "deck", "discard", Some(mia), "p1", true);
    }
    game.state
        .trigger_auto_abilities_for_player(&game.state.player1.id.clone());
    game.state
        .process_pending_auto_abilities(&game.state.player1.id.clone());

    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectTarget { target, options, .. } if target == "conditional_optional" => {
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
                // The optional discard picks 1 from hand.
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

/// 1. ミア deck→discard; accept → 1 hand card is discarded AND ミア is recovered to hand.
#[test]
fn mia_accept_discards_one_and_recovers_self() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = game.id(MIA);
    let f1 = game.id(FILLER);
    let f2 = game.new_id(FILLER);
    // ミア is now in the discard; two cards in hand.
    game.state.player1.waitroom.cards.push(mia);
    game.state.player1.hand.cards.push(f1);
    game.state.player1.hand.cards.push(f2);

    trigger_mia(&mut game, mia, vec![mia], true);

    assert!(
        game.state.player1.hand.cards.contains(&mia),
        "ミア should be recovered to hand on accept, hand={:?}",
        game.state.player1.hand.cards
    );
    // Discarded 1 from hand, recovered 1 → hand still has 2 cards.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "accept: discarded 1 + recovered 1 → hand stays at 2, got {:?}",
        game.state.player1.hand.cards
    );
    // One of the two original hand cards was discarded to the waitroom.
    assert!(
        game.state.player1.waitroom.cards.contains(&f1)
            || game.state.player1.waitroom.cards.contains(&f2),
        "one of the original hand cards should be discarded to the waitroom"
    );
}

/// 2. Skip → nothing discarded, ミア stays in the discard.
#[test]
fn mia_skip_no_discard_no_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = game.id(MIA);
    let f1 = game.id(FILLER);
    let f2 = game.new_id(FILLER);
    game.state.player1.waitroom.cards.push(mia);
    game.state.player1.hand.cards.push(f1);
    game.state.player1.hand.cards.push(f2);
    let hand_before = game.state.player1.hand.cards.len();

    trigger_mia(&mut game, mia, vec![mia], false);

    assert!(
        game.state.player1.waitroom.cards.contains(&mia),
        "skipping leaves ミア in the discard"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "skipping discards nothing from hand"
    );
}

/// 3. Empty hand → the optional discard cannot happen, so ミア is NOT recovered.
/// そうしたとき — recover is contingent on actually discarding.
#[test]
fn mia_empty_hand_no_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = game.id(MIA);
    game.state.player1.waitroom.cards.push(mia);

    trigger_mia(&mut game, mia, vec![mia], true);

    assert!(
        game.state.player1.waitroom.cards.contains(&mia),
        "empty hand → cannot discard, so ミア stays in the discard"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&mia),
        "empty hand → no discard, so no recover to hand"
    );
}

/// 4. The recovered card is specifically ミア (self), not a random discard card.
/// Put another card in the discard that is NOT ミア — it must not be recovered.
#[test]
fn mia_recovers_only_self() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = game.id(MIA);
    let other = game.id("PL!-sd1-019-SD"); // µ's live card, NOT ミア
    let f1 = game.id(FILLER);
    game.state.player1.waitroom.cards.push(mia);
    game.state.player1.waitroom.cards.push(other);
    game.state.player1.hand.cards.push(f1);

    trigger_mia(&mut game, mia, vec![mia], true);

    assert!(
        game.state.player1.hand.cards.contains(&mia),
        "ミア recovered to hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&other),
        "the non-ミア discard card must NOT be recovered"
    );
}

/// 5. The each_time only fires when ミア goes deck→discard. If the moved card is
/// a different card, ab#0 does not fire (self_target trigger).
#[test]
fn mia_does_not_fire_for_other_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = game.id(MIA);
    let f1 = game.id(FILLER);
    game.state.player1.waitroom.cards.push(mia);
    game.state.player1.hand.cards.push(f1);

    // Another card (not ミア) went deck→discard.
    trigger_mia(&mut game, mia, vec![f1], true);

    assert!(
        !game.state.player1.hand.cards.contains(&mia),
        "ab#0 must not fire when a different card goes to the discard"
    );
    assert_eq!(game.state.player1.hand.cards.len(), 1, "hand unchanged");
}
