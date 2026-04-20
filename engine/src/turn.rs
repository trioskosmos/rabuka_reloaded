use crate::game_state::{GameState, Phase, TurnPhase, GameResult};
use crate::zones::MemberArea;

pub struct TurnEngine;

impl TurnEngine {
    pub fn advance_phase(game_state: &mut GameState) {
        // Advance phase according to rules 7.1.2, 7.3.3, and 8.1.2
        let current_phase = game_state.current_phase.clone();
        let current_turn_phase = game_state.current_turn_phase.clone();
        
        // Handle normal phase sub-phases (Rule 7.3.3)
        if current_turn_phase == TurnPhase::FirstAttackerNormal || current_turn_phase == TurnPhase::SecondAttackerNormal {
            match current_phase {
                Phase::Active => {
                    // Rule 7.4: Activate all energy and stage cards (automatic)
                    game_state.active_player_mut().activate_all_energy();
                    for area in [MemberArea::LeftSide, MemberArea::Center, MemberArea::RightSide] {
                        if let Some(card) = game_state.active_player_mut().stage.get_area_mut(area) {
                            if card.orientation == Some(crate::zones::Orientation::Wait) {
                                card.orientation = Some(crate::zones::Orientation::Active);
                            }
                        }
                    }
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Energy;
                }
                Phase::Energy => {
                    // Rule 7.5: Draw energy card (automatic)
                    Self::check_timing(game_state);
                    game_state.active_player_mut().draw_energy();
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Draw;
                }
                Phase::Draw => {
                    // Rule 7.6: Draw card (automatic)
                    Self::check_timing(game_state);
                    game_state.active_player_mut().draw_card();
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Main;
                }
                Phase::Main => {
                    // Rule 7.7: Main phase complete, advance to next turn phase
                    Self::check_timing(game_state);
                    if current_turn_phase == TurnPhase::FirstAttackerNormal {
                        game_state.current_turn_phase = TurnPhase::SecondAttackerNormal;
                        game_state.current_phase = Phase::Active;
                    } else {
                        game_state.current_turn_phase = TurnPhase::Live;
                        game_state.current_phase = Phase::LiveCardSet;
                    }
                }
                _ => {}
            }
        }
        // Handle live phase sub-phases (Rule 8.1.2)
        else if current_turn_phase == TurnPhase::Live {
            match current_phase {
                Phase::LiveCardSet => {
                    // Rule 8.2: Both players set live cards (automatic)
                    Self::check_timing(game_state);
                    // First attacker sets live cards
                    Self::player_set_live_cards(game_state.first_attacker_mut());
                    Self::check_timing(game_state);
                    // Second attacker sets live cards
                    Self::player_set_live_cards(game_state.second_attacker_mut());
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::FirstAttackerPerformance;
                }
                Phase::FirstAttackerPerformance => {
                    // Rule 8.3: First attacker performs (automatic)
                    Self::check_timing(game_state);
                    let blade_heart_count = {
                        // Take resolution_zone first to avoid borrow conflicts
                        let mut resolution_zone = std::mem::take(&mut game_state.resolution_zone);
                        let player = game_state.first_attacker_mut();
                        let result = Self::player_perform_live(player, &mut resolution_zone);
                        // Put resolution_zone back
                        game_state.resolution_zone = resolution_zone;
                        result
                    };
                    game_state.player1_cheer_blade_heart_count = blade_heart_count;
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::SecondAttackerPerformance;
                }
                Phase::SecondAttackerPerformance => {
                    // Rule 8.3: Second attacker performs (automatic)
                    Self::check_timing(game_state);
                    let blade_heart_count = {
                        // Take resolution_zone first to avoid borrow conflicts
                        let mut resolution_zone = std::mem::take(&mut game_state.resolution_zone);
                        let player = game_state.second_attacker_mut();
                        let result = Self::player_perform_live(player, &mut resolution_zone);
                        // Put resolution_zone back
                        game_state.resolution_zone = resolution_zone;
                        result
                    };
                    game_state.player2_cheer_blade_heart_count = blade_heart_count;
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::LiveVictoryDetermination;
                }
                Phase::LiveVictoryDetermination => {
                    // Rule 8.4: Determine live victory (automatic)
                    Self::check_timing(game_state);
                    Self::execute_live_victory_determination(game_state);
                }
                _ => {}
            }
        }
    }

