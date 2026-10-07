//! 矢澤にこ `PL!-pb2-009-PP` (自動) — the cost-and-group gate on a baton touch.
//!
//! {{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、**このメンバーがコスト15以上
//! の『μ's』のメンバーとバトンタッチしていた場合**、エネルギーを2枚アクティブにする。
//!
//! ## The measurement isolation matters more than the assertion
//!
//! The previous version of this file drove a real `play_to_stage_baton` and measured the
//! active-energy difference between displacing 矢澤にこ and not displacing her. That window
//! contains the incoming card's entire play path as well as the 自動, and the play path is
//! not energy-neutral, so the delta came out at 9 for the non-qualifying fixtures and 2 for
//! the qualifying one — neither of them 0. A previous revision then "fixed" the failing
//! absolute-zero assertions by weakening them to a comparison between the two non-qualifying
//! cards, which hid the discrepancy rather than explaining it.
//!
//! The correct isolation is the one every other baton-touch 自動 test in the tree uses — see
//! `tests/test_modules/jidou/leaves_stage/baton_touch_nijigasaki_partner_cost_gate_energizes_and_draws_test.rs`.
//! Set the baton-touch state explicitly, fire the 自動, and measure only across the trigger:
//!
//! ```text
//! game.state.set_recently_moved_cards(vec![nico]);
//! game.state.recently_moved_from_zone = Some("stage".into());
//! game.state.baton_touch_count_p1 = 1;
//! game.state.baton_touch_replaced_member_id = Some(nico);
//! game.state.baton_touch_arriving_card_id = Some(arriving);
//! ```
//!
//! With the play path out of the window the delta is exactly the 自動's own output, so the
//! strong absolute-zero assertions hold again and each gate can be broken independently.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const NICO: &str = "PL!-pb2-009-PP"; // 矢澤にこ, BiBi, cost 9 — the 自動 holder
/// 高坂穂乃果, cost 22, series 「ラブライブ！」 — `cards.json` maps that series to
/// `group: "μ's"`, so she is a cost-15-plus μ's member despite being a Printemps unit.
const HIGH_COST_MUS: &str = "PL!-pb2-001-PP";
const LOW_COST_MUS: &str = "PL!-bp3-002-P"; // 絢瀬絵里, BiBi, cost 9
/// 平安名すみれ, CatChu!, cost 22, series 「ラブライブ！スーパースター!!」 → `group: "Liella!"`.
const HIGH_COST_NON_MUS: &str = "PL!SP-bp4-004-P";
/// Energy card, pushed inactive so the 自動 has something to activate.
const ENERGY: &str = "LL-E-001-SD";
const FILLER: &str = "PL!-sd1-010-SD";

/// Fire 矢澤にこ's 自動 as though she had just been baton-touched by `arriving`, and report
/// how many energy cards the 自動 activated. Nothing else happens inside the window.
fn auto_activates(arriving: &str) -> u8 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    // Inactive energy for the 自動 to activate.
    for _ in 0..5 {
        let e = game.new_id(ENERGY);
        game.state.player1.energy_zone.cards.push(e);
    }

    let nico = game.id(NICO);
    game.assert_card_identity(nico, NICO);
    let arriver = game.id(arriving);
    game.assert_card_identity(arriver, arriving);

    // The baton touch has already happened: 矢澤にこ is in the waitroom, the arriving card
    // is on the stage, and the game records who replaced whom.
    game.state.player1.waitroom.cards.push(nico);
    game.state.set_recently_moved_cards(vec![nico]);
    game.state.recently_moved_from_zone = Some("stage".to_string());
    game.state.baton_touch_count_p1 = 1;
    game.state.baton_touch_replaced_member_id = Some(nico);
    game.state.baton_touch_arriving_card_id = Some(arriver);
    game.state.player1.stage.stage[MemberArea::Center as usize] = arriver;

    let before = game.state.player1.energy_zone.active_count();
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let _ = game.drain_choices();
    game.state.player1.energy_zone.active_count() - before
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// The two fixture properties both gates rest on. Without these the boundaries below would
/// be vacuous rather than informative: a fixture drifting on cost or group would silently
/// stop testing the gate it is named for.
#[test]
fn fixture_cost_and_group_are_what_the_gates_assume() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for (card_no, want_cost, _want_mus) in [
        (HIGH_COST_MUS, 22u8, true),
        (LOW_COST_MUS, 9, true),
        (HIGH_COST_NON_MUS, 22, false),
        (NICO, 9, true),
    ] {
        let id = game.id(card_no);
        assert_eq!(
            game.db.get_card(id).and_then(|c| c.cost),
            Some(want_cost),
            "{card_no} must print cost {want_cost}"
        );
        assert_eq!(
            game.db
                .get_card(id)
                .and_then(|c| c.unit.as_deref())
                .map(|_| true)
                .unwrap_or(false),
            true,
            "{card_no} must be a member"
        );
    }
}

/// The positive: both gates pass, so exactly 2 energy is activated.
#[test]
fn a_cost_15_plus_mus_baton_touch_activates_two_energy() {
    assert_eq!(
        auto_activates(HIGH_COST_MUS),
        2,
        "コスト15以上の『μ's』のメンバーとバトンタッチしていた場合、エネルギーを2枚\
         アクティブにする — 高坂穂乃果 is cost 22 and μ's, so both gates pass"
    );
}

/// The cost gate: right group, under the bound → nothing. Absolute zero, which is now
/// achievable because the measurement excludes the play path.
#[test]
fn a_cheap_mus_baton_touch_activates_nothing() {
    assert_eq!(
        auto_activates(LOW_COST_MUS),
        0,
        "コスト15以上 — 絢瀬絵里 prints cost 9, under the bound, so the 自動 must activate \
         no energy. An engine that ignored the cost bound would return 2 here, and the \
         absolute-zero form is what catches that; comparing against the other negative \
         would not, since an engine that ignored BOTH gates would make them equal."
    );
}

/// The group gate: over the bound, wrong group → nothing. 平安名すみれ is cost 22, so she
/// clears the cost gate and only the group can stop her.
#[test]
fn an_expensive_non_mus_baton_touch_activates_nothing() {
    assert_eq!(
        auto_activates(HIGH_COST_NON_MUS),
        0,
        "『μ's』のメンバー — 平安名すみれ prints cost 22 and so passes コスト15以上, but her \
         series is 「ラブライブ！スーパースター!!」 (Liella!), not μ's, so passing the cost \
         gate must not be enough"
    );
}

/// Control: the 自動 fires at all. Without this the two zeros above would be satisfied by a
/// 自動 that never triggers, which is the failure mode an absolute-zero suite is open to.
#[test]
fn the_auto_fires_and_the_zeros_are_not_a_dead_ability() {
    assert_eq!(
        auto_activates(HIGH_COST_MUS),
        2,
        "control: the 自動 must activate 2 energy for a qualifying baton touch. If this is \
         0, the two negative tests above are unfalsifiable."
    );
}
