use crate::ability::debug::AbDebug;
use crate::ability_queue::QueueState;
use crate::card::CardDatabase;
use crate::card::HeartColor;
use crate::game_state::{GameState, LOG_BOUND_RULE, LOG_BOUND_STRUCTURED};
use crate::player::Player;
use crate::types::PerformanceSnapshot;
use crate::zones::Orientation;
use crate::{HashMap, HashSet};
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

fn heart_color_index(color: &HeartColor) -> Option<usize> {
    match color {
        HeartColor::BAll | HeartColor::Draw | HeartColor::Score => None,
        _ => Some(color.index()),
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct TempEffectDisplay {
    pub effect_type: String,
    pub duration: String,
    pub created_turn: u8,
    pub target_player_id: String,
    pub description: String,
    #[cfg_attr(feature = "serde_support", serde(default))]
    #[cfg(feature = "serde_support")]
    pub effect_data: Option<serde_json::Value>,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct ReplacementEffectDisplay {
    pub card_id: i16,
    pub player_id: String,
    pub original_event: String,
    pub is_choice_based: bool,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AbilityQueueEntryDisplay {
    pub card_no: String,
    pub player_id: String,
    pub trigger_type: String,
    pub completed: bool,
    pub cost_paid: bool,
    pub effect_started: bool,
    pub choice_player_id: Option<String>,
    pub ability_text: String,
    pub card_id: Option<i16>,
    pub ability_index: usize,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct DebutTriggerDisplay {
    pub ability_key: String,
    pub card_id: i16,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AbilityApplicationDisplay {
    pub source_card_id: i16,
    pub effect_type: String,
    pub target_card_id: i16,
    pub amount: i16,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct CardDisplay {
    pub card_no: String,
    pub name: String,
    #[cfg_attr(feature = "serde_support", serde(rename = "type"))]
    pub card_type: String,
    pub orientation: Option<String>,
    pub base_heart: Option<HashMap<String, u8>>,
    pub blade: u8,
    pub total_blade: u8,
    pub id: i16,
    pub ability_text: Option<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub hidden: bool,

    // ════════════════════════════════════════════════════════════════
    // Texticon / status badge fields
    //
    // These are rendered as small image badges on the card in the UI.
    // There are TWO categories:
    //
    //  1. Additive bonuses (bonus_*) — from Modify* / GainResource actions
    //     → shown with +/- prefix: e.g. "+2 Blade", "-1 Cost", "+3 Score"
    //     → appears when the value is non-zero
    //
    //  2. Set/override values (set_*) — from Set* actions
    //     → shown with PLAIN number (no +/-): e.g. "5 Blade", "3 Hearts"
    //     → represents an absolute override, not an incremental change
    //
    //  3. Trigger badges (bonus_triggers) — from GainAbility actions
    //     → shown as trigger-type texticon: jyouji.png, live_success.png, etc.
    //     → no numeric value, just the icon
    //
    // ════════════════════════════════════════════════════════════════

    // ── Additive bonuses (ModifyBlade, GainResource blade) ──
    // Shows icon_blade.png with "+N" or "-N"
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub bonus_blade: i32,

    // ── Additive heart bonuses (ModifyHeart, GainResource heart) ──
    // Shows per-color heart icons with "+N" or "-N"
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub bonus_hearts: Vec<i32>,

    // ── Additive score bonus (ModifyScore, GainAbility score) ──
    // Shows icon_score.png with "+N" or "-N"
    // NOTE: GainAbility(constant trigger) contributes to this via
    // recalculate_constants → score_modifier, but does NOT add a
    // jyouji.png trigger icon. The trigger type is in bonus_triggers.
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub bonus_score: i32,

    // ── Additive cost modifier (ModifyCost) ──
    // Shows icon_energy.png with "+N" or "-N"
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub bonus_cost: i32,

    // ── Set/override blade (SetBladeCount, SetBladeType) ──
    // Shows icon_blade.png with plain "N" (no +/-)
    // Separate from bonus_blade so both can appear simultaneously:
    // e.g. set_blade=5 (base) + bonus_blade=+2 (modifier)
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub set_blade: i32,

    // ── Set/override hearts (SetHeartType) ──
    // Shows per-color heart icons with plain "N" (no +/-)
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub set_hearts: Vec<i32>,

    // ── Set/override score (SetScore) ──
    // Shows icon_score.png with plain "N" (no +/-)
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub set_score: i32,

    // ── Set/override cost (SetCost, SetCostToUse) ──
    // Shows icon_energy.png with plain "N" (no +/-)
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub set_cost: i32,

    // ── Gained ability trigger texticons (GainAbility) ──
    // Populated from gained_card_abilities in game_state_to_display.
    // Each entry is a trigger type name like "jyouji", "live_success",
    // "toujyou", "kidou", "jidou" — the frontend maps these to texticon
    // images.
    // Without this field, gain_ability effects would have NO texticon
    // indicator on the card, even though they grant persistent abilities.
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub bonus_triggers: Vec<String>,

    // ── Heart color transform (SetHeartType → "transform heart to X") ──
    // Shows a heart texticon overlay indicating all hearts now count as
    // that color.
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub heart_transform: Option<String>,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cost: Option<u8>,
}

#[derive(Clone, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct ZoneDisplay {
    pub cards: Vec<CardDisplay>,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct PlayerDisplay {
    pub hand: ZoneDisplay,
    pub energy: ZoneDisplay,
    pub stage: StageDisplay,
    pub live_zone: ZoneDisplay,
    pub success_live_card_zone: ZoneDisplay,
    pub waitroom: ZoneDisplay,
    pub discard: ZoneDisplay,
    pub main_deck_count: usize,
    pub energy_deck_count: usize,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub last_resolution_cards: Vec<CardDisplay>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub score_modifiers: HashMap<i16, i32>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub total_hearts: Vec<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub live_need_hearts: Vec<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub selected_need_hearts: Vec<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub current_score: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub live_card_scores: HashMap<String, u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub gained_abilities: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub active_restrictions: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub need_heart_modifiers: HashMap<String, Vec<i32>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub mulligan_selection: Option<Vec<usize>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub live_card_selection: Option<Vec<usize>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub blade_buffs: Vec<i32>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub heart_buffs: Vec<Vec<i32>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cost_reduction: i32,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub prevent_baton_touch: i32,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub prevent_baton: i32,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub deployed_this_turn: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub debut_count_this_turn: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub id: String,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub name: String,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub is_first_attacker: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub exclusion_zone: ZoneDisplay,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub energy_active_count: usize,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub stage_hearts: Option<HashMap<String, u8>>,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct StageDisplay {
    pub left_side: Option<CardDisplay>,
    pub center: Option<CardDisplay>,
    pub right_side: Option<CardDisplay>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub left_under: Vec<CardDisplay>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub center_under: Vec<CardDisplay>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub right_under: Vec<CardDisplay>,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct RevealedCardDisplay {
    pub card_id: i16,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub source_card_id: Option<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub source_card_name: Option<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub owner: i8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub is_private: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub reveal_type: String,
}

impl RevealedCardDisplay {
    pub fn from_card_and_meta(
        card_id: i16,
        meta: Option<&crate::core::game_state::RevealedCardMeta>,
    ) -> Self {
        RevealedCardDisplay {
            card_id,
            source_card_id: meta.and_then(|m| m.source),
            source_card_name: meta.and_then(|m| m.source_name.clone()),
            owner: meta.and_then(|m| m.owner).map(|o| o as i8).unwrap_or(-1i8),
            is_private: meta.map(|m| m.is_private).unwrap_or(false),
            reveal_type: meta.map(|m| m.reveal_type.to_string()).unwrap_or_default(),
        }
    }
}

#[derive()]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct GameStateDisplay {
    pub turn: u8,
    pub phase: String,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub active_player: String,
    pub player1: PlayerDisplay,
    pub player2: PlayerDisplay,
    #[cfg(feature = "serde_support")]
    pub pending_choice: Option<serde_json::Value>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub looked_cards: ZoneDisplay,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub rule_log: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub structured_log: Vec<crate::types::LogEntry>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub performance_results: Option<HashMap<String, PerformanceSnapshot>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub performance_history: Vec<PerformanceSnapshot>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub game_over: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub winner: Option<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub waiting_for_opponent: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub mode: String,
    // --- Internal tracking fields for game state modal ---
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub current_turn_phase: String,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub game_result: String,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub is_first_turn: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub turn_order_changed: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub baton_touch_count: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub baton_touch_zero_cost: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub baton_touch_replaced_member_cost: Option<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub baton_touch_replaced_member_id: Option<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub baton_touch_arriving_card_id: Option<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub deck_refresh_pending: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub loop_detected: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub draw_state: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub live_being_performed: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cards_moved_this_turn: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cards_appeared_this_turn: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub areas_placed_this_turn: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub last_area_move_card_id: Option<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub last_area_move_by_player: Option<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub last_energy_placed_by_effect: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub batch_movements: Vec<crate::types::MovementEvent>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub last_energy_placed_by_player: Option<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub position_change_occurred_this_turn: bool,
    /// Batch-level stage-area-to-stage-area position changes in execution
    /// order. Each entry records one card's movement between left/center/right
    /// positions on stage. A swap between two cards produces 2 entries.
    /// Cleared per ability batch (after all triggered abilities resolve).
    ///
    /// UI use: drive position-change animations on the stage display. Iterate
    /// in order and animate each card from old_position → new_position
    /// (0=left, 1=center, 2=right). All entries in this list are one batch
    /// and should animate together (e.g. a formation change where all 3 cards
    /// rearrange produces 4-6 entries).
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub position_changes: Vec<crate::types::PositionChangeEvent>,
    /// Same data as position_changes but keyed by card ID for per-card
    /// lookups. Each card's entry is its full position-change history
    /// within this batch — useful when a card moves multiple times
    /// (e.g. left→center in one swap, then center→right in another).
    ///
    /// UI use: display a "card moved" badge/tooltip on individual member
    /// cards showing where it moved. Check if a card ID appears here to
    /// highlight it as "moved this batch". The Vec length tells you how
    /// many times it moved (usually 1, up to 3 for a full rotation).
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub position_changes_by_card: HashMap<i16, Vec<crate::types::PositionChangeEvent>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub formation_change_occurred_this_turn: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub opponent_live_success_this_turn: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub opponent_live_no_excess_heart_this_turn: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub self_no_excess_heart_this_turn: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub opponent_live_surplus_count: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub self_live_surplus_count: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub live_success_triggered_this_turn: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub live_surplus_ready_this_turn: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cheer_checks_required: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cheer_checks_done: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub turn_limited_abilities_used: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub auto_ability_trigger_counts: HashMap<String, u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub turn_limit_usage: HashMap<String, u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub non_stackable_effects: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub prohibition_effects: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub delayed_prohibition_effects: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cannot_live_players: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cannot_activate_members: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub constant_cannot_activate_members: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub constant_ability_statuses: Vec<crate::types::ConstantAbilityStatus>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub negated_abilities: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub temporary_effects: Vec<TempEffectDisplay>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub replacement_effects: Vec<ReplacementEffectDisplay>,

    // === New comprehensive fields ===
    // Ability Queue
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub ability_queue_state: String,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub ability_queue_current_index: usize,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub ability_queue_entries: Vec<AbilityQueueEntryDisplay>,

    // RPS
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub rps_winner: Option<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub player1_rps_choice: Option<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub player2_rps_choice: Option<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub pending_rps_player_id: Option<u8>,

    // Card/Ability Runtime
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub activating_card: Option<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub activating_ability_index: Option<usize>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub just_completed_ability_key: Option<u32>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub this_batch_triggered_ability_ids: Vec<u32>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub turn1_abilities_played: Vec<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub turn2_abilities_played: HashMap<String, u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub card_instance_mapping: HashMap<String, u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub card_instance_counter: u8,

    // Move Tracking
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub recently_moved_cards: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub recently_moved_from_zone: Option<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub last_vacated_stage_area: Option<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub debut_ability_triggers: Vec<DebutTriggerDisplay>,

    // Live/Cheer
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub live_cheer_count: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cheer_check_completed: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub player1_cheer_blade_heart_count: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub player2_cheer_blade_heart_count: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub player1_cheer_revealed_cards: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub player2_cheer_revealed_cards: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub revealed_cards: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub initial_yell_revealed_cards: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub re_yell_revealed_cards: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub revealed_card_info: Vec<RevealedCardDisplay>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub revealed_cost_card_info: Vec<RevealedCardDisplay>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub heart_color_decision_phase: String,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub live_owned_hearts: HashMap<String, Vec<[String; 2]>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub opponent_choice_declined: bool,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub pending_success_replacement_card_id: Option<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub pending_success_replacement_player_id: Option<String>,

    // Resolution/Misc
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub resolution_zone_cards: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub revealed_cost_cards: Vec<i16>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub ability_applications: Vec<AbilityApplicationDisplay>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub effect_creation_counter: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub last_state_change_wait_to_active_count: u8,

    // GameModifiers constant* breakdown
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub constant_blade_bonuses: HashMap<i16, i32>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub constant_cost_bonuses: HashMap<i16, i32>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub constant_score_bonuses: HashMap<i16, i32>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub constant_heart_bonuses: HashMap<i16, HashMap<String, i32>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub constant_global_need_heart: Vec<[String; 3]>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub constant_score_sources: Vec<[String; 3]>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub blade_type_modifiers: HashMap<i16, String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub heart_override: HashMap<i16, [String; 2]>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub delayed_cannot_active: HashMap<i16, u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub last_cost_discard_count: u8,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub last_cost_energy_count: u8,

    // Cheer/Blade heart tracking
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub mulligan_selected_indices: Vec<usize>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub live_success_total_score: Option<u8>,
}

pub fn card_to_display(
    card_id: i16,
    card_db: &CardDatabase,
    orientation: Option<Orientation>,
    blade_modifier: i32,
) -> Option<CardDisplay> {
    card_db.get_card(card_id).map(|card| {
        let base_heart = card.base_heart.as_ref().map(|bh| {
            bh.hearts
                .iter()
                .map(|(color, count)| (color.as_str().to_string(), *count))
                .collect()
        });
        CardDisplay {
            card_no: card.card_no.to_string(),
            name: card.name.to_string(),
            card_type: format!("{:?}", card.card_type),
            orientation: orientation.map(|o| format!("{:?}", o)),
            base_heart,
            blade: card.blade,
            total_blade: if orientation == Some(Orientation::Wait) {
                0
            } else {
                crate::constants::saturate_u8((card.blade as i32) + blade_modifier)
            },
            id: card_id,
            ability_text: Some(card.ability_text().to_string()),
            bonus_blade: blade_modifier,
            bonus_hearts: Vec::new(),
            bonus_score: 0,
            bonus_cost: 0,
            set_blade: 0,
            set_hearts: Vec::new(),
            set_score: 0,
            set_cost: 0,
            bonus_triggers: Vec::new(),
            heart_transform: None,
            hidden: false,
            cost: card.cost,
        }
    })
}

/// Build a CardDisplay with both additive and set/override modifiers.
///
/// ── Texticon status indicator summary ──
///
/// Ability Action          | bonus_* (with +/-) | set_* (plain) | bonus_triggers
/// ────────────────────────┼────────────────────┼───────────────┼───────────────
/// ModifyBlade             | bonus_blade        | —             | —
/// GainResource(blade)     | bonus_blade        | —             | —
/// SetBladeCount           | —                  | set_blade     | —
/// SetBladeType            | —                  | set_blade     | —
/// ModifyHeart             | bonus_hearts       | —             | —
/// GainResource(heart)     | bonus_hearts       | —             | —
/// SetHeartType            | —                  | set_hearts    | —
/// ModifyScore             | bonus_score        | —             | —
/// SetScore                | —                  | set_score     | —
/// ModifyCost              | bonus_cost         | —             | —
/// SetCost / SetCostToUse  | —                  | set_cost      | —
/// GainAbility(constant)   | bonus_score*       | —             | "jyouji"
/// GainAbility(live_start) | —                  | —             | "live_start"
/// GainAbility(live_success)| —                 | —             | "live_success"
/// GainAbility(debut)      | —                  | —             | "toujyou"
///
/// * GainAbility with constant trigger also adds bonus_score in
///   recalculate_constants (via score_modifier), but the trigger
///   texticon is only in bonus_triggers.
///
/// One-shot actions (Draw, Discard, Look, Reveal, Charge, Move,
/// PositionChange, ChangeState, etc.) produce NO texticon because
/// they have no persistent status on the card.
///
/// Restriction / Prohibition actions affect game rules but are tracked
/// in prohibition_effects internally, not as card texticons.
///
pub fn card_to_display_full(
    card_id: i16,
    card_db: &CardDatabase,
    orientation: Option<Orientation>,
    // Additive modifiers (accumulated via add_* / +=) — shown with +/- prefix
    blade_additive: i32,
    score_additive: i32,
    heart_additive: &HashMap<crate::card::HeartColor, i32>,
    // Absolute set/override modifiers (set via set_*) — shown without +/- prefix
    blade_set: i32,
    score_set: i32,
    heart_set: &HashMap<crate::card::HeartColor, i32>,
    cost_additive: i32,
    cost_set: i32,
    heart_transform: Option<crate::card::HeartColor>,
    // Trigger texticon badges for gained abilities (e.g. "jyouji", "live_success")
    bonus_triggers: &[String],
) -> Option<CardDisplay> {
    card_db.get_card(card_id).map(|card| {
        let base_heart = card.base_heart.as_ref().map(|bh| {
            bh.hearts
                .iter()
                .map(|(color, count)| (color.as_str().to_string(), *count))
                .collect()
        });
        // Additive hearts (shown with +/-)
        let mut bonus_hearts = vec![0i32; 8];
        for (color, &val) in heart_additive {
            if let Some(idx) = heart_color_index(color) {
                bonus_hearts[idx] += val;
            }
        }
        // Set/override hearts (shown without +/-)
        let mut set_hearts = vec![0i32; 8];
        for (color, &val) in heart_set {
            if let Some(idx) = heart_color_index(color) {
                set_hearts[idx] += val;
            }
        }
        let total_blade = if blade_set != 0 {
            crate::constants::saturate_u8(blade_set + blade_additive)
        } else {
            crate::constants::saturate_u8((card.blade as i32) + blade_additive)
        };
        let transform_str = heart_transform.map(|hc| hc.as_str().to_string());
        CardDisplay {
            card_no: card.card_no.to_string(),
            name: card.name.to_string(),
            card_type: format!("{:?}", card.card_type),
            orientation: orientation.map(|o| format!("{:?}", o)),
            base_heart,
            blade: card.blade,
            total_blade: if orientation == Some(Orientation::Wait) {
                0
            } else {
                total_blade
            },
            id: card_id,
            ability_text: Some(card.ability_text().to_string()),
            // ── Additive bonuses (shown with +/- prefix) ──────────────
            // ModifyBlade / GainResource(blade) → icon_blade.png "+N"/"-N"
            bonus_blade: blade_additive,
            // ModifyHeart / GainResource(heart) → heart_X.png "+N"/"-N"
            bonus_hearts,
            // ModifyScore / GainAbility(score)   → icon_score.png "+N"/"-N"
            // GainAbility(constant) sets this via recalculate_constants.
            bonus_score: score_additive,
            // ModifyCost → icon_energy.png "+N"/"-N"
            bonus_cost: cost_additive,
            // ── Set/override values (shown as plain number, no +/-) ──
            // SetBladeCount/SetBladeType → icon_blade.png "N"
            set_blade: blade_set,
            // SetHeartType → heart_X.png "N"
            set_hearts,
            // SetScore → icon_score.png "N"
            set_score: score_set,
            // SetCost/SetCostToUse → icon_energy.png "N"
            set_cost: cost_set,
            // ── Gained ability trigger texticons ─────────────────────
            // gain_ability → e.g. jyouji.png, live_success.png, etc.
            // Populated from game_state.gained_card_abilities.
            bonus_triggers: bonus_triggers.to_vec(),
            // ── Heart transform overlay ──────────────────────────────
            // SetHeartType("transform heart to X") → heart_X.png overlay
            heart_transform: transform_str,
            hidden: false,
            cost: card.cost,
        }
    })
}

pub fn zone_to_display(card_ids: &[i16], card_db: &CardDatabase) -> ZoneDisplay {
    ZoneDisplay {
        cards: card_ids
            .iter()
            .filter_map(|&id| card_to_display(id, card_db, None, 0))
            .collect(),
    }
}

pub fn zone_to_display_full(
    card_ids: &[i16],
    card_db: &CardDatabase,
    blade_additive: &HashMap<i16, i32>,
    blade_set: &HashMap<i16, i32>,
    score_additive: &HashMap<i16, i32>,
    score_set: &HashMap<i16, i32>,
    heart_additive: &HashMap<i16, HashMap<crate::card::HeartColor, i32>>,
    heart_set: &HashMap<i16, HashMap<crate::card::HeartColor, i32>>,
    heart_color_multiplier: &HashMap<i16, crate::card::HeartColor>,
    cost_additive: &HashMap<i16, i32>,
    cost_set: &HashMap<i16, i32>,
    bonus_triggers: &HashMap<i16, Vec<String>>,
) -> ZoneDisplay {
    ZoneDisplay {
        cards: card_ids
            .iter()
            .filter_map(|&id| {
                card_to_display_full(
                    id,
                    card_db,
                    None,
                    blade_additive.get(&id).copied().unwrap_or(0),
                    score_additive.get(&id).copied().unwrap_or(0),
                    &heart_additive.get(&id).cloned().unwrap_or_default(),
                    blade_set.get(&id).copied().unwrap_or(0),
                    score_set.get(&id).copied().unwrap_or(0),
                    &heart_set.get(&id).cloned().unwrap_or_default(),
                    cost_additive.get(&id).copied().unwrap_or(0),
                    cost_set.get(&id).copied().unwrap_or(0),
                    heart_color_multiplier.get(&id).copied(),
                    bonus_triggers
                        .get(&id)
                        .map_or(&[] as &[String], |v| v.as_slice()),
                )
            })
            .collect(),
    }
}

pub fn stage_to_display(
    stage: &crate::zones::Stage,
    card_db: &CardDatabase,
    blade_additive: &HashMap<i16, i32>,
    blade_set: &HashMap<i16, i32>,
    orientation_modifiers: &HashMap<i16, crate::core::game_modifiers::CardOrientation>,
    heart_additive: &HashMap<i16, HashMap<crate::card::HeartColor, i32>>,
    heart_set: &HashMap<i16, HashMap<crate::card::HeartColor, i32>>,
    score_additive: &HashMap<i16, i32>,
    score_set: &HashMap<i16, i32>,
    heart_color_multiplier: &HashMap<i16, crate::card::HeartColor>,
    cost_additive: &HashMap<i16, i32>,
    cost_set: &HashMap<i16, i32>,
    bonus_triggers: &HashMap<i16, Vec<String>>,
) -> StageDisplay {
    let blade_add = |cid: i16| blade_additive.get(&cid).copied().unwrap_or(0);
    let blade_set_fn = |cid: i16| blade_set.get(&cid).copied().unwrap_or(0);
    let score_add = |cid: i16| score_additive.get(&cid).copied().unwrap_or(0);
    let score_set_fn = |cid: i16| score_set.get(&cid).copied().unwrap_or(0);
    let heart_add = |cid: i16| heart_additive.get(&cid).cloned().unwrap_or_default();
    let heart_set_fn = |cid: i16| heart_set.get(&cid).cloned().unwrap_or_default();
    let heart_xform = |cid: i16| heart_color_multiplier.get(&cid).copied();
    let cost_add = |cid: i16| cost_additive.get(&cid).copied().unwrap_or(0);
    let cost_set_fn = |cid: i16| cost_set.get(&cid).copied().unwrap_or(0);
    let triggers_fn = |cid: i16| {
        bonus_triggers
            .get(&cid)
            .map_or(&[] as &[String], |v| v.as_slice())
    };
    let orientation = |cid: i16| {
        orientation_modifiers.get(&cid).map(|o| match o {
            crate::core::game_modifiers::CardOrientation::Wait => Orientation::Wait,
            _ => Orientation::Active,
        })
    };
    StageDisplay {
        left_side: if stage.stage[0] != -1 {
            card_to_display_full(
                stage.stage[0],
                card_db,
                orientation(stage.stage[0]),
                blade_add(stage.stage[0]),
                score_add(stage.stage[0]),
                &heart_add(stage.stage[0]),
                blade_set_fn(stage.stage[0]),
                score_set_fn(stage.stage[0]),
                &heart_set_fn(stage.stage[0]),
                cost_add(stage.stage[0]),
                cost_set_fn(stage.stage[0]),
                heart_xform(stage.stage[0]),
                triggers_fn(stage.stage[0]),
            )
        } else {
            None
        },
        center: if stage.stage[1] != -1 {
            card_to_display_full(
                stage.stage[1],
                card_db,
                orientation(stage.stage[1]),
                blade_add(stage.stage[1]),
                score_add(stage.stage[1]),
                &heart_add(stage.stage[1]),
                blade_set_fn(stage.stage[1]),
                score_set_fn(stage.stage[1]),
                &heart_set_fn(stage.stage[1]),
                cost_add(stage.stage[1]),
                cost_set_fn(stage.stage[1]),
                heart_xform(stage.stage[1]),
                triggers_fn(stage.stage[1]),
            )
        } else {
            None
        },
        right_side: if stage.stage[2] != -1 {
            card_to_display_full(
                stage.stage[2],
                card_db,
                orientation(stage.stage[2]),
                blade_add(stage.stage[2]),
                score_add(stage.stage[2]),
                &heart_add(stage.stage[2]),
                blade_set_fn(stage.stage[2]),
                score_set_fn(stage.stage[2]),
                &heart_set_fn(stage.stage[2]),
                cost_add(stage.stage[2]),
                cost_set_fn(stage.stage[2]),
                heart_xform(stage.stage[2]),
                triggers_fn(stage.stage[2]),
            )
        } else {
            None
        },
        left_under: stage.under_cards[0]
            .iter()
            .filter_map(|&id| card_to_display(id, card_db, None, 0))
            .collect(),
        center_under: stage.under_cards[1]
            .iter()
            .filter_map(|&id| card_to_display(id, card_db, None, 0))
            .collect(),
        right_under: stage.under_cards[2]
            .iter()
            .filter_map(|&id| card_to_display(id, card_db, None, 0))
            .collect(),
    }
}

pub fn player_to_display(
    player: &Player,
    card_db: &CardDatabase,
    // Combined modifier totals (additive + set) — used for score/stat computations
    blade_modifiers: &HashMap<i16, i32>,
    score_modifiers: &HashMap<i16, i32>,
    heart_modifiers: &HashMap<i16, HashMap<crate::card::HeartColor, i32>>,
    // ── Set/override maps ──────────────────────────────────────────
    // These hold the "set" portion of ModifierEntry (absolute overrides).
    // Extracted separately so card badges can display them without +/-.
    // See CardDisplay doc for the full additive-vs-set breakdown.
    blade_set: &HashMap<i16, i32>,
    score_set: &HashMap<i16, i32>,
    heart_set: &HashMap<i16, HashMap<crate::card::HeartColor, i32>>,
    orientation_modifiers: &HashMap<i16, crate::core::game_modifiers::CardOrientation>,
    gained_abilities: &HashMap<i16, Vec<String>>,
    need_heart_modifiers: &HashMap<i16, HashMap<crate::card::HeartColor, i32>>,
    prohibition_effects: &[String],
    cannot_activate_members: &[String],
    mulligan_selection: Option<&[usize]>,
    live_card_selection: Option<&[usize]>,
    heart_color_multiplier: &HashMap<i16, crate::card::HeartColor>,
    // Cost modifier totals (additive + set)
    cost_modifiers: &HashMap<i16, i32>,
    cost_set: &HashMap<i16, i32>,
    // ── Gained ability trigger texticon badges ─────────────────────
    // Populated from gained_card_abilities in game_state_to_display.
    // Each entry is a trigger type name → frontend renders texticon.
    // gain_ability without this would leave no icon on the card.
    bonus_triggers: &HashMap<i16, Vec<String>>,
) -> PlayerDisplay {
    let energy_cards: Vec<(i16, Option<Orientation>)> = player
        .energy_zone
        .cards
        .iter()
        .enumerate()
        .map(|(i, &card_id)| {
            let orientation = if i < player.energy_zone.active_count() as usize {
                Some(Orientation::Active)
            } else {
                Some(Orientation::Wait)
            };
            (card_id, orientation)
        })
        .collect();

    let energy_display = ZoneDisplay {
        cards: energy_cards
            .iter()
            .filter_map(|(card_id, orientation)| {
                card_to_display(*card_id, card_db, *orientation, 0)
            })
            .collect(),
    };

    let waitroom_display = zone_to_display(&player.waitroom.cards, card_db);

    // Calculate total hearts including modifiers (7 elements: heart00-heart06)
    let mut total_hearts = vec![0u8; 8];

    // Add base hearts from stage cards (accounting for heart_color_multiplier transforms)
    for &card_id in &player.stage.stage {
        if card_id == -1 {
            continue;
        }
        if let Some(card) = card_db.get_card(card_id) {
            if let Some(ref base_heart) = card.base_heart {
                if let Some(&override_color) = heart_color_multiplier.get(&card_id) {
                    // Heart transform: sum all base hearts into the override color
                    if let Some(idx) = heart_color_index(&override_color) {
                        let total: u8 = base_heart.hearts.values_sum();
                        total_hearts[idx] += total;
                    }
                } else {
                    for (color, count) in &base_heart.hearts {
                        if let Some(idx) = heart_color_index(color) {
                            total_hearts[idx] += count;
                        }
                    }
                }
            }
        }
    }

    // Add heart modifiers from stage cards
    for &card_id in &player.stage.stage {
        if card_id != -1 {
            if let Some(card_heart_modifiers) = heart_modifiers.get(&card_id) {
                for (color, modifier) in card_heart_modifiers {
                    if let Some(index) = heart_color_index(color) {
                        total_hearts[index] =
                            crate::constants::saturate_u8(total_hearts[index] as i32 + modifier);
                    }
                }
            }
        }
    }

    // Compute live_need_hearts: sum of need_heart from cards in live_card_zone
    let mut live_need_hearts = vec![0u8; 8];
    for &cid in &player.live_card_zone.cards {
        if let Some(card) = card_db.get_card(cid) {
            if let Some(ref need) = card.need_heart {
                for (color, count) in &need.hearts {
                    if let Some(idx) = heart_color_index(color) {
                        live_need_hearts[idx] += count;
                    }
                }
            }
        }
    }

    // Apply need_heart_modifiers to live_need_hearts
    for (&cid, colors) in need_heart_modifiers {
        if player.live_card_zone.cards.contains(&cid) {
            for (color, &val) in colors {
                if let Some(idx) = heart_color_index(color) {
                    live_need_hearts[idx] =
                        crate::constants::saturate_u8(live_need_hearts[idx] as i32 + val);
                }
            }
        }
    }

    // Compute selected_need_hearts: sum of need_heart from selected hand cards (preview)
    let mut selected_need_hearts = vec![0u8; 8];
    if let Some(selected) = live_card_selection {
        for &idx in selected {
            if idx < player.hand.cards.len() {
                let cid = player.hand.cards[idx];
                if let Some(card) = card_db.get_card(cid) {
                    if let Some(ref need) = card.need_heart {
                        for (color, count) in &need.hearts {
                            if let Some(ci) = heart_color_index(color) {
                                selected_need_hearts[ci] += count;
                            }
                        }
                    }
                }
                // Apply need_heart_modifiers to selected hand card
                if let Some(colors) = need_heart_modifiers.get(&cid) {
                    for (color, &val) in colors {
                        if let Some(ci) = heart_color_index(color) {
                            selected_need_hearts[ci] =
                                crate::constants::saturate_u8(selected_need_hearts[ci] as i32 + val);
                        }
                    }
                }
            }
        }
    }

    // Compute live_card_scores: card_no -> total score
    let mut live_card_scores = HashMap::default();
    for &cid in &player.live_card_zone.cards {
        if let Some(card) = card_db.get_card(cid) {
            let base = card.score.unwrap_or(0);
            let bonus = score_modifiers.get(&cid).copied().unwrap_or(0);
            live_card_scores.insert(card.card_no.to_string(), crate::constants::saturate_u8(base as i32 + bonus));
        }
    }
    for &cid in &player.success_live_card_zone.cards {
        if let Some(card) = card_db.get_card(cid) {
            let base = card.score.unwrap_or(0);
            let bonus = score_modifiers.get(&cid).copied().unwrap_or(0);
            live_card_scores.insert(card.card_no.to_string(), crate::constants::saturate_u8(base as i32 + bonus));
        }
    }

    // Compute current_score: sum of stage member base scores + all score modifiers
    let mut current_score = 0u8;
    for &cid in &player.stage.stage {
        if let Some(card) = card_db.get_card(cid) {
            current_score += card.score.unwrap_or(0) as u8;
        }
    }
    for (_, &val) in score_modifiers {
        current_score = crate::constants::saturate_u8(current_score as i32 + val);
    }

    // Collect gained abilities for this player's cards
    let player_card_ids: HashSet<i16> = player
        .stage
        .stage
        .iter()
        .chain(&player.hand.cards)
        .chain(&player.live_card_zone.cards)
        .chain(&player.success_live_card_zone.cards)
        .copied()
        .filter(|&id| id != -1)
        .collect();
    let mut my_gained: Vec<String> = Vec::new();
    for (cid, abilities) in gained_abilities {
        if player_card_ids.contains(cid) {
            for a in abilities {
                my_gained.push(format!("Card#{}: {}", cid, a));
            }
        }
    }

    // Collect need_heart_modifiers for live cards (card_no -> [h00..h06] modifiers)
    let mut nh_mods = HashMap::default();
    for (&cid, colors) in need_heart_modifiers {
        if player.live_card_zone.cards.contains(&cid)
            || player.success_live_card_zone.cards.contains(&cid)
        {
            if let Some(card) = card_db.get_card(cid) {
                let mut arr = vec![0i32; 8];
                for (color, &val) in colors {
                    if let Some(idx) = heart_color_index(color) {
                        arr[idx] = val;
                    }
                }
                nh_mods.insert(card.card_no.to_string(), arr);
            }
        }
    }

    // Collect active restrictions for this player
    let mut restrictions: Vec<String> = Vec::new();
    for pe in prohibition_effects.iter() {
        restrictions.push(pe.clone());
    }
    if cannot_activate_members
        .iter()
        .any(|t| t == "self" || t == &player.id)
    {
        restrictions.push("cannot_activate_members".to_string());
    }

    let stage_hearts_display = player.stage_hearts.as_ref().map(|sh| {
        sh.hearts
            .iter()
            .map(|(color, count)| (color.as_str().to_string(), *count))
            .collect()
    });

    // Compute additive maps = total - set (for bonus_* display)
    let blade_additive: HashMap<i16, i32> = blade_modifiers
        .iter()
        .map(|(&k, &total)| (k, total - blade_set.get(&k).copied().unwrap_or(0)))
        .collect();
    let score_additive: HashMap<i16, i32> = score_modifiers
        .iter()
        .map(|(&k, &total)| (k, total - score_set.get(&k).copied().unwrap_or(0)))
        .collect();
    let heart_additive: HashMap<i16, HashMap<crate::card::HeartColor, i32>> = heart_modifiers
        .iter()
        .map(|(&k, colors)| {
            let set_colors = heart_set.get(&k).cloned().unwrap_or_default();
            let add: HashMap<crate::card::HeartColor, i32> = colors
                .iter()
                .map(|(&c, &total)| (c, total - set_colors.get(&c).copied().unwrap_or(0)))
                .collect();
            (k, add)
        })
        .collect();
    let cost_additive: HashMap<i16, i32> = cost_modifiers
        .iter()
        .map(|(&k, &total)| (k, total - cost_set.get(&k).copied().unwrap_or(0)))
        .collect();

    let prevent_baton = if restrictions
        .iter()
        .any(|r| r.contains("cannot_baton") || r.contains("prevent_baton"))
    {
        1
    } else {
        0
    };

    PlayerDisplay {
        energy: energy_display,
        hand: zone_to_display_full(
            &player.hand.cards,
            card_db,
            &blade_additive,
            blade_set,
            &score_additive,
            score_set,
            &heart_additive,
            heart_set,
            heart_color_multiplier,
            &cost_additive,
            cost_set,
            bonus_triggers,
        ),
        stage: stage_to_display(
            &player.stage,
            card_db,
            &blade_additive,
            blade_set,
            orientation_modifiers,
            &heart_additive,
            heart_set,
            &score_additive,
            score_set,
            heart_color_multiplier,
            &cost_additive,
            cost_set,
            bonus_triggers,
        ),
        live_zone: zone_to_display_full(
            &player.live_card_zone.cards,
            card_db,
            &blade_additive,
            blade_set,
            &score_additive,
            score_set,
            &heart_additive,
            heart_set,
            heart_color_multiplier,
            &cost_additive,
            cost_set,
            bonus_triggers,
        ),
        success_live_card_zone: zone_to_display_full(
            &player.success_live_card_zone.cards,
            card_db,
            &blade_additive,
            blade_set,
            &score_additive,
            score_set,
            &heart_additive,
            heart_set,
            heart_color_multiplier,
            &cost_additive,
            cost_set,
            bonus_triggers,
        ),
        waitroom: waitroom_display.clone(),
        discard: waitroom_display,
        main_deck_count: player.main_deck.len(),
        energy_deck_count: player.energy_deck.cards.len(),
        last_resolution_cards: player
            .last_resolution_cards
            .iter()
            .filter_map(|&id| card_to_display(id, card_db, None, 0))
            .collect(),
        score_modifiers: score_modifiers.clone(),
        total_hearts,
        live_need_hearts,
        selected_need_hearts,
        current_score,
        live_card_scores,
        gained_abilities: my_gained,
        active_restrictions: restrictions.clone(),
        need_heart_modifiers: nh_mods,
        mulligan_selection: mulligan_selection.map(|v| v.to_vec()),
        live_card_selection: live_card_selection.map(|v| v.to_vec()),
        // Derive display fields from existing modifier data
        blade_buffs: player
            .stage
            .stage
            .iter()
            .map(|&cid| {
                if cid != -1 {
                    *blade_modifiers.get(&cid).unwrap_or(&0)
                } else {
                    0
                }
            })
            .collect(),
        heart_buffs: player
            .stage
            .stage
            .iter()
            .map(|&cid| {
                if cid == -1 {
                    return vec![0i32; 6];
                }
                let mut arr = vec![0i32; 6];
                if let Some(card_hm) = heart_modifiers.get(&cid) {
                    for (color, modifier) in card_hm {
                        let idx = match color {
                            crate::card::HeartColor::Heart01 => 0,
                            crate::card::HeartColor::Heart02 => 1,
                            crate::card::HeartColor::Heart03 => 2,
                            crate::card::HeartColor::Heart04 => 3,
                            crate::card::HeartColor::Heart05 => 4,
                            crate::card::HeartColor::Heart06 => 5,
                            _ => continue,
                        };
                        arr[idx] = *modifier;
                    }
                }
                arr
            })
            .collect(),
        cost_reduction: 0,
        prevent_baton_touch: prevent_baton,
        prevent_baton,
        deployed_this_turn: player.deployed_this_turn.iter().copied().collect(),
        debut_count_this_turn: player.debut_count_this_turn,
        id: player.id.clone(),
        name: player.name.to_string(),
        is_first_attacker: player.is_first_attacker,
        exclusion_zone: zone_to_display(&player.exclusion_zone.cards, card_db),
        energy_active_count: player.energy_zone.active_count() as usize,
        stage_hearts: stage_hearts_display,
    }
}

pub fn game_state_to_display(game_state: &GameState) -> GameStateDisplay {
    // Collect publicly visible revealed cards + pending_choice selection cards
    let mut looked_ids: Vec<i16> = game_state.looked_at_cards.to_vec();
    looked_ids.extend(&game_state.revealed_cards);
    #[cfg(feature = "serde_support")]
    {
        if let Some(ref pc) = game_state.get_pending_choice_json() {
            if let Some(cards) = pc.get("selection_cards").and_then(|v| v.as_array()) {
                for val in cards {
                    if let Some(id) = val
                        .get("id")
                        .and_then(|v| v.as_i64())
                        .or_else(|| val.as_i64())
                    {
                        looked_ids.push(id as i16);
                    }
                }
            }
        }
    }
    looked_ids.sort();
    looked_ids.dedup();

    // Create a mutable copy of rule_log to add ability debug logs
    let mut rule_log = game_state.rule_log.clone();
    AbDebug::flush_to_rule_log(&mut rule_log);
    // Cap rule_log to prevent unbounded growth
    if rule_log.len() > LOG_BOUND_RULE {
        rule_log.drain(0..rule_log.len() - LOG_BOUND_RULE);
    }

    // Structured log for rich UI rendering
    let mut structured_log = game_state.structured_log.clone();
    AbDebug::flush_to_structured_log(&mut structured_log, game_state.turn_number);
    if structured_log.len() > LOG_BOUND_STRUCTURED {
        structured_log.drain(0..structured_log.len() - LOG_BOUND_STRUCTURED);
    }

    // Build performance results (grouped by player_id)
    let perf_history = game_state.performance_snapshots.clone();
    let mut perf_results: Option<HashMap<String, PerformanceSnapshot>> = None;
    if !perf_history.is_empty() {
        let mut map = HashMap::default();
        for snap in &perf_history {
            map.insert(snap.player_id.clone(), snap.clone());
        }
        perf_results = Some(map);
    }

    let mulligan_player_id = match game_state.current_phase {
        crate::game_state::Phase::MulliganFirstAttacker => {
            Some(game_state.first_attacker().id.clone())
        }
        crate::game_state::Phase::MulliganSecondAttacker => {
            Some(if game_state.first_attacker().id == game_state.player1.id {
                game_state.player2.id.clone()
            } else {
                game_state.player1.id.clone()
            })
        }
        _ => None,
    };
    let mulligan_indices_usize: Vec<usize> = game_state
        .mulligan_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect();
    let live_indices_usize: Vec<usize> = game_state
        .live_card_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect();
    let p1_mulligan = mulligan_player_id
        .as_ref()
        .is_some_and(|id| *id == game_state.player1.id)
        .then_some(mulligan_indices_usize.as_slice());
    let p2_mulligan = mulligan_player_id
        .as_ref()
        .is_some_and(|id| *id == game_state.player2.id)
        .then_some(mulligan_indices_usize.as_slice());

    let live_card_player_id = match game_state.current_phase {
        crate::game_state::Phase::LiveCardSetFirstAttacker => {
            Some(game_state.first_attacker().id.clone())
        }
        crate::game_state::Phase::LiveCardSetSecondAttacker => {
            Some(if game_state.first_attacker().id == game_state.player1.id {
                game_state.player2.id.clone()
            } else {
                game_state.player1.id.clone()
            })
        }
        _ => None,
    };
    let p1_live_selection = live_card_player_id
        .as_ref()
        .is_some_and(|id| *id == game_state.player1.id)
        .then_some(live_indices_usize.as_slice());
    let p2_live_selection = live_card_player_id
        .as_ref()
        .is_some_and(|id| *id == game_state.player2.id)
        .then_some(live_indices_usize.as_slice());

    let mut blade_flat: HashMap<i16, i32> = HashMap::with_capacity_and_hasher(
        game_state.mods.blade_modifiers.len(),
        Default::default(),
    );
    let mut blade_set_flat: HashMap<i16, i32> = HashMap::with_capacity_and_hasher(
        game_state.mods.blade_modifiers.len(),
        Default::default(),
    );
    for (&k, v) in &game_state.mods.blade_modifiers {
        blade_flat.insert(k, v.total());
        blade_set_flat.insert(k, v.set as i32);
    }

    let mut score_flat: HashMap<i16, i32> = HashMap::with_capacity_and_hasher(
        game_state.mods.score_modifiers.len(),
        Default::default(),
    );
    let mut score_set_flat: HashMap<i16, i32> = HashMap::with_capacity_and_hasher(
        game_state.mods.score_modifiers.len(),
        Default::default(),
    );
    for (&k, v) in &game_state.mods.score_modifiers {
        score_flat.insert(k, v.total());
        score_set_flat.insert(k, v.set as i32);
    }

    // Gained constant abilities (gain_ability 常時 → +N live score) apply
    // through constant_score_sources / constant_total_score_bonus so they are
    // counted once in the live total. Surface the same value as the hosting
    // card's bonus_score so it renders with a normal "+N score" badge.
    for (cid, _text, val) in &game_state.mods.constant_score_sources {
        if *val != 0 {
            *score_flat.entry(*cid).or_insert(0) += *val as i32;
        }
    }

    let mut heart_flat: HashMap<i16, HashMap<crate::card::HeartColor, i32>> =
        HashMap::with_capacity_and_hasher(
            game_state.mods.heart_modifiers.len(),
            Default::default(),
        );
    let mut heart_set_flat: HashMap<i16, HashMap<crate::card::HeartColor, i32>> =
        HashMap::with_capacity_and_hasher(
            game_state.mods.heart_modifiers.len(),
            Default::default(),
        );
    for (&k, colors) in &game_state.mods.heart_modifiers {
        let total: HashMap<crate::card::HeartColor, i32> =
            colors.iter().map(|(&c, e)| (c, e.total())).collect();
        let set: HashMap<crate::card::HeartColor, i32> =
            colors.iter().map(|(&c, e)| (c, e.set as i32)).collect();
        heart_flat.insert(k, total);
        heart_set_flat.insert(k, set);
    }
    let need_heart_flat: HashMap<i16, HashMap<crate::card::HeartColor, i32>> = game_state
        .mods
        .need_heart_modifiers
        .iter()
        .map(|(&k, colors)| {
            let flat: HashMap<crate::card::HeartColor, i32> =
                colors.iter().map(|(&c, e)| (c, e.total())).collect();
            (k, flat)
        })
        .collect();

    let mut cost_flat: HashMap<i16, i32> =
        HashMap::with_capacity_and_hasher(game_state.mods.cost_modifiers.len(), Default::default());
    let mut cost_set_flat: HashMap<i16, i32> =
        HashMap::with_capacity_and_hasher(game_state.mods.cost_modifiers.len(), Default::default());
    for (&k, v) in &game_state.mods.cost_modifiers {
        cost_flat.insert(k, v.total());
        cost_set_flat.insert(k, v.set as i32);
    }

    // ── Build bonus_triggers map from gained_card_abilities ────────
    // gain_ability and gain_ability_from_source store the gained ability
    // as an Ability struct in gained_card_abilities.  Each ability has a
    // triggers field ("常時", "ライブ成功時", etc.) that maps to a
    // texticon filename via trigger_to_texticon().
    //
    // The frontend renders these as trigger texticon badges on the card.
    // Without this, gain_ability effects would leave NO visible indicator
    // that the card has a gained ability, even though the effect (e.g.
    // +1 score, extra blade) applies.
    let mut bonus_triggers: HashMap<i16, Vec<String>> = HashMap::default();
    // Also scan gained_abilities (flat strings, used by older code path)
    // for trigger keywords and record matching texticons.
    for (&card_id, texts) in &game_state.gained_abilities {
        for text in texts {
            if text.contains("【常時】") || text.contains("[Always]") || text.contains("常時")
            {
                bonus_triggers
                    .entry(card_id)
                    .or_default()
                    .push("jyouji".to_string());
            } else if text.contains("【ライブ成功時】") || text.contains("live_success") {
                bonus_triggers
                    .entry(card_id)
                    .or_default()
                    .push("live_success".to_string());
            }
        }
    }
    // Prefer structured Ability entries (LIVE_SUCCESS trigger path in
    // execute_gain_ability) over the flat text scan above.
    for (&card_id, abilities) in &game_state.gained_card_abilities {
        for ability in abilities {
            if let Some(ref triggers) = ability.triggers {
                let icon_name = crate::triggers::trigger_to_texticon(triggers);
                bonus_triggers.entry(card_id).or_default().push(icon_name);
            }
        }
    }
    // De-duplicate trigger texticons so a trigger (e.g. "常時" → jyouji) renders
    // exactly one badge no matter how many times the gained ability was recorded
    // or how many recalculate_constants passes ran.
    for triggers in bonus_triggers.values_mut() {
        let mut seen: HashSet<String> = HashSet::default();
        triggers.retain(|t| seen.insert(t.clone()));
    }
    let temp_effects: Vec<TempEffectDisplay> = game_state
        .temporary_effects
        .iter()
        .map(|te| TempEffectDisplay {
            effect_type: te.effect_type.clone(),
            duration: format!("{:?}", te.duration),
            created_turn: te.created_turn,
            target_player_id: te.target_player_id.clone(),
            description: te.description.clone(),
            #[cfg(feature = "serde_support")]
            effect_data: te
                .effect_data
                .as_ref()
                .and_then(|d| serde_json::to_value(d).ok()),
        })
        .collect();

    let repl_effects: Vec<ReplacementEffectDisplay> = game_state
        .replacement_effects
        .iter()
        .map(|re| ReplacementEffectDisplay {
            card_id: re.card_id,
            player_id: re.player_id.clone(),
            original_event: re.original_event.clone(),
            is_choice_based: re.is_choice_based,
        })
        .collect();

    let turn_phase_str = format!("{:?}", game_state.current_turn_phase);
    let game_result_str = format!("{:?}", game_state.game_result);

    // Ability queue
    let queue_entries: Vec<AbilityQueueEntryDisplay> = game_state
        .ability_queue
        .iter()
        .map(|entry| {
            let trigger_str = format!("{:?}", entry.trigger_type);
            AbilityQueueEntryDisplay {
                card_no: entry.card_no.to_string(),
                player_id: entry.player_id.clone(),
                trigger_type: trigger_str,
                completed: entry.completed,
                cost_paid: entry.cost_paid,
                effect_started: entry.effect_started,
                choice_player_id: entry.choice_player_id.clone(),
                ability_text: entry.ability.full_text.clone(),
                card_id: entry.card_id,
                ability_index: entry.ability_index,
            }
        })
        .collect();
    let queue_state_str = format!("{:?}", game_state.ability_queue.get_state());
    let queue_current_idx = match game_state.ability_queue.get_state() {
        QueueState::Idle => 0,
        QueueState::WaitingForAutoAbilityChoice { .. } => 0,
        QueueState::PayingCost { entry_index } => *entry_index as usize,
        QueueState::WaitingForChoice { entry_index, .. } => *entry_index as usize,
        QueueState::ExecutingEffect { entry_index } => *entry_index as usize,
        QueueState::Completed { entry_index } => *entry_index as usize,
    };

    // Debut triggers
    let debut_triggers: Vec<DebutTriggerDisplay> = game_state
        .debut_ability_triggers
        .iter()
        .map(|(key, cid)| DebutTriggerDisplay {
            ability_key: key.clone(),
            card_id: *cid,
        })
        .collect();

    // Ability applications
    let ability_apps: Vec<AbilityApplicationDisplay> = game_state
        .ability_applications
        .iter()
        .map(|app| AbilityApplicationDisplay {
            source_card_id: app.source_card_id,
            effect_type: app.effect_type.as_str().to_string(),
            target_card_id: app.target_card_id,
            amount: app.amount,
        })
        .collect();

    // Live owned hearts: HashMap<String, Vec<(String, u8)>> -> HashMap<String, Vec<[String; 2]>>
    let live_owned: HashMap<String, Vec<[String; 2]>> = game_state
        .live_owned_hearts
        .iter()
        .map(|(pid, pairs)| {
            let converted: Vec<[String; 2]> = pairs
                .iter()
                .map(|(color, count)| [color.clone(), count.to_string()])
                .collect();
            (pid.clone(), converted)
        })
        .collect();

    // Constant heart bonuses: HashMap<i16, HashMap<String, i32>>
    let const_heart: HashMap<i16, HashMap<String, i32>> = game_state
        .mods
        .constant_heart_bonuses
        .iter()
        .map(|(cid, map)| {
            (
                *cid,
                map.iter().map(|(k, &v)| (k.clone(), v as i32)).collect(),
            )
        })
        .collect();

    // Delayed cannot active: HashMap<i16, u8>
    let delayed_cannot: HashMap<i16, u8> = game_state.mods.delayed_cannot_active.clone();

    // Last vacated stage area
    let last_vacated = game_state.last_vacated_stage_area.map(|idx| match idx {
        0 => "LeftSide".to_string(),
        1 => "Center".to_string(),
        2 => "RightSide".to_string(),
        _ => format!("Slot{}", idx),
    });

    // Mulligan indices
    let mulligan_indices: Vec<usize> = game_state
        .mulligan_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect();

    let player1 = player_to_display(
        &game_state.player1,
        &game_state.card_database,
        &blade_flat,
        &score_flat,
        &heart_flat,
        &blade_set_flat,
        &score_set_flat,
        &heart_set_flat,
        &game_state.mods.orientation_modifiers,
        &game_state.gained_abilities,
        &need_heart_flat,
        &game_state.prohibition_effects,
        &game_state.cannot_activate_members,
        p1_mulligan,
        p1_live_selection,
        &game_state.mods.heart_color_multiplier,
        &cost_flat,
        &cost_set_flat,
        &bonus_triggers,
    );
    let player2 = player_to_display(
        &game_state.player2,
        &game_state.card_database,
        &blade_flat,
        &score_flat,
        &heart_flat,
        &blade_set_flat,
        &score_set_flat,
        &heart_set_flat,
        &game_state.mods.orientation_modifiers,
        &game_state.gained_abilities,
        &need_heart_flat,
        &game_state.prohibition_effects,
        &game_state.cannot_activate_members,
        p2_mulligan,
        p2_live_selection,
        &game_state.mods.heart_color_multiplier,
        &cost_flat,
        &cost_set_flat,
        &bonus_triggers,
    );

    GameStateDisplay {
        turn: game_state.turn_number,
        phase: format!("{:?}", game_state.current_phase),
        active_player: game_state.active_player().id.clone(),
        player1,
        player2,
        #[cfg(feature = "serde_support")]
        pending_choice: game_state.get_pending_choice_json(),
        looked_cards: zone_to_display(&looked_ids, &game_state.card_database),
        rule_log,
        structured_log,
        performance_results: perf_results,
        performance_history: perf_history,
        game_over: game_state.game_ended,
        waiting_for_opponent: false,
        mode: String::new(),
        winner: match game_state.game_result {
            crate::types::GameResult::FirstAttackerWins => {
                Some(game_state.first_attacker().id.clone())
            }
            crate::types::GameResult::SecondAttackerWins => {
                Some(if game_state.player1.is_first_attacker {
                    game_state.player2.id.clone()
                } else {
                    game_state.player1.id.clone()
                })
            }
            _ => None,
        },
        current_turn_phase: turn_phase_str,
        game_result: game_result_str,
        is_first_turn: game_state.is_first_turn,
        turn_order_changed: game_state.turn_order_changed,
        baton_touch_count: game_state.baton_touch_count_p1 + game_state.baton_touch_count_p2,
        baton_touch_zero_cost: game_state.baton_touch_zero_cost,
        baton_touch_replaced_member_cost: game_state.baton_touch_replaced_member_cost,
        baton_touch_replaced_member_id: game_state.baton_touch_replaced_member_id,
        baton_touch_arriving_card_id: game_state.baton_touch_arriving_card_id,
        deck_refresh_pending: game_state.deck_refresh_pending,
        loop_detected: game_state.loop_detected,
        draw_state: game_state.draw_state,
        live_being_performed: game_state.live_being_performed,
        cards_moved_this_turn: game_state.cards_moved_this_turn.iter().copied().collect(),
        cards_appeared_this_turn: game_state
            .cards_appeared_this_turn
            .iter()
            .copied()
            .collect(),
        areas_placed_this_turn: game_state.areas_placed_this_turn.iter().cloned().collect(),
        last_area_move_card_id: game_state.last_area_move_card_id(),
        last_area_move_by_player: game_state.last_area_move_by_player().map(|s| s.to_string()),
        last_energy_placed_by_effect: game_state.last_energy_placed_by_effect(),
        batch_movements: game_state.batch_movements.to_vec(),
        last_energy_placed_by_player: game_state
            .last_energy_placed_by_player()
            .map(|s| s.to_string()),
        position_change_occurred_this_turn: game_state.position_change_occurred_this_turn,
        position_changes: game_state.position_change_events.to_vec(),
        position_changes_by_card: {
            let mut map = HashMap::default();
            for event in &game_state.position_change_events {
                map.entry(event.moved_card_id)
                    .or_insert_with(Vec::new)
                    .push(event.clone());
            }
            map
        },
        formation_change_occurred_this_turn: game_state.formation_change_occurred_this_turn,
        opponent_live_success_this_turn: game_state.opponent_live_success_this_turn,
        opponent_live_no_excess_heart_this_turn: game_state.opponent_live_no_excess_heart_this_turn,
        self_no_excess_heart_this_turn: game_state.self_no_excess_heart_this_turn,
        opponent_live_surplus_count: game_state.opponent_live_surplus_count,
        self_live_surplus_count: game_state.self_live_surplus_count,
        live_success_triggered_this_turn: game_state.live_success_triggered_this_turn,
        live_surplus_ready_this_turn: game_state.live_surplus_ready_this_turn,
        cheer_checks_required: game_state.cheer_checks_required,
        cheer_checks_done: game_state.cheer_checks_done,
        turn_limited_abilities_used: game_state
            .turn_limited_abilities_used
            .iter()
            .map(|((card_id, ability_index, turn), _v)| {
                format!("{}_{}_{}", card_id, ability_index, turn)
            })
            .collect(),
        auto_ability_trigger_counts: game_state
            .auto_ability_trigger_counts
            .iter()
            .cloned()
            .collect(),
        turn_limit_usage: game_state.turn_limit_usage.iter().cloned().collect(),
        non_stackable_effects: game_state.non_stackable_effects.iter().cloned().collect(),
        prohibition_effects: game_state.prohibition_effects.to_vec(),
        delayed_prohibition_effects: game_state.delayed_prohibition_effects.to_vec(),
        cannot_live_players: game_state.cannot_live_players.to_vec(),
        cannot_activate_members: game_state.cannot_activate_members.to_vec(),
        constant_cannot_activate_members: game_state
            .constant_cannot_activate_members
            .iter()
            .cloned()
            .collect(),
        constant_ability_statuses: game_state.constant_ability_statuses.to_vec(),
        negated_abilities: game_state.negated_abilities.iter().copied().collect(),
        temporary_effects: temp_effects,
        replacement_effects: repl_effects,
        ability_queue_state: queue_state_str,
        ability_queue_current_index: queue_current_idx,
        ability_queue_entries: queue_entries,
        rps_winner: game_state.rps_winner,
        player1_rps_choice: game_state.player1_rps_choice,
        player2_rps_choice: game_state.player2_rps_choice,
        pending_rps_player_id: game_state.pending_rps_player_id,
        activating_card: game_state.activating_card,
        activating_ability_index: game_state.activating_ability_index,
        just_completed_ability_key: game_state.just_completed_ability_key,
        this_batch_triggered_ability_ids: game_state
            .this_batch_triggered_ability_ids
            .iter()
            .copied()
            .collect(),
        turn1_abilities_played: game_state.turn1_abilities_played.iter().cloned().collect(),
        turn2_abilities_played: game_state
            .turn2_abilities_played
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect(),
        card_instance_mapping: game_state
            .card_instance_mapping
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect(),
        card_instance_counter: game_state.card_instance_counter,
        recently_moved_cards: game_state
            .recently_moved_cards
            .clone()
            .unwrap_or_default()
            .to_vec(),
        recently_moved_from_zone: game_state.recently_moved_from_zone.clone(),
        last_vacated_stage_area: last_vacated,
        debut_ability_triggers: debut_triggers,
        live_cheer_count: game_state.live_cheer_count,
        cheer_check_completed: game_state.cheer_check_completed,
        player1_cheer_blade_heart_count: game_state.player1_cheer_blade_heart_count,
        player2_cheer_blade_heart_count: game_state.player2_cheer_blade_heart_count,
        player1_cheer_revealed_cards: game_state.player1_cheer_revealed_cards.to_vec(),
        player2_cheer_revealed_cards: game_state.player2_cheer_revealed_cards.to_vec(),
        revealed_cards: game_state.revealed_cards.to_vec(),
        revealed_card_info: game_state
            .revealed_cards
            .iter()
            .enumerate()
            .map(|(i, &cid)| {
                RevealedCardDisplay::from_card_and_meta(cid, game_state.revealed_card_meta.get(i))
            })
            .collect(),
        initial_yell_revealed_cards: game_state.initial_yell_revealed_cards.to_vec(),
        re_yell_revealed_cards: game_state.re_yell_revealed_cards.to_vec(),
        heart_color_decision_phase: game_state.heart_color_decision_phase.clone(),
        live_owned_hearts: live_owned,
        opponent_choice_declined: game_state.opponent_choice_declined,
        pending_success_replacement_card_id: game_state.pending_success_replacement_card_id,
        pending_success_replacement_player_id: game_state
            .pending_success_replacement_player_id
            .clone(),
        resolution_zone_cards: game_state.resolution_zone.cards.iter().copied().collect(),
        revealed_cost_cards: game_state.revealed_cost_cards.to_vec(),
        revealed_cost_card_info: game_state
            .revealed_cost_cards
            .iter()
            .enumerate()
            .map(|(i, &cid)| {
                RevealedCardDisplay::from_card_and_meta(
                    cid,
                    game_state.revealed_cost_card_meta.get(i),
                )
            })
            .collect(),
        ability_applications: ability_apps,
        effect_creation_counter: game_state.effect_creation_counter,
        last_state_change_wait_to_active_count: game_state.last_state_change_wait_to_active_count,
        constant_blade_bonuses: game_state
            .mods
            .constant_blade_bonuses
            .iter()
            .map(|(&k, &v)| (k, v as i32))
            .collect(),
        constant_cost_bonuses: game_state
            .mods
            .constant_cost_bonuses
            .iter()
            .map(|(&k, &v)| (k, v as i32))
            .collect(),
        constant_score_bonuses: game_state
            .mods
            .constant_score_bonuses
            .iter()
            .map(|(&k, &v)| (k, v as i32))
            .collect(),
        constant_heart_bonuses: const_heart,
        constant_global_need_heart: game_state
            .mods
            .constant_global_need_heart
            .iter()
            .map(|(cid, s, v)| [cid.to_string(), s.clone(), v.to_string()])
            .collect(),
        constant_score_sources: game_state
            .mods
            .constant_score_sources
            .iter()
            .map(|(cid, s, v)| [cid.to_string(), s.clone(), v.to_string()])
            .collect(),
        blade_type_modifiers: game_state
            .mods
            .blade_type_modifiers
            .iter()
            .map(|(k, v)| (*k, format!("{:?}", v)))
            .collect(),
        heart_override: game_state
            .mods
            .heart_override
            .iter()
            .map(|(k, (hc, v))| (*k, [format!("{:?}", hc), v.to_string()]))
            .collect(),
        delayed_cannot_active: delayed_cannot,
        last_cost_discard_count: game_state.mods.last_cost_discard_count,
        last_cost_energy_count: game_state.mods.last_cost_energy_count,
        mulligan_selected_indices: mulligan_indices,
        live_success_total_score: None,
    }
}
