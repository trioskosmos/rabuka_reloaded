use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_s_bp7_013_n_choose_self_recycles_members_preserves_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp7-013-N");
    game.add_to_stage(MemberArea::Center, dia);
    let m1 = game.id("PL!S-sd1-001-SD");
    let m2 = game.id("PL!S-sd1-001-SD");
    let life = game.id("PL!-sd1-019-SD");
    game.state.player1.waitroom.cards.push(m1);
    game.state.player1.waitroom.cards.push(life);
    game.state.player1.waitroom.cards.push(m2);
    fire_trigger(&mut game, dia, AbilityTrigger::Debut, "登場");
    assert!(game.has_pending_choice(), "自分/相手 player choice expected");
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(0),
        other => panic!("expected SelectTarget for player pick, got {other:?}"),
    }
    let mut picked = 0;
    while game.has_pending_choice() && picked < 4 {
        let idxs: Vec<usize> = game
            .state
            .player1
            .waitroom
            .cards
            .iter()
            .enumerate()
            .filter(|(_, &c)| game.db.get_card(c).is_some_and(|cc| matches!(cc.card_type, rabuka_engine::card::CardType::Member)))
            .map(|(i, _)| i)
            .collect();
        if idxs.is_empty() {
            break;
        }
        game.select_indices(&idxs);
        picked += 1;
    }
    assert_eq!(game.state.player1.main_deck.cards.last().copied(), Some(m2), "member cards recycled to the BOTTOM of the deck");
    assert_eq!(game.state.player1.main_deck.cards.len(), 2, "both members moved under each other");
    assert!(game.state.player1.waitroom.cards.contains(&life), "the live card was not a legal target and stayed");
    assert_eq!(game.state.player1.waitroom.cards.len(), 1);
}

#[test]
fn pl_s_bp7_013_n_choose_opponent_active_player_selects_recycled_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp7-013-N");
    game.add_to_stage(MemberArea::Center, dia);
    let m1 = game.id("PL!S-sd1-001-SD");
    let m2 = game.id("PL!S-sd1-001-SD");
    let life = game.id("PL!-sd1-019-SD");
    game.state.player1.waitroom.cards.push(m1);
    game.state.player1.waitroom.cards.push(life);
    game.state.player1.waitroom.cards.push(m2);
    let m3 = game.id("PL!S-sd1-001-SD");
    let m4 = game.id("PL!S-sd1-001-SD");
    let life2 = game.id("PL!-sd1-019-SD");
    game.state.player2.waitroom.cards.push(m3);
    game.state.player2.waitroom.cards.push(life2);
    game.state.player2.waitroom.cards.push(m4);
    fire_trigger(&mut game, dia, AbilityTrigger::Debut, "登場");
    assert!(game.has_pending_choice(), "自分/相手 player choice expected");
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(1),
        other => panic!("expected SelectTarget for player pick, got {other:?}"),
    }
    assert!(game.has_pending_choice(), "Card selection from opponent's waitroom expected");
    let entry = game.state.ability_queue.current_entry().expect("Queue entry");
    assert_eq!(entry.choice_player_id.as_deref(), Some("p1"), "BUG: choice should be routed to P1 (active player), not opponent");
    let mut picked = 0;
    while game.has_pending_choice() && picked < 4 {
        let filtered_idxs = match game.state.get_pending_choice() {
            Some(rabuka_engine::ability::types::Choice::SelectCard { filtered_indices: Some(idxs), .. }) => idxs.clone(),
            _ => break,
        };
        if filtered_idxs.is_empty() {
            break;
        }
        game.select_indices(&filtered_idxs);
        picked += 1;
    }
    assert_eq!(game.state.player2.main_deck.cards.len(), 2, "both members moved to P2's deck bottom");
    assert!(game.state.player2.waitroom.cards.contains(&life2), "the live card was not a legal target and stayed in P2's waitroom");
    assert_eq!(game.state.player2.waitroom.cards.len(), 1);
}
