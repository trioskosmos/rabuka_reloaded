use crate::constants::MAX_LIVE_CARDS;
use crate::game_state::{GameState, Phase};
use crate::types::LogEntry;
use crate::HashMap;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "3ds")]
extern "C" {
    fn _3ds_tdbg(msg: *const u8);
}

#[cfg(feature = "3ds")]
macro_rules! tdbg {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s = format!("{}\0", msg);
        unsafe { _3ds_tdbg(s.as_ptr()); }
    }};
}
#[cfg(not(feature = "3ds"))]
macro_rules! tdbg {
    ($($arg:tt)*) => {};
}

impl super::TurnEngine {
    /// Log a phase transition to both rule_log and structured_log.
    /// Uses [[key]] translatable markers for bilingual frontend rendering.
    fn log_phase(game_state: &mut GameState, marker_key: &str) {
        let text = format!("[[{}]]", marker_key);
        game_state.push_rule_log(text.clone());
        game_state.push_structured_log(LogEntry {
            text,
            turn: game_state.turn_number,
            player_label: "SYSTEM".into(),
            source_card_id: None,
            source_card_name: None,
            category: "phase_transition".to_string(),
            metadata: None,
        });
    }

    /// Log the start of a new turn.
    /// Uses [[turn_start:turn=N]] translatable marker.
    fn log_turn_start(game_state: &mut GameState) {
        let text = format!("[[turn_start:turn={}]]", game_state.turn_number);
        game_state.push_rule_log(text.clone());
        game_state.push_structured_log(LogEntry {
            text,
            turn: game_state.turn_number,
            player_label: "SYSTEM".into(),
            source_card_id: None,
            source_card_name: None,
            category: "turn_transition".to_string(),
            metadata: Some(crate::core::types::LogMetadata::TurnStart {
                turn: game_state.turn_number,
            }),
        });
    }

    pub fn advance_phase(game_state: &mut GameState) {
        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("advance_phase");
        debug_assert!(
            game_state.phase_invariant(),
            "Phase invariant violated before advance_phase"
        );

        if matches!(
            game_state.current_phase,
            Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker
        ) {
            return;
        }

        if game_state.current_turn_phase == crate::game_state::TurnPhase::FirstAttackerNormal
            || game_state.current_turn_phase == crate::game_state::TurnPhase::SecondAttackerNormal
        {
            match game_state.current_phase {
                Phase::Active => {
                    tdbg!("PHASE_ACTIVE:0");
                    game_state.reset_keyword_tracking();
                    tdbg!("PHASE_ACTIVE:1 reset_keyword_tracking OK");
                    // recalculate_constants skipped — check_timing at the end of
                    // this block calls it internally.
                    // Q135: Weighed members become active during the active phase (7.4.1).
                    // Rule 7.4.1: Only the turn player activates their wait cards.
                    // Q180: "cannot_activate_by_effect" restrictions (e.g. PL!-pb1-009-R 矢澤にこ
                    // ab#1) only block effect-based activation, not the natural Active phase
                    // rule. Therefore cannot_activate_members is NOT checked here.
                    // Per-card constant_cannot_activate_members (e.g. "このメンバーはアクティブ
                    // フェイズにアクティブにしない") still applies.
                    let turn_player = game_state.active_player();
                    let to_activate: Vec<i16> = turn_player
                        .stage
                        .stage
                        .iter()
                        .filter_map(|&cid| {
                            if cid == -1 {
                                return None;
                            }
                            if game_state.mods.get_orientation_modifier(cid) != Some("wait") {
                                return None;
                            }
                            // Skip members with a constant cannot_activate restriction
                            // (per-card, e.g. "このメンバーはアクティブフェイズにアクティブにしない")
                            if game_state
                                .constant_cannot_activate_members
                                .contains(&cid.to_string())
                            {
                                return None;
                            }
                            // Skip members with an active delayed_cannot_active flag
                            if game_state.mods.is_delayed_cannot_active(cid) {
                                return None;
                            }
                            Some(cid)
                        })
                        .collect();
                    tdbg!("PHASE_ACTIVE:3 wait_activate {} cards", to_activate.len());
                    for &cid in &to_activate {
                        game_state.mods.add_orientation_modifier(cid, "active");
                    }
                    tdbg!("PHASE_ACTIVE:4 wait activated");
                    game_state.mods.tick_delayed_cannot_active();
                    tdbg!("PHASE_ACTIVE:5 tick_delayed OK");
                    game_state.active_player_mut().activate_all_energy();
                    tdbg!("PHASE_ACTIVE:6 activate_all_energy OK");
                    // Orientation + energy activation changed → constant outputs stale.
                    game_state.mark_constants_dirty();
                    Self::check_timing(game_state);
                    tdbg!("PHASE_ACTIVE:7 check_timing OK");
                    Self::log_phase(game_state, "phase_energy");
                    tdbg!("PHASE_ACTIVE:8 log_phase OK");
                    game_state.current_phase = Phase::Energy;
                    tdbg!("PHASE_ACTIVE:DONE");
                }
                Phase::Energy => {
                    tdbg!("PHASE_ENERGY:0");
                    let _drawn_card = game_state.active_player_mut().draw_energy();
                    game_state.mark_constants_dirty();
                    Self::check_timing(game_state);
                    Self::log_phase(game_state, "phase_draw");
                    game_state.current_phase = Phase::Draw;
                }
                Phase::Draw => {
                    Self::check_timing(game_state);
                    let _drawn = game_state.active_player_mut().draw_card();
                    // recalculate_constants skipped — check_timing below calls it
                    game_state.mark_constants_dirty();
                    Self::check_timing(game_state);
                    Self::log_phase(game_state, "phase_main");
                    game_state.current_phase = Phase::Main;
                }
                Phase::Main => {
                    tdbg!("PHASE_MAIN:0");
                    Self::check_timing(game_state);
                    if game_state.current_turn_phase
                        == crate::game_state::TurnPhase::FirstAttackerNormal
                    {
                        Self::log_phase(game_state, "phase_active_second");
                        game_state.current_turn_phase =
                            crate::game_state::TurnPhase::SecondAttackerNormal;
                        game_state.current_phase = Phase::Active;
                    } else {
                        Self::log_phase(game_state, "phase_live_set");
                        game_state.current_turn_phase = crate::game_state::TurnPhase::Live;
                        game_state.current_phase = Phase::LiveCardSetFirstAttacker;
                    }
                }
                _ => {}
            }
        } else if game_state.current_turn_phase == crate::game_state::TurnPhase::Live {
            match game_state.current_phase {
                // Q72: Live card set phase can be done even with no members on stage.
                Phase::LiveCardSetFirstAttacker => {
                    game_state.current_phase = Phase::LiveCardSetSecondAttacker;
                }
                // Q104: When discarding N from top of deck with fewer than N cards,
                // refresh in between (handled by draw/peek functions automatically).
                Phase::LiveCardSetSecondAttacker => {
                    game_state.player1.live_card_set_limit_reduction = 0;
                    game_state.player2.live_card_set_limit_reduction = 0;
                    tdbg!("PHASE_LIVE:0");
                    Self::check_timing(game_state);
                    Self::log_phase(game_state, "phase_performance_first");
                    game_state.current_phase = Phase::FirstAttackerPerformance;
                    let first_attacker_id = game_state.first_attacker().id.clone();
                    let second_attacker_id = game_state.second_attacker().id.clone();
                    // Trigger ALL LiveStart abilities for BOTH players before processing,
                    // so that if one player's processing creates a choice (e.g. SelectAutoAbility
                    // from multiple abilities), the other player's abilities are already queued
                    // and will be processed when the choice resolves.
                    Self::trigger_live_start_abilities(game_state, &first_attacker_id);
                    Self::trigger_live_start_abilities(game_state, &second_attacker_id);
                    // each_time LIVE_START triggers fire post-resolution
                    // in process_current_ability (abilities.rs)
                    // Now process both players' abilities
                    game_state.process_pending_auto_abilities(&first_attacker_id);
                    if game_state.has_pending_choice() {
                        return;
                    }
                    game_state.process_pending_auto_abilities(&second_attacker_id);
                    if game_state.has_pending_choice() {
                        return;
                    }
                }
                Phase::FirstAttackerPerformance => {
                    Self::execute_performance_phase(game_state, true);
                }
                Phase::SecondAttackerPerformance => {
                    Self::execute_performance_phase(game_state, false);
                }
                Phase::LiveVictoryDetermination => {
                    Self::execute_live_victory_determination(game_state);
                    if game_state.has_pending_choice() {
                        return;
                    }
                    game_state.clear_revealed_cards();
                    game_state.revealed_cost_cards.clear();
                    game_state.revealed_cost_card_meta.clear();
                    game_state.turn_limited_abilities_used.clear();
                    game_state.cannot_activate_members.clear();
                    game_state.cannot_live_players.clear();
                    game_state.turn_number += 1;
                    Self::log_turn_start(game_state);
                    Self::log_phase(game_state, "phase_active_first");
                    game_state.current_turn_phase =
                        crate::game_state::TurnPhase::FirstAttackerNormal;
                    game_state.current_phase = Phase::Active;
                    game_state.clear_card_movement_tracking();
                    game_state.check_expired_effects();
                }
                _ => {}
            }
        }
    }

