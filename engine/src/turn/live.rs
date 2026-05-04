use crate::card::CardDatabase;
use crate::game_state::GameState;

impl super::TurnEngine {
    /// Compute the score modifier delta for cards in a specific zone since a snapshot.
    fn score_delta_since(current: &crate::mod_map::ModMap<i32>, snapshot: &crate::mod_map::ModMap<i32>, zone_cards: &[i16]) -> u32 {
        let mut total = 0i32;
        for &cid in zone_cards {
            let cur = current.get(cid).copied().unwrap_or(0);
            let prev = snapshot.get(cid).copied().unwrap_or(0);
            total += (cur - prev).max(0);
        }
        total as u32
    }

    pub fn execute_live_victory_determination(game_state: &mut GameState) {
        game_state.player1.stage_hearts = Some(game_state.player1.calculate_stage_hearts(&game_state.card_database));
        game_state.player2.stage_hearts = Some(game_state.player2.calculate_stage_hearts(&game_state.card_database));

        // Rule 11.6: LiveSuccess abilities trigger BEFORE winner determination (Q36)
        // Snapshot before each player's abilities so per-player score deltas are isolated
        let player1_id = game_state.player1.id.clone();
        let player2_id = game_state.player2.id.clone();
        let pre_p1 = game_state.score_modifiers.clone();
        Self::trigger_live_success_abilities(game_state, &player1_id);
        game_state.process_pending_auto_abilities(&player1_id);
        if game_state.pending_choice.is_some() { return; }
        let p1_extra = Self::score_delta_since(&game_state.score_modifiers, &pre_p1, &game_state.player1.live_card_zone.cards);

        let pre_p2 = game_state.score_modifiers.clone();
        Self::trigger_live_success_abilities(game_state, &player2_id);
        game_state.process_pending_auto_abilities(&player2_id);
        if game_state.pending_choice.is_some() { return; }
        let p2_extra = Self::score_delta_since(&game_state.score_modifiers, &pre_p2, &game_state.player2.live_card_zone.cards);

        // Determine winner (scores may have been modified by live_success abilities)
        let player1_score = game_state.player1.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player1_cheer_blade_heart_count, game_state.player1.stage_hearts.as_ref()) + p1_extra;
        let player2_score = game_state.player2.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player2_cheer_blade_heart_count, game_state.player2.stage_hearts.as_ref()) + p2_extra;
        let player1_has_cards = !game_state.player1.live_card_zone.cards.is_empty();
        let player2_has_cards = !game_state.player2.live_card_zone.cards.is_empty();

        eprintln!("LIVE_VICTORY: P1_cards={} P1_score={} P2_cards={} P2_score={}", player1_has_cards, player1_score, player2_has_cards, player2_score);

        let (player1_won, player2_won) = if !player1_has_cards && !player2_has_cards { (false, false) }
        else if player1_has_cards && !player2_has_cards { (true, false) }
        else if !player1_has_cards && player2_has_cards { (false, true) }
        else {
            (player1_score > player2_score, player2_score > player1_score)
        };

        eprintln!("LIVE_VICTORY_RESULT: P1_won={} P2_won={}", player1_won, player2_won);

        if player2_won { game_state.set_opponent_live_success(true); }

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
