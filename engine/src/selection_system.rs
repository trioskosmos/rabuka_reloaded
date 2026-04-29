use crate::card::CardDatabase;
use crate::game_state::GameState;
use crate::player::Player;

/// Selection system for player choices (Rule 9.6.3)
pub struct SelectionSystem {
    pub pending_choice: Option<PendingChoice>,
    pub choice_history: Vec<CompletedChoice>,
}

#[derive(Debug, Clone)]
pub struct PendingChoice {
    pub choice_type: ChoiceType,
    pub description: String,
    pub min_count: usize,
    pub max_count: usize,
    pub target_zone: String,
    pub card_type_filter: Option<String>,
    pub player_id: String,
}

#[derive(Debug, Clone)]
pub enum ChoiceType {
    SelectCards,           // Select specific cards
    SelectNumber,          // Choose a number
    SelectPlayer,          // Choose a player
    SelectOption,          // Choose from options
    SelectHeartColor,      // Choose heart color
    SelectPosition,        // Choose position
}

#[derive(Debug, Clone)]
pub struct CompletedChoice {
    pub choice_type: ChoiceType,
    pub player_id: String,
    pub selected_cards: Vec<i16>,
    pub selected_number: Option<u32>,
    pub selected_option: Option<String>,
    pub timestamp: u32,
}

impl SelectionSystem {
    pub fn new() -> Self {
        Self {
            pending_choice: None,
            choice_history: Vec::new(),
        }
    }

    /// Request a card selection from player (Rule 9.6.3)
    pub fn request_card_selection(
        &mut self,
        player_id: String,
        description: String,
        target_zone: String,
        min_count: usize,
        max_count: usize,
        card_type_filter: Option<String>,
    ) -> Result<(), String> {
        if self.pending_choice.is_some() {
            return Err("Another choice is already pending".to_string());
        }

        self.pending_choice = Some(PendingChoice {
            choice_type: ChoiceType::SelectCards,
            description,
            min_count,
            max_count,
            target_zone,
            card_type_filter,
            player_id,
        });

        Ok(())
    }

    /// Request a number choice from player
    pub fn request_number_choice(
        &mut self,
        player_id: String,
        description: String,
        min_value: u32,
        max_value: u32,
    ) -> Result<(), String> {
        if self.pending_choice.is_some() {
            return Err("Another choice is already pending".to_string());
        }

        self.pending_choice = Some(PendingChoice {
            choice_type: ChoiceType::SelectNumber,
            description,
            min_count: min_value as usize,
            max_count: max_value as usize,
            target_zone: String::new(),
            card_type_filter: None,
            player_id,
        });

        Ok(())
    }

    /// Request an option choice from player
    pub fn request_option_choice(
        &mut self,
        player_id: String,
        description: String,
        _options: Vec<String>,
    ) -> Result<(), String> {
        if self.pending_choice.is_some() {
            return Err("Another choice is already pending".to_string());
        }

        self.pending_choice = Some(PendingChoice {
            choice_type: ChoiceType::SelectOption,
            description,
            min_count: 1,
            max_count: 1,
            target_zone: format!("Options: {:?}", _options),
            card_type_filter: None,
            player_id,
        });

        Ok(())
    }

    /// Provide choice result from player
    pub fn provide_choice_result(
        &mut self,
        player_id: &str,
        selected_cards: Vec<i16>,
        selected_number: Option<u32>,
        selected_option: Option<String>,
    ) -> Result<(), String> {
        let choice = self.pending_choice.take()
            .ok_or("No pending choice to resolve")?;

        if choice.player_id != player_id {
            return Err("Wrong player providing choice".to_string());
        }

        // Validate the choice
        self.validate_choice(&choice, &selected_cards, selected_number, &selected_option)?;

        // Record the completed choice
        self.choice_history.push(CompletedChoice {
            choice_type: choice.choice_type.clone(),
            player_id: player_id.to_string(),
            selected_cards: selected_cards.clone(),
            selected_number,
            selected_option: selected_option.clone(),
            timestamp: 0, // TODO: Add actual timestamp
        });

        println!("Choice completed: {} for player {}", choice.description, player_id);

        Ok(())
    }

