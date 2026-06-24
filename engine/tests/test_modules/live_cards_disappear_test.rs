use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;

fn advance_to_live_card_set(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn count_all_cards(game: &TestGame) -> usize {
    let p1 = &game.state.player1;
    let p2 = &game.state.player2;
    let mut total = 0;
    total += p1.hand.cards.len();
    total += p1.main_deck.cards.len();
    total += p1.energy_zone.cards.len();
    total += p1.stage.stage.iter().filter(|&&c| c != -1).count();
    total += p1.live_card_zone.cards.len();
    total += p1.success_live_card_zone.cards.len();
    total += p1.waitroom.cards.len();
    total += p2.hand.cards.len();
    total += p2.main_deck.cards.len();
    total += p2.energy_zone.cards.len();
    total += p2.stage.stage.iter().filter(|&&c| c != -1).count();
    total += p2.live_card_zone.cards.len();
    total += p2.success_live_card_zone.cards.len();
    total += p2.waitroom.cards.len();
    total += game.state.resolution_zone.cards.len();
    total += game.state.revealed_cards.len();
    total += game.state.revealed_cost_cards.len();
    total += game.state.looked_at_cards.len();
    total as usize
}

#[test]
fn live_cards_stuck_in_live_zone_instead_of_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live1 = game.id("PL!SP-sd1-023-SD");
    let live2 = game.id("PL!SP-sd1-023-SD");
    let live3 = game.id("PL!SP-sd1-023-SD");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(live1);
    game.state.player1.hand.cards.push(live2);
    game.state.player1.hand.cards.push(live3);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player2.stage.stage = [-1, -1, -1];

    let total_before = count_all_cards(&game);
    eprintln!("total cards before: {}", total_before);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live1);
    game.set_live_card(live2);
    game.set_live_card(live3);

    let total_after_set = count_all_cards(&game);
    eprintln!(
        "total after set: {} hand:{} live:{} wait:{} deck:{} res:{}",
        total_after_set,
        game.state.player1.hand.cards.len(),
        game.state.player1.live_card_zone.cards.len(),
        game.state.player1.waitroom.cards.len(),
        game.state.player1.main_deck.cards.len(),
        game.state.resolution_zone.cards.len()
    );

    assert_eq!(game.state.player1.live_card_zone.cards.len(), 3);
    assert!(game.state.player1.waitroom.cards.is_empty());

    game.pass();
    game.pass();

    eprintln!("phase: {:?}", game.state.current_phase);

    let mut safety = 20;
    while game.has_pending_choice() && safety > 0 {
        let ct = game.pending_choice_type().unwrap_or_default();
        eprintln!("choice: {}", ct);
        if ct == "SelectAutoAbility" {
            game.select_indices(&[]);
        } else if ct == "SelectLiveSuccess" {
            TurnEngine::resume_with_choice(&mut game.state, None, Some(vec![0]))
                .expect("resume live success failed");
        } else {
            break;
        }
        safety -= 1;
    }

    for _ in 0..10 {
        if game.state.current_phase.to_string() == "Main"
            || game.state.current_phase.to_string() == "Active"
        {
            break;
        }
        if !game.has_pending_choice() {
            game.pass();
        }
        while game.has_pending_choice() && safety > 0 {
            let ct = game.pending_choice_type().unwrap_or_default();
            if ct == "SelectAutoAbility" {
                game.select_indices(&[]);
            } else if ct == "SelectLiveSuccess" {
                TurnEngine::resume_with_choice(&mut game.state, None, Some(vec![0]))
                    .expect("resume live success failed");
            } else {
                break;
            }
            safety -= 1;
        }
    }

    let total_after = count_all_cards(&game);
    eprintln!(
        "total after: {} hand:{} live:{} wait:{} deck:{} succ:{} res:{} rev:{}",
        total_after,
        game.state.player1.hand.cards.len(),
        game.state.player1.live_card_zone.cards.len(),
        game.state.player1.waitroom.cards.len(),
        game.state.player1.main_deck.cards.len(),
        game.state.player1.success_live_card_zone.cards.len(),
        game.state.resolution_zone.cards.len(),
        game.state.revealed_cards.len()
    );

    assert_eq!(
        total_before,
        total_after,
        "CARDS VANISHED - {} cards lost!",
        total_before - total_after
    );
    assert!(
        game.state.player1.waitroom.cards.len() == 3
            || game.state.player1.live_card_zone.cards.len() == 3,
        "3 live cards should be in waitroom or live_zone, got waitroom={} live_zone={}",
        game.state.player1.waitroom.cards.len(),
        game.state.player1.live_card_zone.cards.len()
    );
}
