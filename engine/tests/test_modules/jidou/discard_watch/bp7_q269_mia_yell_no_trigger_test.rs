/// Q269 — ミア・テイラー PL!N-bp7-011-R＋ ab#0 (自動).
///
/// 自動：このカードがデッキから控え室に置かれたとき、手札を1枚控え室に置いてもよい。
/// そうしたとき、控え室からこのカードを手札に加える。
///
/// Official QA Q269: 「エールでこのカードが公開されました。この場合、[自動]は誘発しますか？」
/// → 「いいえ。誘発しません。」
///
/// A yell reveal (エール) is a *公開/reveal*, NOT a deck→discard placement. The
/// engine's real yell flow (turn/phases.rs) draws the revealed cards into the
/// resolution zone, records them via `push_revealed_card` (which appends to the
/// revealed-card list and pushes NO zone-change movement event), and only then
/// runs the auto-ability scan. Because ミア's ab#0 is keyed on a deck→discard
/// movement event, a yell reveal must never trigger it.
///
/// The helper `yell_reveal_and_scan` reproduces that exact real flow, so these
/// tests fail if a buggy `push_revealed_card`/yell path ever emits a movement
/// event. A positive control proves the scan is live via a genuine deck→discard
/// movement.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

const MIA: &str = "PL!N-bp7-011-R\u{ff0b}"; // ミア・テイラー
const FILLER: &str = "PL!-sd1-010-SD";

/// Put ミア among the deck's top `idx` (index 0 = top), give p1 a filler in hand.
fn deck_top_mia(game: &mut TestGame, idx: usize) -> i16 {
    let mia = game.id(MIA);
    let f = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..idx {
        game.state.player1.main_deck.cards.push(f);
    }
    game.state.player1.main_deck.cards.push(mia);
    game.state.player1.hand.cards.push(f);
    mia
}

/// Reproduce the real yell flow for p1 (mirrors turn/phases.rs): draw `count`
/// cards into the resolution zone, record each as a revealed card, then run the
/// auto-ability scan. Returns the revealed card ids.
fn yell_reveal_and_scan(game: &mut TestGame, count: u8) -> Vec<i16> {
    let pid = game.state.player1.id.clone();
    game.state.perform_cheer_check(&pid, count).unwrap();
    let revealed: Vec<i16> = game.state.resolution_zone.cards.iter().copied().collect();
    for &cid in &revealed {
        game.state.push_revealed_card(cid, None, false, Some(0), "yell");
    }
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);
    revealed
}

/// 1. Yell reveals ミア from the top → ab#0 does NOT fire.
#[test]
fn q269_yell_reveal_does_not_trigger_mia_auto() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = deck_top_mia(&mut game, 0);
    let hand_before = game.state.player1.hand.cards.len();

    let revealed = yell_reveal_and_scan(&mut game, 1);
    assert!(revealed.contains(&mia), "yell must reveal ミア");

    assert!(
        !game.has_pending_choice(),
        "Q269: yell-reveal must not create a discard/recover choice (ab#0 not triggered)"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&mia),
        "Q269: ミア must not be recovered to hand by a yell reveal"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Q269: yell reveal must not discard anything from hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&mia),
        "Q269: ミア must not move to the waitroom from a yell reveal"
    );
}

/// 2. ミア revealed mid-yell (not the very top) → still no trigger.
#[test]
fn q269_yell_reveal_mid_pool_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = deck_top_mia(&mut game, 2);
    let hand_before = game.state.player1.hand.cards.len();

    let revealed = yell_reveal_and_scan(&mut game, 3);
    assert!(revealed.contains(&mia), "yell must reveal ミア among the 3");

    assert!(
        !game.has_pending_choice(),
        "Q269: a mid-pool yell reveal must also not trigger ab#0"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&mia)
            && game.state.player1.hand.cards.len() == hand_before,
        "Q269: ミア not recovered, hand not discarded"
    );
}

/// 3. A yell reveals a ミア while ANOTHER ミア copy already sits in the waitroom.
/// The yell still moves nothing deck→discard, so ab#0 must not recover it.
#[test]
fn q269_yell_reveal_does_not_recover_existing_waitroom_copy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia_revealed = deck_top_mia(&mut game, 0);
    let mia_waitroom = game.new_id(MIA);
    game.state.player1.waitroom.cards.push(mia_waitroom);
    let hand_before = game.state.player1.hand.cards.len();

    let revealed = yell_reveal_and_scan(&mut game, 1);
    assert!(revealed.contains(&mia_revealed), "yell reveals a ミア");

    assert!(
        !game.has_pending_choice(),
        "Q269: revealing a ミア by yell must not fire ab#0 even with a ミア in waitroom"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&mia_revealed)
            && !game.state.player1.hand.cards.contains(&mia_waitroom),
        "Q269: neither ミア copy is recovered to hand"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Q269: no recover/discard"
    );
}

