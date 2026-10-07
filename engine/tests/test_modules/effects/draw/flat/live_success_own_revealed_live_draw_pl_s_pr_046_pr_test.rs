//! `PL!S-PR-046-PR` ab#0 — ライブ成功時:
//!
//! ```text
//! {{ライブ成功時}}エールによって公開された自分のカードの中にライブカードがある場合、
//! カードを1枚引く。
//! ```
//!
//! The `location_condition revealed_cards target=self card_type=live_card`
//! shape is a family (20 abilities carry it — `PL!S-bp6-009-P` ab#1,
//! `PL!S-bp3-005-P` 渡辺曜, `PL!S-bp2-004-P` シャドー・デイズ, …), and the
//! revealed pool itself is heavily tested. What THIS card pins down is the
//! 「自分の」 scope: at LiveVictoryDetermination the pool holds BOTH
//! performers' yell reveals (nothing clears it between the two performances —
//! `clear_revealed_cards` runs only after LVD at phases.rs:313), and each
//! entry carries its performer's seat in `revealed_card_meta.owner`
//! (phases.rs:437-439). An opponent's revealed live card must not satisfy
//! 「自分のカードの中にライブカードがある場合」.
//!
//! The trigger is fired directly (渡辺曜/WWD idiom) rather than through a real
//! live window, so no other card's LiveSuccess (START:DASH!!'s own draw, say)
//! pollutes the deck delta measured here.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const SUBJECT: &str = "PL!S-PR-046-PR";
/// Non-live member — a revealed card that must NOT satisfy 「ライブカードがある」.
const MEMBER: &str = "PL!-sd1-010-SD";
/// Non-live energy card, for the same reason.
const ENERGY: &str = "LL-E-001-SD";
/// START:DASH!! — the live card type the clause looks for.
const LIVE: &str = "PL!-sd1-019-SD";

/// Reveal through the same door the performance phase uses
/// (phases.rs:437-439): `push_revealed_card` records `owner` = performer seat
/// alongside the id — the tag 「自分の」 scopes against.
fn reveal(game: &mut TestGame, seat: u8, cards: &[i16]) {
    for &cid in cards {
        game.state.push_revealed_card(cid, None, false, Some(seat), "yell");
    }
}

fn setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id(MEMBER);
    fill_decks(game, filler);
    let subject = game.id(SUBJECT);
    game.assert_card_identity(subject, SUBJECT);
    game.state.player1.stage.stage = [subject, -1, -1];
    subject
}

/// Fire the subject's ライブ成功時 directly and answer whatever it raises.
fn fire_live_success(game: &mut TestGame, subject: i16) {
    let card = game.db.get_card(subject).unwrap();
    let ability = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .unwrap_or_else(|| panic!("missing ライブ成功時 ability"));
    let ability_id = format!("{}_{}", card.card_no, ability.full_text);
    let card_no = card.card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card_no),
        Some(subject),
        None,
        None,
    );
    game.state.activating_card = Some(subject);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_choices();
}

#[test]
fn own_reveals_contain_a_live_card_and_draw_one_pl_s_pr_046_pr() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let subject = setup(&mut game);
    let revealed = [game.id(LIVE), game.id(MEMBER)];
    reveal(&mut game, 0, &revealed);
    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    fire_live_success(&mut game, subject);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "自分の revealed にライブカードが1枚 → カードを1枚引く"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "…drawn to hand"
    );
}

#[test]
fn own_reveals_without_a_live_card_draw_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let subject = setup(&mut game);
    let revealed = [game.id(MEMBER), game.id(ENERGY)];
    reveal(&mut game, 0, &revealed);
    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    fire_live_success(&mut game, subject);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "自分の revealed にライブカードがない → 引かない"
    );
    assert_eq!(game.state.player1.hand.cards.len(), hand_before);
}

#[test]
fn only_the_opponents_revealed_live_card_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let subject = setup(&mut game);
    let own_reveals = [game.id(MEMBER)];
    let opp_reveals = [game.id(LIVE)];
    reveal(&mut game, 0, &own_reveals);
    reveal(&mut game, 1, &opp_reveals);
    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    fire_live_success(&mut game, subject);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "「自分のカードの中に」 — 相手のエールで公開されたライブカードは数えない"
    );
    assert_eq!(game.state.player1.hand.cards.len(), hand_before);
}
