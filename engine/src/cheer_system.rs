use crate::card::CardDatabase;
use crate::game_state::GameState;
use crate::player::Player;

/// Cheer system for live phase (Rule 8.3.11)
pub struct CheerSystem {
    pub total_blade_count: u32,
    pub cheer_cards: Vec<i16>,
    pub heart_icons_extracted: Vec<HeartIcon>,
    // Cards revealed from deck during cheer (Rule 8.3.11) - stored temporarily before moving to resolution zone
    pub revealed_cards: Vec<i16>,
}

#[derive(Debug, Clone)]
pub struct HeartIcon {
    pub color: HeartColor,
    pub is_wild: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeartColor {
    Pink,    // 桃
    Red,     // 赤
    Yellow,  // 黄
    Green,   // 緑
    Blue,    // 青
    Purple,  // 紫
}

impl CheerSystem {
    pub fn new() -> Self {
        Self {
            total_blade_count: 0,
            cheer_cards: Vec::new(),
            heart_icons_extracted: Vec::new(),
            revealed_cards: Vec::new(),
        }
    }

    /// Execute cheer process (Rule 8.3.11)
    pub fn execute_cheer(
        &mut self,
        player: &mut Player,
        _game_state: &mut GameState,
        card_database: &CardDatabase,
    ) -> Result<(), String> {
        // Step 1: Count total blades from all active stage members (Rule 8.3.10)
        self.total_blade_count = self.count_total_blades(player, card_database)?;
        
        if self.total_blade_count == 0 {
            return Ok(()); // No cheer if no blades
        }

        // Step 2: Reveal cards from deck equal to blade count (Rule 8.3.11)
        self.cheer_cards = self.reveal_from_deck(player, self.total_blade_count);

        // Step 3: Extract heart icons from revealed cards (Rule 8.3.12)
        // Clone to avoid borrow checker issue
        let cheer_cards_clone = self.cheer_cards.clone();
        self.extract_heart_icons(&cheer_cards_clone, card_database)?;

        // Step 4: Draw cards based on heart icons (Rule 8.3.12.1)
        self.draw_cards_from_hearts(player)?;

        // Step 5: Store heart icons for live success check (Rule 8.3.14)
        self.store_live_owned_hearts(_game_state, &player.id);

        println!("Cheer completed: {} blades, {} cards revealed, {} hearts extracted", 
            self.total_blade_count, self.cheer_cards.len(), self.heart_icons_extracted.len());

        Ok(())
    }

    /// Count total blades from all active stage members (Rule 8.3.10)
    fn count_total_blades(&self, player: &Player, card_database: &CardDatabase) -> Result<u32, String> {
        let mut total = 0u32;

        // Check all member areas - simplified since we don't have orientation tracking
        if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::LeftSide) {
            if let Some(card) = card_database.get_card(card_id) {
                total += card.blade;
            }
        }
        if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::Center) {
            if let Some(card) = card_database.get_card(card_id) {
                total += card.blade;
            }
        }
        if let Some(card_id) = player.stage.get_area(crate::zones::MemberArea::RightSide) {
            if let Some(card) = card_database.get_card(card_id) {
                total += card.blade;
            }
        }

