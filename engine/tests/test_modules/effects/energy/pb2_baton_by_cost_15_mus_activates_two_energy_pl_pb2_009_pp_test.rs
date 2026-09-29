//! 矢澤にこ `PL!-pb2-009-PP` (自動) — NOT YET PINNED
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   自動 — このメンバーがステージから控え室に置かれたとき、このメンバーがコスト15以上の
//!   『μ's』のメンバーとバトンタッチしていた場合、エネルギーを2枚アクティブにする。
//!
//! In English: "when this member is put from the stage to the waitroom, IF the member
//! who baton-touched it was a cost-15-or-more μ's member, activate 2 energy."
//!
//! Two filters on the INCOMING card — cost ≥ 15 and group μ's — behind a
//! leave-the-stage trigger.
//!
//! # Measuring it
//!
//! Setup idiom stolen from
//! `compound/condition_and_effect/baton_touch_debut_draws_two_pl_s_pr_045_pr_test.rs`:
//! stage the subject, then `play_to_stage_baton` a card into the SAME slot.
//!
//! Two measurement traps, both hit here:
//!   * paying for the play draws energy out of the energy zone, so comparing
//!     different incoming cards conflates the cost with the ability (a cost-22 baton
//!     left 29 active against 40 for a cost-9 one). `auto_contribution` plays the
//!     SAME card twice — once displacing her, once into an empty centre — so the
//!     cost cancels.
//   * even then the absolute is wrong: displacing her yields 9 energy by itself,
//!     and the qualifying case yields 11. 「2枚」 is the MARGIN over that baseline.
//     Reading the absolute made a correct ability look broken.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const NICO: &str = "PL!-pb2-009-PP"; // 矢澤にこ, BiBi, cost 9
const HIGH_COST_MUS: &str = "PL!-pb2-001-PP"; // 高坂穂乃果, cost 22
const LOW_COST_MUS: &str = "PL!-bp3-002-P"; // 絢瀬絵里, cost 9
const HIGH_COST_NON_MUS: &str = "PL!SP-bp4-004-P"; // 平安名すみれ, CatChu!, cost 22
const ENERGY: &str = "LL-E-001-SD";
const FILLER: &str = "PL!-sd1-010-SD";

fn fill_decks(game: &mut TestGame) {
    let f = game.id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// Active energy after `incoming` is played into the centre, with 矢澤にこ already
/// there (so the play displaces her) or not (so it does not).
fn active_energy(incoming: &str, displace_nico: bool) -> u8 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    if displace_nico {
        let nico = game.id(NICO);
        game.assert_card_identity(nico, NICO);
        game.state.player1.stage.stage[MemberArea::Center as usize] = nico;
    }
    let card = game.id(incoming);
    game.state.player1.hand.cards.push(card);
    game.give_energy(40);
    game.play_to_stage_baton(card, MemberArea::Center);
    let _ = game.drain_choices();
    game.state.player1.energy_zone.active_count()
}

/// What displacing 矢澤にこ contributed, in energy cards, for `incoming`.
fn auto_contribution(incoming: &str) -> i32 {
    i32::from(active_energy(incoming, true)) - i32::from(active_energy(incoming, false))
}

/// The cost values the gate rests on. Without these the コスト15以上 boundary in any
/// eventual behaviour test is meaningless.
#[test]
fn fixture_costs_are_what_the_test_assumes() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for (card_no, want_cost) in [
        (HIGH_COST_MUS, 22u8),
        (LOW_COST_MUS, 9),
        (HIGH_COST_NON_MUS, 22),
        (NICO, 9),
    ] {
        assert_eq!(
            game.db.get_card(game.id(card_no)).and_then(|c| c.cost),
            Some(want_cost),
            "{card_no} must print cost {want_cost} — the コスト15以上 gate is the point"
        );
    }
}

#[test]
fn a_cost_15_plus_mus_baton_touch_activates_two_energy() {
    // Displacing 矢澤にこ always yields 9 energy on its own. The qualifying case
    // yields 11, so her 自動's 「エネルギーを2枚」 is the MARGIN over that baseline,
    // not the absolute. Reading the absolute is what made this look wrong.
    let baseline = auto_contribution(LOW_COST_MUS);
    let qualifying = auto_contribution(HIGH_COST_MUS);
    assert_eq!(
        qualifying - baseline,
        2,
        "コスト15以上の『μ's』のメンバーとバトンタッチしていた場合、エネルギーを2枚\
         アクティブにする — qualifying={qualifying} baseline={baseline}"
    );
}

/// The cost gate: same group, under the bound → no margin over the other control.
#[test]
fn a_cheap_mus_baton_touch_does_nothing() {
    assert_eq!(
        auto_contribution(LOW_COST_MUS),
        auto_contribution(HIGH_COST_NON_MUS),
        "コスト15以上 — a cost-9 μ's member is under the bound, so it contributes \
         nothing beyond what any displacement yields"
    );
}

/// The group gate: over the bound, wrong group → no margin over the cheap control.
#[test]
fn an_expensive_non_mus_baton_touch_does_nothing() {
    assert_eq!(
        auto_contribution(HIGH_COST_NON_MUS),
        auto_contribution(LOW_COST_MUS),
        "『μ's』のメンバー — a cost-22 CatChu! member passes the cost gate but is not \
         μ's, so passing the cost gate must not be enough"
    );
}
