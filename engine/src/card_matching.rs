use crate::card::CardDatabase;
use crate::player::Player;

/// Card matching system for exact name and group resolution (Q236/Q237)
pub struct CardMatchingSystem {
    pub exact_matching_enabled: bool,
    pub group_resolution_enabled: bool,
}

impl CardMatchingSystem {
    pub fn new() -> Self {
        Self {
            exact_matching_enabled: true,
            group_resolution_enabled: true,
        }
    }

    /// Check if card names match exactly (Q237 - no partial matching)
    pub fn exact_name_match(&self, target_name: &str, card_name: &str) -> bool {
        if !self.exact_matching_enabled {
            return true; // Disable exact matching if needed
        }
        
        // Exact match required - no partial matching allowed
        target_name == card_name
    }

    /// Check if card belongs to specified group (Q235 - 『虹ヶ咲』 resolution)
    pub fn group_match(&self, card_database: &CardDatabase, card_id: i16, group_name: &str) -> bool {
        if !self.group_resolution_enabled {
            return false;
        }

        if let Some(card) = card_database.get_card(card_id) {
            // Check if card's group field matches the target group
            // Also check series as fallback for cards that derive group from series
            let card_group = &card.group;
            let card_series = &card.series;
            
            // Match by exact group name, or check if group_name is contained in card's group/series
            return card_group == group_name 
                || card_group.contains(group_name)
                || card_series.contains(group_name);
        }
        false
    }

