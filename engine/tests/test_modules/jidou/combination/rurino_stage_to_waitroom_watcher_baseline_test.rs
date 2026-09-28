//! PL!HS-bp6-019-N 大沢瑠璃乃 — a 自動 **無制限** stage→waitroom watcher. This file
//! pins the single-departure grant exactly and records why the two-departure
//! version is not reachable, with both halves of the obstacle verified.
//!
//!   * ab#0 自動 (無制限) 「このメンバーがステージから控え室に置かれたとき、カードを
//!     2枚引き、手札を2枚控え室に置く。」
//!   * `PL!HS-pb1-019-N` 大沢瑠璃乃 ab#0 起動 「このメンバーをステージから控え室に
//!     置く：自分の控え室からメンバーカードを1枚手札に加える。」
//!
//! ## What is proven
//!
//! One departure draws **exactly 2** and discards 2. That is the baseline a
//! two-departure test would double, and it is asserted as an exact count rather
//! than a bound.
//!
//! ## Why the two-departure test is not here — both halves verified
//!
//! A second departure in the same turn is the only thing that separates 無制限
//! from ターン1回. It needs the member back on stage mid-turn, and each obstacle
//! was driven rather than assumed:
//!
//! 1. **Re-entry — SOLVED.** Her other print's 起動 puts itself in the 控え室 and
//!    pulls a member back to hand. Driven end to end, the card under test does
//!    come back to 手札 in the same turn, and it has **no cost or group gate**.
//!    (The recovery cards that look like candidates and are *not* usable:
//!    `PL!HS-bp1-003-R＋` 乙宗 梢's 起動 recovers only 「4コスト以下の『蓮ノ空』の
//!    メンバー」 and this member costs 5; the live-phase recoveries return her in
//!    the *next* turn, which cannot distinguish 無制限 from ターン1回.)
//!
//! 1. **The first departure.** See above.
//!
//! 2. **The second departure — BLOCKED by the area lock.** Playing her back
//!    consumes that area's turn lock, and `can_baton_touch` rejects a
//!    replacement in an area deployed this turn: *"Cannot baton touch: area is
//!    locked this turn"*. The round trip uses the 起動 card's area (deployed
//!    turn 1) and the re-played slot, so by the second departure every area
//!    holding a member is locked. This is Rule 9.6.2.1 behaviour, not an
//!    engine defect.
//!
//! So the 無制限 ceiling for this family is not observable in one turn with the
//! current card pool: the re-entry works, and the area lock forbids the second
//! exit. A future two-departure test needs a way to send her to the 控え室 that is
//! **not** a baton touch (a card effect that moves a standing member out), at
//! which point the assertion here is the baseline to double.
//!
//! ## A card-number trap worth stating
//!
//! The two prints are `PL!HS-bp6-019-N` (the watcher) and `PL!HS-**pb**1-019-N`
//! (the re-entry) — **`bp` vs `pb`**, transposed. The similarly-numbered
//! `PL!HS-bp1-019-L` is a *different card* ("Dream Believers", no such ability),
//! and `get_card_id`'s rarity fallback silently resolves a mistyped `bp1` to it.
//! Both numbers are asserted below: a wrong-card fixture here fails as a missing
//! ability quoting an unrelated card's name, the most confusing failure mode this
//! audit hit.

use crate::helpers::*;
use crate::test_modules::support::baton_swap_auto_helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

const RURINO: &str = "PL!HS-bp6-019-N"; // 自動 無制限: draw 2, discard 2
const REENTRY: &str = "PL!HS-pb1-019-N"; // 起動: self→waitroom, recover a member to hand
const ARRIVER: &str = "PL!-sd1-002-SD";
const FILLER: &str = "PL!-sd1-010-SD";

fn deck_len(game: &TestGame) -> usize {
    game.state.player1.main_deck.cards.len()
}

#[test]
fn rurino_stage_to_waitroom_watcher_draws_exactly_two() {
    let mut game = TestGame::new(load_real_database());
    let watcher = game.id(RURINO);
    let reentry = game.id(REENTRY);
    game.assert_card_identity(watcher, RURINO);
    game.assert_card_identity(reentry, REENTRY);
    game.assert_card_in_group(watcher, "蓮ノ空", "the `HS` prefix is the set, not the group");

    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    game.give_energy(15);
    game.state.player1.hand.cards.clear();
    game.state.player1.stage.stage = [reentry, watcher, -1];

    let deck0 = deck_len(&game);
    let wait0 = game.state.player1.waitroom.cards.len();

    let arriver = game.new_id(ARRIVER);
    replace_member_by_baton_touch(&mut game, watcher, arriver, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() {
        guard += 1;
        assert!(guard < 20, "runaway choice prompts");
        assert!(
            matches!(game.get_pending_choice(), Choice::SelectAutoAbility { .. }),
            "瑠璃乃's 自動 is unconditional (draw 2, discard 2); an unexpected prompt \
             means something else is being answered: {:?}",
            game.get_pending_choice()
        );
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&watcher),
        "she reached the 控え室, so the trigger event really happened"
    );
    assert_eq!(
        deck0 - deck_len(&game),
        2,
        "カードを2枚引き — exact, not a bound: this is the baseline a two-departure \
         test would double."
    );
    assert!(
        game.state.player1.waitroom.cards.len() >= wait0 + 2,
        "手札を2枚控え室に置く (the two discards join the baton arrival and herself)"
    );
}
