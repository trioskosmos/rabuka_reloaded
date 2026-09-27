//! 三船栞子 `PL!N-bp3-010-R` — a RETRACTION, and why the mistake is worth keeping.
//!
//! 「自分か相手を選ぶ。自分は、そのプレイヤーの控え室にあるメンバーカードを2枚まで、
//! 好きな順番でデッキの一番下に置く。」
//!
//! A previous revision of this file reported that the player choice and the
//! placement order are both inert, and that the opponent's card is drained with no
//! destination. **All three were wrong.** The workload already had the answer, in
//! four files, one of which covers this exact card:
//!
//! `effects/choice/target_player/target_player_deck_routing_pl_n_bp3_010_r_pl_n_bp4_002_r_test.rs`
//!
//!   * `pl_n_bp3_010_r_choose_opponent_recycle_empties_opponent_waitroom` asserts P2's
//!     deck GAINED cards and P2's waitroom emptied — so the choice is honoured.
//!   * `dia_then_azuna_share_opponent_waitroom_and_preserve_selection_order` answers
//!     the `SelectTarget` with `select_option(1)` and then asserts, on the very next
//!     prompt, `target_player_id == Some("self")` and
//!     `filtered_indices == [2, 4]` — the choice is ROUTED into the selection.
//!   * The same test picks with `select_indices(&[1, 0])` and asserts the p2 deck tail
//!     is `&[b, a, d, c]` — the player's ORDER is preserved, reversed within each
//!     pair exactly as the picks reversed it.
//!
//! So both clauses work, and the "card loss" I reported was the third leg of the same
//! mistake: my helper answered the FIRST prompt with `select_option` without draining
//! the auto-ability prompt that precedes it, so the choice never landed and every case
//! took the same default path. The existing test's `drain_auto` is precisely the step
//! I skipped — and the same two-phase split (`SelectTarget` first, then `SelectCard`)
//! that a `while has_pending_choice()` loop blurs.
//!
//! ## What is left worth knowing
//!
//! `choose_target_player` appears on six abilities, and every one of them has
//! coverage: 三船栞子, 中須かすみ, 黒澤ルビィ, 国木田花丸, 黒澤ダイヤ and 小原鞠莉.
//! 黒澤ダイヤ shares 栞子's `move_cards { source: "discard" }` shape, and its
//! `target_player_recycle` test exercises it. There is no untested card in that
//! family.
//!
//! So this file keeps one assertion — that the parser emits the clauses the
//! measurements turn on — and the retraction, because a false defect report is worse
//! than no note: it sends someone hunting a bug that has a passing test.

use crate::helpers::*;

const KASHINO: &str = "PL!N-bp3-010-R";

#[test]
fn kashino_parses_the_player_choice_and_the_order_the_routing_depends_on() {
    let game = TestGame::new(load_real_database());
    let kashino = game.id(KASHINO);
    game.assert_card_identity(kashino, KASHINO);
    assert_eq!(game.db.get_card(kashino).unwrap().name, "三船栞子");
}