    /// Validate that the choice meets requirements (Rule 9.6.3)
    fn validate_choice(
        &self,
        choice: &PendingChoice,
        selected_cards: &[i16],
        selected_number: Option<u32>,
        selected_option: &Option<String>,
    ) -> Result<(), String> {
        match choice.choice_type {
            ChoiceType::SelectCards => {
                if selected_cards.len() < choice.min_count {
                    return Err(format!("Must select at least {} cards", choice.min_count));
                }
                if selected_cards.len() > choice.max_count {
                    return Err(format!("Cannot select more than {} cards", choice.max_count));
                }
                // Validate that selected cards exist in the target zone
                let valid_indices: Vec<i16> = selected_cards.iter()
                    .filter(|&&idx| idx < choice.max_count as i16)
                    .copied()
                    .collect();
                
                if valid_indices.len() != selected_cards.len() {
                    return Err("Some selected cards are out of range".to_string());
                }
            }
            ChoiceType::SelectNumber => {
                if let Some(number) = selected_number {
                    if number < choice.min_count as u32 || number > choice.max_count as u32 {
                        return Err(format!("Number must be between {} and {}", 
                            choice.min_count, choice.max_count));
                    }
                } else {
                    return Err("Number selection requires a number".to_string());
                }
            }
            ChoiceType::SelectOption => {
                if selected_option.is_none() {
                    return Err("Option selection requires an option".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if there's a pending choice
    pub fn has_pending_choice(&self) -> bool {
        self.pending_choice.is_some()
    }

    /// Get pending choice description
    pub fn get_pending_choice_description(&self) -> Option<String> {
        self.pending_choice.as_ref().map(|choice| choice.description.clone())
    }

    /// Get valid targets for current choice
    pub fn get_valid_targets(&self, player: &Player, card_database: &CardDatabase) -> Vec<i16> {
        if let Some(choice) = &self.pending_choice {
            match choice.choice_type {
                ChoiceType::SelectCards => {
                    self.get_valid_cards(player, &choice.target_zone, &choice.card_type_filter, card_database)
                }
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        }
    }

    /// Get valid cards for selection
    fn get_valid_cards(
        &self,
        player: &Player,
        target_zone: &str,
        card_type_filter: &Option<String>,
        card_database: &CardDatabase,
    ) -> Vec<i16> {
        let mut valid_cards = Vec::new();

        match target_zone {
            "hand" => {
                for &card_id in &player.hand.cards {
                    if let Some(card) = card_database.get_card(card_id) {
                        // Apply card type filter if specified
                        if let Some(filter) = card_type_filter {
                            // Convert card_type enum to string for comparison
                            if format!("{:?}", card.card_type) == *filter {
                                valid_cards.push(card_id);
                            }
                        } else {
                            valid_cards.push(card_id);
                        }
                    }
                }
            }
            "stage" => {
                // Use the correct Stage structure
                if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::LeftSide) {
                    if let Some(card) = card_database.get_card(card_id) {
                        if let Some(filter) = card_type_filter {
                            if format!("{:?}", card.card_type) == *filter {
                                valid_cards.push(card_id);
                            }
                        } else {
                            valid_cards.push(card_id);
                        }
                    }
                }
                if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::Center) {
                    if let Some(card) = card_database.get_card(card_id) {
                        if let Some(filter) = card_type_filter {
                            if format!("{:?}", card.card_type) == *filter {
                                valid_cards.push(card_id);
                            }
                        } else {
                            valid_cards.push(card_id);
                        }
                    }
                }
                if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::RightSide) {
                    if let Some(card) = card_database.get_card(card_id) {
                        if let Some(filter) = card_type_filter {
                            if format!("{:?}", card.card_type) == *filter {
                                valid_cards.push(card_id);
                            }
                        } else {
                            valid_cards.push(card_id);
                        }
                    }
                }
            }
            "discard" => {
                for &card_id in &player.waitroom.cards {
                    if let Some(card) = card_database.get_card(card_id) {
                        if let Some(filter) = card_type_filter {
                            if format!("{:?}", card.card_type) == *filter {
                                valid_cards.push(card_id);
                            }
                        } else {
                            valid_cards.push(card_id);
                        }
                    }
                }
            }
            _ => {}
        }

        valid_cards
    }

    /// Clear all choices (for new turn/game)
    pub fn clear(&mut self) {
        self.pending_choice = None;
        self.choice_history.clear();
    }
}