    pub fn execute_main_phase_action(game_state: &mut GameState, action: &str) {
        // Execute player choice action during MAIN phase
        match action {
            "play_member_left" => {
                // TODO: Implement playing member to left side
            }
            "play_member_center" => {
                // TODO: Implement playing member to center
            }
            "play_member_right" => {
                // TODO: Implement playing member to right side
            }
            "play_energy" => {
                // TODO: Implement playing energy
            }
            "pass" => {
                // Player passes, advance phase
                Self::advance_phase(game_state);
            }
            _ => {}
        }
    }

    fn execute_live_victory_determination(game_state: &mut GameState) {
        // Rule 8.4: Determine live victory
        // Rule 8.4.2.1: Add cheer blade heart count to score
        let player1_score = game_state.player1.live_card_zone.calculate_live_score(game_state.player1_cheer_blade_heart_count);
        let player2_score = game_state.player2.live_card_zone.calculate_live_score(game_state.player2_cheer_blade_heart_count);
        let player1_has_cards = !game_state.player1.live_card_zone.cards.is_empty();
        let player2_has_cards = !game_state.player2.live_card_zone.cards.is_empty();
        
        let mut player1_won = false;
        let mut player2_won = false;
        
        // Rule 8.4.6.1: If both players have no cards, no one wins
        if !player1_has_cards && !player2_has_cards {
            // No winner, keep current first attacker
        }
        // Rule 8.4.6.2: Compare scores
        else if player1_has_cards && !player2_has_cards {
            player1_won = true;
        } else if !player1_has_cards && player2_has_cards {
            player2_won = true;
        } else {
            // Both have cards, compare scores
            if player1_score > player2_score {
                player1_won = true;
            } else if player2_score > player1_score {
                player2_won = true;
            } else {
                // Tie, both win
                player1_won = true;
                player2_won = true;
            }
        }
        
        // Rule 8.4.4: Live success event for players with cards
        if player1_has_cards {
            // TODO: Trigger LiveSuccess abilities for player1
        }
        if player2_has_cards {
            // TODO: Trigger LiveSuccess abilities for player2
        }
        
        // Rule 8.4.7: Move winning live card to success zone
        if player1_won && player2_won {
            // Both won - check if either has 2 cards
            if game_state.player1.live_card_zone.cards.len() == 2 {
                // Player1 has 2 cards, doesn't move
            } else {
                Self::move_live_to_success(&mut game_state.player1);
            }
            if game_state.player2.live_card_zone.cards.len() == 2 {
                // Player2 has 2 cards, doesn't move
            } else {
                Self::move_live_to_success(&mut game_state.player2);
            }
        } else if player1_won {
            Self::move_live_to_success(&mut game_state.player1);
        } else if player2_won {
            Self::move_live_to_success(&mut game_state.player2);
        }
        
        // Rule 8.4.8: Move remaining live cards and cheer cards to discard
        for card in game_state.player1.live_card_zone.clear() {
            game_state.player1.waitroom.cards.push(card);
        }
        for card in game_state.player2.live_card_zone.clear() {
            game_state.player2.waitroom.cards.push(card);
        }
        
        // Rule 8.4.9-8.4.12: Turn end trigger loop
        loop {
            // Rule 8.4.9: Check timing
            Self::check_timing(game_state);
            
            // Rule 8.4.10: Trigger 'turn end' abilities that haven't triggered yet
            // TODO: Implement automatic ability triggering - track which abilities have triggered this turn
            
            // Rule 8.4.11: Check timing again
            Self::check_timing(game_state);
            
            // Rule 8.4.11: End 'until end of turn' and 'during this turn' effects
            // TODO: Implement effect expiration - track active effects and their durations
            
            // Rule 8.4.12: Loop back to 8.4.9 if new abilities triggered
            // TODO: Implement loop detection - check if any new abilities were triggered in this iteration
            // For now, break the loop to prevent infinite loop
            break;
        }
        
        // Rule 8.4.13: Winner becomes first attacker next turn
        if player1_won && !player2_won {
            game_state.player1.is_first_attacker = true;
            game_state.player2.is_first_attacker = false;
        } else if player2_won && !player1_won {
            game_state.player1.is_first_attacker = false;
            game_state.player2.is_first_attacker = true;
        }
        // If both won or no one won, keep current first attacker
        
        // End turn
        game_state.turn_number += 1;
        game_state.reset_keyword_tracking();
        game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
        game_state.current_phase = Phase::Active;
    }