    /// Find cards by exact name match
    pub fn find_cards_by_exact_name(
        &self,
        player: &Player,
        card_database: &CardDatabase,
        target_name: &str,
        search_zones: &[&str],
    ) -> Vec<i16> {
        let mut matching_cards = Vec::new();

        for zone in search_zones {
            match *zone {
                "hand" => {
                    for &card_id in &player.hand.cards {
                        if let Some(card) = card_database.get_card(card_id) {
                            if self.exact_name_match(target_name, &card.name) {
                                matching_cards.push(card_id);
                            }
                        }
                    }
                }
                "stage" => {
                    // Use the correct Stage structure
                    if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::LeftSide) {
                        if let Some(card) = card_database.get_card(card_id) {
                            if self.exact_name_match(target_name, &card.name) {
                                matching_cards.push(card_id);
                            }
                        }
                    }
                    if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::Center) {
                        if let Some(card) = card_database.get_card(card_id) {
                            if self.exact_name_match(target_name, &card.name) {
                                matching_cards.push(card_id);
                            }
                        }
                    }
                    if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::RightSide) {
                        if let Some(card) = card_database.get_card(card_id) {
                            if self.exact_name_match(target_name, &card.name) {
                                matching_cards.push(card_id);
                            }
                        }
                    }
                }
                "discard" => {
                    for &card_id in &player.waitroom.cards {
                        if let Some(card) = card_database.get_card(card_id) {
                            if self.exact_name_match(target_name, &card.name) {
                                matching_cards.push(card_id);
                            }
                        }
                    }
                }
                _ => continue,
            };
        }

        matching_cards
    }

    /// Find cards by group name
    pub fn find_cards_by_group(
        &self,
        player: &Player,
        card_database: &CardDatabase,
        group_name: &str,
        search_zones: &[&str],
    ) -> Vec<i16> {
        let mut matching_cards = Vec::new();

        for zone in search_zones {
            match *zone {
                "hand" => {
                    for &card_id in &player.hand.cards {
                        if self.group_match(card_database, card_id, group_name) {
                            matching_cards.push(card_id);
                        }
                    }
                }
                "stage" => {
                    // Use the correct Stage structure
                    if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::LeftSide) {
                        if self.group_match(card_database, card_id, group_name) {
                            matching_cards.push(card_id);
                        }
                    }
                    if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::Center) {
                        if self.group_match(card_database, card_id, group_name) {
                            matching_cards.push(card_id);
                        }
                    }
                    if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::RightSide) {
                        if self.group_match(card_database, card_id, group_name) {
                            matching_cards.push(card_id);
                        }
                    }
                }
                "discard" => {
                    for &card_id in &player.waitroom.cards {
                        if self.group_match(card_database, card_id, group_name) {
                            matching_cards.push(card_id);
                        }
                    }
                }
                _ => continue,
            };
        }

        matching_cards
    }

    /// Validate deck size requirement (Q234 - need 3+ cards to pay cost)
    pub fn validate_deck_size(&self, player: &Player, required_size: usize) -> bool {
        player.main_deck.cards.len() >= required_size
    }

    /// Check if multiple triggers of same ability can stack (Q233)
    pub fn can_trigger_stack(&self, _ability_id: &str, current_trigger_count: u32) -> bool {
        // For now, allow unlimited stacking unless explicitly disabled
        // In a full implementation, this would check ability-specific rules
        current_trigger_count < 10 // Reasonable limit to prevent infinite loops
    }

    /// Determine who chooses position for opponent's cards (Q223)
    pub fn get_position_choice_player(&self, action_performer: &str, target_player: &str) -> String {
        // Rule: When ability makes opponent's member move, opponent chooses destination
        if action_performer != target_player {
            target_player.to_string()
        } else {
            action_performer.to_string()
        }
    }

    /// Handle deck bottom placement when deck is too small (Q226)
    pub fn handle_deck_bottom_placement(&self, player: &mut Player, cards: &[i16]) -> Result<(), String> {
        for &card_id in cards {
            // If deck is empty or too small, place at bottom
            player.main_deck.cards.push(card_id);
        }
        Ok(())
    }

    /// Calculate baton touch cost reduction (Q228)
    pub fn calculate_baton_touch_reduction(
        &self,
        player: &Player,
        card_database: &CardDatabase,
        cost_reduction_card_id: i16,
    ) -> Result<u32, String> {
        // Find the cost reduction card on stage
        let mut reduction = 0u32;

        // Check all stage areas for the cost reduction card
        if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::LeftSide) {
            if card_id == cost_reduction_card_id {
                if let Some(card) = card_database.get_card(cost_reduction_card_id) {
                    if let Some(cost) = &card.cost {
                        reduction = *cost; // cost is u32, not a struct with value field
                    }
                }
            }
        }
        if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::Center) {
            if card_id == cost_reduction_card_id {
                if let Some(card) = card_database.get_card(cost_reduction_card_id) {
                    if let Some(cost) = &card.cost {
                        reduction = *cost;
                    }
                }
            }
        }
        if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::RightSide) {
            if card_id == cost_reduction_card_id {
                if let Some(card) = card_database.get_card(cost_reduction_card_id) {
                    if let Some(cost) = &card.cost {
                        reduction = *cost;
                    }
                }
            }
        }

        Ok(reduction)
    }

    /// Calculate dynamic score with modifiers (Q231/Q232)
    pub fn calculate_final_score(
        &self,
        base_score: u32,
        score_modifiers: &[i32],
        cheer_score_bonus: u32,
    ) -> u32 {
        let mut final_score = base_score as i32;
        
        // Apply all modifiers
        for modifier in score_modifiers {
            final_score += modifier;
        }
        
        // Add cheer bonus
        final_score += cheer_score_bonus as i32;
        
        // Score cannot be negative
        if final_score < 0 {
            final_score = 0;
        }
        
        final_score as u32
    }

    /// Check if hand size is <= X for conditional effects (Q229)
    pub fn check_hand_size_condition(&self, player: &Player, max_size: usize) -> bool {
        player.hand.cards.len() <= max_size
    }

    /// Handle empty hand draws (Q229 - draw without discarding)
    pub fn handle_empty_hand_draw(&self, player: &mut Player, draw_count: usize) -> Result<(), String> {
        for _ in 0..draw_count {
            if let Some(card_id) = player.main_deck.cards.pop() {
                player.hand.add_card(card_id);
            } else {
                break; // Deck is empty
            }
        }
        Ok(())
    }
}
