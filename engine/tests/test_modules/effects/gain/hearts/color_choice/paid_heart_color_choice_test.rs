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

#[test]
fn kasumi_n_bp3_002_live_start_discard_cost_grants_chosen_heart_only_to_other_nijigasaki() {
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
    assert!(
        game.has_pending_choice(),
        "cost prompt appears"
    );
    game.select_indices(&[0]);

    // Specify heart colour.
    assert!(
        game.has_pending_choice(),
        "heart colour specification must be asked"
    );
    game.select_option(4); // some colour; identity checked below

    // Exactly one colour was granted — to the OTHER 虹ヶ咲 member only.
    let colours = [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ];
    let granted_other: Vec<_> = colours
        .iter()
        .filter(|&&c| game.state.mods.get_heart_modifier(other, c) > 0)
        .collect();
    assert_eq!(
        granted_other.len(),
        1,
        "exactly one specified colour granted to the other 虹ヶ咲 member"
    );
    for c in colours {
        assert_eq!(
            game.state.mods.get_heart_modifier(kasumu, c),
            0,
            "exclude_self: かすみ herself gains nothing"
        );
        assert_eq!(
            game.state.mods.get_heart_modifier(outsider, c),
            0,
            "non-虹ヶ咲 members gain nothing"
        );
    }
}