    fn move_live_to_success(player: &mut crate::player::Player) {
        // Move top card from live card zone to success live card zone
        if let Some(top_card) = player.live_card_zone.remove_top_card() {
            player.success_live_card_zone.cards.push(top_card);
        }
    }

    fn move_live_to_exclusion(player: &mut crate::player::Player) {
        // Move all live cards to exclusion zone
        for card in player.live_card_zone.clear() {
            player.exclusion_zone.cards.push(crate::zones::CardInZone {
                card: card,
                orientation: Some(crate::zones::Orientation::Wait),
                face_state: crate::zones::FaceState::FaceDown,
                energy_underneath: Vec::new(),
            });
        }
    }

    fn check_timing(game_state: &mut GameState) {
        // Rule 9.5: Check timing - process rule processing per rules 10.2-10.6
        
        // Rule 10.2: Refresh (already handled in player.refresh())
        game_state.player1.refresh();
        game_state.player2.refresh();
        
        // Rule 10.4: Check for duplicate members
        Self::check_duplicate_members(&mut game_state.player1);
        Self::check_duplicate_members(&mut game_state.player2);
        
        // Rule 10.5: Check for invalid cards
        Self::check_invalid_cards(&mut game_state.player1);
        Self::check_invalid_cards(&mut game_state.player2);
        
        // Rule 10.6: Check for invalid resolution zone
        Self::check_invalid_resolution_zone(game_state);
    }

    fn check_duplicate_members(player: &mut crate::player::Player) {
        // Rule 10.4: Check for duplicate members in same area
        // Rule 10.4.1: If multiple members in one area, keep the last one, send others to discard
        let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
        
        for area in areas {
            if let Some(card_in_zone) = player.stage.get_area_mut(area) {
                // Check if there are energy cards underneath (indicating multiple cards)
                if !card_in_zone.energy_underneath.is_empty() {
                    // Keep only the top card (the last one placed)
                    // Move all energy-underneath cards to discard
                    for energy_card in card_in_zone.energy_underneath.drain(..) {
                        player.waitroom.cards.push(energy_card);
                    }
                }
            }
        }
    }

    fn check_invalid_cards(player: &mut crate::player::Player) {
        // Rule 10.5: Check for invalid cards in zones
        // Rule 10.5.1: Non-live cards in live card zone
        player.live_card_zone.cards.retain(|c| c.is_live());
        
        // Rule 10.5.2: Non-energy cards in energy zone
        player.energy_zone.cards.retain(|c| c.card.is_energy());
        
        // Rule 10.5.3: Energy cards without member above in member area
        // Check each member area
        for area in [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide] {
            if let Some(card_in_zone) = player.stage.get_area_mut(area) {
                // If no member above, move energy cards to energy deck
                if card_in_zone.card.is_energy() {
                    let energy_card = card_in_zone.card.clone();
                    *card_in_zone = crate::zones::CardInZone {
                        card: crate::card::Card {
                            card_no: String::new(),
                            img: None,
                            name: String::new(),
                            product: String::new(),
                            card_type: crate::card::CardType::Energy,
                            series: String::new(),
                            unit: None,
                            cost: None,
                            base_heart: None,
                            blade_heart: None,
                            blade: 0,
                            rare: String::new(),
                            ability: String::new(),
                            faq: Vec::new(),
                            _img: None,
                            score: None,
                            need_heart: None,
                            special_heart: None,
                            abilities: Vec::new(),
                        },
                        orientation: None,
                        face_state: crate::zones::FaceState::FaceDown,
                        energy_underneath: Vec::new(),
                    };
                    // Move to energy deck
                    player.energy_deck.cards.push_back(energy_card);
                }
            }
        }
    }

    fn check_invalid_resolution_zone(_game_state: &mut GameState) {
        // Rule 10.6: Invalid resolution zone processing
        // Rule 10.6.1: Cards in resolution zone that are not being played/resolved/cheered go to discard
        // Simplified: For now, assume all cards in resolution zone should be moved to discard
        // after they're done being processed
        // This would need to track which cards are currently being played/resolved
    }