    fn execute_performance_phase(game_state: &mut GameState, is_first: bool) {
        let mut resolution_zone = core::mem::take(&mut game_state.resolution_zone);
        // Take snapshots of modifier state BEFORE auto-ability triggers
        // (these are type-converted flat copies, not references — no borrow conflict)
        let hm: HashMap<i16, HashMap<crate::card::HeartColor, i32>> = game_state
            .mods
            .heart_modifiers
            .iter()
            .map(|(&k, colors)| {
                let flat: HashMap<crate::card::HeartColor, i32> =
                    colors.iter().map(|(&c, e)| (c, e.total())).collect();
                (k, flat)
            })
            .collect();
        let nhm: HashMap<
            i16,
            HashMap<crate::card::HeartColor, crate::core::game_modifiers::ModifierEntry>,
        > = game_state
            .mods
            .need_heart_modifiers
            .iter()
            .map(|(&k, colors)| {
                let flat: HashMap<
                    crate::card::HeartColor,
                    crate::core::game_modifiers::ModifierEntry,
                > = colors.iter().map(|(&c, e)| (c, *e)).collect();
                (k, flat)
            })
            .collect();
        let nhm_flat: Vec<(
            i16,
            crate::card::HeartColor,
            crate::core::game_modifiers::ModifierEntry,
        )> = nhm
            .iter()
            .flat_map(|(&k, colors)| colors.iter().map(move |(&c, e)| (k, c, *e)))
            .collect();
        let player_id = if is_first {
            game_state.player1.id.clone()
        } else {
            game_state.player2.id.clone()
        };
        let cannot_live = game_state.cannot_live_players.contains(&player_id);
        let performer_id = player_id.clone();

        // Phase A: yell + blade heart (rules 8.3.10-8.3.12).
        // Borrow game_state fields directly (no clones) within a scope so the
        // borrows drop before later mutable game_state methods.
        let mut yell_data = {
            let card_db = &game_state.card_database;
            let bm = &game_state.mods.blade_modifiers;
            let ho = &game_state.mods.heart_override;
            let btm = &game_state.mods.blade_type_modifiers;
            let om = &game_state.mods.orientation_modifiers;
            let hcm = &game_state.mods.heart_color_multiplier;
            let player = if is_first {
                &mut game_state.player1
            } else {
                &mut game_state.player2
            };
            Self::player_perform_live(
                player,
                &mut resolution_zone,
                &performer_id,
                card_db,
                bm,
                ho,
                &hm,
                btm,
                om,
                &nhm,
                hcm,
                cannot_live,
            )
        };

        let turn = game_state.turn_number;
        let note_icons = yell_data.note_icons;

        // Collect revealed card IDs from resolution zone before success check drains it
        let revealed_ids: Vec<i16> = resolution_zone.cards.iter().copied().collect();
        log::debug!(
            "[YELL_DEBUG] pid={} is_first={} revealed_ids={:?}",
            player_id,
            is_first,
            revealed_ids
        );
        let yell_owner: Option<u8> = if performer_id == game_state.player1.id {
            Some(0)
        } else {
            Some(1)
        };
        for cid in &revealed_ids {
            game_state.push_revealed_card(*cid, None, false, yell_owner);
        }
        // Save initial yell cards BEFORE auto abilities fire, since
        // re-yell abilities may discard them (clearing revealed_cards).
        // Only save once — P2 performance would overwrite with empty.
        if game_state.initial_yell_revealed_cards.is_empty() {
            game_state.initial_yell_revealed_cards = game_state.revealed_cards.clone();
        }
        for cid in &revealed_ids {
            game_state.cheer_revealed_cards_first(is_first).push(*cid);
        }
        *game_state.cheer_blade_heart_count_mut(is_first) = note_icons;

        // If cards moved from live_card_zone to waitroom during yell phase
        // (cannot_live path), set recently_moved_cards so the 8.3.13 check
        // captures auto abilities that trigger on that zone change (e.g. Riko BP6).
        if !yell_data.moved_live_card_ids.is_empty() {
            game_state.recently_moved_cards = Some(yell_data.moved_live_card_ids.clone());
        }

        // Rule 8.3.13: Check timing — auto abilities fire here.
        // "When you yell" abilities grant hearts that feed into 8.3.14.
        // Set flag so on_yell abilities know a yell actually happened.
        game_state.yell_occurred = !revealed_ids.is_empty();
        game_state.trigger_auto_abilities_for_player(&performer_id);
        game_state.process_pending_auto_abilities(&performer_id);
        game_state.yell_occurred = false;

        // Rule 8.3.13.1: If a re-yell occurred, replace yell data with newly-revealed cards.
        if game_state.re_yell_occurred {
            // Save re-yell revealed cards for display
            game_state.re_yell_revealed_cards = game_state.revealed_cards.clone();
            let card_db = &game_state.card_database;
            let mut new_yell_cards: Vec<crate::core::types::YellCardResult> = Vec::new();
            let mut new_total_hearts = [0u32; 8];
            let mut new_cheer_count = 0u32;
            for &cid in &game_state.revealed_cards {
                if let Some(card) = card_db.get_card(cid) {
                    let mut bh = [0u32; 8];
                    let mut notes = 0u32;
                    if let Some(ref bheart) = card.blade_heart {
                        for (color, count) in &bheart.hearts {
                            use crate::card::HeartColor;
                            match color {
                                HeartColor::Draw => {}
                                HeartColor::Score => {
                                    notes += count;
                                    new_cheer_count += count;
                                }
                                _ => {
                                    let idx = color.index();
                                    if idx < 8 {
                                        bh[idx] += count;
                                        new_total_hearts[idx] += count;
                                    }
                                }
                            }
                        }
                    }
                    new_yell_cards.push(crate::core::types::YellCardResult {
                        card_id: cid,
                        blade_hearts: bh,
                        note_icons: notes,
                        draw_icons: 0,
                        card_no: card.card_no.to_string().into(),
                    });
                }
            }
            yell_data.yell_cards = new_yell_cards;
            yell_data.total_hearts = new_total_hearts;
            yell_data.note_icons = new_cheer_count;
            game_state.re_yell_occurred = false;
        }

        // Capture current heart modifiers (includes ability-granted hearts from
        // the 8.3.13 check timing) for use during the live success check.
        // Only heart_modifiers needs a flat copy (type conversion); others borrow directly.
        let current_hm: HashMap<i16, HashMap<crate::card::HeartColor, i32>> = game_state
            .mods
            .heart_modifiers
            .iter()
            .map(|(&k, colors)| (k, colors.iter().map(|(&c, e)| (c, e.total())).collect()))
            .collect();

        // Rule 8.3.14-8.3.16: Heart calculation + live success check.
        let perf_data = {
            let current_ho = &game_state.mods.heart_override;
            let current_hcm = &game_state.mods.heart_color_multiplier;
            let player = if is_first {
                &mut game_state.player1
            } else {
                &mut game_state.player2
            };
            Self::check_live_success(
                player,
                &mut resolution_zone,
                &game_state.card_database,
                &nhm,
                current_ho,
                &current_hm,
                current_hcm,
                &yell_data.live_card_ids,
                &yell_data.allocations,
                &yell_data.yell_cards,
                yell_data.yell_count,
                yell_data.note_icons,
                &yell_data.member_contributions,
                &yell_data.total_hearts,
                &yell_data.heart_sources,
                &yell_data.blade_sources,
            )
        };
        // player borrow ends here — game_state accessible again

        // If live cards moved to waitroom during the heart/live check
        // (requirement failure path), set recently_moved_cards and re-check
        // auto abilities so zone-change triggers fire (e.g. Riko BP6).
        if !perf_data.moved_live_card_ids.is_empty() {
            game_state.recently_moved_cards = Some(perf_data.moved_live_card_ids.clone());
            game_state.trigger_auto_abilities_for_player(&performer_id);
            game_state.process_pending_auto_abilities(&performer_id);
        }

        drop(resolution_zone);
        let (perf_player_id, _perf_player) = if is_first {
            let p = game_state.first_attacker();
            (p.id.clone(), p)
        } else {
            let p = game_state.second_attacker();
            (p.id.clone(), p)
        };
        // Enrich member contributions from ability_applications before snapshot
        let mut mc = perf_data.member_contributions.clone();
        let mut bd = crate::types::Breakdown {
            hearts: perf_data.heart_sources.clone(),
            blades: perf_data.blade_sources.clone(),
            allocations: perf_data.allocations.clone(),
            requirements: Vec::new(),
            transforms: Vec::new(),
            scores: Vec::new(),
        };
        let mut tas = Vec::new();
        // Rule: split ability_applications by performer. LiveStart abilities fire for
        // BOTH players before the first performance phase; consuming all at once would
        // leave the second player's snapshot empty. Only enrich apps whose source or
        // target card belongs to the current performer.
        let performer_owned_ids: Vec<i16> = {
            let p = if perf_player_id == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            let mut ids = Vec::new();
            for &cid in p.stage.stage.iter() {
                if cid != -1 {
                    ids.push(cid);
                }
            }
            for &cid in p.live_card_zone.cards.iter() {
                if cid != -1 {
                    ids.push(cid);
                }
            }
            for &cid in p.success_live_card_zone.cards.iter() {
                if cid != -1 {
                    ids.push(cid);
                }
            }
            ids
        };
        let all_apps = core::mem::take(&mut game_state.ability_applications);
        let mut apps = Vec::new();
        let mut other_apps = Vec::new();
        for app in all_apps {
            let belongs = (app.target_card_id != -1
                && performer_owned_ids.contains(&app.target_card_id))
                || (app.source_card_id != -1 && performer_owned_ids.contains(&app.source_card_id));
            if belongs {
                apps.push(app);
            } else {
                other_apps.push(app);
            }
        }
        game_state.ability_applications = other_apps;
        crate::turn::live::enrich_from_applications(
            &mut mc,
            &mut bd,
            &mut tas,
            &apps,
            &game_state.card_database,
        );

        // Also collect draw effect triggered ability
        if perf_data.draw_effects_occurred {
            tas.push(crate::types::TriggeredAbility {
                source_card_id: -1,
                name: "Draw Effect".to_string(),
                card_name: crate::types::ArcStr::default(),
                effect_text: "カードを引く効果が発動しました".to_string().into(),
                condition_text: None,
                is_public: true,
            });
        }

        let mut snap = crate::turn::live::build_snapshot(
            turn,
            &perf_player_id,
            &perf_data,
            &game_state.card_database,
            note_icons,
            &nhm_flat,
        );
        // Add constant score source info into breakdown.scores
        {
            let stage_cards: Vec<i16> = if is_first {
                game_state.player1.stage.stage.to_vec()
            } else {
                game_state.player2.stage.stage.to_vec()
            };
            for (cid, text, val) in &game_state.mods.constant_score_sources {
                if stage_cards.contains(cid) {
                    bd.scores.push(crate::types::ScoreLine {
                        source: text.clone(),
                        value: val.unsigned_abs(),
                    });
                }
            }
        }

        // Replace placeholder data with enriched versions from ability_applications
        snap.member_contributions = mc;
        snap.breakdown = bd;
        snap.triggered_abilities = tas;
        game_state.push_performance_snapshot(snap);
        let pid = perf_player_id;
        Self::trigger_auto_abilities_for_player(game_state, &pid);
        game_state.process_pending_auto_abilities(&pid);
        // Also scan the opponent's auto-abilities (e.g. "when opponent performs a live")
        let opponent_id = if pid == game_state.player1.id {
            game_state.player2.id.clone()
        } else {
            game_state.player1.id.clone()
        };
        Self::trigger_auto_abilities_for_player(game_state, &opponent_id);
        game_state.process_pending_auto_abilities(&opponent_id);
        if perf_data.draw_effects_occurred {
            Self::trigger_auto_abilities_for_player(game_state, &pid);
            game_state.process_pending_auto_abilities(&pid);
            Self::trigger_auto_abilities_for_player(game_state, &opponent_id);
            game_state.process_pending_auto_abilities(&opponent_id);
        }
        if is_first {
            Self::log_phase(game_state, "phase_performance_second");
            game_state.current_phase = Phase::SecondAttackerPerformance;
        } else {
            Self::log_phase(game_state, "phase_victory");
            game_state.current_phase = Phase::LiveVictoryDetermination;
        };
    }

