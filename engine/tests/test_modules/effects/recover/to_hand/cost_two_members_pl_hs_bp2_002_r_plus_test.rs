use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const KOTORI_CLEAN: &str = "PL!-pb1-021-PR";

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

fn trigger_auto_as(
    game: &mut TestGame,
    cid: i16,
    trigger: AbilityTrigger,
    trigger_str: &str,
    player_id: &str,
) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .expect("card should have the requested trigger ability");
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        player_id.to_string(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(player_id);
}

#[test]
fn repeated_sayaka_copies_keep_fetch_choices_and_constants_separate_per_player() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka_p1 = game.id("PL!HS-bp2-002-R＋");
    let sayaka_p2 = game.new_id("PL!HS-bp2-002-R＋");
    let big_p1 = game.id("PL!S-bp5-009-R");
    let big_p2 = game.new_id("PL!S-bp5-009-R");
    let cheap_p1_a = game.id("PL!-bp3-012-PR");
    let cheap_p1_b = game.new_id("PL!SP-pb1-018-N");
    let cheap_p2_a = game.id("PL!-bp3-012-PR");
    let cheap_p2_b = game.new_id("PL!SP-pb1-018-N");

    game.state.player1.stage.stage = [big_p1, sayaka_p1, -1];
    game.state.player2.stage.stage = [big_p2, sayaka_p2, -1];
    game.state.player1.waitroom.cards.extend([cheap_p1_a, cheap_p1_b]);
    game.state.player2.waitroom.cards.extend([cheap_p2_a, cheap_p2_b]);

    let p1_hand_before = game.state.player1.hand.cards.len();
    let p2_hand_before = game.state.player2.hand.cards.len();
    trigger_auto_as(
        &mut game,
        sayaka_p1,
        AbilityTrigger::Debut,
        "登場",
        "p1",
    );
    game.assert_pending_choice_type("SelectCard", "P1 Sayaka should fetch");
    game.select_indices(&[0, 1]);
    assert_eq!(game.state.player1.hand.cards.len(), p1_hand_before + 2);
    assert_eq!(game.state.player2.hand.cards.len(), p2_hand_before);
    assert!(!game.state.player1.waitroom.cards.contains(&cheap_p1_a));
    assert!(!game.state.player1.waitroom.cards.contains(&cheap_p1_b));
    assert!(game.state.player2.waitroom.cards.contains(&cheap_p2_a));
    assert!(game.state.player2.waitroom.cards.contains(&cheap_p2_b));

    trigger_auto_as(
        &mut game,
        sayaka_p2,
        AbilityTrigger::Debut,
        "登場",
        "p2",
    );
    game.assert_pending_choice_type("SelectCard", "P2 Sayaka should fetch");
    game.select_indices(&[0, 1]);
    assert_eq!(game.state.player2.hand.cards.len(), p2_hand_before + 2);
    assert_eq!(game.state.player1.hand.cards.len(), p1_hand_before + 2);
    assert!(!game.state.player2.waitroom.cards.contains(&cheap_p2_a));
    assert!(!game.state.player2.waitroom.cards.contains(&cheap_p2_b));

    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(sayaka_p1), 3);
    assert_eq!(game.state.mods.get_blade_modifier(sayaka_p2), 3);
    game.state.player1.stage.stage[0] = -1;
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(sayaka_p1), 0);
    assert_eq!(game.state.mods.get_blade_modifier(sayaka_p2), 3);
}

#[test]
fn pl_hs_bp2_002_r_plus_recovers_two_cost_two_members_not_pricey_members_or_lives() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp2-002-R＋");
    let cheap_a = game.id("PL!-bp3-012-PR");
    let cheap_b = game.id("PL!SP-pb1-018-N");
    let pricey = game.id(KOTORI_CLEAN);
    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.stage.stage[1] = sayaka;
    for cid in [pricey, cheap_a, live, cheap_b] {
        game.state.player1.waitroom.cards.push(cid);
    }
    let hand_before = game.state.player1.hand.cards.len();
    trigger_auto(&mut game, sayaka, AbilityTrigger::Debut, "登場");
    game.assert_pending_choice_type("SelectCard", "fetch should ask which members");
    game.select_indices(&[0, 1]);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 2,
        "two fetched members join the hand"
    );
    let waitroom = &game.state.player1.waitroom.cards;
    assert!(
        !waitroom.contains(&cheap_a) && !waitroom.contains(&cheap_b),
        "fetched cards leave the waitroom"
    );
    assert!(
        waitroom.contains(&pricey),
        "cost-5 member exceeds コスト2以下 → stays"
    );
    assert!(
        waitroom.contains(&live),
        "live card is not a メンバー → stays"
    );
}