    fn player_set_live_cards(player: &mut crate::player::Player) {
        // Rule 8.2: Player sets live cards face-down and draws equal amount
        // Simplified: AI chooses up to 3 live cards to set
        let live_cards: Vec<_> = player.hand.cards.iter()
            .filter(|c| c.is_live())
            .cloned()
            .collect();
        
        if live_cards.is_empty() {
            return;
        }
        
        // Set up to 3 cards
        let cards_to_set = std::cmp::min(3, live_cards.len());
        for i in 0..cards_to_set {
            let card = live_cards[i].clone();
            // Remove from hand
            if let Some(pos) = player.hand.cards.iter().position(|c| c.card_no == card.card_no) {
                player.hand.cards.remove(pos);
            }
            // Add to live card zone face-down
            let _ = player.live_card_zone.add_card(card, true);
        }
        
        // Draw equal amount
        for _ in 0..cards_to_set {
            let _ = player.draw_card();
        }
    }

    fn player_perform_live(player: &mut crate::player::Player, resolution_zone: &mut crate::zones::ResolutionZone) -> u32 {
        // Rule 8.3: Player performs live - check heart requirements
        // Rule 8.3.4: Reveal cards, discard non-live cards
        player.live_card_zone.cards.retain(|c| c.is_live());
        
        // Rule 8.3.6: If no live cards, end performance
        if player.live_card_zone.cards.is_empty() {
            return 0;
        }
        
        // Rule 8.3.7: Live start event
        // TODO: Trigger LiveStart abilities
        
        // Rule 8.3.10: Calculate total blades from active members
        let total_blades = player.stage.total_blades();
        
        // Rule 8.3.11: エール (cheer) - move cards from main deck to resolution zone
        for _ in 0..total_blades {
            if let Some(card) = player.main_deck.cards.pop_front() {
                resolution_zone.cards.push(card);
            }
        }
        
        // Rule 8.3.12: Check blade hearts on cards in resolution zone
        let mut blade_heart_count = 0;
        for card in &resolution_zone.cards {
            if let Some(ref blade_heart) = card.blade_heart {
                blade_heart_count += blade_heart.hearts.values().sum::<u32>();
            }
        }
        
        // Rule 8.3.12.1: Draw cards based on blade heart count
        for _ in 0..blade_heart_count {
            let _ = player.draw_card();
        }
        
        // Rule 8.3.14: Calculate live-owned hearts from stage and blade hearts
        let stage_hearts = player.stage.get_available_hearts();
        let mut live_owned_hearts = stage_hearts.clone();
        
        // Add blade hearts from resolution zone
        for card in &resolution_zone.cards {
            if let Some(ref blade_heart) = card.blade_heart {
                for (color, count) in &blade_heart.hearts {
                    *live_owned_hearts.hearts.entry(color.clone()).or_insert(0) += count;
                }
            }
        }
        
        // Rule 8.3.15: Check if each live card can satisfy required hearts
        let mut remaining_hearts = live_owned_hearts.clone();
        let mut live_cards_to_remove = Vec::new();
        
        for card in &player.live_card_zone.cards {
            if let Some(ref need_heart) = card.need_heart {
                let can_satisfy = need_heart.hearts.iter().all(|(color, needed)| {
                    remaining_hearts.hearts.get(color).unwrap_or(&0) >= needed
                });
                
                if can_satisfy {
                    // Consume hearts
                    for (color, needed) in &need_heart.hearts {
                        if let Some(count) = remaining_hearts.hearts.get_mut(color) {
                            *count -= needed;
                        }
                    }
                } else {
                    live_cards_to_remove.push(card.clone());
                }
            }
        }
        
        // Rule 8.3.16: If any fails, all live cards go to discard
        if !live_cards_to_remove.is_empty() {
            // Move all live cards to discard
            for card in player.live_card_zone.clear() {
                player.waitroom.cards.push(card);
            }
        }
        
        // Move resolution zone cards to discard
        for card in resolution_zone.cards.drain(..) {
            player.waitroom.cards.push(card);
        }
        
        blade_heart_count
    }
}
