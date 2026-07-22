use super::observation::PublicObservation;
use crate::game_setup::Action;

pub const CARD_EMBED_DIM: usize = 128;
pub const ZONE_EMBED_DIM: usize = 16;
pub const ACTION_TYPE_EMBED_DIM: usize = 16;
pub const POSITION_FEATURES: usize = 4;
pub const GLOBAL_FEATURES: usize = 28; // 12 phase one-hot + 16 scalar features
pub const ACTION_ENC_DIM: usize =
    ACTION_TYPE_EMBED_DIM + CARD_EMBED_DIM + ZONE_EMBED_DIM + POSITION_FEATURES;

pub const NUM_ZONES: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneId {
    MyHand = 0,
    MyStagePos0 = 1,
    MyStagePos1 = 2,
    MyStagePos2 = 3,
    MyEnergy = 4,
    MyWaitroom = 5,
    MyLive = 6,
    MySuccess = 7,
    OppStagePos0 = 8,
    OppStagePos1 = 9,
    OppStagePos2 = 10,
    OppEnergy = 11,
    OppWaitroom = 12,
    OppLive = 13,
    OppSuccess = 14,
}

impl ZoneId {
    pub fn all() -> [Self; NUM_ZONES] {
        [
            Self::MyHand,
            Self::MyStagePos0,
            Self::MyStagePos1,
            Self::MyStagePos2,
            Self::MyEnergy,
            Self::MyWaitroom,
            Self::MyLive,
            Self::MySuccess,
            Self::OppStagePos0,
            Self::OppStagePos1,
            Self::OppStagePos2,
            Self::OppEnergy,
            Self::OppWaitroom,
            Self::OppLive,
            Self::OppSuccess,
        ]
    }

    pub fn is_sum_zone(self) -> bool {
        matches!(
            self,
            Self::MyHand
                | Self::MyEnergy
                | Self::MyWaitroom
                | Self::MyLive
                | Self::MySuccess
                | Self::OppEnergy
                | Self::OppWaitroom
                | Self::OppLive
                | Self::OppSuccess
        )
    }

    pub fn is_positional_zone(self) -> bool {
        matches!(
            self,
            Self::MyStagePos0
                | Self::MyStagePos1
                | Self::MyStagePos2
                | Self::OppStagePos0
                | Self::OppStagePos1
                | Self::OppStagePos2
        )
    }

    pub fn as_usize(self) -> usize {
        self as usize
    }
}

pub struct EncodedState {
    pub my_hand: Vec<f32>,
    pub my_stage: [Vec<f32>; 3],
    pub my_energy: Vec<f32>,
    pub my_waitroom: Vec<f32>,
    pub my_live: Vec<f32>,
    pub my_success: Vec<f32>,
    pub opp_stage: [Vec<f32>; 3],
    pub opp_energy: Vec<f32>,
    pub opp_waitroom: Vec<f32>,
    pub opp_live: Vec<f32>,
    pub opp_success: Vec<f32>,
    pub globals: Vec<f32>,
}

impl EncodedState {
    pub fn flatten(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(EncodedState::state_dim());
        out.extend(&self.my_hand);
        for pos in &self.my_stage {
            out.extend(pos);
        }
        out.extend(&self.my_energy);
        out.extend(&self.my_waitroom);
        out.extend(&self.my_live);
        out.extend(&self.my_success);
        for pos in &self.opp_stage {
            out.extend(pos);
        }
        out.extend(&self.opp_energy);
        out.extend(&self.opp_waitroom);
        out.extend(&self.opp_live);
        out.extend(&self.opp_success);
        out.extend(&self.globals);
        out
    }

    pub fn state_dim() -> usize {
        // 8 sum zones × 128 + 6 positional zones × (128 + 4) + globals
        8 * CARD_EMBED_DIM + 6 * (CARD_EMBED_DIM + POSITION_FEATURES) + GLOBAL_FEATURES
    }

    /// Returns (start, end) ranges for each zone in the flat encoding.
    pub fn zone_ranges() -> [(ZoneId, (usize, usize)); NUM_ZONES] {
        let mut offset = 0;
        let mut ranges = [(ZoneId::MyHand, (0, 0)); NUM_ZONES];
        for (i, z) in ZoneId::all().iter().enumerate() {
            let dim = if z.is_sum_zone() {
                CARD_EMBED_DIM
            } else {
                CARD_EMBED_DIM + POSITION_FEATURES
            };
            ranges[i] = (*z, (offset, offset + dim));
            offset += dim;
        }
        ranges
    }
}

pub struct ActionEncoding {
    pub action_type: u8,
    pub target_card_id: i16,
    pub target_zone: u8,
    pub position: u8,
}

impl ActionEncoding {
    pub fn encode(
        &self,
        card_embed: &[f32],
        zone_embed: &[f32],
        action_type_embed: &[f32],
    ) -> Vec<f32> {
        let mut v = Vec::with_capacity(ACTION_ENC_DIM);
        v.extend_from_slice(action_type_embed);
        let cid = self.target_card_id.max(0) as usize;
        let base = cid * CARD_EMBED_DIM;
        let ce = if base + CARD_EMBED_DIM <= card_embed.len() {
            &card_embed[base..base + CARD_EMBED_DIM]
        } else {
            &card_embed[..CARD_EMBED_DIM] // fallback to first card
        };
        v.extend_from_slice(ce);
        let zid = (self.target_zone as usize).min(NUM_ZONES - 1);
        let zbase = zid * ZONE_EMBED_DIM;
        v.extend_from_slice(&zone_embed[zbase..zbase + ZONE_EMBED_DIM]);
        v.push((self.position as f32) / 3.0);
        v.push(0.0);
        v.push(0.0);
        v
    }
}

#[derive(Debug, Clone)]
pub struct ActionTargetZone {
    pub zone: ZoneId,
    pub position: u8,
}

/// Determine the target zone for an action.
pub fn action_target_zone(action: &Action, obs: &PublicObservation) -> ActionTargetZone {
    use crate::game_setup::ActionType;
    match action.action_type {
        ActionType::PlayMemberToStage => {
            let pos = action
                .parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .map(|s| match s {
                    "left" => 0u8,
                    "center" => 1,
                    "right" => 2,
                    _ => 0,
                })
                .unwrap_or(0);
            ActionTargetZone {
                zone: ZoneId::MyStagePos0,
                position: pos,
            }
        }
        ActionType::UseAbility => {
            // Target is the card whose ability we're using
            if let Some(cid) = action.parameters.as_ref().and_then(|p| p.card_id) {
                // Check if it's on our stage
                if obs.me.stage.contains(&cid) {
                    let pos = obs.me.stage.iter().position(|&c| c == cid).unwrap_or(0) as u8;
                    return ActionTargetZone {
                        zone: ZoneId::MyStagePos0,
                        position: pos,
                    };
                }
            }
            ActionTargetZone {
                zone: ZoneId::MyHand,
                position: 0,
            }
        }
        ActionType::SetLiveCard => ActionTargetZone {
            zone: ZoneId::MyLive,
            position: 0,
        },
        ActionType::SelectLiveCard
        | ActionType::ConfirmLiveCardSet
        | ActionType::SkipLiveCardSet => ActionTargetZone {
            zone: ZoneId::MyLive,
            position: 0,
        },
        ActionType::Pass | ActionType::PassRemaining => ActionTargetZone {
            zone: ZoneId::MyHand,
            position: 0,
        },
        _ => ActionTargetZone {
            zone: ZoneId::MyHand,
            position: 0,
        },
    }
}
