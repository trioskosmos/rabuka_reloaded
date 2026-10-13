use crate::core::game_state::GameState;
use crate::core::types::{Duration, TurnPhase, ReplacementEffect};
use smallvec::SmallVec;
// `String`/`Vec` (the replacement-effect signature) and
// `ToString` (the player-id string) are std-prelude types; under
// `no_std` they need the gated `alloc` import.
#[cfg(feature = "no_std")]
use alloc::{string::{String, ToString}, vec::Vec};

impl GameState {
    /// Has `effect` run out of time?
    ///
    /// `AsLongAs` and `Unless` have no condition re-evaluation implemented, so
    /// both approximate to "ends with the live" and say so loudly when they
    /// actually fire — an arm that is unreachable today must not become a
    /// silent wrong answer the day a caller starts passing one.
    fn effect_has_expired(&self, effect: &crate::core::types::TemporaryEffect) -> bool {
        match effect.duration {
            Duration::LiveEnd | Duration::ThisLive => self.current_turn_phase != TurnPhase::Live,
            Duration::ThisTurn => self.turn_number > effect.created_turn,
            Duration::Permanent => false,
            Duration::AsLongAs | Duration::Unless => {
                log::warn!(
                    "{:?} temporary effect expired via live-end approximation \
                     (condition re-eval not implemented): {}",
                    effect.duration,
                    effect.description
                );
                self.current_turn_phase != TurnPhase::Live
            }
        }
    }

    /// Is this ability invalidation still in force?
    ///
    /// The inverse shape of [`Self::effect_has_expired`]: an invalidation is
    /// KEPT while its window is open, rather than dropped when one closes.
    /// Takes the two clock values rather than `&self` so the `retain` that
    /// calls it does not have to borrow the whole GameState.
    fn invalidation_still_active(phase: TurnPhase, turn: u8, entry: &crate::core::types::AbilityInvalidation) -> bool {
        match entry.duration {
            Duration::LiveEnd | Duration::ThisLive | Duration::AsLongAs | Duration::Unless => {
                phase == TurnPhase::Live
            }
            Duration::ThisTurn => turn == entry.created_turn,
            Duration::Permanent => true,
        }
    }

    /// Is this duration one that only lasts as long as the live does?
    fn is_live_scoped(duration: Duration) -> bool {
        matches!(
            duration,
            Duration::LiveEnd | Duration::ThisLive | Duration::AsLongAs | Duration::Unless
        )
    }

    /// Drop the state that belongs to a finished live, but only when a
    /// live-scoped effect is what expired. A ThisTurn effect expiring between
    /// turns must NOT wipe the multiplier mid-live.
    fn clear_finished_live_state(&mut self, expired: &[usize]) {
        let any_live_scoped_expired = expired
            .iter()
            .any(|&i| Self::is_live_scoped(self.temporary_effects[i].duration));
        if !any_live_scoped_expired {
            return;
        }
        self.mods.heart_color_multiplier.clear();
        // LiveEnd-scoped cheer-check state: the base and all
        // modify_yell_count modifiers belong to the finished live.
        self.cheer_check_base = None;
        self.yell_count_modifiers.clear();
    }