/// 4. Positive control: a genuine deck→discard movement DOES fire ab#0, proving
/// the scan is live (the no-triggers above are specifically because yell
/// ≠ deck→discard).
#[test]
fn q269_control_deck_to_discard_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = game.id(MIA);
    game.state.player1.waitroom.cards.push(mia);
    game.state.player1.hand.cards.push(game.id(FILLER));
    game.state.player1.hand.cards.push(game.new_id(FILLER));

    game.state
        .push_movement_event(mia, "deck", "discard", Some(mia), "p1", true);
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);

    // Accept the optional recover, then the hand-discard that enables そうしたとき.
    assert!(
        accept_mia_recover(&mut game),
        "control: a deck→discard movement must offer the conditional_optional recover"
    );
    assert!(
        game.state.player1.hand.cards.contains(&mia),
        "control: deck→discard movement must recover ミア to hand (scan is live)"
    );
}

/// Drive ミア's ab#0 optional: answer the conditional_optional (accept=1 for a
/// real 2-option choice) plus the follow-up hand-discard SelectCard. Returns
/// true if a conditional_optional was actually presented.
fn accept_mia_recover(game: &mut TestGame) -> bool {
    let mut guard = 0;
    let mut recovered = false;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectTarget {
                target, options, ..
            } if target == "conditional_optional" => {
                let accept = options.as_ref().map(|o| o.len() > 1).unwrap_or(false);
                game.select_choice_option(if accept { 1 } else { 0 });
                recovered = true;
            }
            Choice::SelectCard { count, .. } => {
                if *count > 0 {
                    game.select_indices(&[0]);
                } else {
                    game.select_indices(&[]);
                }
            }
            _ => break,
        }
    }
    recovered
}

// ====================================================================
// Q277 — ミア ab#0 + refresh-before-auto-resolve.
// デッキのカードを控え室に置く能力を解決した結果デッキがちょうど0枚になったとき、
// 自動能力の解決前にリフレッシュが行われる。結果、控え室からこのカードが無くなるため、
// 自動能力によって手札には加えられなくなる。
// ====================================================================

/// Control: ミア milled to waitroom with NO refresh → the auto fires and
/// recovers ミア to hand. Proves the recover path is live.
#[test]
fn q277_control_no_refresh_recovers_mia() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = game.id(MIA);
    game.state.player1.waitroom.cards.push(mia);
    game.state.player1.hand.cards.push(game.id(FILLER));
    game.state.player1.hand.cards.push(game.new_id(FILLER));

    game.state
        .push_movement_event(mia, "deck", "discard", Some(mia), "p1", true);
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);

    assert!(
        accept_mia_recover(&mut game),
        "control: without a refresh ミア's recover must be offered"
    );
    assert!(
        game.state.player1.hand.cards.contains(&mia),
        "control: ミア is still in the waitroom → recovered to hand"
    );
}

/// Q277 core: the mill empties the deck to exactly 0, so a refresh happens
/// BEFORE the 自動 resolves. The refresh shuffles the waitroom (incl. ミア)
/// back into the deck, so ミア can no longer be recovered to hand.
#[test]
fn q277_refresh_before_auto_resolve_prevents_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = game.id(MIA);
    // ミア was just milled deck→discard (the 自動 is queued) and now sits in the
    // waitroom. Give a hand card so the optional discard COULD be paid.
    game.state.player1.waitroom.cards.push(mia);
    game.state.push_movement_event(mia, "deck", "discard", Some(mia), "p1", true);
    let hand_card = game.id(FILLER);
    game.state.player1.hand.cards.push(hand_card);

    // The same mill emptied the deck to exactly 0 → refresh happens BEFORE the
    // 自動 resolves (Q277): the waitroom (incl. ミア) is shuffled into the deck.
    game.state.player1.main_deck.cards.clear();
    game.state.player1.refresh();
    assert!(
        !game.state.player1.waitroom.cards.contains(&mia)
            && game.state.player1.main_deck.cards.contains(&mia),
        "setup: refresh must move ミア out of the waitroom into the deck"
    );

    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);
    accept_mia_recover(&mut game);

    assert!(
        !game.state.player1.hand.cards.contains(&mia),
        "Q277: refresh before the auto resolves removes ミア from the waitroom, so it cannot be recovered"
    );
    assert!(
        game.state.player1.main_deck.cards.contains(&mia),
        "Q277: after refresh ミア is in the deck, not the waitroom"
    );
}