    // Q16: First player determined by RPS. Q17: First player mulligans first.
    // Q18: Only one mulligan per player. Q19: Mulligan is optional (can skip).
    pub(crate) fn handle_mulligan_selection(
        game_state: &mut GameState,
        card_id: Option<i16>,
        _card_indices: Option<Vec<usize>>,
    ) -> Result<(), String> {
        let idx = if let Some(indices) = _card_indices {
            indices.first().copied().unwrap_or(0)
        } else if let Some(cid) = card_id {
            game_state
                .active_player()
                .get_card_index_by_id(cid)
                .unwrap_or(0)
        } else {
            0
        };
        if let Some(pos) = game_state
            .mulligan_selected_indices
            .iter()
            .position(|&x| x == idx)
        {
            game_state.mulligan_selected_indices.remove(pos);
        } else {
            game_state.mulligan_selected_indices.push(idx);
        }
        Ok(())
    }

    pub(crate) fn handle_mulligan_confirmation(
        game_state: &mut GameState,
        card_indices: Option<Vec<usize>>,
    ) -> Result<(), String> {
        let is_first_turn_active = game_state.current_phase == Phase::MulliganSecondAttacker;
        let next_phase = match game_state.current_phase {
            Phase::MulliganFirstAttacker => Phase::MulliganSecondAttacker,
            Phase::MulliganSecondAttacker => Phase::Active,
            _ => return Ok(()),
        };
        // Use provided indices (from PVP/local selection) or fallback to server state
        let mulligan_indices =
            card_indices.unwrap_or_else(|| game_state.mulligan_selected_indices.to_vec());
        // Sort descending so removals don't shift other targets
        let mut sorted_indices = mulligan_indices.clone();
        sorted_indices.sort_unstable();
        sorted_indices.dedup();
        let mut removed_count = 0;
        let player = game_state.active_player_mut();
        for &idx in sorted_indices.iter().rev() {
            if idx < player.hand.cards.len() {
                let card = player.hand.cards.remove(idx);
                player.main_deck.cards.push(card);
                removed_count += 1;
            }
        }
        player.main_deck.shuffle();
        for _ in 0..removed_count {
            if let Some(card) = player.main_deck.draw() {
                player.hand.add_card(card);
            }
        }
        game_state.mulligan_selected_indices.clear();
        if is_first_turn_active && next_phase == Phase::Active {
            Self::log_turn_start(game_state);
            Self::log_phase(game_state, "phase_active_first");
        }
        game_state.current_phase = next_phase;
        log::debug!("Mulligan confirmed: {} cards mulliganed", removed_count);
        Ok(())
    }

