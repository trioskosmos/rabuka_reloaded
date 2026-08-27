use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass();
    game.pass();
}

#[test]
fn pl_hs_sd1_008_live_start_pay_cost_select_heart01_target_ally() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let izumi = game.id("PL!HS-sd1-008-SD");
    let kozue = game.id("PL!HS-bp1-012-PR");
    let tsuzuri = game.id("PL!HS-PR-004-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kozue, izumi, tsuzuri];
    let cost1 = game.new_id("PL!HS-bp1-012-PR");
    let cost2 = game.new_id("PL!HS-PR-004-PR");
    game.add_to_hand(cost1);
    game.add_to_hand(cost2);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    // Step 1: Pay optional cost — SelectCard: pick 2 蓮ノ空 from hand
    assert!(game.has_pending_choice(), "Should prompt for optional cost");
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[0, 1]);

    // Step 2: SelectHeartColor — pick heart01 from [heart01, heart04, heart05, heart06]
    assert!(game.has_pending_choice(), "Should prompt for heart color");
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectHeartColor".to_string())
    );
    game.select_indices(&[1]); // heart01 index

    // Step 3: SelectCard — pick target member from stage (蓮ノ空, exclude_self)
    assert!(game.has_pending_choice(), "Should prompt for target member");
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[0]); // kozue

    assert!(!game.has_pending_choice(), "All choices should be resolved");

    // Verify kozue gained +2 heart01
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kozue, HeartColor::Heart01),
        2,
        "kozue should have +2 heart01"
    );
    // izumi (self) should not gain hearts
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(izumi, HeartColor::Heart01),
        0,
        "izumi (self) should not gain heart01"
    );
    // tsuzuri should not gain heart01
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(tsuzuri, HeartColor::Heart01),
        0,
        "tsuzuri should not gain heart01"
    );
}

#[test]
fn pl_hs_sd1_008_live_start_skip_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let izumi = game.id("PL!HS-sd1-008-SD");
    let kozue = game.id("PL!HS-bp1-012-PR");
    let tsuzuri = game.id("PL!HS-PR-004-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kozue, izumi, tsuzuri];
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 10 {
        safety += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            break;
        }
    }

    let total: i32 = [
        HeartColor::Heart01,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ]
    .iter()
    .map(|&c| {
        game.state.mods.get_heart_modifier(kozue, c)
            + game.state.mods.get_heart_modifier(tsuzuri, c)
    })
    .sum();
    assert_eq!(total, 0, "Skipping cost should grant no hearts");
}
