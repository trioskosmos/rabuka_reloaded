use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!SP-pb2-005-R 葉月恋
///
/// ab#0 (登場): バトンタッチして登場した場合、このバトンタッチで控え室に置かれた
///   『Liella!』のメンバーカードを1枚、このメンバーの下に置く。
///
/// ab#1 (常時): このメンバーは、このメンバーの下に置かれている『Liella!』の
///   メンバーカードが持つ起動能力をすべて得る。

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// ab#0: Baton touch 葉月恋 over a Liella! member → member is placed under.
#[test]
fn hazuki_baton_touch_places_liella_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hazuki = game.id("PL!SP-pb2-005-R");
    let liella_member = game.new_id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = liella_member;
    game.state.player1.hand.cards.push(hazuki);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);
    game.give_energy(25);

    game.play_to_stage(hazuki, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let under = game.state.player1.stage.get_under_cards(MemberArea::Center);
    assert!(
        under.contains(&liella_member),
        "Liella! member should be under hazuki"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&liella_member),
        "Liella! member should NOT be in waitroom"
    );
}

/// ab#0: Baton touch over a non-Liella! member → nothing placed under.
#[test]
fn hazuki_baton_touch_non_liella_places_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hazuki = game.id("PL!SP-pb2-005-R");
    let non_liella = game.new_id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = non_liella;
    game.state.player1.hand.cards.push(hazuki);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);
    game.give_energy(25);

    game.play_to_stage(hazuki, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let under = game.state.player1.stage.get_under_cards(MemberArea::Center);
    assert!(
        under.is_empty() || !under.contains(&non_liella),
        "Non-Liella! member should not be placed under"
    );
}

/// ab#0 then ab#1: Baton touch 葉月恋 over a Liella! member with 起動 ability → verify
/// both under-placement AND gained_abilities.
#[test]
fn hazuki_full_workflow_gains_abilities_from_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hazuki = game.id("PL!SP-pb2-005-R");
    // PL!SP-sd1-006-SD (若菜四季) has a 起動 ability and is Liella!
    let liella_with_kidou = game.new_id("PL!SP-sd1-006-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = liella_with_kidou;
    game.state.player1.hand.cards.push(hazuki);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);
    game.give_energy(25);

    game.play_to_stage(hazuki, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let under = game.state.player1.stage.get_under_cards(MemberArea::Center);
    assert!(
        under.contains(&liella_with_kidou),
        "Liella! member with 起動 should be under hazuki"
    );

    // Trigger constant re-evaluation by passing through phases
    game.pass();
    game.pass();

    let gained = game.state.gained_abilities.get(&hazuki);
    assert!(gained.is_some(), "Hazuki should have gained abilities");
    if let Some(list) = gained {
        let has_kidou = list.iter().any(|e| e.starts_with("ability_from_source:"));
        assert!(has_kidou, "Hazuki should have copied 起動 abilities");
    }
}

/// ab#1 only: Place 葉月恋 on stage with a Liella! member (with 起動) under,
/// then activate the copied 起動 ability — all within p1's main phase.
#[test]
fn hazuki_activates_kidou_copied_from_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hazuki = game.id("PL!SP-pb2-005-R");
    // PL!SP-sd1-006-SD (若菜四季): 起動: move self stage→waitroom → return live from discard
    let liella_with_kidou = game.id("PL!SP-sd1-006-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage[1] = hazuki;
    game.state.player1.stage.under_cards[1].push(liella_with_kidou);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(live);
    fill_decks(&mut game);
    game.give_energy(5);

    // Force recalculate_constants so ab#1 copies the kidou ability
    game.state.recalculate_constants();

    // gained_card_abilities should now have the copied Ability struct
    assert!(
        game.state.gained_card_abilities.contains_key(&hazuki),
        "Hazuki should have gained card abilities"
    );
    let gained_list = &game.state.gained_card_abilities[&hazuki];
    assert!(
        gained_list
            .iter()
            .any(|a| a.triggers.as_ref().is_some_and(|t| &**t == "起動")),
        "Copied ability should have 起動 trigger"
    );

    // Now activate the copied 起動 ability — still p1's main phase
    game.activate_ability(hazuki);

    // The 起動 ability (from 若菜四季): self-stage→waitroom cost (auto via self_cost) +
    // return 1 live from discard (auto-selects when exactly 1 live in discard)
    // No pending choices expected since both steps auto-resolve.
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&hazuki),
        "Hazuki should be in waitroom after 起動 cost"
    );
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "Live card should be returned to hand by 起動 effect"
    );
}

/// ab#1 only: Non-Liella! card under → filter excludes it → no abilities gained.
#[test]
fn hazuki_non_liella_under_no_abilities_gained() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hazuki = game.id("PL!SP-pb2-005-R");
    let non_liella = game.id("PL!-sd1-005-SD");
    let _filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = hazuki;
    game.state.player1.stage.under_cards[1].push(non_liella);
    fill_decks(&mut game);
    game.give_energy(5);

    game.pass();
    game.pass();

    let gained = game.state.gained_abilities.get(&hazuki);
    assert!(
        gained.is_none() || gained.unwrap().is_empty(),
        "Non-Liella! under should not produce gained abilities"
    );
}