        Ok(total)
    }

    /// Reveal cards from deck equal to blade count (Rule 8.3.11)
    pub fn reveal_from_deck(&mut self, player: &mut Player, count: u32) -> Vec<i16> {
        let mut revealed = Vec::new();
        for _ in 0..count {
            if let Some(card_id) = player.main_deck.draw() {
                revealed.push(card_id);
            }
        }
        // Store revealed cards temporarily in CheerSystem (Rule 8.3.11)
        // These will be moved to resolution zone by the caller
        self.revealed_cards.extend(&revealed);
        revealed
    }

    /// Extract heart icons from revealed cards (Rule 8.3.12)
    fn extract_heart_icons(&mut self, card_ids: &[i16], card_database: &CardDatabase) -> Result<(), String> {
        self.heart_icons_extracted.clear();

        for &card_id in card_ids {
            if let Some(card) = card_database.get_card(card_id) {
                // Extract blade heart icons from the card
                if let Some(blade_heart) = &card.blade_heart {
                    // blade_heart has a 'hearts' HashMap<HeartColor, u32>
                    for (heart_color, count) in &blade_heart.hearts {
                        // Convert HeartColor enum to string for parsing
                        let color_str = format!("{:?}", heart_color);
                        let heart_icon = HeartIcon {
                            color: self.parse_heart_color(&color_str),
                            is_wild: color_str == "Pink", // Assume Pink is wild for now
                        };
                        // Add one heart icon per count
                        for _ in 0..*count {
                            self.heart_icons_extracted.push(heart_icon.clone());
                        }
                    }
                }
                // Energy gain: Draw energy cards from top of deck and place in energy zone
                // TODO: Implement energy gain when card.energy_gain field is available
                // if let Some(gain) = &card.energy_gain {
                //     let count = gain.count.unwrap_or(1);
                //     for _ in 0..count {
                //         if let Some(card_id) = player.main_deck.draw() {
                //             player.energy_zone.add_card(card_id);
                //             eprintln!("Gained energy card {} from deck to energy zone", card_id);
                //         } else {
                //             eprintln!("Deck empty, could not gain energy");
                //             break;
                //         }
                //     }
                // }
            }
        }

        Ok(())
    }

    /// Parse heart color string
    fn parse_heart_color(&self, color_str: &str) -> HeartColor {
        match color_str {
            "pink" | "桃" => HeartColor::Pink,
            "red" | "赤" => HeartColor::Red,
            "yellow" | "黄" => HeartColor::Yellow,
            "green" | "緑" => HeartColor::Green,
            "blue" | "青" => HeartColor::Blue,
            "purple" | "紫" => HeartColor::Purple,
            "wild" | "白" => HeartColor::Pink, // Wild hearts can be any color, default to pink
            _ => HeartColor::Pink, // Default
        }
    }

    /// Draw cards based on heart icons (Rule 8.3.12.1)
    fn draw_cards_from_hearts(&self, player: &mut Player) -> Result<(), String> {
        let draw_count = self.heart_icons_extracted.len() as u32;

        for _ in 0..draw_count {
            if let Some(card_id) = player.main_deck.cards.pop() {
                player.hand.add_card(card_id);
            } else {
                break; // Deck is empty
            }
        }

        println!("Drew {} cards from heart icons", draw_count);
        Ok(())
    }

    /// Store heart icons for live success check (Rule 8.3.14)
    pub fn store_live_owned_hearts(&self, game_state: &mut GameState, player_id: &str) {
        // Store actual heart icons extracted during cheer for live success validation (Rule 8.3.14)
        // Convert HeartIcon to (color, count) tuples for game state
        let hearts: Vec<(String, u32)> = self.heart_icons_extracted.iter()
            .map(|heart| (format!("{:?}", heart.color), 1))
            .collect();
        let count = hearts.len();
        
        // Store in game_state for later use during live success validation
        game_state.live_owned_hearts.insert(player_id.to_string(), hearts);
        
        eprintln!("Stored {} live owned hearts for player {}", count, player_id);
    }

    /// Get total hearts of specific color (for live success check)
    pub fn get_hearts_of_color(&self, color: &HeartColor) -> usize {
        self.heart_icons_extracted.iter()
            .filter(|heart| heart.is_wild || heart.color == *color)
            .count()
    }

    /// Get total heart count (for live success check)
    pub fn get_total_heart_count(&self) -> usize {
        self.heart_icons_extracted.len()
    }

    /// Clear cheer data (for next live)
    pub fn clear(&mut self) {
        self.total_blade_count = 0;
        self.cheer_cards.clear();
        self.heart_icons_extracted.clear();
    }
}