    /// Undo one expired effect's modifiers.
    ///
    /// Returns true when the effect was a gained ability: that is the one case
    /// whose removal invalidates a cached constant and so forces a
    /// `recalculate_constants` afterwards.
    fn revert_expired_effect(&mut self, effect: &crate::core::types::TemporaryEffect) -> bool {
        match effect.effect_type.as_str() {
            "activation_cost_increase" | "activation_cost_decrease" => {
                self.prohibition_effects
                    .retain(|p| !p.contains(&effect.effect_type));
            }
            "set_blade_count" => {
                if let Some(ref data) = effect.effect_data {
                    if let Some(card_id) = data.card_id() {
                        self.mods.clear_blade_set_modifier(card_id);
                        log::debug!("Cleared set_blade_count modifier for card {}", card_id);
                    }
                }
            }
            s if s.starts_with("gain_blade") => {
                if let Some(ref data) = effect.effect_data {
                    for item in data.items() {
                        self.mods.remove_blade_modifier(item.card_id, item.amount);
                        log::debug!(
                            "Reverted {} blades from card {}",
                            item.amount,
                            item.card_id
                        );
                    }
                }
            }
            "gain_surplus_heart" => {
                if let Some(ref data) = effect.effect_data {
                    if let Some(old) = data.old_value() {
                        let is_p1 = data.is_p1().unwrap_or(true);
                        if is_p1 {
                            self.self_live_surplus_count = old;
                        } else {
                            self.opponent_live_surplus_count = old;
                        }
                        log::debug!("Restored surplus count (is_p1={}) to {}", is_p1, old);
                    }
                }
            }
            s if s.starts_with("gain_heart") => {
                if let Some(ref data) = effect.effect_data {
                    for item in data.items() {
                        let color_str = item.color.unwrap_or("heart01");
                        let color = crate::card::parse_heart_color(color_str);
                        self.mods
                            .remove_heart_modifier(item.card_id, color, item.amount);
                        log::debug!(
                            "Reverted {} hearts from card {} (color {:?})",
                            item.amount,
                            item.card_id,
                            color
                        );
                    }
                }
            }
            "heart_override" => {
                if let Some(ref data) = effect.effect_data {
                    if let Some(card_id) = data.card_id() {
                        self.mods.remove_heart_override(card_id);
                        log::debug!("Removed heart override for card {}", card_id);
                    }
                }
            }
            "modify_cost" => {
                if let Some(ref data) = effect.effect_data {
                    for item in data.items() {
                        self.mods.remove_cost_modifier(item.card_id, item.amount);
                        log::debug!(
                            "Reverted cost modifier {} from card {}",
                            item.amount,
                            item.card_id
                        );
                    }
                }
            }
            s if s.starts_with("gain_ability:") => {
                // Structured path: the registration stashed the owning
                // card + immediate-application info in effect_data, so
                // revert exactly what was applied. Only per-card score
                // gains get an immediate modifier reverted; live-total
                // gains were never applied per card (they live in the
                // p*_constant_total_score_bonus accumulator and expire
                // with the gained_card_abilities entry itself).
                if let Some(crate::core::types::EffectData::GainAbility {
                    card_id,
                    amount,
                    is_live_total,
                }) = effect.effect_data.as_ref()
                {
                    if !is_live_total && *amount != 0 {
                        self.mods.remove_score_modifier(*card_id, *amount);
                        log::debug!(
                            "Reverted gained ability score modifier +{} for card {}",
                            amount,
                            card_id
                        );
                    }
                    self.clear_gained_abilities_for_card(*card_id);
                }
                return true;
            }
            "set_heart_type" => {
                if let Some(ref data) = effect.effect_data {
                    if let Some(card_id) = data.card_id() {
                        self.mods.heart_color_multiplier.remove(&card_id);
                        log::debug!("Removed heart color multiplier for card {}", card_id);
                    }
                }
            }
            s if s.starts_with("set_blade_type:") => {
                if let Some(ref data) = effect.effect_data {
                    if let Some(card_id) = data.card_id() {
                        self.mods.clear_blade_type_modifier(card_id);
                        log::debug!("Cleared blade type modifier for card {}", card_id);
                    }
                }
            }
            s if s.starts_with("modify_score_") => {
                if let Some(ref data) = effect.effect_data {
                    for item in data.items() {
                        if s == "modify_score_set" {
                            self.mods.clear_score_set_modifier(item.card_id);
                            log::debug!("Cleared score set modifier for card {}", item.card_id);
                        } else {
                            self.mods.remove_score_modifier(item.card_id, item.amount);
                            log::debug!(
                                "Removed score modifier {} from card {}",
                                item.amount,
                                item.card_id
                            );
                        }
                    }
                }
            }
            _ => {
                // An effect kind with no revert arm means its modifiers
                // LEAK past expiry. Loud on purpose -- extend this match.
                log::warn!(
                    "expired temporary effect '{}' has no revert handler; \
                     its modifiers were NOT reverted. description={}",
                    effect.effect_type,
                    effect.description
                );
            }
        }
        false
    }

    /// Clear every list that only means something while a live is running.
    fn clear_live_only_state(&mut self) {
        if self.current_turn_phase == TurnPhase::Live {
            return;
        }
        // e.g. "cannot_live", wait immunities, activation-cost prohibitions.
        self.prohibition_effects.clear();
        self.cannot_live_players.clear();
        self.wait_immune_members.clear();
    }

    pub fn check_expired_effects(&mut self) {
        // A live-total gain_ability registered a 常時 into gained_card_abilities
        // and cached its +1 into p*_constant_total_score_bonus. Expiring must
        // re-derive that accumulator AFTER the registration is cleared, or the
        // cached bonus outlives the ability itself (PL!S-bp3-001-R＋: the +1
        // persisted into turn 2 after ライブ終了時まで). Mirrors the gain side,
        // which refreshes constants right after registering.
        let mut expired_gain_ability = false;

        // The expiry list is almost always empty or a single
        // entry, and this runs on every `execute_and_settle` —
        // a `Vec` allocated (and immediately dropped) per call
        // for a count that fits inline. `temporary_effects` is
        // itself a small per-game window, so 8 covers it without
        // a spill.
        let expired_indices: SmallVec<[usize; 8]> = self
            .temporary_effects
            .iter()
            .enumerate()
            .filter(|(_, effect)| self.effect_has_expired(effect))
            .map(|(i, _)| i)
            .collect();

        let phase = self.current_turn_phase;
        let turn = self.turn_number;
        self.ability_invalidations
            .retain(|entry| Self::invalidation_still_active(phase, turn, entry));

        self.clear_finished_live_state(&expired_indices);

        // Reverse order so each index stays valid as earlier ones are removed.
        for i in expired_indices.into_iter().rev() {
            let effect = self.temporary_effects.remove(i);
            expired_gain_ability |= self.revert_expired_effect(&effect);
        }

        self.clear_live_only_state();

        // Refresh AFTER removals + zone-exit clears: the cached
        // p*_constant_total_score_bonus must reflect the post-expiry
        // constant landscape (see expired_gain_ability above).
        if expired_gain_ability {
            self.recalculate_constants();
        }
    }

    pub fn add_replacement_effect(
        &mut self,
        card_id: i16,
        player_id: impl Into<crate::core::player::PlayerId>,
        original_event: String,
        replacement_effects: Vec<crate::card::AbilityEffect>,
        is_choice_based: bool,
    ) {
        self.replacement_effects.push(ReplacementEffect {
            card_id,
            player_id: player_id.into().to_string(),
            original_event,
            replacement_effects,
            is_choice_based,
            applied_this_event: false,
        });
    }

    pub fn reset_replacement_effect_flags(&mut self) {
        for effect in &mut self.replacement_effects {
            effect.applied_this_event = false;
        }
    }

    pub fn mark_replacement_effect_applied(&mut self, index: usize) {
        if let Some(effect) = self.replacement_effects.get_mut(index) {
            effect.applied_this_event = true;
        }
    }
}
