use crate::card::CardDatabase;
use crate::game_state::GameState;

impl super::TurnEngine {
    pub fn execute_live_victory_determination(game_state: &mut GameState) {
        let player1_stage_hearts = game_state.player1.calculate_stage_hearts(&game_state.card_database);
        let player2_stage_hearts = game_state.player2.calculate_stage_hearts(&game_state.card_database);
        game_state.player1.stage_hearts = Some(player1_stage_hearts);
        game_state.player2.stage_hearts = Some(player2_stage_hearts);

        let player1_score = game_state.player1.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player1_cheer_blade_heart_count, game_state.player1.stage_hearts.as_ref());
        let player2_score = game_state.player2.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player2_cheer_blade_heart_count, game_state.player2.stage_hearts.as_ref());
        let player1_has_cards = !game_state.player1.live_card_zone.cards.is_empty();
        let player2_has_cards = !game_state.player2.live_card_zone.cards.is_empty();

        let mut player1_won = false;
        let mut player2_won = false;

        if !player1_has_cards && !player2_has_cards {}
        else if player1_has_cards && !player2_has_cards { player1_won = true; }
        else if !player1_has_cards && player2_has_cards { player2_won = true; }
        else {
            if player1_score > player2_score { player1_won = true; }
            else if player2_score > player1_score { player2_won = true; }
        }

        if player2_won { game_state.set_opponent_live_success(true); }

        let player1_id = game_state.player1.id.clone();
        let player2_id = game_state.player2.id.clone();

        if player1_won { Self::trigger_live_success_abilities(game_state, &player1_id);
            game_state.process_pending_auto_abilities(&player1_id); }
        if player2_won { Self::trigger_live_success_abilities(game_state, &player2_id);
            game_state.process_pending_auto_abilities(&player2_id); }

        if game_state.pending_choice.is_some() { return; }

        let card_db = game_state.card_database.clone();
        Self::move_restricted_cards_to_discard(&mut game_state.player1, &card_db);
        Self::move_restricted_cards_to_discard(&mut game_state.player2, &card_db);
        Self::move_live_to_success_and_handle_wins(game_state, player1_won, player2_won);
    }

    fn move_restricted_cards_to_discard(player: &mut crate::player::Player, card_db: &CardDatabase) {
        let mut cards_to_remove = Vec::new();
        for (index, card_id) in player.live_card_zone.cards.iter().enumerate() {
            if let Some(card) = card_db.get_card(*card_id) {
                let has_restriction = card.abilities.iter().any(|ability| {
                    if let Some(ref effect) = ability.effect {
                        let restricted_dest = effect.restricted_destination.as_deref().or_else(|| effect.destination.as_deref());
                        effect.action == "restriction" && effect.restriction_type.as_deref() == Some("cannot_place")
                            && (restricted_dest == Some("success_live_zone") || restricted_dest == Some("live_card_zone"))
                    } else { false }
                });
                if has_restriction { cards_to_remove.push(index); }
            }
        }
        for &idx in cards_to_remove.iter().rev() {
            if idx < player.live_card_zone.cards.len() {
                let card_id = player.live_card_zone.cards.remove(idx);
                player.waitroom.add_card(card_id);
            }
        }
    }

    fn move_live_to_success(player: &mut crate::player::Player, card_index: usize, _card_db: &CardDatabase) {
        let card_id = player.live_card_zone.cards[card_index];
        player.live_card_zone.cards.remove(card_index);
        player.success_live_card_zone.cards.push(card_id);
    }

    fn move_live_to_success_and_handle_wins(game_state: &mut GameState, player1_won: bool, player2_won: bool) {
        let card_db = game_state.card_database.clone();
        let p1_cards = game_state.player1.live_card_zone.cards.len();
        let p2_cards = game_state.player2.live_card_zone.cards.len();

        if player1_won {
            for i in (0..p1_cards).rev() { Self::move_live_to_success(&mut game_state.player1, i, &card_db); }
        } else {
            for i in (0..p1_cards).rev() { game_state.player1.waitroom.add_card(game_state.player1.live_card_zone.cards.remove(i)); }
        }
        if player2_won {
            for i in (0..p2_cards).rev() { Self::move_live_to_success(&mut game_state.player2, i, &card_db); }
        } else {
            for i in (0..p2_cards).rev() { game_state.player2.waitroom.add_card(game_state.player2.live_card_zone.cards.remove(i)); }
        }
    }

    pub fn player_perform_live(
        player: &mut crate::player::Player,
        _resolution_zone: &mut crate::zones::ResolutionZone,
        _player_id: &str, card_db: &CardDatabase,
        _blade_modifiers: &crate::mod_map::ModMap<i32>,
        _heart_override: &std::collections::HashMap<i16, (crate::card::HeartColor, u32)>,
        _heart_modifiers: &std::collections::HashMap<i16, std::collections::HashMap<crate::card::HeartColor, i32>>,
    ) -> u32 {
        let stage_hearts = player.calculate_stage_hearts(card_db);
        let total_score = player.live_card_zone.calculate_live_score(card_db, 0, Some(&stage_hearts));
        println!("Live performance for {}: score={}", player.name, total_score);
        0
    }
}
