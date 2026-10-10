use crate::game_state::{AbilityTrigger, GameState};
use crate::types::LogEntry;
use crate::HashSet;
use smallvec::SmallVec;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl super::TurnEngine {
    /// Queue the 登場 abilities of a member that just debuted.
    ///
    /// `card_id` is the id of the card that ACTUALLY debuted. It is authoritative and takes
    /// precedence over `card_no`: a deck may hold two copies of one print, and matching on
    /// the card NUMBER alone resolves the debut to whichever copy sits in the lowest area
    /// index — so the second copy of a card to debut silently re-ran the first copy's  ability
    /// (and, if that copy was already in wait, its optional self-wait cost found no active
    /// candidate and was never offered to the player at all).
    ///
    /// `card_no` remains as a fallback for callers that only know the print. `None` card_no
    /// is a hard no-op: there is nothing to match on.
    pub(crate) fn trigger_debut_abilities(
        game_state: &mut GameState,
        player_id: &str,
        card_no: &str,
        _cost_paid: u8,
        baton_touch_used: bool,
        card_id: Option<i16>,
    ) {
        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("trig::debut");
        // (ability_index, card_no, card_id): the index identifies the
        // ability on the card, so the enqueue goes through the
        // by-index entry point instead of formatting a
        // `{card_no}_{full_text}` key per matched ability. The index
        // comes from `trigger_ability_indices`, which is built from
        // `has_trigger(Debut)`, so it is exactly the ability the
        // string key used to re-find by full text (no card in the
        // corpus carries two abilities with the same full text).
        let mut abilities_to_trigger: SmallVec<[(usize, crate::core::types::ArcStr, i16); 4]> = SmallVec::new();

        let card_id_opt = card_id;
        {
            let player = if player_id == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            let areas = [
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            ];
            for area in areas {
                let Some(area_card_id) = player.stage.get_area(area) else {
                    continue;
                };
                // Prefer the exact instance that debuted. Only fall back to the print number
                // when the caller could not name an id — matching on the number alone is
                // ambiguous with duplicate copies (see the doc comment).
                if let Some(debuted) = card_id {
                    if area_card_id != debuted {
                        continue;
                    }
                } else if let Some(card) = game_state.card_database.get_card(area_card_id) {
                    if card.card_no.as_ref() != card_no {
                        continue;
                    }
                } else {
                    continue;
                }
                let Some(card) = game_state.card_database.get_card(area_card_id) else {
                    continue;
                };
                let card_id = area_card_id;
                log::trace!(
                    "[DEBUT_SCAN] player={} source={} ({}, id={}) requested={} by_id={}",
                    player_id,
                    card.name,
                    card.card_no,
                    card_id,
                    card_no,
                    card_id_opt.is_some()
                );
                // The index is built from `has_trigger(Debut)`,
                // so every ability it yields carries the 登場
                // trigger — the per-ability `has_trigger` test
                // this loop used to run is already answered.
                let debut_indices = game_state
                    .card_database
                    .trigger_ability_indices(card_id, crate::triggers::TriggerKind::Debut);
                for &ability_index_u16 in debut_indices {
                    let ability_index = usize::from(ability_index_u16);
                    let Some(ability_ref) = card.abilities.get(ability_index) else {
                        continue;
                    };
                    let ability = ability_ref.resolve();
                    log::log!(
                        log::Level::Debug,
                        "[TRIGGER_MATCH] player={} source={} ({}, id={}) ability={} trigger=Debut match=true triggers={:?}",
                        player_id, card.name, card.card_no, card_id,
                        ability_index, ability.triggers
                    );
                    if !crate::zones::check_effect_position(
                        ability
                            .effect
                            .as_ref()
                            .and_then(|effect| effect.activation_position_any()),
                        area,
                    ) {
                        continue;
                    }
                    if !baton_touch_used
                        && ability
                            .effect
                            .as_ref()
                            .and_then(|effect| effect.condition.as_ref())
                            .is_some_and(|condition| {
                                condition.get_baton_touch_trigger().unwrap_or(false)
                            })
                    {
                        continue;
                    }
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(core::sync::atomic::Ordering::Relaxed)
                    {
                        GameState::push_structured_log_to(
                            &mut game_state.structured_log,
                            LogEntry {
                                text: format!("{} {} [ステージ]: 能力確認 [登場]", player_id, card.name),
                                turn: game_state.turn_number,
                                player_label: player_id.to_string(),
                                source_card_id: Some(card_id),
                                source_card_name: Some(card.name.to_string()),
                                category: "trigger_evaluation".to_string(),
                                metadata: Some(
                                    crate::core::types::LogMetadata::TriggerEvaluation {
                                        trigger: "debut".to_string(),
                                        zone: "stage".to_string(),
                                        result: "pending".to_string(),
                                        ability_index,
                                        ability_text: ability.full_text.clone(),
                                    },
                                ),
                            },
                        );
                    }
                    abilities_to_trigger.push((
                        ability_index,
                        card.card_no.clone(),
                        card_id,
                    ));
                }
                // No `break` here: the loop is bounded to the three areas, and with a
                // specific `card_id` only the area holding it can match. Breaking on the
                // first match was only ever correct because matching was by print number,
                // where the first match WAS the answer — which is exactly what mis-resolved
                // duplicate copies.
            }
        }

        let moved_snapshot = game_state.recently_moved_cards.clone();
        for (ability_index, card_no, stage_card_id) in abilities_to_trigger {
            game_state.trigger_auto_ability_by_index(
                AbilityTrigger::Debut,
                player_id,
                Some(&*card_no),
                Some(stage_card_id),
                ability_index,
                moved_snapshot.as_ref(),
                None,
            );
        }
    }

    pub fn is_trigger_suppressed(game_state: &GameState, player_id: &str, trigger_name: &str) -> bool {
        let card_database = &game_state.card_database;
        // Fast path: no card in the pool carries a
        // SuppressAbilityTrigger ability, so no board card
        // can either. The pool currently has zero such
        // cards, so this returns before the board walk —
        // the old code resolved every ability of every
        // stage and live card on every live start to
        // answer the same question.
        if card_database.suppress_cards.is_empty() {
            return false;
        }
        let player = if player_id == game_state.player1.id {
            &game_state.player1
        } else {
            &game_state.player2
        };
        player
            .stage
            .stage
            .iter()
            .chain(player.live_card_zone.cards.iter())
            .copied()
            .filter(|&card_id| card_id != -1)
            .filter(|&card_id| card_database.has_suppress_ability(card_id))
            .filter_map(|card_id| card_database.get_card(card_id))
            .any(|card| {
                card.abilities.iter().any(|ability_ref| {
                    ability_ref.resolve().effect.as_ref().is_some_and(|effect| {
                        effect.action == crate::ability::enums::ActionType::SuppressAbilityTrigger
                            && effect.suppressed_trigger_any() == Some(trigger_name)
                    })
                })
            })
    }

    pub fn trigger_live_start_abilities(game_state: &mut GameState, player_id: &str) {
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::trace!(
                "[LIVE_START_SCAN] player={} phase={:?} turn_phase={:?}",
                player_id,
                game_state.current_phase,
                game_state.current_turn_phase
            );
        }
        if Self::is_trigger_suppressed(game_state, player_id, "live_start") {
            log::debug!(
                "[LIVE_START_SUPPRESSED] live_start abilities suppressed for player {}",
                player_id
            );
            return;
        }
        let player = if player_id == game_state.player1.id {
            &game_state.player1
        } else {
            &game_state.player2
        };
        if game_state.current_phase == crate::game_state::Phase::FirstAttackerPerformance
            && !player.is_first_attacker
            && player.live_card_zone.cards.is_empty()
        {
            return;
        }

        // (ability_index, card_no, explicit_card_id) — see the
        // debut scan: the index identifies the ability, so the
        // enqueue skips the `{card_no}_{full_text}` format.
        let mut abilities_to_trigger: Vec<(usize, String, Option<i16>)> = Vec::new();
        // Track (card_id, ability_idx) to prevent the same card's ability from
        // firing twice when the card is both a stage member AND a live card.
        let mut seen: HashSet<(i16, usize)> = HashSet::default();

        {
            let player = if player_id == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            log::trace!(
                "[LIVE_START_SCAN] player={} live_zone_cards={:?} stage={:?}",
                player_id,
                player.live_card_zone.cards,
                player.stage.stage
            );
            let live_cards = player.live_card_zone.cards.iter().map(|&id| (id, None));
            let stage_cards = player.stage.stage.iter().enumerate().map(|(index, &id)| {
                let area = crate::zones::MemberArea::from_index(index)
                    .unwrap_or(crate::zones::MemberArea::RightSide);
                (id, Some(area))
            });
            for (card_id, position) in live_cards.chain(stage_cards) {
                if position.is_none() {
                    log::trace!(
                        "[LIVE_START_SCAN] checking card={} negated={}",
                        card_id,
                        game_state.is_ability_invalidated(card_id, &AbilityTrigger::LiveStart)
                    );
                }
                if (position.is_some() && card_id == -1)
                    || game_state.is_ability_invalidated(card_id, &AbilityTrigger::LiveStart)
                {
                    continue;
                }
                let Some(card) = game_state.card_database.get_card(card_id) else {
                    continue;
                };
                let (zone, zone_label) = if position.is_some() {
                    ("stage", "ステージ")
                } else {
                    ("live_card_zone", "ライブ置場")
                };
                // The index is built from `has_trigger(LiveStart)`,
                // so every ability it yields carries the trigger —
                // the per-ability `has_trigger` test this loop used
                // to run is already answered.
                let live_start_indices = game_state
                    .card_database
                    .trigger_ability_indices(card_id, crate::triggers::TriggerKind::LiveStart);
                for &ability_index_u16 in live_start_indices {
                    let ability_index = usize::from(ability_index_u16);
                    let Some(ability_ref) = card.abilities.get(ability_index) else {
                        continue;
                    };
                    let ability = ability_ref.resolve();
                    if position.is_some_and(|area| {
                        !crate::zones::check_effect_position(
                            ability
                                .effect
                                .as_ref()
                                .and_then(|effect| effect.activation_position_any()),
                            area,
                        )
                    }) {
                        continue;
                    }
                    if position.is_none() {
                        log::trace!(
                            "[LIVE_START_SCAN] source={} ({}, id={}) ability={} triggers={:?}",
                            card.name,
                            card.card_no,
                            card_id,
                            ability_index,
                            ability.triggers
                        );
                    }
                    if !seen.insert((card_id, ability_index)) {
                        continue;
                    }
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(core::sync::atomic::Ordering::Relaxed)
                    {
                        GameState::push_structured_log_to(
                            &mut game_state.structured_log,
                            LogEntry {
                                text: format!(
                                    "{} {} [{zone_label}]: 能力確認 [ライブ開始時]",
                                    player_id, card.name
                                ),
                                turn: game_state.turn_number,
                                player_label: player_id.to_string(),
                                source_card_id: Some(card_id),
                                source_card_name: Some(card.name.to_string()),
                                category: "trigger_evaluation".to_string(),
                                metadata: Some(
                                    crate::core::types::LogMetadata::TriggerEvaluation {
                                        trigger: "live_start".to_string(),
                                        zone: zone.to_string(),
                                        result: "pending".to_string(),
                                        ability_index,
                                        ability_text: ability.full_text.clone(),
                                    },
                                ),
                            },
                        );
                    }
                    abilities_to_trigger.push((
                        ability_index,
                        card.card_no.to_string(),
                        Some(card_id),
                    ));
                }
            }
        }

        log::debug!(
            "[LIVE_START_TRIGGER] player={} matched={} enqueuing",
            player_id,
            abilities_to_trigger.len()
        );
        game_state.push_debug_note_fmt(format_args!(
            "live_start scan {}: {} ability(ies) fired",
            player_id,
            abilities_to_trigger.len()
        ));
        for (ability_index, card_no, explicit_card_id) in abilities_to_trigger {
            log::trace!(
                "[LIVE_START_TRIGGER] enqueue ability={} card_no={}",
                ability_index,
                card_no
            );
            game_state.trigger_auto_ability_by_index(
                AbilityTrigger::LiveStart,
                player_id,
                Some(card_no.as_str()),
                explicit_card_id,
                ability_index,
                None,
                None,
            );
        }
    }

    pub fn trigger_auto_abilities_for_player(game_state: &mut GameState, player_id: &str) {
        log::trace!("[AUTO_SCAN] checking stage for player {}", player_id);
        // Delegate to GameState's method, which handles the scan + enqueue
        game_state.trigger_auto_abilities_for_player(player_id);
    }

    pub fn trigger_live_success_abilities(game_state: &mut GameState, player_id: &str) {
        // Evaluate constant modify_required_hearts abilities on cards in the
        // success_live_card_zone before checking live success conditions.
        game_state.evaluate_success_zone_constant_abilities();

        // Restore performance-time need_heart_modifiers that were cleared above.
        // This preserves modifications from live_start triggers and other non-constant
        // sources, ensuring should_trigger_live_success uses the correct requirements.
        game_state.restore_performance_need_heart_modifiers();

        // Printed abilities are keyed by index (see the debut
        // scan); gained abilities keep the
        // `{card_no}_gained_{idx}` string key because
        // `enqueue_gained_ability` parses the gained index
        // back out of it.
        let mut printed_to_trigger: Vec<(usize, String, i16)> = Vec::new();
        let mut gained_to_trigger: Vec<(String, String, i16)> = Vec::new();
        let mut seen: HashSet<(i16, usize)> = HashSet::default();

        // LiveSuccess only triggers when the live card's need_heart is satisfied
        if !game_state.should_trigger_live_success(if player_id == game_state.player1.id {
            &game_state.player1
        } else {
            &game_state.player2
        }) {
            return;
        }

        {
            let player = if player_id == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            let stage_cards = [
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            ]
            .into_iter()
            .enumerate()
            .map(|(index, area)| (player.stage.stage[index], Some(area)));
            let live_cards = player.live_card_zone.cards.iter().map(|&id| (id, None));
            for (card_id, position) in stage_cards.chain(live_cards) {
                if position.is_some()
                    && (card_id == -1
                        || game_state.is_ability_invalidated(card_id, &AbilityTrigger::LiveSuccess))
                {
                    continue;
                }
                let Some(card) = game_state.card_database.get_card(card_id) else {
                    continue;
                };
                let card_no = card.card_no.to_string();
                let (zone, zone_label) = if position.is_some() {
                    ("stage", "ステージ")
                } else {
                    ("live_card_zone", "ライブ置場")
                };
                // The index is built from `has_trigger(LiveSuccess)`,
                // so every ability it yields carries the trigger —
                // the per-ability `has_trigger` test this loop used
                // to run is already answered.
                let live_success_indices = game_state
                    .card_database
                    .trigger_ability_indices(card_id, crate::triggers::TriggerKind::LiveSuccess);
                for &ability_index_u16 in live_success_indices {
                    let ability_index = usize::from(ability_index_u16);
                    let Some(ability_ref) = card.abilities.get(ability_index) else {
                        continue;
                    };
                    let ability = ability_ref.resolve();
                    if position.is_some_and(|area| {
                        !crate::zones::check_effect_position(
                            ability
                                .effect
                                .as_ref()
                                .and_then(|effect| effect.activation_position_any()),
                            area,
                        )
                    }) {
                        continue;
                    }
                    if !seen.insert((card_id, ability_index)) {
                        continue;
                    }
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(core::sync::atomic::Ordering::Relaxed)
                    {
                        GameState::push_structured_log_to(
                            &mut game_state.structured_log,
                            LogEntry {
                                text: format!(
                                    "{} {} [{zone_label}]: 能力確認 [ライブ成功時]",
                                    player_id, card.name
                                ),
                                turn: game_state.turn_number,
                                player_label: player_id.to_string(),
                                source_card_id: Some(card_id),
                                source_card_name: Some(card.name.to_string()),
                                category: "trigger_evaluation".to_string(),
                                metadata: Some(
                                    crate::core::types::LogMetadata::TriggerEvaluation {
                                        trigger: "live_success".to_string(),
                                        zone: zone.to_string(),
                                        result: "pending".to_string(),
                                        ability_index,
                                        ability_text: ability.full_text.clone(),
                                    },
                                ),
                            },
                        );
                    }
                    printed_to_trigger.push((
                        ability_index,
                        card_no.clone(),
                        card_id,
                    ));
                }
                let Some(gained_list) = game_state.gained_card_abilities.get(&card_id) else {
                    continue;
                };
                for (gained_index, ability) in gained_list.iter().enumerate() {
                    if position.is_some_and(|area| {
                        !crate::zones::check_effect_position(
                            ability
                                .effect
                                .as_ref()
                                .and_then(|effect| effect.activation_position_any()),
                            area,
                        )
                    }) {
                        continue;
                    }
                    let ability_index =
                        crate::ability::types::GAINED_ABILITY_INDEX_BASE + gained_index;
                    if !ability.has_trigger(crate::triggers::TriggerKind::LiveSuccess)
                        || !seen.insert((card_id, ability_index))
                    {
                        continue;
                    }
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(core::sync::atomic::Ordering::Relaxed)
                    {
                        GameState::push_structured_log_to(
                            &mut game_state.structured_log,
                            LogEntry {
                                text: format!(
                                    "{} card#{card_id} [success/獲得]: 能力確認 [ライブ成功時]",
                                    player_id
                                ),
                                turn: game_state.turn_number,
                                player_label: player_id.to_string(),
                                source_card_id: Some(card_id),
                                source_card_name: None,
                                category: "trigger_evaluation".to_string(),
                                metadata: Some(crate::core::types::LogMetadata::TriggerEvaluation {
                                    trigger: "live_success".to_string(),
                                    zone: "success_gained".to_string(),
                                    result: "pending".to_string(),
                                    ability_index,
                                    ability_text: ability.full_text.clone(),
                                }),
                            },
                        );
                    }
                    gained_to_trigger.push((
                        format!("{}_gained_{}", card_no, gained_index),
                        card_no.clone(),
                        card_id,
                    ));
                }
            }
        }

        for (ability_index, card_no, source_card_id) in printed_to_trigger {
            game_state.trigger_auto_ability_by_index(
                AbilityTrigger::LiveSuccess,
                player_id,
                Some(card_no.as_str()),
                Some(source_card_id),
                ability_index,
                None,
                None,
            );
        }

        for (ability_id, card_no, source_card_id) in gained_to_trigger {
            game_state.trigger_auto_ability(
                ability_id,
                AbilityTrigger::LiveSuccess,
                player_id,
                Some(card_no),
                Some(source_card_id),
                None,
                None,
            );
        }
    }
}
