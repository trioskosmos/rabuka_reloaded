use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .expect("card should have the requested trigger ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn nico_bp3_009_activation_self_wait_cost_grants_only_chosen_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let niko = game.id("PL!-bp3-009-R＋");
    game.state.player1.stage.stage[1] = niko;

    game.activate_ability(niko);

    // Cost (self-wait) applies first.
    assert_eq!(
        game.state.mods.get_orientation_modifier(niko),
        Some("wait"),
        "activation cost waits this member"
    );

    // Heart colour selection appears; pick heart03 (option index 1).
    assert!(
        game.has_pending_choice(),
        "heart colour choice should be pending"
    );
    game.select_option(1);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(niko, HeartColor::Heart03),
        1,
        "chosen heart03 granted until live end"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(niko, HeartColor::Heart01),
        0,
        "unchosen colours are not granted"
    );
}

/// Drive かすみ's ability paying the optional cost, choose `option`, and return the single
/// heart colour granted to the OTHER 『虹ヶ咲』 member.
///
/// The colour is a real six-way decision, so it is reported by NAME. The previous version of
/// this test selected an option and then asserted only `granted.len() == 1` — which an
/// implementation that ignores the choice and always applies one fixed colour would satisfy.
/// `nico_bp3_009_activation_self_wait_cost_grants_only_chosen_heart` above pins its colour
/// explicitly; this one did not, despite a comment claiming it did.
fn granted_colour_to_other_for_option(option: i16) -> (HeartColor, i16, TestGame) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumu = game.id("PL!N-bp3-002-R"); // 中須かすみ QU4RTZ (虹ヶ咲)
    let other = game.id("PL!N-bp1-006-R＋"); // 近江彼方 (虹ヶ咲)
    let outsider = game.id("PL!S-bp5-009-R"); // Aqours — never targeted

    game.state.player1.stage.stage = [other, kasumu, outsider];
    game.add_to_hand(game.id(FILLER));

    trigger_auto(&mut game, kasumu, AbilityTrigger::LiveStart, "ライブ開始時");

    // Optional cost 「手札を1枚控え室に置いてもよい」 is presented directly as
    // a skippable hand SelectCard — pick the single card to pay.
    assert!(game.has_pending_choice(), "cost prompt appears");
    game.select_indices(&[0]);

    // Specify heart colour.
    assert!(
        game.has_pending_choice(),
        "好きなハートの色を1つ指定する — the colour specification must be asked"
    );
    game.select_option(option);

    let colours = [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ];
    let granted_other: Vec<HeartColor> = colours
        .into_iter()
        .filter(|&c| game.state.mods.get_heart_modifier(other, c) > 0)
        .collect();
    assert_eq!(
        granted_other.len(),
        1,
        "exactly one specified colour granted to the other 虹ヶ咲 member"
    );
    for c in [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ] {
        assert_eq!(
            game.state.mods.get_heart_modifier(kasumu, c),
            0,
            "このメンバー以外 — かすみ herself gains nothing"
        );
        assert_eq!(
            game.state.mods.get_heart_modifier(outsider, c),
            0,
            "『虹ヶ咲』のメンバー only — an Aqours member gains nothing"
        );
    }
    let chosen = granted_other[0];
    let amount = game.state.mods.get_heart_modifier(other, chosen);
    (chosen, amount, game)
}

#[test]
fn kasumi_n_bp3_002_live_start_discard_cost_grants_chosen_heart_only_to_other_nijigasaki() {
    let (chosen, amount, _game) = granted_colour_to_other_for_option(4);

    assert_eq!(
        chosen,
        HeartColor::Heart05,
        "the colour the PLAYER chose must be the one granted — option 4 of heart01..heart06 \
         is heart05. Any other colour means the selection is being ignored"
    );
    assert_eq!(amount, 1, "そのハートを1つ得る — exactly +1");
}

/// The same ability, a different option, so no single hard-coded colour satisfies both.
#[test]
fn kasumi_the_granted_colour_follows_the_option_not_a_fixed_default() {
    let (first, first_amount, _g1) = granted_colour_to_other_for_option(4);
    let (second, second_amount, _g2) = granted_colour_to_other_for_option(1);

    assert_eq!(first, HeartColor::Heart05, "option 4 is heart05");
    assert_eq!(second, HeartColor::Heart02, "option 1 is heart02");
    assert_ne!(
        first, second,
        "two different options must grant two different colours — if these are equal the \
         engine is not reading the player's selection at all"
    );
    assert_eq!(first_amount, 1, "option 4 grants +1 of that colour");
    assert_eq!(second_amount, 1, "option 1 grants +1 of that colour");
}
