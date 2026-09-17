use crate::helpers::*;

pub fn fill_both_main_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

pub fn trigger_printed_ability_and_resolve_choices(game: &mut TestGame, card_id: i16, trigger_str: &str) -> bool {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        if trigger_str == "登場" {
            rabuka_engine::core::types::AbilityTrigger::Debut
        } else if trigger_str == "ライブ開始時" {
            rabuka_engine::core::types::AbilityTrigger::LiveStart
        } else if trigger_str == "起動" {
            rabuka_engine::core::types::AbilityTrigger::Activation
        } else {
            rabuka_engine::core::types::AbilityTrigger::Auto
        },
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
    let mut offered = false;
    while game.has_pending_choice() {
        offered = true;
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            Some("SelectTarget") => {
                game.select_option(0);
            }
            Some("SelectCard") => {
                game.select_indices(&[0]);
            }
            Some("SelectPosition") => {
                game.select_indices(&[0]);
            }
            Some("SelectHeartColor") | Some("SelectHeartType") => {
                game.select_indices(&[0]);
            }
            _ => break,
        }
    }
    offered
}

pub fn decline_pending_choices_with_limit(game: &mut TestGame, max: usize) {
    for _ in 0..max {
        if !game.has_pending_choice() {
            return;
        }
        game.select_indices(&[]);
    }
    panic!("resolve_all_up_to_20: exceeded {} iters", max);
}
