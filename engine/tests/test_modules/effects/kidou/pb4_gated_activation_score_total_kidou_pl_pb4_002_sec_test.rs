//! 絢瀬絵里 `PL!-bp4-002-SEC` — a 起動 gated on the success live card zone's score total.
//!
//! {{kidou.png|起動}}{{turn1.png|ターン1回}}手札を2枚控え室に置く：自分の控え室から『μ's』の
//! ライブカードを1枚手札に加える。**この能力は、自分の成功ライブカード置き場にあるカードの
//! スコアの合計が6以上の場合のみ起動できる。**
//!
//! ## Why this file exists
//!
//! 「のみ起動できる」 is an **activation gate**, not an effect condition. It is rare: out of
//! 985 distinct ability nodes in the whole corpus this is one of only **six** with no test
//! file naming it, and this is the only one of the six that gates whether the player may pay
//! the cost at all.
//!
//! The parse is correct and worth quoting, because the gate is easy to lose: it carries
//! `comparison_type: "score"`, `aggregate: "total"`, `location: "success_live_card_zone"`,
//! `count: 6`, `operator: ">="`. Note there is NO group filter on the gate — the printed
//! clause constrains only the score total — so any live card contributes.
//!
//! ## The gate must be measured at the OFFER, not by forcing activation
//!
//! The resolver evaluates `activation_condition_parsed` only when the cost is still unpaid
//! (`engine/src/ability/resolver.rs:586-588`, `if !cost_already_paid`). An earlier version
//! of this file called `activate_ability` and asserted on the outcome; that pays the cost
//! first and therefore **skips the gate entirely**. It reported the gate as OPEN with an
//! empty success live card zone — a total of 0, which is not ≥ 6. That was measuring the
//! bypass, not the rule, and it would have read as an engine bug.
//!
//! 「のみ起動できる」 is a statement about what the player may do, so the offer list from
//! `generate_possible_actions` is the thing to measure.
//!
//! ## The boundary is pinned exactly
//!
//! Fixtures put the total on 5 and on 6, one either side of `>= 6`:
//!   * `PL!-bp6-023-L` (score 4) + `PL!-bp4-026-L` (score 1) = **5** — must NOT be offered
//!   * `PL!SP-bp1-027-L` (score 6) alone = **6** — must be offered
//! A `<=` implementation, or `>` in place of `>=`, would fail one of these.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-bp4-002-SEC";
/// score 4
const SCORE_4: &str = "PL!-bp6-023-L";
/// score 1
const SCORE_1: &str = "PL!-bp4-026-L";
/// score 6 — the exact boundary
const SCORE_6: &str = "PL!SP-bp1-027-L";

/// Stage ERI with `lives` in our own success live card zone, and report whether her 起動 is
/// **offered**.
fn offered_with_success_zone(lives: &[&str]) -> bool {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let mut total = 0i32;
    for l in lives {
        let id = game.id(l);
        let score = game
            .db
            .get_card(id)
            .map(|c| c.score)
            .unwrap_or_else(|| panic!("{l} is not in the database"))
            .unwrap_or_else(|| panic!("{l} has no printed score, so it cannot pin the bound"));
        game.assert_card_score(id, i32::from(score));
        total += i32::from(score);
        game.state.player1.success_live_card_zone.cards.push(id);
    }

    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    game.state.player1.hand.cards.push(eri);
    // The cost discards two hand cards, so the hand needs two besides ERI to be payable.
    game.state.player1.hand.cards.push(game.new_id("PL!-bp3-012-N"));
    game.state.player1.hand.cards.push(game.new_id("PL!-bp3-012-N"));
    game.give_energy(20);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(eri, MemberArea::Center);
    // `drain_auto_ability_choices` already no-ops on an empty queue and on any
    // prompt that is not an auto-ability choice, so the `if has_pending_choice()`
    // wrapper around it changed nothing.
    game.drain_auto_ability_choices();

    // `TestGame::generated_actions()` cannot be used here: it calls
    // `get_pending_choice()` first (`helpers/choices.rs:308`), which panics with
    // "No pending choice" when the turn is not inside a prompt. Asking for the offer list
    // with nothing pending is exactly the normal case. Call the engine entry point
    // directly, as `copied_ability_registration_lifecycle_test.rs` does.
    let offered = rabuka_engine::game_setup::generate_possible_actions(&game.state)
        .into_iter()
        .any(|a| {
            a.action_type == rabuka_engine::game_setup::ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(eri)
                && a.parameters.as_ref().and_then(|p| p.disabled) != Some(true)
        });
    eprintln!("success-zone score total = {total} (gate is >= 6); offered = {offered}");
    offered
}

/// CONTROL: at exactly the printed total the 起動 is offered, so the card, the fixture and
/// the offer path are all sound. Without this the two below would be unfalsifiable.
#[test]
fn a_success_zone_total_of_six_offers_the_ability() {
    assert!(
        offered_with_success_zone(&[SCORE_6]),
        "スコアの合計が6以上の場合のみ起動できる — one score-6 live totals 6, so the 起動 \
         must be offered"
    );
}

/// One point below the printed total, the 起動 must not be offered at all.
#[test]
fn a_success_zone_total_of_five_withholds_the_ability() {
    assert!(
        !offered_with_success_zone(&[SCORE_4, SCORE_1]),
        "スコアの合計が6以上 — score 4 + score 1 totals 5, one below the bound, so the 起動 \
         must NOT be offered. Exact-boundary negative: a `<`/`<=` implementation, or `>` in \
         place of `>=`, would fail here."
    );
}

/// An empty success live card zone totals 0, which is not ≥ 6.
#[test]
fn an_empty_success_zone_withholds_the_ability() {
    assert!(
        !offered_with_success_zone(&[]),
        "an empty 成功ライブカード置き場 totals 0, so the 起動 must not be offered"
    );
}
