//! 宮下愛 `PL!N-sd2-005-SD2` — 「手札を2枚控え室に置いてもよい：好きなハートの色を1つ指定する。
//! ライブ終了時まで、そのハートを2つ得る。」
//!
//! ## Why the choice has to be pinned by NAME
//!
//! The colour is a genuine six-way decision: `execute_specify_heart_color`
//! (`engine/src/ability/effects/required_hearts.rs:352-365`) presents
//! `Choice::SelectHeartColor` with heart01..heart06 and heart00 deliberately excluded.
//!
//! The previous version of this file checked the SHAPE of the outcome — exactly one colour
//! boosted, by exactly +2 — by reading whichever colour ended up boosted. That is satisfied
//! by an implementation that ignores the player's selection and always applies the first
//! option, so the six options were indistinguishable to it. `heart_colour_choice_and_optional_banking_test.rs`
//! even records that choice-identity coverage was wanted and abandoned.
//!
//! These tests therefore name the expected colour, and pick a different one in each, so
//! "always option 0" fails immediately.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const AI: &str = "PL!N-sd2-005-SD2";

/// Drive 愛's ability paying the optional cost, choose `option`, and return the single
/// heart colour the engine boosted by the expected amount.
fn boosted_colour_for_option(option: i16) -> (HeartColor, i16) {
    let mut game = TestGame::new(load_real_database());
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id(AI);
    game.assert_card_identity(me, AI);
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id("PL!-sd1-010-SD"));
    game.add_to_hand(game.new_id("PL!S-sd1-001-SD"));

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice(), "discard-2 cost prompted");
    game.select_indices(&[0, 1]);
    assert!(
        game.has_pending_choice(),
        "好きなハートの色を1つ指定する — the colour choice must be offered after the cost"
    );
    game.select_option(option);

    let all = [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ];
    let boosted: Vec<HeartColor> = all
        .iter()
        .filter(|c| game.state.mods.get_heart_modifier(me, **c) > 0)
        .copied()
        .collect();
    assert_eq!(
        boosted.len(),
        1,
        "そのハートを2つ得る — exactly one colour is boosted, whichever was chosen"
    );
    let chosen = boosted[0];
    let amount = game.state.mods.get_heart_modifier(me, chosen);
    (chosen, amount)
}

#[test]
fn pl_n_sd2_005_sd2_paid_discard_grants_exactly_two_hearts_of_one_color() {
    let (chosen, amount) = boosted_colour_for_option(3);

    assert_eq!(
        chosen,
        HeartColor::Heart04,
        "the colour the PLAYER chose must be the one boosted — option 3 of heart01..heart06 \
         is heart04. A different colour here means the selection is being ignored"
    );
    assert_eq!(
        amount, 2,
        "そのハートを2つ得る — exactly +2, not the whole colour set"
    );
}

/// The same ability, a different option. Together with the test above this is the
/// choice-identity pair: no single hard-coded colour can satisfy both.
#[test]
fn pl_n_sd2_005_sd2_the_chosen_colour_follows_the_option_not_a_fixed_default() {
    let (first_chosen, first_amount) = boosted_colour_for_option(3);
    let (second_chosen, second_amount) = boosted_colour_for_option(5);

    assert_eq!(first_chosen, HeartColor::Heart04, "option 3 is heart04");
    assert_eq!(second_chosen, HeartColor::Heart06, "option 5 is heart06");
    assert_ne!(
        first_chosen, second_chosen,
        "two different options must grant two different colours — if these are equal the \
         engine is not reading the player's selection at all"
    );
    assert_eq!(first_amount, 2, "option 3 grants +2 of that colour");
    assert_eq!(second_amount, 2, "option 5 grants +2 of that colour");
}

/// The first option specifically, because "always apply option 0" is the most plausible
/// wrong implementation and option 0 is the one a default would pick.
#[test]
fn pl_n_sd2_005_sd2_first_option_grants_heart01_not_a_default() {
    let (chosen, amount) = boosted_colour_for_option(0);

    assert_eq!(
        chosen,
        HeartColor::Heart01,
        "option 0 is heart01; this pins the first option so the default cannot pass by \
         coincidence with the other cases"
    );
    assert_eq!(amount, 2, "そのハートを2つ得る");
}
