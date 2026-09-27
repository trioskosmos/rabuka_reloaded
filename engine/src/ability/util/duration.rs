// Duration helpers + temporary-effect registration.
//
// Extracted from `ability/util.rs` so expiry semantics live in one place.

use crate::game_state::Duration;

pub fn parse_duration(s: &str) -> Option<Duration> {
    match s {
        "this_turn" => Some(Duration::ThisTurn),
        "live_end" => Some(Duration::LiveEnd),
        "as_long_as" => Some(Duration::AsLongAs),
        "unless" => Some(Duration::Unless),
        "permanent" => Some(Duration::Permanent),
        _ => None,
    }
}

/// Effect kinds that GameState::check_expired_effects knows how to REVERT.
/// Registering a temporary effect outside this set means its modifiers will
/// silently leak past expiry — a new kind must extend BOTH this list and the
/// expiry match. push_temporary_effect warns on violations.
pub(crate) fn is_revertable_effect_type(effect_type: &str) -> bool {
    matches!(
        effect_type,
        "activation_cost_increase"
            | "activation_cost_decrease"
            | "set_blade_count"
            | "gain_surplus_heart"
            | "heart_override"
            | "modify_cost"
            | "set_heart_type"
    ) || effect_type.starts_with("gain_blade")
        || effect_type.starts_with("gain_heart")
        || effect_type.starts_with("gain_ability:")
        || effect_type.starts_with("set_blade_type:")
        || effect_type.starts_with("modify_score_")
}

pub fn push_temporary_effect(
    game_state: &mut crate::game_state::GameState,
    effect_type: &str,
    duration: Option<&str>,
    target_player_id: &str,
    description: &str,
    effect_data: Option<crate::core::types::EffectData>,
) {
    if let Some(d) = duration {
        if d != "permanent" {
            let Some(duration) = parse_duration(d) else {
                log::error!(
                    "unsupported duration code '{}' for temporary effect '{}': {}",
                    d,
                    effect_type,
                    description
                );
                return;
            };
            if !is_revertable_effect_type(effect_type) {
                log::warn!(
                    "temporary effect type '{}' has no expiry revert handler; \
                     its modifiers will LEAK past expiry. Extend \
                     GameState::check_expired_effects and is_revertable_effect_type. \
                     description={}",
                    effect_type,
                    description
                );
            }
            game_state
                .temporary_effects
                .push(crate::game_state::TemporaryEffect {
                    effect_type: effect_type.to_string(),
                    duration,
                    created_turn: game_state.turn_number,
                    created_phase: game_state.current_phase,
                    target_player_id: target_player_id.to_string(),
                    description: description.to_string(),
                    creation_order: 0,
                    effect_data,
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_duration_codes() {
        assert_eq!(parse_duration("this_turn"), Some(Duration::ThisTurn));
        assert_eq!(parse_duration("live_end"), Some(Duration::LiveEnd));
        assert_eq!(parse_duration("as_long_as"), Some(Duration::AsLongAs));
        assert_eq!(parse_duration("unless"), Some(Duration::Unless));
        assert_eq!(parse_duration("permanent"), Some(Duration::Permanent));
    }

    #[test]
    fn rejects_noncanonical_duration_codes() {
        assert_eq!(parse_duration("turn_end"), None);
        assert_eq!(parse_duration("this_live"), None);
        assert_eq!(parse_duration("until_used"), None);
        assert_eq!(parse_duration("next_turn"), None);
    }
}
