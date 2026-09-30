//! 矢澤にこ `PL!-pb2-009-PP` (自動)
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
//!   * even then the absolute is not the ability: `active_energy` starts from
//!     `give_energy(40)` and the play pays the incoming card's COST out of that
//!     zone, so what is left is `40 − cost`, plus 2 when the gate is met.
//!     「2枚」 is the MARGIN `auto_contribution` measures, never the absolute.
//!     Reading the absolute made a correct ability look broken.
//!
//! The two gates are asserted INDEPENDENTLY, each against 0. Comparing the two
//! negatives to each other cannot fail: both operands are `auto_contribution` of
//! cards that fail the gate, so an engine that dropped the group AND the cost
//! filter would move them together and still satisfy the equality.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const NICO: &str = "PL!-pb2-009-PP"; // 矢澤にこ, BiBi, cost 9
/// 高坂穂乃果, cost 22, series 「ラブライブ！」 — `cards.json` maps that series to
/// `group: "μ's"`, so she is a cost-15-plus μ's member despite being a Printemps unit.
const HIGH_COST_MUS: &str = "PL!-pb2-001-PP";
const LOW_COST_MUS: &str = "PL!-bp3-002-P"; // 絢瀬絵里, BiBi, cost 9
/// 平安名すみれ, CatChu!, cost 22, series 「ラブライブ！スーパースター!!」 → `group: "Liella!"`.
const HIGH_COST_NON_MUS: &str = "PL!SP-bp4-004-P";
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

/// The two fixture properties the gate rests on. Without these the コスト15以上 and
/// 『μ's』 boundaries in the behaviour tests are meaningless: a fixture that drifts
/// on either would make the matching negative vacuous rather than informative.
///
/// The group check is the engine's own matcher rather than a re-implementation, so it
/// cannot drift from what the 自動's condition actually filters on
/// (`ability::util::card_matches_group_str`).
#[test]
fn fixture_cost_and_group_are_what_the_gates_assume() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for (card_no, want_cost, want_mus) in [
        (HIGH_COST_MUS, 22u8, true),
        (LOW_COST_MUS, 9, true),
        (HIGH_COST_NON_MUS, 22, false),
        (NICO, 9, true),
    ] {
        let id = game.id(card_no);
        assert_eq!(
            game.db.get_card(id).and_then(|c| c.cost),
            Some(want_cost),
            "{card_no} must print cost {want_cost} — the コスト15以上 gate is the point"
        );
        assert_eq!(
            rabuka_engine::ability::util::card_matches_group_str(&game.db, id, Some("\u{03bc}'s")),
            want_mus,
            "{card_no} must be {want_mus} for the 『μ's』 gate — cost {want_cost}, \
             group membership is the other half of the condition"
        );
    }
}

#[test]
fn a_cost_15_plus_mus_baton_touch_activates_two_energy() {
    // LOW_COST_MUS fails the cost gate, so its contribution is the zero baseline and
    // the difference below is HIGH_COST_MUS's contribution on its own — the
    // 「エネルギーを2枚」 and nothing else.
    let baseline = auto_contribution(LOW_COST_MUS);
    let qualifying = auto_contribution(HIGH_COST_MUS);
    assert_eq!(
        qualifying - baseline,
        2,
        "コスト15以上の『μ's』のメンバーとバトンタッチしていた場合、エネルギーを2枚\
         アクティブにする — qualifying={qualifying} baseline={baseline}"
    );
}

/// The cost gate: right group, under the bound → the ability contributes NOTHING.
/// Asserted against 0 in absolute terms, not against the other negative: an engine
/// that ignored the cost bound entirely would still make the two negatives equal.
#[test]
fn a_cheap_mus_baton_touch_does_nothing() {
    assert_eq!(
        auto_contribution(LOW_COST_MUS),
        0,
        "コスト15以上 — 絢瀬絵里 prints cost 9, under the bound, so displacing 矢澤にこ \
         with her adds no energy on top of the cost the play already paid"
    );
}

/// The group gate: over the bound, wrong group → the ability contributes NOTHING.
/// Asserted against 0 in absolute terms. This one is the control that the group half
/// of the condition is live at all: 平安名すみれ is cost 22, so she clears the cost
/// gate, and only the group can stop her.
#[test]
fn an_expensive_non_mus_baton_touch_does_nothing() {
    assert_eq!(
        auto_contribution(HIGH_COST_NON_MUS),
        0,
        "『μ's』のメンバー — 平安名すみれ prints cost 22 and so passes コスト15以上, but \
         her series is 「ラブライブ！スーパースター!!」 (Liella!), not μ's, so passing \
         the cost gate must not be enough"
    );
}