    pub(crate) fn handle_mulligan_skip(game_state: &mut GameState) -> Result<(), String> {
        let is_first_turn_active = game_state.current_phase == Phase::MulliganSecondAttacker;
        game_state.mulligan_selected_indices.clear();
        let next_phase = match game_state.current_phase {
            Phase::MulliganFirstAttacker => Phase::MulliganSecondAttacker,
            Phase::MulliganSecondAttacker => Phase::Active,
            _ => return Ok(()),
        };
        if is_first_turn_active && next_phase == Phase::Active {
            Self::log_turn_start(game_state);
            Self::log_phase(game_state, "phase_active_first");
        }
        game_state.current_phase = next_phase;
        Ok(())
    }

    pub(crate) fn handle_set_live_card(
        game_state: &mut GameState,
        card_id: Option<i16>,
    ) -> Result<(), String> {
        let cid = card_id.ok_or("No card selected for live card set")?;
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!(
                "[SET_LIVE] phase={:?} cid={}",
                game_state.current_phase,
                cid
            );
        }
        let player = game_state.active_player_mut();
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!(
                "[SET_LIVE] hand.len={} live_zone_before={:?}",
                player.hand.cards.len(),
                player.live_card_zone.cards
            );
        }
        let idx = player
            .get_card_index_by_id(cid)
            .ok_or("Selected card not found in hand")?;
        if !player.hand.cards.is_empty() && idx < player.hand.cards.len() {
            let card = player.hand.cards.remove(idx);
            let live_cards = &mut player.live_card_zone.cards;
            if live_cards.len() >= MAX_LIVE_CARDS {
                return Err("Live card zone is full".to_string());
            }
            live_cards.push(card);
            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                log::debug!(
                    "[SET_LIVE] live_zone_after={:?}",
                    player.live_card_zone.cards
                );
            }
            Ok(())
        } else {
            Err("Invalid card selection".to_string())
        }
    }

    pub(crate) fn handle_live_card_selection(
        game_state: &mut GameState,
        card_id: Option<i16>,
        _card_indices: Option<Vec<usize>>,
    ) -> Result<(), String> {
        let idx = if let Some(indices) = _card_indices {
            indices.first().copied().unwrap_or(0)
        } else if let Some(cid) = card_id {
            game_state
                .active_player()
                .get_card_index_by_id(cid)
                .unwrap_or(0)
        } else {
            0
        };
        if let Some(pos) = game_state
            .live_card_selected_indices
            .iter()
            .position(|&x| x == idx)
        {
            game_state.live_card_selected_indices.remove(pos);
        } else {
            let player = game_state.active_player();
            let reduction = i32::try_from(player.live_card_set_limit_reduction).unwrap_or(0);
            let max_allowed = (MAX_LIVE_CARDS as i32 - reduction).max(0) as usize;
            if game_state.live_card_selected_indices.len() >= max_allowed {
                return Err("Cannot select more live cards: limit reached".to_string());
            }
            game_state.live_card_selected_indices.push(idx);
        }
        Ok(())
    }

    pub(crate) fn handle_live_card_confirmation(
        game_state: &mut GameState,
        card_indices: Option<Vec<usize>>,
    ) -> Result<(), String> {
        let is_second = matches!(game_state.current_phase, Phase::LiveCardSetSecondAttacker);
        let live_indices = card_indices
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| game_state.live_card_selected_indices.to_vec());
        let mut sorted_indices: Vec<usize> = live_indices.clone();
        sorted_indices.sort_unstable();
        sorted_indices.dedup();
        let player = game_state.active_player_mut();
        let max_live = MAX_LIVE_CARDS - player.live_card_zone.cards.len();
        let mut placed = 0usize;
        for &idx in sorted_indices.iter().rev() {
            if placed >= max_live {
                break;
            }
            if idx < player.hand.cards.len() {
                let card = player.hand.cards.remove(idx);
                player.live_card_zone.cards.push(card);
                placed += 1;
            }
        }
        for _ in 0..placed {
            let _ = player.draw_card();
        }
        game_state.live_card_selected_indices.clear();
        if is_second {
            Self::advance_phase(game_state);
        } else {
            game_state.current_phase = Phase::LiveCardSetSecondAttacker;
        }
        log::debug!("Live card confirmation: {} cards placed", placed);
        Ok(())
    }

    pub(crate) fn handle_live_card_skip(game_state: &mut GameState) -> Result<(), String> {
        game_state.live_card_selected_indices.clear();
        match game_state.current_phase {
            Phase::LiveCardSetFirstAttacker => {
                game_state.current_phase = Phase::LiveCardSetSecondAttacker;
            }
            Phase::LiveCardSetSecondAttacker => {
                Self::advance_phase(game_state);
            }
            _ => {}
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    // Q70: An area that had a member placed in it this turn cannot have another
    // member placed in it by any means. Q71: If the member leaves the area,
    // the now-empty area can receive a new member the same turn.
    // Q87: Baton touch can be performed multiple times per turn, but a member
    // who entered via baton touch cannot baton touch again that turn.
    pub fn handle_play_member_to_stage(
        game_state: &mut GameState,
        card_id: Option<i16>,
        card_indices: Option<Vec<usize>>, // For double baton: [area1_idx, area2_idx]
        stage_area: Option<crate::zones::MemberArea>,
        use_baton_touch: Option<bool>,
    ) -> Result<(), String> {
        let use_baton_touch = use_baton_touch.unwrap_or(false);

        // Clear stale baton touch state from any previous action this turn
        game_state.clear_baton_touch_tracking();

        let card_db = game_state.card_database.clone();

        // Recalculate constant cost modifiers (hand-based cost reductions, etc.)
        // BEFORE paying cost, so the modifiers are in effect.
        tdbg!("PHASE_EXEC:0 recalc");
        game_state.mark_constants_dirty();
        game_state.recalculate_constants();
        tdbg!("PHASE_EXEC:1 recalc OK");

        let player = game_state.active_player_mut();
        let idx = if let Some(cid) = card_id {
            player
                .get_card_index_by_id(cid)
                .ok_or_else(|| format!("Card with id {} not found in hand", cid))?
        } else {
            player
                .hand
                .cards
                .iter()
                .position(|c| card_db.get_card(*c).is_some_and(|card| card.is_member()))
                .ok_or_else(|| "No member cards in hand".to_string())?
        };

        let card_id = player.hand.cards[idx];

        // Check if double baton: card_indices provides the 2 area indices to replace
        let double_baton_areas: Option<[crate::zones::MemberArea; 2]> =
            card_indices.as_ref().and_then(|indices| {
                if indices.len() == 2 {
                    let areas = [
                        crate::zones::MemberArea::LeftSide,
                        crate::zones::MemberArea::Center,
                        crate::zones::MemberArea::RightSide,
                    ];
                    Some([areas[indices[0]], areas[indices[1]]])
                } else {
                    None
                }
            });

        let area = if let Some(ref db_areas) = double_baton_areas {
            // For double baton, stage_area specifies which of the 2 vacated areas to place in
            stage_area.unwrap_or(db_areas[0])
        } else {
            stage_area.unwrap_or_else(|| {
                let areas = [
                    crate::zones::MemberArea::LeftSide,
                    crate::zones::MemberArea::Center,
                    crate::zones::MemberArea::RightSide,
                ];
                if let Some(empty) = areas.iter().find(|&&a| player.stage.get_area(a).is_none()) {
                    *empty
                } else if !use_baton_touch {
                    areas[0]
                } else {
                    areas[0]
                }
            })
        };

        let card_no = card_db
            .get_card(card_id)
            .map(|c| c.card_no.to_string())
            .unwrap_or_default();
        let player_id = player.id.clone();

        // If double baton with explicit areas (card_indices from UI), replace ALL specified members
        // BEFORE placing the card. Single baton via the area buttons stays single — the
        // constant ability (play_baton_touch, count>1) is offered as separate gold buttons.
        if let Some(db_areas) = double_baton_areas {
            // Calculate cost before modifying state
            let card_entry = card_db.get_card(card_id);
            let card_cost = card_entry.and_then(|c| c.cost).unwrap_or(0);
            let replaced_costs: Vec<u32> = {
                let player = game_state.active_player();
                db_areas
                    .iter()
                    .filter_map(|&area| {
                        player
                            .stage
                            .get_area(area)
                            .and_then(|cid| card_db.get_card(cid))
                            .and_then(|c| c.cost)
                    })
                    .collect()
            };
            let combined_reduction: u32 = replaced_costs.iter().sum();
            let hand_count = game_state.active_player().hand.cards.len();
            let stage = &game_state.active_player().stage;
            let success_zone = &game_state.active_player().success_live_card_zone.cards;
            let cost_reduction = crate::ability::util::calculate_play_cost_reduction(
                stage,
                success_zone,
                hand_count,
                card_id,
                &card_db,
            );
            let final_cost = card_cost
                .saturating_sub(cost_reduction)
                .saturating_sub(combined_reduction);
            if final_cost > 0 {
                let player = game_state.active_player_mut();
                if player.energy_zone.active_count() < final_cost as usize {
                    return Err("Not enough energy to play this card".to_string());
                }
                player.energy_zone.pay_energy(final_cost as usize)?;
            }
            // Check cannot_baton_touch protection for each target member
            {
                let player = game_state.active_player();
                for &area2 in &db_areas {
                    if let Some(existing_card_id) = player.stage.get_area(area2) {
                        let has_protection =
                            card_db
                                .get_card(existing_card_id)
                                .is_some_and(|existing_card| {
                                    existing_card.abilities.iter().any(|a| {
                                        a.resolve().effect.as_ref().is_some_and(|ef| {
                                            if ef.restriction_type_any().as_deref()
                                                != Some("cannot_baton_touch")
                                            {
                                                return false;
                                            }
                                            if let Some(ref exclude_groups) =
                                                ef.exclude_group_names_any()
                                            {
                                                if crate::ability::util::card_matches_any_group(
                                                    &card_db,
                                                    card_id,
                                                    exclude_groups,
                                                ) {
                                                    return false;
                                                }
                                            }
                                            true
                                        })
                                    })
                                });
                        if has_protection {
                            return Err(
                                "Cannot baton touch: member has baton touch discard protection"
                                    .to_string(),
                            );
                        }
                    }
                }
            }
            // Replace both specified members first
            let double_replaced_ids: Vec<i16> = {
                let player = game_state.active_player_mut();
                let mut replaced = Vec::new();
                for &area2 in &db_areas {
                    if let Some(existing_card_id) = player.stage.get_area(area2) {
                        let _ = player
                            .remove_member_from_stage_with_recycling(area2 as usize, &card_db);
                        player.waitroom.cards.push(existing_card_id);
                        replaced.push(existing_card_id);
                    }
                }
                replaced
            };
            for &replaced_id in &double_replaced_ids {
                game_state.push_movement_event(
                    replaced_id,
                    "stage",
                    "waitroom",
                    Some(card_id),
                    &player_id,
                    false,
                );
            }
            // Track the non-placement vacated area for empty_area deployment
            let other_vacated = if db_areas[0] != area {
                Some(db_areas[0] as usize)
            } else {
                Some(db_areas[1] as usize)
            };
            game_state.last_vacated_stage_area = other_vacated;
            // Remove card from hand
            let player = game_state.active_player_mut();
            player.hand.cards.remove(idx);
            // Place card in chosen placement area
            player.stage.stage[area as usize] = card_id;
            // Rule 9.6.2.1.2.1: Card came from hand (non-stage), track it.
            // remove_member_from_stage_with_recycling already cleaned up the old card IDs.
            player.deployed_this_turn.insert(card_id);
            // Record 2 baton touches
            for _ in 0..2 {
                game_state.record_baton_touch(&player_id, Some(card_id));
            }
            game_state.baton_touch_replaced_member_id =
                double_baton_areas.as_ref().and_then(|_areas| {
                    let player = game_state.active_player();
                    player.waitroom.cards.last().copied()
                });
            game_state.active_player_mut().debut_count_this_turn += 1;
            game_state.record_card_appearance(card_id, "hand");
            game_state.baton_touch_arriving_card_id = Some(card_id);

            Self::trigger_debut_abilities(game_state, &player_id, &card_no, final_cost, true);
            Self::trigger_auto_abilities_for_player(game_state, &player_id);
            let db_opponent_id = if player_id == game_state.player1.id {
                game_state.player2.id.clone()
            } else {
                game_state.player1.id.clone()
            };
            Self::trigger_auto_abilities_for_player(game_state, &db_opponent_id);
            game_state.process_pending_auto_abilities(&player_id);
            tdbg!("PHASE_AUTO:0 recalc");
            game_state.mark_constants_dirty();
            game_state.recalculate_constants();
            tdbg!("PHASE_AUTO:1 recalc OK");

            log::debug!("[TRACK_MOVE] card_id={} player_id={}", card_id, player_id);
            return Ok(());
        }

        // Resolve the cost modifier for the member that baton touch would replace
        // at `area` (if any) using disjoint immutable reads. This replaces the
        // previous full `game_state.mods.clone()` per action: move_card_from_hand_to_stage
        // only reads a single cost modifier from `mods`, so pass just that scalar.
        let replaced_member_cost_mod = game_state
            .active_player()
            .stage
            .get_area(area)
            .map(|cid| game_state.mods.get_cost_modifier(cid))
            .unwrap_or(0);

        let player = game_state.active_player_mut();
        let (cost_paid, baton_touch_used, replaced_member_cost, replaced_member_id) = player
            .move_card_from_hand_to_stage(
                idx,
                area,
                use_baton_touch,
                &card_db,
                replaced_member_cost_mod,
            )?;
        game_state.baton_touch_zero_cost = baton_touch_used && cost_paid == 0;
        game_state.baton_touch_replaced_member_cost = replaced_member_cost;
        game_state.baton_touch_replaced_member_id = replaced_member_id;

        // Per Q24, baton touch step 2 (old card → waitroom) happens before step 4
        // (new card → stage). Enqueue movement-based triggers BEFORE appearance
        // triggers so they resolve first when process_pending runs below.
        if baton_touch_used {
            game_state.record_baton_touch(&player_id, Some(card_id));
            game_state.baton_touch_arriving_card_id = Some(card_id);
            if let Some(replaced_id) = replaced_member_id {
                game_state.push_movement_event(
                    replaced_id,
                    "stage",
                    "waitroom",
                    Some(card_id),
                    &player_id,
                    false,
                );
            }
            // Enqueue movement-triggered auto abilities (baton_touch, discard, etc.)
            // but do NOT process yet — let them queue before appearance triggers.
            Self::trigger_auto_abilities_for_player(game_state, &player_id);
            let bt_opponent_id = if player_id == game_state.player1.id {
                game_state.player2.id.clone()
            } else {
                game_state.player1.id.clone()
            };
            Self::trigger_auto_abilities_for_player(game_state, &bt_opponent_id);
        }

        // Phase 2: record appearance and debut triggers
        game_state.active_player_mut().debut_count_this_turn += 1;
        game_state.record_card_appearance(card_id, "hand");

        // Track area move for movement_condition "moves"
        log::debug!("[TRACK_MOVE] card_id={} player_id={}", card_id, player_id);

        Self::trigger_debut_abilities(
            game_state,
            &player_id,
            &card_no,
            cost_paid,
            baton_touch_used,
        );
        Self::trigger_auto_abilities_for_player(game_state, &player_id);
        let sb_opponent_id = if player_id == game_state.player1.id {
            game_state.player2.id.clone()
        } else {
            game_state.player1.id.clone()
        };
        Self::trigger_auto_abilities_for_player(game_state, &sb_opponent_id);
        // Process ALL queued abilities now: movement-triggered (baton_touch, etc.)
        // are ahead of appearance-triggered in the queue, so they resolve first.
        game_state.process_pending_auto_abilities(&player_id);
        tdbg!("PHASE_AUTO2:0 recalc");
        game_state.mark_constants_dirty();
        game_state.recalculate_constants();
        tdbg!("PHASE_AUTO2:1 recalc OK");

        if baton_touch_used {
            for area in [
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            ] {
                let card_no = if let Some(card_id) = game_state.active_player().stage.get_area(area)
                {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        let bt_card_id = card_id;
                        card.abilities
                            .iter()
                            .filter(|ar| {
                                ar.resolve()
                                    .triggers
                                    .as_ref()
                                    .is_some_and(|t| t.contains(crate::triggers::BATON_TOUCH))
                            })
                            .map(|ar| {
                                let ability = ar.resolve();
                                (
                                    format!("{}_{}", card.card_no, ability.full_text),
                                    card.card_no.to_string(),
                                    bt_card_id,
                                )
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
                for (ability_id, card_no, bt_card_id) in card_no {
                    game_state.trigger_auto_ability(
                        ability_id,
                        crate::game_state::AbilityTrigger::Debut,
                        player_id.clone(),
                        Some(card_no),
                        Some(bt_card_id),
                        None,
                        None,
                    );
                }
            }
        }

        // Discard triggers are handled by the unified moved-cards scan
        // inside trigger_auto_abilities_for_player_with_event.
        if replaced_member_id.is_some() {
            game_state.process_pending_auto_abilities(&player_id);
        }

        Ok(())
    }

    pub fn setup_initial_energy(game_state: &mut GameState) {
        for _ in 0..3 {
            if let Some(card_id) = game_state.player1.energy_deck.draw() {
                let _ = game_state
                    .player1
                    .energy_zone
                    .add_card(card_id, &game_state.card_database);
            }
            if let Some(card_id) = game_state.player2.energy_deck.draw() {
                let _ = game_state
                    .player2
                    .energy_zone
                    .add_card(card_id, &game_state.card_database);
            }
        }
    }

    pub(crate) fn handle_rps_choice_p1(
        game_state: &mut GameState,
        choice: i32,
    ) -> Result<(), String> {
        game_state.player1_rps_choice = Some(choice);
        Self::resolve_rps_if_both_chosen(game_state)
    }
    pub(crate) fn handle_rps_choice_p2(
        game_state: &mut GameState,
        choice: i32,
    ) -> Result<(), String> {
        game_state.player2_rps_choice = Some(choice);
        Self::resolve_rps_if_both_chosen(game_state)
    }

    fn rps_choice_name(choice: i32) -> &'static str {
        match choice {
            0 => "グー",
            1 => "パー",
            2 => "チョキ",
            _ => "?",
        }
    }

    fn push_rps_log(game_state: &mut GameState, p1: i32, p2: i32, winner_str: &str) {
        let p1_name = Self::rps_choice_name(p1);
        let p2_name = Self::rps_choice_name(p2);
        let text = format!("P1: {} vs P2: {} → {}", p1_name, p2_name, winner_str);
        game_state.push_rule_log(text.clone());
        game_state.push_structured_log(LogEntry {
            text,
            turn: game_state.turn_number,
            player_label: "SYSTEM".into(),
            source_card_id: None,
            source_card_name: None,
            category: "rps".into(),
            metadata: Some(crate::core::types::LogMetadata::RpsResult {
                p1_choice: Self::rps_choice_name(p1).to_string(),
                p2_choice: Self::rps_choice_name(p2).to_string(),
                p1_value: p1 as u32,
                p2_value: p2 as u32,
                winner: winner_str.to_string(),
            }),
        });
    }

    fn resolve_rps_if_both_chosen(game_state: &mut GameState) -> Result<(), String> {
        let p1_choice = match game_state.player1_rps_choice {
            Some(c) => c,
            None => return Ok(()),
        };
        let p2_choice = match game_state.player2_rps_choice {
            Some(c) => c,
            None => return Ok(()),
        };

        let rps_winner = match (p1_choice, p2_choice) {
            (0, 2) | (1, 0) | (2, 1) => {
                Self::push_rps_log(game_state, p1_choice, p2_choice, "P1の勝利");
                1
            }
            (2, 0) | (0, 1) | (1, 2) => {
                Self::push_rps_log(game_state, p1_choice, p2_choice, "P2の勝利");
                2
            }
            _ => {
                Self::push_rps_log(game_state, p1_choice, p2_choice, "引き分け　再選択");
                game_state.player1_rps_choice = None;
                game_state.player2_rps_choice = None;
                return Ok(());
            }
        };
        game_state.rps_winner = Some(rps_winner);
        game_state.current_phase = Phase::ChooseFirstAttacker;
        Ok(())
    }
}
