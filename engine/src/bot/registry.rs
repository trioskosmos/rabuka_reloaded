//! Central bot registry: version dispatch without renames.
//!
//! Adding a new bot version is now three steps and NO renames:
//!   1. add `strategy_v8.rs` with the generic entry points
//!      `choose_action` / `choose_live_set` / `choose_mulligan`
//!      (keep any versioned names inside the module if you like);
//!   2. add a `V8` variant below;
//!   3. add three dispatch lines in the `impl` block.
//!
//! `bot_arena.rs` (and any future harness) calls only this registry, so it
//! never names a versioned function directly.

use crate::card::CardDatabase;
use crate::game_setup::Action;
use crate::game_state::GameState;

use super::{conductor, strategy, strategy_v2, strategy_v3, strategy_v4, strategy_v5, strategy_v6, strategy_v7};

/// Every bot the arena can field. The string form is the CLI name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BotKind {
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    Conductor,
    Random,
}

impl BotKind {
    /// All CLI names, for help text.
    pub const ALL: &'static [&'static str] = &[
        "v1",
        "v2",
        "v3",
        "v4",
        "v5",
        "v6",
        "v7",
        "conductor",
        "random",
    ];

    pub fn parse(s: &str) -> Self {
        match s {
            "v1" => BotKind::V1,
            "v2" => BotKind::V2,
            "v3" => BotKind::V3,
            "v4" => BotKind::V4,
            "v5" => BotKind::V5,
            "v6" => BotKind::V6,
            "v7" => BotKind::V7,
            "conductor" => BotKind::Conductor,
            _ => BotKind::Random,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            BotKind::V1 => "v1",
            BotKind::V2 => "v2",
            BotKind::V3 => "v3",
            BotKind::V4 => "v4",
            BotKind::V5 => "v5",
            BotKind::V6 => "v6",
            BotKind::V7 => "v7",
            BotKind::Conductor => "conductor",
            BotKind::Random => "random",
        }
    }

    /// Main-phase (and any other action-phase) decision.
    /// `v2_policy` / `plan` are only read by the generations that need them;
    /// every other bot ignores them, so the arena can pass shared values.
    pub fn choose_action(
        self,
        gs: &GameState,
        actions: &[Action],
        me: u8,
        v2_policy: &strategy_v2::V2Policy,
        plan: &strategy_v3::V3Plan,
    ) -> Action {
        let _ = v2_policy;
        match self {
            BotKind::V1 => strategy::choose_action_heuristic(gs, actions, me),
            BotKind::V2 => strategy_v2::choose_action_heuristic_v2(gs, actions, me),
            BotKind::V3 => strategy_v3::choose_action_heuristic_v3(gs, actions, me, plan),
            BotKind::V4 => strategy_v4::choose_action(gs, actions, me),
            BotKind::V5 => strategy_v5::choose_action(gs, actions, me),
            BotKind::V6 => strategy_v6::choose_action(gs, actions, me),
            BotKind::V7 => strategy_v7::choose_action(gs, actions, me),
            BotKind::Conductor => conductor::choose_main_conductor(gs, actions, me),
            BotKind::Random => {
                // Deterministic harness random is handled by the caller; fall
                // back to the first action so this stays total.
                actions.first().cloned().unwrap_or_else(|| Action {
                    description: "pass".into(),
                    description_ja: None,
                    action_type: crate::game_setup::ActionType::Pass,
                    parameters: None,
                    selected: None,
                })
            }
        }
    }

    /// Live-card-set decision.
    pub fn choose_live_set(
        self,
        gs: &GameState,
        actions: &[Action],
        db: &CardDatabase,
        v2_policy: &strategy_v2::V2Policy,
        plan: &strategy_v3::V3Plan,
    ) -> Action {
        match self {
            BotKind::V1 => strategy::choose_live_set_action(gs, actions, db),
            BotKind::V2 => strategy_v2::choose_live_set_action_v2(gs, actions, db, v2_policy),
            BotKind::V3 => strategy_v3::choose_live_set_action_v3(gs, actions, db, v2_policy, plan),
            BotKind::V4 => strategy_v4::choose_live_set(gs, actions, db),
            BotKind::V5 => strategy_v5::choose_live_set(gs, actions, db),
            BotKind::V6 => strategy_v6::choose_live_set(gs, actions, db),
            BotKind::V7 => strategy_v7::choose_live_set(gs, actions, db),
            BotKind::Conductor => conductor::choose_live_set_conductor(gs, actions, db),
            BotKind::Random => actions
                .first()
                .cloned()
                .unwrap_or_else(|| strategy::choose_live_set_action(gs, actions, db)),
        }
    }

    /// Mulligan decision.
    pub fn choose_mulligan(self, gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
        match self {
            BotKind::V1 | BotKind::Random => actions
                .iter()
                .find(|a| {
                    matches!(
                        a.action_type,
                        crate::game_setup::ActionType::ConfirmMulligan
                            | crate::game_setup::ActionType::SkipMulligan
                    )
                })
                .or_else(|| actions.first())
                .cloned()
                .unwrap_or_else(|| Action {
                    description: "skip".into(),
                    description_ja: None,
                    action_type: crate::game_setup::ActionType::SkipMulligan,
                    parameters: None,
                    selected: None,
                }),
            BotKind::V2 => strategy_v2::choose_mulligan_action_v2(gs, actions, db),
            BotKind::V3 => strategy_v3::choose_mulligan_action_v3(gs, actions, db),
            BotKind::V4 => strategy_v4::choose_mulligan(gs, actions, db),
            BotKind::V5 => strategy_v5::choose_mulligan(gs, actions, db),
            BotKind::V6 => strategy_v6::choose_mulligan(gs, actions, db),
            BotKind::V7 => strategy_v7::choose_mulligan(gs, actions, db),
            BotKind::Conductor => strategy_v4::choose_mulligan(gs, actions, db),
        }
    }
}
