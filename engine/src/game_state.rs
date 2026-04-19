use crate::player::Player;
use crate::zones::ResolutionZone;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    ActivePhase,
    EnergyPhase,
    DrawPhase,
    MainPhase,
    LiveCardSetPhase,
    FirstAttackerPerformancePhase,
    SecondAttackerPerformancePhase,
    LiveVictoryDeterminationPhase,
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
    pub current_phase: Phase,
    pub turn_number: u32,
    pub resolution_zone: ResolutionZone,
    pub is_first_turn: bool,
    pub live_cheer_count: u32,
}

impl GameState {
    pub fn new(player1: Player, player2: Player) -> Self {
        let is_first_turn = true;
        GameState {
            player1,
            player2,
            current_phase: Phase::ActivePhase,
            turn_number: 1,
            resolution_zone: ResolutionZone::new(),
            is_first_turn,
            live_cheer_count: 0,
        }
    }

    pub fn active_player(&self) -> &Player {
        match self.current_phase {
            Phase::ActivePhase
            | Phase::EnergyPhase
            | Phase::DrawPhase
            | Phase::MainPhase => {
                if self.turn_number % 2 == 1 {
                    &self.player1
                } else {
                    &self.player2
                }
            }
            Phase::LiveCardSetPhase => {
                if self.player1.is_first_attacker {
                    &self.player1
                } else {
                    &self.player2
                }
            }
            Phase::FirstAttackerPerformancePhase => {
                if self.player1.is_first_attacker {
                    &self.player1
                } else {
                    &self.player2
                }
            }
            Phase::SecondAttackerPerformancePhase => {
                if self.player1.is_first_attacker {
                    &self.player2
                } else {
                    &self.player1
                }
            }
            Phase::LiveVictoryDeterminationPhase => {
                &self.player1
            }
        }
    }

    pub fn active_player_mut(&mut self) -> &mut Player {
        match self.current_phase {
            Phase::ActivePhase
            | Phase::EnergyPhase
            | Phase::DrawPhase
            | Phase::MainPhase => {
                if self.turn_number % 2 == 1 {
                    &mut self.player1
                } else {
                    &mut self.player2
                }
            }
            Phase::LiveCardSetPhase => {
                if self.player1.is_first_attacker {
                    &mut self.player1
                } else {
                    &mut self.player2
                }
            }
            Phase::FirstAttackerPerformancePhase => {
                if self.player1.is_first_attacker {
                    &mut self.player1
                } else {
                    &mut self.player2
                }
            }
            Phase::SecondAttackerPerformancePhase => {
                if self.player1.is_first_attacker {
                    &mut self.player2
                } else {
                    &mut self.player1
                }
            }
            Phase::LiveVictoryDeterminationPhase => {
                &mut self.player1
            }
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
