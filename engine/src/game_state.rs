use crate::player::Player;
use crate::zones::ResolutionZone;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnPhase {
    FirstAttackerNormal,   // Rule 7.3.2.1
    SecondAttackerNormal,  // Rule 7.3.2.1
    Live,                  // Rule 8.1
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    // Normal phase sub-phases (Rule 7.3.3)
    Active,
    Energy,
    Draw,
    Main,
    // Live phase sub-phases (Rule 8.1.2)
    LiveCardSet,
    FirstAttackerPerformance,
    SecondAttackerPerformance,
    LiveVictoryDetermination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameResult {
    FirstAttackerWins,
    SecondAttackerWins,
    Draw,
    Ongoing,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub player1: Player,
    pub player2: Player,
    pub current_turn_phase: TurnPhase,
    pub current_phase: Phase,
    pub turn_number: u32,
    pub resolution_zone: ResolutionZone,
    pub is_first_turn: bool,
    pub live_cheer_count: u32,
    // Keyword tracking
    pub turn1_abilities_played: std::collections::HashSet<String>, // Track Turn1 abilities played this turn
    pub turn2_abilities_played: std::collections::HashMap<String, u32>, // Track Turn2 abilities played this turn
    // Rule 8.4.2.1: Track cheer blade heart counts for victory determination
    pub player1_cheer_blade_heart_count: u32,
    pub player2_cheer_blade_heart_count: u32,
}

impl GameState {
    pub fn new(player1: Player, player2: Player) -> Self {
        let is_first_turn = true;
        GameState {
            player1,
            player2,
            current_turn_phase: TurnPhase::FirstAttackerNormal,
            current_phase: Phase::Active,
            turn_number: 1,
            resolution_zone: ResolutionZone::new(),
            is_first_turn,
            live_cheer_count: 0,
            turn1_abilities_played: HashSet::new(),
            turn2_abilities_played: HashMap::new(),
            player1_cheer_blade_heart_count: 0,
            player2_cheer_blade_heart_count: 0,
        }
    }

    pub fn active_player(&self) -> &Player {
        // Rule 7.2: Determine active player based on turn phase
        match self.current_turn_phase {
            TurnPhase::FirstAttackerNormal => self.first_attacker(),
            TurnPhase::SecondAttackerNormal => self.second_attacker(),
            TurnPhase::Live => {
                // Rule 7.2.1.2: In phases without specified turn player, first attacker is active
                self.first_attacker()
            }
        }
    }

    pub fn active_player_mut(&mut self) -> &mut Player {
        match self.current_turn_phase {
            TurnPhase::FirstAttackerNormal => {
                if self.player1.is_first_attacker {
                    &mut self.player1
                } else {
                    &mut self.player2
                }
            }
            TurnPhase::SecondAttackerNormal => {
                if self.player1.is_first_attacker {
                    &mut self.player2
                } else {
                    &mut self.player1
                }
            }
            TurnPhase::Live => {
                if self.player1.is_first_attacker {
                    &mut self.player1
                } else {
                    &mut self.player2
                }
            }
        }
    }

    pub fn first_attacker(&self) -> &Player {
        if self.player1.is_first_attacker {
            &self.player1
        } else {
            &self.player2
        }
    }

    pub fn first_attacker_mut(&mut self) -> &mut Player {
        if self.player1.is_first_attacker {
            &mut self.player1
        } else {
            &mut self.player2
        }
    }

    pub fn second_attacker(&self) -> &Player {
        if self.player1.is_first_attacker {
            &self.player2
        } else {
            &self.player1
        }
    }

    pub fn second_attacker_mut(&mut self) -> &mut Player {
        if self.player1.is_first_attacker {
            &mut self.player2
        } else {
            &mut self.player1
        }
    }

    pub fn non_active_player(&self) -> &Player {
        if std::ptr::eq(self.active_player(), &self.player1) {
            &self.player2
        } else {
            &self.player1
        }
    }

    pub fn non_active_player_mut(&mut self) -> &mut Player {
        if std::ptr::eq(self.active_player(), &self.player1) {
            &mut self.player2
        } else {
            &mut self.player1
        }
    }

    pub fn can_play_turn1_ability(&self, ability_id: &str) -> bool {
        // Rule 11.2: Turn1 - ability can only be played once per turn
        !self.turn1_abilities_played.contains(ability_id)
    }

    pub fn can_play_turn2_ability(&self, ability_id: &str) -> bool {
        // Rule 11.3: Turn2 - ability can only be played twice per turn
        let count = self.turn2_abilities_played.get(ability_id).unwrap_or(&0);
        *count < 2
    }

    pub fn record_turn1_ability(&mut self, ability_id: String) {
        self.turn1_abilities_played.insert(ability_id);
    }

    pub fn record_turn2_ability(&mut self, ability_id: String) {
        *self.turn2_abilities_played.entry(ability_id).or_insert(0) += 1;
    }

    pub fn reset_keyword_tracking(&mut self) {
        // Reset keyword tracking at start of new turn
        self.turn1_abilities_played.clear();
        self.turn2_abilities_played.clear();
        // Reset cheer blade heart counts at start of new turn
        self.player1_cheer_blade_heart_count = 0;
        self.player2_cheer_blade_heart_count = 0;
    }

    pub fn check_victory(&self) -> GameResult {
        let p1_victory = self.player1.has_victory();
        let p2_victory = self.player2.has_victory();

        if p1_victory && !p2_victory {
            GameResult::FirstAttackerWins
        } else if p2_victory && !p1_victory {
            GameResult::SecondAttackerWins
        } else if p1_victory && p2_victory {
            GameResult::Draw
        } else {
            GameResult::Ongoing
        }
    }
}
