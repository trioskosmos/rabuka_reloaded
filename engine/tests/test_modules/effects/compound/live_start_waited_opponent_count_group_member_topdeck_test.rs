use crate::helpers::*;

const KARIN: &str = "PL!N-bp4-004-R\u{ff0b}";
const FILLER: &str = "PL!-sd1-010-SD";

fn trigger_live_start_all(game: &mut TestGame) {
    let pid = game.state.player1.id.clone();
    let stage_cards: Vec<i16> = game.state.player1.stage.stage.iter().copied().filter(|&c| c != -1).collect();
    for card_id in stage_cards {
        let card = match game.db.get_card(card_id) {
            Some(c) => c,
            None => continue,
        };
        for ab in card.resolved_abilities() {
            if ab.triggers.as_deref() == Some("ライブ開始時") {
                game.state.trigger_auto_ability(
                    format!("{}_{}", card.card_no, ab.full_text),
                    rabuka_engine::core::types::AbilityTrigger::LiveStart,
                    pid.clone(),
                    Some(card.card_no.to_string()),
                    Some(card_id),
                    None,
                    None,
                );
            }
        }
    }
    game.state.process_pending_auto_abilities(&pid);
}

fn resolve_all_choices(game: &mut TestGame) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_indices(&[]);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }
}

#[test]
fn live_start_waited_opponent_group_topdeck_increases_deck_with_two_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id(KARIN);
    let nijigasaku_member = game.id("PL!N-bp1-012-R\u{ff0b}"); // 鐘嵐珠, 虹ヶ咲

    game.state.player1.stage.stage = [karin, -1, -1];

    // Manually set 2 opponent members to Wait state
    let opp1 = game.id(FILLER);
    let opp2 = game.id(FILLER);
    game.state.player2.stage.stage = [opp1, opp2, -1];
    game.state.mods.add_orientation_modifier(opp1, "wait");
    game.state.mods.add_orientation_modifier(opp2, "wait");

    // Put 虹ヶ咲 members in discard
    game.state.player1.waitroom.cards.push(nijigasaku_member);
    game.state.player1.waitroom.cards.push(game.id("PL!N-bp1-012-R\u{ff0b}"));
    // Also add non-虹ヶ咲 filler to discard (should be ignored)
    game.state.player1.waitroom.cards.push(game.id(FILLER));

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }

    let deck_before = game.state.player1.main_deck.cards.len();

    trigger_live_start_all(&mut game);
    resolve_all_choices(&mut game);

    // ab#1: should have placed 虹ヶ咲 cards on deck top
    // dynamic_count = 2 (two wait members), so up to 2 虹ヶ咲 members selected
    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_after > deck_before,
        "Deck should have gained cards from ab#1: before={}, after={}",
        deck_before,
        deck_after
    );
}

#[test]
fn live_start_waited_opponent_group_topdeck_initially_active_opponent_net_deck_unchanged() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id(KARIN);

    game.state.player1.stage.stage = [karin, -1, -1];

    // No wait members on opponent stage
    let opp = game.id(FILLER);
    game.state.player2.stage.stage = [opp, -1, -1];

    // Put 虹ヶ咲 members in discard
    game.state.player1.waitroom.cards.push(game.id("PL!N-bp1-012-R\u{ff0b}"));

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }

    let deck_before = game.state.player1.main_deck.cards.len();

    trigger_live_start_all(&mut game);
    resolve_all_choices(&mut game);

    // ab#1: dynamic_count=0, so no cards should be placed on deck
    let deck_after = game.state.player1.main_deck.cards.len();
    assert_eq!(
        deck_before, deck_after,
        "No cards should be placed on deck when 0 wait members: before={}, after={}",
        deck_before, deck_after
    );
}

#[test]
fn live_start_waited_opponent_group_topdeck_single_eligible_loses_at_most_one_deck_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id(KARIN);

    game.state.player1.stage.stage = [karin, -1, -1];

    // Set 3 opponent members to Wait via ab#0's change_state effect
    // (use the draw+wait ability to naturally create wait state)
    let opp1 = game.id(FILLER);
    let opp2 = game.id(FILLER);
    let opp3 = game.id(FILLER);
    game.state.player2.stage.stage = [opp1, opp2, opp3];

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }

    // Put 1 虹ヶ咲 member in discard
    game.state.player1.waitroom.cards.push(game.id("PL!N-bp1-012-R\u{ff0b}"));
    // Add non-eligible fillers
    for _ in 0..3 {
        game.state.player1.waitroom.cards.push(game.id(FILLER));
    }

    let deck_before = game.state.player1.main_deck.cards.len();

    trigger_live_start_all(&mut game);
    resolve_all_choices(&mut game);

    // ab#1: dynamic_count should pick up the wait members from ab#0
    // At minimum, ab#0 should have drawn 1 card and waited at least 1 member
    let deck_after = game.state.player1.main_deck.cards.len();
    // deck lost 1 from draw; may or may not gain from ab#1
    assert!(
        deck_after >= deck_before - 1,
        "Deck should not lose more than draw: before={}, after={}",
        deck_before,
        deck_after
    );
}
