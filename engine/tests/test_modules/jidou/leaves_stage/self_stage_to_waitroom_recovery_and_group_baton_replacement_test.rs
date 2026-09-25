use crate::helpers::*;
use crate::test_modules::support::baton_swap_auto_helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn resolve_rurino_choices(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() {
        guard += 1;
        assert!(guard < 20, "Rurino created too many choice prompts");
        assert!(
            matches!(game.get_pending_choice(), Choice::SelectAutoAbility { .. }),
            "Rurino's exact draw-then-discard flow must not ask for a choice: {:?}",
            game.get_pending_choice()
        );
        game.select_indices(&[]);
    }
}

#[test]
fn self_stage_to_waitroom_recovers_group_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp7-014-N");
    let niji = game.id(NIJI_LIVE);
    let arriver = game.id("PL!-sd1-002-SD");
    game.state.player1.waitroom.cards.push(niji);

    replace_member_by_baton_touch(&mut game, kasumi, arriver, MemberArea::Center);
    resolve_auto_choices_accepting_optionals(&mut game);

    assert!(
        game.state.player1.waitroom.cards.contains(&kasumi),
        "かすみ should be in the waitroom after baton touch"
    );
    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "かすみ ab#0 should add a 虹ヶ咲 live card from the discard to hand"
    );
}

#[test]
fn other_member_stage_to_waitroom_does_not_trigger_self_live_recovery() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp7-014-N");
    let other = game.id("PL!S-sd1-001-SD"); // a different member to move
    let niji = game.id(NIJI_LIVE);
    let arriver = game.id("PL!-sd1-002-SD");
    game.state.player1.waitroom.cards.push(niji);
    // かすみ stays on stage; the OTHER member is the one baton-touched off.
    game.state.player1.stage.set_area(MemberArea::LeftSide, kasumi);

    replace_member_by_baton_touch(&mut game, other, arriver, MemberArea::Center);
    resolve_auto_choices_accepting_optionals(&mut game);

    assert!(
        game.state.player1.stage.get_area(MemberArea::LeftSide) == Some(kasumi),
        "かすみ should still be on stage"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&niji),
        "かすみ ab#0 must NOT fire when a DIFFERENT member moves (self-target as written)"
    );
}

#[test]
fn self_stage_to_waitroom_draw_two_discard_one_leaves_one_card_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let toko = game.id("PL!HS-bp2-015-N");
    let arriver = game.id("PL!-sd1-002-SD");
    append_twenty_filler_cards(&mut game);

    replace_member_by_baton_touch(&mut game, toko, arriver, MemberArea::Center);
    resolve_auto_choices_accepting_optionals(&mut game);

    // hand empty after playing arriver; auto draws 2, discards 1 → final 1.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "藤島慈: draw 2, discard 1 → net hand +1 (from 0 after playing arriver)"
    );
}

#[test]
fn self_stage_to_waitroom_draws_then_discards_the_exact_two_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rurino = game.id("PL!HS-bp6-019-N");
    let arriver = game.id("PL!-sd1-002-SD");
    let draw_first = game.id("PL!N-bp1-026-L");
    let draw_second = game.id("PL!SP-bp1-023-L");
    let deck_remainder = game.id("PL!S-sd1-001-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(draw_first);
    game.state.player1.main_deck.cards.push(draw_second);
    game.state.player1.main_deck.cards.push(deck_remainder);
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();

    replace_member_by_baton_touch(&mut game, rurino, arriver, MemberArea::Center);
    resolve_rurino_choices(&mut game);

    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::Center),
        Some(arriver),
        "the arriving member should remain on the baton-touched area"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.as_slice(),
        &[deck_remainder],
        "Rurino must draw exactly the two top cards and leave the rest in order"
    );
    assert!(
        game.state.player1.hand.cards.is_empty(),
        "Rurino must discard the two newly drawn cards"
    );
    let waitroom = &game.state.player1.waitroom.cards;
    assert_eq!(waitroom.len(), 3, "only Rurino and its two discarded cards should be in the waitroom");
    assert_eq!(waitroom[0], rurino, "the baton-touched member should be in the waitroom");
    assert!(waitroom.contains(&draw_first));
    assert!(waitroom.contains(&draw_second));
}

#[test]
fn self_stage_to_waitroom_look_five_adds_live_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yuiguri = game.id("PL!HS-bp2-013-N");
    let arriver = game.id("PL!-sd1-002-SD");
    let niji = game.id(NIJI_LIVE);
    // Put the live card on deck top so it's among the looked-at 5.
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(niji);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }

    replace_member_by_baton_touch(&mut game, yuiguri, arriver, MemberArea::Center);
    resolve_auto_choices_accepting_optionals(&mut game);

    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "夕霧綴理 ab#0 should reveal a live card from the looked-at 5 into hand"
    );
}

#[test]
fn self_stage_to_waitroom_optional_discard_grants_member_heart05_and_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp6-018-N");
    let arriver = game.id("PL!-sd1-002-SD");
    let target = game.id("PL!S-sd1-001-SD"); // a stage member to buff
    // A card in hand for the optional discard.
    game.state.player1.hand.cards.push(game.id(FILLER));
    game.state.player1.stage.set_area(MemberArea::LeftSide, target);

    replace_member_by_baton_touch(&mut game, sayaka, arriver, MemberArea::Center);
    resolve_auto_choices_accepting_optionals(&mut game); // accepts conditional_optional, picks a member

    let h05 = game.state.mods.get_heart_modifier(target, HeartColor::Heart05);
    let bl = game.state.mods.get_blade_modifier(target);
    assert!(
        h05 >= 1 && bl >= 1,
        "村野さやか ab#0 should grant heart05+blade to a stage member (got h05={} blade={})",
        h05,
        bl
    );
}

#[test]
fn group_baton_replacement_places_energy_under_arriving_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = game.id("PL!N-bp7-019-N"); // 虹ヶ咲 member herself
    let arriver = game.id(NIJI_MEMBER); // 上原歩夢, 虹ヶ咲 member
    let energy = game.id(ENERGY);
    game.state.player1.energy_deck.cards.push(energy);

    // Baton-touch 歩夢 over せつ菜 (虹ヶ咲 → 虹ヶ咲) on center (slot 1).
    replace_member_by_baton_touch(&mut game, setsuna, arriver, MemberArea::Center);
    resolve_auto_choices_accepting_optionals(&mut game);

    assert!(
        game.state.player1.waitroom.cards.contains(&setsuna),
        "せつ菜 should be in the waitroom after baton touch"
    );
    assert_eq!(
        game.state.player1.stage.under_cards[1].len(),
        1,
        "せつ菜 ab#0 should place 1 energy under the arriving 虹ヶ咲 member"
    );
    assert_eq!(
        game.state.player1.stage.under_cards[1][0],
        energy,
        "the card placed under the arriving member is the energy card"
    );
}
