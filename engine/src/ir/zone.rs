use serde::{Deserialize, Serialize};

/// All possible zones in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Zone {
    MainDeck,
    EnergyDeck,
    Hand,
    StageLeft,
    StageCenter,
    StageRight,
    UnderMember,
    EnergyZone,
    LiveCardZone,
    SuccessLiveZone,
    Waitroom,
    Exclusion,
    LookedAt,
    Revealed,
    SameArea,       // "そのメンバーがいたエリア"
    EmptyArea,      // "メンバーのいないエリア"
    DeckTop,
    DeckBottom,
    DeckPosition(u32), // "デッキの一番上からN枚目"
}

impl Zone {
    pub fn is_stage(&self) -> bool {
        matches!(self, Zone::StageLeft | Zone::StageCenter | Zone::StageRight)
    }

    pub fn is_hidden(&self) -> bool {
        matches!(
            self,
            Zone::MainDeck | Zone::EnergyDeck | Zone::Hand | Zone::LookedAt
        )
    }
}
