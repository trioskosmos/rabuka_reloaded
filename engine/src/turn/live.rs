use crate::ability::enums::Zone;
use crate::card::{BladeColor, CardDatabase, HeartColor};
use crate::game_state::GameState;
use crate::types::{
    Allocation, BladeSource, HeartSource, LivePerformanceData, MemberContribution, YellCardResult,
};
use std::collections::HashMap;

fn empty_h7() -> [u32; 7] {
    [0u32; 7]
}

impl super::TurnEngine {
    fn score_delta_since(
        current: &HashMap<i16, i32>,
        snapshot: &HashMap<i16, i32>,
        zone_cards: &[i16],
    ) -> u32 {
        let mut total = 0i32;
        for &cid in zone_cards {
            let cur = current.get(&cid).copied().unwrap_or(0);
            let prev = snapshot.get(&cid).copied().unwrap_or(0);
            total += (cur - prev).max(0);
        }
        total as u32
    }

    pub fn execute_live_victory_determination(game_state: &mut GameState) {
        let p1_mult = &game_state.mods.heart_color_multiplier.clone();
        let p2_mult = &game_state.mods.heart_color_multiplier.clone();
        // Include yell blade hearts in stage_hearts so should_trigger_live_success
        // checks against the same total hearts the performance used.
        let mut p1_stage = game_state
            .player1
            .calculate_stage_hearts(&game_state.card_database, p1_mult);
        let mut p2_stage = game_state
            .player2
            .calculate_stage_hearts(&game_state.card_database, p2_mult);
        for snap in &game_state.performance_snapshots {
            let target = if snap.player_id == game_state.player1.id {
                &mut p1_stage
            } else {
                &mut p2_stage
            };
            for yc in &snap.yell_cards {
                for (i, &count) in yc.blade_hearts.iter().enumerate() {
                    if count > 0 {
                        let color = match i {
                            0 => crate::card::HeartColor::Heart00,
                            1 => crate::card::HeartColor::Heart01,
                            2 => crate::card::HeartColor::Heart02,
                            3 => crate::card::HeartColor::Heart03,
                            4 => crate::card::HeartColor::Heart04,
                            5 => crate::card::HeartColor::Heart05,
                            6 => crate::card::HeartColor::Heart06,
                            _ => continue,
                        };
                        *target.hearts.entry(color).or_insert(0) += count as u32;
                    }
                }
            }
        }
        game_state.player1.stage_hearts = Some(p1_stage);
        game_state.player2.stage_hearts = Some(p2_stage);

        let player1_id = game_state.player1.id.clone();
        let player2_id = game_state.player2.id.clone();

        // Capture pre-trigger score modifiers so we can avoid double-counting:
        // calculate_live_score gets the PRE values, and pX_extra carries only
        // the delta from LiveSuccess-triggered abilities.
        let pre_score_flat: HashMap<i16, i32> = game_state
            .mods
            .score_modifiers
            .iter()
            .map(|(&k, e)| (k, e.total()))
            .collect();
        let mut pre_merged: HashMap<i16, i32> = pre_score_flat.clone();
        for (&k, entry) in &game_state.mods.score_modifiers {
            pre_merged.insert(k, entry.total());
        }

        let p1_extra;
        let p2_extra;
        if game_state.live_success_triggered_this_turn {
            p1_extra = 0;
            p2_extra = 0;
        } else {
            game_state.live_success_triggered_this_turn = true;

            // Compute initial surplus from stage hearts + yell before triggers fire,
            // so the condition evaluator can read stored values during LiveSuccess.
            for snap in &game_state.performance_snapshots {
                let total_hearts: u32 = snap.total_hearts.iter().sum();
                let player = if snap.player_id == player1_id {
                    &game_state.player1
                } else {
                    &game_state.player2
                };
                let required: u32 = player
                    .live_card_zone
                    .cards
                    .iter()
                    .filter_map(|&id| game_state.card_database.get_card(id))
                    .filter_map(|c| c.need_heart.as_ref())
                    .flat_map(|nh| nh.hearts.values())
                    .sum();
                let surplus = total_hearts.saturating_sub(required);
                if snap.player_id == player2_id {
                    game_state.opponent_live_surplus_count = surplus;
                } else {
                    game_state.self_live_surplus_count = surplus;
                }
            }
            game_state.live_surplus_ready_this_turn = true;

            Self::trigger_live_success_abilities(game_state, &player1_id);
            game_state.process_pending_auto_abilities(&player1_id);
            if game_state.has_pending_choice() {
                return;
            }
            let score_cur: HashMap<i16, i32> = game_state
                .mods
                .score_modifiers
                .iter()
                .map(|(&k, e)| (k, e.total()))
                .collect();
            p1_extra = Self::score_delta_since(
                &score_cur,
                &pre_score_flat,
                &game_state.player1.live_card_zone.cards,
            );

            Self::trigger_live_success_abilities(game_state, &player2_id);
            game_state.process_pending_auto_abilities(&player2_id);
            if game_state.has_pending_choice() {
                return;
            }
            let score_cur2: HashMap<i16, i32> = game_state
                .mods
                .score_modifiers
                .iter()
                .map(|(&k, e)| (k, e.total()))
                .collect();
            p2_extra = Self::score_delta_since(
                &score_cur2,
                &pre_score_flat,
                &game_state.player2.live_card_zone.cards,
            );
        }

        // Determine winner — use PRE-trigger modifiers so LiveSuccess
        // triggered changes only apply via pX_extra (no double-count).
        let need_heart_flat: HashMap<
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
                > = colors.iter().map(|(&c, e)| (c, e.clone())).collect();
                (k, flat)
            })
            .collect();
        let player1_score = game_state.player1.live_card_zone.calculate_live_score(
            &game_state.card_database,
            game_state.player1_cheer_blade_heart_count,
            game_state.player1.stage_hearts.as_ref(),
            Some(&need_heart_flat),
            Some(&pre_merged),
        ) + p1_extra;
        let player2_score = game_state.player2.live_card_zone.calculate_live_score(
            &game_state.card_database,
            game_state.player2_cheer_blade_heart_count,
            game_state.player2.stage_hearts.as_ref(),
            Some(&need_heart_flat),
            Some(&pre_merged),
        ) + p2_extra;
        let player1_has_cards = !game_state.player1.live_card_zone.cards.is_empty();
        let player2_has_cards = !game_state.player2.live_card_zone.cards.is_empty();

        let (player1_won, player2_won) = if !player1_has_cards && !player2_has_cards {
            (false, false)
        } else if player1_has_cards && !player2_has_cards {
            (true, false)
        } else if !player1_has_cards && player2_has_cards {
            (false, true)
        } else if player1_score > player2_score {
            (true, false)
        } else if player2_score > player1_score {
            (false, true)
        } else {
            (true, true) // Rule 8.4.6.2: equal scores = both win
        };

        // Finalize snapshots for both players
        for snap in game_state.performance_snapshots.iter_mut() {
            let player = if snap.player_id == player1_id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            // Determine pass/fail for each live card.
            // NOTE: We iterate over snap.lives (built from perf.live_card_ids captured
            // before the zone was cleared by Rule 8.3.16) rather than
            // player.live_card_zone.cards which may have been emptied.
            for i in 0..snap.lives.len() {
                let lc_id = snap.lives[i].card_id;
                if let Some(card) = game_state.card_database.get_card(lc_id) {
                    snap.lives[i].card_id = lc_id;
                    snap.lives[i].card_no = card.card_no.clone();
                    if let Some(ref nh) = card.need_heart {
                        let mut filled = empty_h7();
                        // Build filled array from heart allocations targeting this live
                        for alloc in &snap.breakdown.allocations {
                            if alloc.target_idx == i {
                                filled[alloc.color] += alloc.amount;
                            }
                        }
                        let mut required_arr = empty_h7();
                        for (color, needed) in &nh.hearts {
                            let idx = color.index();
                            required_arr[idx] = *needed;
                        }
                        // Use the engine's upstream heart satisfaction check (handles heart00 wildcard)
                        let stage_hearts =
                            player
                                .stage_hearts
                                .clone()
                                .unwrap_or(player.calculate_stage_hearts(
                                    &game_state.card_database,
                                    &std::collections::HashMap::new(),
                                ));
                        let passed = card.satisfies_heart_requirement(&stage_hearts);
                        // Populate adjustments from need_heart_modifiers
                        let mut adjustments = Vec::new();
                        if let Some(color_mods) = game_state.mods.need_heart_modifiers.get(&lc_id) {
                            for (color, entry) in color_mods {
                                let total = entry.total();
                                if total != 0 {
                                    let label = format!("{:?}", color);
                                    adjustments.push(crate::types::Adjustment {
                                        adjustment_type: "requirement".to_string(),
                                        desc: format!(
                                            "{} {}",
                                            if total > 0 { "+" } else { "" },
                                            total
                                        ),
                                        value: total,
                                        color: color.index(),
                                        source: format!("Heart req modifier ({})", label),
                                    });
                                }
                            }
                        }
                        snap.lives[i].adjustments = adjustments;
                        snap.lives[i].required = required_arr;
                        snap.lives[i].filled = filled;
                        snap.lives[i].passed = passed;
                    } else {
                        snap.lives[i].passed = true;
                    }
                    let base_score = card.get_score() as i32;
                    let mod_score = game_state.mods.get_score_modifier(lc_id);
                    snap.lives[i].score = (base_score + mod_score).max(0) as u32;
                }
            }

            // Compute per-card spare (余剰ハート): remaining hearts from the pool
            // after this card's allocation. For each live card, spare = total available
            // minus all allocations up to and including this card.
            // This correctly shows how many hearts remain in the pool after each card
            // consumes its share from the shared pool.
            let mut cumulative_used = empty_h7();
            for i in 0..snap.lives.len() {
                // Accumulate allocations for this live card
                for alloc in &snap.breakdown.allocations {
                    if alloc.target_idx == i {
                        cumulative_used[alloc.color] += alloc.amount;
                    }
                }
                // spare = total_hearts - cumulative_used (what remains after this card)
                let mut spare = empty_h7();
                for idx in 0..7 {
                    spare[idx] = snap.total_hearts[idx].saturating_sub(cumulative_used[idx]);
                }
                snap.lives[i].spare = spare;
            }

            let is_first = snap.player_id == player1_id;
            snap.p0_wins = player1_won;
            snap.p1_wins = player2_won;
            snap.total_score = if is_first {
                player1_score
            } else {
                player2_score
            };
            // Rule 8.3.16: If ANY live card failed its heart requirement, ALL lives fail.
            // The zone was already cleared in player_perform_live, so snap.lives[i].passed
            // may still be true for cards that would have passed individually.
            if player.live_card_zone.cards.is_empty() {
                for lr in snap.lives.iter_mut() {
                    lr.passed = false;
                }
            }
            // Success: at least one live card passed + total_score > 0
            snap.success = snap.lives.iter().any(|l| l.passed) && snap.total_score > 0;
        }

        // Compute surplus from finalized snapshots.
        // Surplus = remaining hearts after filling all live card requirements.
        // Uses actual filled allocations (not just required) to handle cases where
        // available hearts are less than required.
        let mut p2_surplus = 0u32;
        let mut p1_surplus = 0u32;
        for snap in &game_state.performance_snapshots {
            let total_available: u32 = snap.total_hearts.iter().sum();
            let total_filled: u32 = snap.lives.iter().flat_map(|l| l.filled.iter()).sum();
            let surplus = total_available.saturating_sub(total_filled);
            eprintln!(
                "[SURPLUS] player={} total_avail={} total_filled={} surplus={} lives={}",
                snap.player_id,
                total_available,
                total_filled,
                surplus,
                snap.lives.len()
            );
            for (i, l) in snap.lives.iter().enumerate() {
                eprintln!(
                    "[SURPLUS]   live[{}] passed={} required={:?} filled={:?} spare={:?}",
                    i, l.passed, l.required, l.filled, l.spare
                );
            }
            for a in &snap.breakdown.allocations {
                eprintln!(
                    "[SURPLUS]   alloc target={} color={} amount={} wildcard={}",
                    a.target_idx, a.color, a.amount, a.wildcard
                );
            }
            if snap.player_id == player2_id {
                p2_surplus = surplus;
            } else {
                p1_surplus = surplus;
            }
        }
        game_state.opponent_live_surplus_count = p2_surplus;
        game_state.self_live_surplus_count = p1_surplus;
        game_state.live_surplus_ready_this_turn = true;
        if player2_won {
            game_state.set_opponent_live_success(p2_surplus == 0);
        }
        if player1_won {
            game_state.self_no_excess_heart_this_turn = p1_surplus == 0;
        }

        // Push snapshots to rule log
        let card_db = game_state.card_database.clone();
        for snap in &game_state.performance_snapshots {
            for line in snapshot_to_rule_log(snap, &card_db) {
                game_state.rule_log.push(line);
            }
        }

        Self::move_restricted_cards_to_discard(&mut game_state.player1, &card_db);
        Self::move_restricted_cards_to_discard(&mut game_state.player2, &card_db);
        let p1_before = game_state.player1.success_live_card_zone.cards.len();
        let p2_before = game_state.player2.success_live_card_zone.cards.len();
        Self::move_live_to_success_and_handle_wins(game_state, player1_won, player2_won);
        // Rule 8.4.13: If only one player moved a card to success this live, they become first attacker
        let p1_now = game_state.player1.success_live_card_zone.cards.len();
        let p2_now = game_state.player2.success_live_card_zone.cards.len();
        let p1_added = p1_now > p1_before;
        let p2_added = p2_now > p2_before;
        if p1_added && !p2_added {
            game_state.player1.is_first_attacker = true;
            game_state.player2.is_first_attacker = false;
        } else if p2_added && !p1_added {
            game_state.player1.is_first_attacker = false;
            game_state.player2.is_first_attacker = true;
        }
    }

    fn move_restricted_cards_to_discard(
        player: &mut crate::player::Player,
        card_db: &CardDatabase,
    ) {
        let mut cards_to_remove = Vec::new();
        for (index, card_id) in player.live_card_zone.cards.iter().enumerate() {
            if let Some(card) = card_db.get_card(*card_id) {
                let has_restriction = card.abilities.iter().any(|ability| {
                    if let Some(ref effect) = ability.effect {
                        let restricted_dest = effect
                            .restricted_destination
                            .as_deref()
                            .or_else(|| effect.destination.as_deref());
                        crate::ability::enums::ActionType::from_str(&effect.action)
                            == Some(crate::ability::enums::ActionType::Restriction)
                            && effect.restriction_type.as_deref() == Some("cannot_place")
                            && matches!(
                                restricted_dest.and_then(Zone::from_str),
                                Some(Zone::SuccessLiveZone | Zone::LiveCardZone)
                            )
                    } else {
                        false
                    }
                });
                if has_restriction {
                    cards_to_remove.push(index);
                }
            }
        }
        for &idx in cards_to_remove.iter().rev() {
            if idx < player.live_card_zone.cards.len() {
                let card_id = player.live_card_zone.cards.remove(idx);
                player.waitroom.add_card(card_id);
            }
        }
    }

    fn process_player_live_result(
        player: &mut crate::player::Player,
        won: bool,
        must_skip: bool,
        can_place: bool,
    ) {
        let card_count = player.live_card_zone.cards.len();
        if won && !must_skip && card_count > 0 {
            let card_id = player.live_card_zone.cards.remove(0);
            if can_place {
                player.success_live_card_zone.cards.push(card_id);
            } else {
                player.waitroom.cards.push(card_id);
            }
        }
        while !player.live_card_zone.cards.is_empty() {
            player
                .waitroom
                .add_card(player.live_card_zone.cards.remove(0));
        }
    }

    fn move_live_to_success_and_handle_wins(
        game_state: &mut GameState,
        player1_won: bool,
        player2_won: bool,
    ) {
        let p1_id = game_state.player1.id.clone();
        let p2_id = game_state.player2.id.clone();
        let p1_cards = game_state.player1.live_card_zone.cards.len();
        let p2_cards = game_state.player2.live_card_zone.cards.len();
        let p1_must_skip = player1_won && player2_won && p1_cards >= 2;
        let p2_must_skip = player1_won && player2_won && p2_cards >= 2;

        // Check if multiple live cards need a success zone choice (Rule 8.4.7)
        if player1_won && !p1_must_skip && p1_cards > 1 {
            let p1_top = game_state.player1.live_card_zone.cards.last().copied();
            let p1_can_place = p1_top.map_or(false, |cid| {
                game_state.can_place_card_in_zone(cid, Zone::SuccessLiveZone.to_str(), &p1_id)
            });
            if p1_can_place {
                let options: Vec<crate::ability::types::LiveSuccessOption> = game_state
                    .player1
                    .live_card_zone
                    .cards
                    .iter()
                    .enumerate()
                    .map(|(i, &cid)| crate::ability::types::LiveSuccessOption {
                        card_name: game_state
                            .card_database
                            .get_card(cid)
                            .map(|c| c.name.clone())
                            .unwrap_or_default(),
                        card_index: i,
                    })
                    .collect();
                let choice = crate::ability::types::Choice::SelectLiveSuccess {
                    player_id: p1_id.clone(),
                    count: 1,
                    options,
                    description: "Choose which live card goes to your success zone".to_string(),
                };
                game_state.ability_queue.pause_for_choice(choice);
                return;
            }
        }
        if player2_won && !p2_must_skip && p2_cards > 1 {
            let p2_top = game_state.player2.live_card_zone.cards.last().copied();
            let p2_can_place = p2_top.map_or(false, |cid| {
                game_state.can_place_card_in_zone(cid, Zone::SuccessLiveZone.to_str(), &p2_id)
            });
            if p2_can_place {
                let options: Vec<crate::ability::types::LiveSuccessOption> = game_state
                    .player2
                    .live_card_zone
                    .cards
                    .iter()
                    .enumerate()
                    .map(|(i, &cid)| crate::ability::types::LiveSuccessOption {
                        card_name: game_state
                            .card_database
                            .get_card(cid)
                            .map(|c| c.name.clone())
                            .unwrap_or_default(),
                        card_index: i,
                    })
                    .collect();
                let choice = crate::ability::types::Choice::SelectLiveSuccess {
                    player_id: p2_id.clone(),
                    count: 1,
                    options,
                    description: "Choose which live card goes to your success zone".to_string(),
                };
                game_state.ability_queue.pause_for_choice(choice);
                return;
            }
        }

        // Single card or no choice needed — process normally
        let p1_top = game_state.player1.live_card_zone.cards.last().copied();
        let p2_top = game_state.player2.live_card_zone.cards.last().copied();
        let p1_can_place = p1_top.map_or(false, |cid| {
            game_state.can_place_card_in_zone(cid, Zone::SuccessLiveZone.to_str(), &p1_id)
        });
        let p2_can_place = p2_top.map_or(false, |cid| {
            game_state.can_place_card_in_zone(cid, Zone::SuccessLiveZone.to_str(), &p2_id)
        });

        Self::process_player_live_result(
            &mut game_state.player1,
            player1_won,
            p1_must_skip,
            p1_can_place,
        );
        Self::process_player_live_result(
            &mut game_state.player2,
            player2_won,
            p2_must_skip,
            p2_can_place,
        );
    }

    /// Handle the result of a live success zone card choice.
    pub(crate) fn handle_live_success_choice(
        game_state: &mut GameState,
        selected_index: usize,
        player_id: &str,
    ) -> Result<(), String> {
        let player = if player_id == game_state.player1.id {
            &mut game_state.player1
        } else if player_id == game_state.player2.id {
            &mut game_state.player2
        } else {
            return Err("Player not found for live success choice".to_string());
        };
        if selected_index >= player.live_card_zone.cards.len() {
            return Err("Invalid card index for live success choice".to_string());
        }
        let card_id = player.live_card_zone.cards.remove(selected_index);
        player.success_live_card_zone.cards.push(card_id);
        while !player.live_card_zone.cards.is_empty() {
            player
                .waitroom
                .add_card(player.live_card_zone.cards.remove(0));
        }
        Ok(())
    }

    pub fn player_perform_live(
        player: &mut crate::player::Player,
        resolution_zone: &mut crate::zones::ResolutionZone,
        _player_id: &str,
        card_db: &CardDatabase,
        blade_modifiers: &HashMap<i16, i32>,
        heart_override: &HashMap<i16, (HeartColor, u32)>,
        heart_modifiers: &HashMap<i16, HashMap<HeartColor, i32>>,
        blade_type_modifiers: &HashMap<i16, BladeColor>,
        orientation_modifiers: &HashMap<i16, String>,
        need_heart_modifiers: &HashMap<i16, HashMap<HeartColor, i32>>,
        cannot_live: bool,
    ) -> LivePerformanceData {
        // Q68/Rule: "cannot_live" discards live cards during performance; no yell, no live.
        if cannot_live {
            player
                .waitroom
                .cards
                .extend(player.live_card_zone.cards.drain(..));
            return LivePerformanceData {
                yell_count: 0,
                note_icons: 0,
                revealed_ids: Vec::new(),
                member_contributions: Vec::new(),
                yell_cards: Vec::new(),
                total_hearts: [0; 7],
                allocations: Vec::new(),
                heart_sources: Vec::new(),
                blade_sources: Vec::new(),
                draw_effects_occurred: false,
                live_card_ids: vec![],
            };
        }

        // Q32/Rule 8.3.6: If the live card zone is empty, no yell, no card processing.
        if player.live_card_zone.cards.is_empty() {
            return LivePerformanceData {
                yell_count: 0,
                note_icons: 0,
                revealed_ids: Vec::new(),
                member_contributions: Vec::new(),
                yell_cards: Vec::new(),
                total_hearts: [0; 7],
                allocations: Vec::new(),
                heart_sources: Vec::new(),
                blade_sources: Vec::new(),
                draw_effects_occurred: false,
                live_card_ids: vec![],
            };
        }

        let total_blade =
            player
                .stage
                .total_blades(card_db, blade_modifiers, orientation_modifiers);

        // Capture member contributions (base values + modifiers)
        let mut member_contributions = Vec::new();
        for i in 0..3 {
            let cid = player.stage.stage[i];
            if cid == -1 {
                continue;
            }
            let mut base_h = empty_h7();
            let mut bonus_h = empty_h7();
            let mut base_blades = 0u32;
            let mut draw_icons = 0u32;
            let ability_heart_bonuses = Vec::new();
            let ability_blade_bonuses = Vec::new();

            if let Some(card) = card_db.get_card(cid) {
                base_blades = card.blade;
                if let Some(ref bh) = card.base_heart {
                    for (color, count) in &bh.hearts {
                        let idx = color.index();
                        if idx < 7 {
                            base_h[idx] += count;
                        }
                    }
                }
                if let Some(ref sh) = card.special_heart {
                    for (color, count) in &sh.hearts {
                        if *color == HeartColor::Draw {
                            draw_icons += count;
                        }
                    }
                }
            }

            let bl_mod = blade_modifiers.get(&cid).copied().unwrap_or(0);
            let bonus_blades = if bl_mod > 0 { bl_mod as u32 } else { 0u32 };

            if let Some(mods) = heart_modifiers.get(&cid) {
                for (color, delta) in mods {
                    let idx = color.index();
                    if idx < 7 && *delta > 0 {
                        bonus_h[idx] += *delta as u32;
                    }
                }
            }

            // Check for heart override
            if let Some(&(override_color, override_count)) = heart_override.get(&cid) {
                base_h = empty_h7();
                let idx = override_color.index();
                if idx < 7 {
                    base_h[idx] = override_count;
                }
            }

            member_contributions.push(MemberContribution {
                source_id: cid,
                slot: i,
                base_hearts: base_h,
                bonus_hearts: bonus_h,
                base_blades,
                bonus_blades,
                base_notes: 0,
                bonus_notes: 0,
                draw_icons,
                ability_heart_bonuses,
                ability_blade_bonuses,
                card_no: card_db
                    .get_card(cid)
                    .map(|c| c.card_no.clone())
                    .unwrap_or_default(),
            });
        }

        // Yell cards
        let mut yell_cards = Vec::new();
        for _ in 0..total_blade {
            if let Some(card_id) = player.main_deck.draw() {
                resolution_zone.cards.push(card_id);
            }
        }

        // Compute owned hearts from stage
        let mut owned_hearts =
            player
                .stage
                .get_available_hearts(card_db, heart_override, heart_modifiers);

        let blade_to_heart = |bc: BladeColor| -> HeartColor {
            match bc {
                BladeColor::Peach => HeartColor::Heart01,
                BladeColor::Red => HeartColor::Heart02,
                BladeColor::Yellow => HeartColor::Heart03,
                BladeColor::Green => HeartColor::Heart04,
                BladeColor::Blue => HeartColor::Heart05,
                BladeColor::Purple => HeartColor::Heart06,
                BladeColor::All => HeartColor::Heart00,
            }
        };
        let override_color = (0..3)
            .filter_map(|i| {
                let cid = player.stage.stage[i];
                if cid == -1 {
                    None
                } else {
                    blade_type_modifiers.get(&cid).copied().map(blade_to_heart)
                }
            })
            .next();

        // Process yell cards and build YellCardResult + track heart allocations
        let mut cheer_icon_count = 0u32;
        let mut heart_sources: Vec<HeartSource> = Vec::new();
        let mut blade_sources: Vec<BladeSource> = Vec::new();
        let mut allocations: Vec<Allocation> = Vec::new();
        let mut total_hearts_arr = empty_h7();

        // Stage hearts source
        let stage_heart_str = "Stage (base)".to_string();
        let mut stage_heart_arr = empty_h7();
        for (color, count) in &owned_hearts.hearts {
            let idx = color.index();
            if idx < 7 {
                stage_heart_arr[idx] += count;
            }
        }
        heart_sources.push(HeartSource {
            source_type: "stage".into(),
            source: stage_heart_str,
            value: stage_heart_arr,
        });
        for (color, count) in &owned_hearts.hearts {
            let idx = color.index();
            if idx < 7 {
                total_hearts_arr[idx] += count;
            }
        }

        // Add stage blade source
        blade_sources.push(BladeSource {
            source_type: "stage".into(),
            source: "Stage members".into(),
            value: total_blade,
        });

        for card_id in &resolution_zone.cards {
            if let Some(card) = card_db.get_card(*card_id) {
                let mut bh_arr = empty_h7();
                let mut note_icons = 0u32;
                let mut draw_icons = 0u32;

                if let Some(ref bh) = card.blade_heart {
                    for (color, count) in &bh.hearts {
                        let effective_color = override_color.unwrap_or(*color);
                        if effective_color == HeartColor::BAll {
                            *owned_hearts.hearts.entry(HeartColor::Heart00).or_insert(0) += count;
                            bh_arr[0] += count;
                            total_hearts_arr[0] += count;
                        } else if effective_color == HeartColor::Draw {
                            draw_icons += count;
                            // Process draw effects immediately
                            for _ in 0..*count {
                                if let Some(new_card) = player.main_deck.draw() {
                                    player.hand.add_card(new_card);
                                }
                            }
                        } else if effective_color == HeartColor::Score {
                            note_icons += count;
                            cheer_icon_count += count;
                        } else {
                            let idx = effective_color.index();
                            if idx < 7 {
                                *owned_hearts.hearts.entry(effective_color).or_insert(0) += count;
                                bh_arr[idx] += count;
                                total_hearts_arr[idx] += count;
                            }
                            // Regular hearts do NOT add to score (Rule 8.4.2 — only ♪ icons count)
                        }
                    }
                } else {
                    // No blade_heart — no hearts from this yell card
                }

                yell_cards.push(YellCardResult {
                    card_id: *card_id,
                    blade_hearts: bh_arr,
                    note_icons,
                    draw_icons,
                    card_no: card_db
                        .get_card(*card_id)
                        .map(|c| c.card_no.clone())
                        .unwrap_or_default(),
                });
            }
        }

        // Yell heart source
        let mut yell_heart_arr = empty_h7();
        for yc in &yell_cards {
            for i in 0..7 {
                yell_heart_arr[i] += yc.blade_hearts[i];
            }
        }
        if yell_heart_arr.iter().any(|&v| v > 0) {
            heart_sources.push(HeartSource {
                source_type: "yell".into(),
                source: "Yell cards".into(),
                value: yell_heart_arr,
            });
        }
        blade_sources.push(BladeSource {
            source_type: "yell".into(),
            source: format!("{} blades", total_blade),
            value: total_blade,
        });

        // Live card special hearts
        for &lc_id in &player.live_card_zone.cards {
            if let Some(card) = card_db.get_card(lc_id) {
                if let Some(ref sh) = card.special_heart {
                    for (color, count) in &sh.hearts {
                        if *color == HeartColor::Draw {
                            for _ in 0..*count {
                                if let Some(new_card) = player.main_deck.draw() {
                                    player.hand.add_card(new_card);
                                }
                            }
                        } else if *color == HeartColor::Score {
                            cheer_icon_count += count;
                        }
                    }
                }
            }
        }

        // Build heart allocations (direct fill per live card, using adjusted need_heart).
        // IMPORTANT: `remaining` is shared across all live cards so that hearts allocated
        // to earlier cards are not double-allocated to later cards.
        let live_card_ids: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();
        let mut remaining = owned_hearts.clone();
        for live_idx in 0..live_card_ids.len() {
            if let Some(card) = card_db.get_card(live_card_ids[live_idx]) {
                // Apply need_heart modifiers (e.g. heart requirement -1)
                let effective_need = card.need_heart.as_ref().map(|nh| {
                    let mut adjusted = nh.clone();
                    if let Some(card_mods) = need_heart_modifiers.get(&live_card_ids[live_idx]) {
                        for (color, delta) in card_mods {
                            let entry = adjusted.hearts.entry(*color).or_insert(0);
                            *entry = (*entry as i32 + delta).max(0) as u32;
                        }
                    }
                    adjusted
                });
                let use_need = effective_need.clone().or_else(|| card.need_heart.clone());
                if let Some(ref nh) = use_need {
                    // Phase 1: fill specific-color requirements (Heart01-Heart06)
                    for (color, needed) in &nh.hearts {
                        if *color == HeartColor::Heart00 {
                            continue; // handled in Phase 3
                        }
                        let c_idx = color.index();
                        let avail = *remaining.hearts.get(color).unwrap_or(&0);
                        let used = avail.min(*needed);
                        if used > 0 {
                            allocations.push(Allocation {
                                target_idx: live_idx,
                                target_name: card.name.clone(),
                                source_type: "stage".into(),
                                source_name: "Stage hearts".into(),
                                source_slot: None,
                                wildcard: false,
                                color: c_idx,
                                amount: used,
                                is_bonus: false,
                            });
                            *remaining.hearts.entry(*color).or_insert(0) -= used;
                        }
                    }
                    // Phase 2: fill specific-color deficits with wildcard Heart00
                    let mut wildcard_avail =
                        *remaining.hearts.get(&HeartColor::Heart00).unwrap_or(&0);
                    if wildcard_avail > 0 {
                        let mut wildcard_used = 0u32;
                        for (color, needed) in &nh.hearts {
                            if *color == HeartColor::Heart00 {
                                continue; // handled in Phase 3
                            }
                            let c_idx = color.index();
                            let already_filled = allocations
                                .iter()
                                .filter(|a| {
                                    a.target_idx == live_idx && a.color == c_idx && !a.wildcard
                                })
                                .map(|a| a.amount)
                                .sum::<u32>();
                            let still_needed = needed.saturating_sub(already_filled);
                            if still_needed > 0 && wildcard_avail > wildcard_used {
                                let fill = still_needed.min(wildcard_avail - wildcard_used);
                                allocations.push(Allocation {
                                    target_idx: live_idx,
                                    target_name: card.name.clone(),
                                    source_type: "stage".into(),
                                    source_name: "Wildcard (Heart00)".into(),
                                    source_slot: None,
                                    wildcard: true,
                                    color: c_idx,
                                    amount: fill,
                                    is_bonus: false,
                                });
                                wildcard_used += fill;
                            }
                        }
                        if wildcard_used > 0 {
                            *remaining.hearts.entry(HeartColor::Heart00).or_insert(0) -=
                                wildcard_used;
                            wildcard_avail -= wildcard_used;
                        }
                    }
                    // Phase 3: fill Heart00 (Any) requirements from any available hearts
                    // Heart00 can be satisfied by any colored heart or by Heart00 itself
                    let any_needed = *nh.hearts.get(&HeartColor::Heart00).unwrap_or(&0);
                    if any_needed > 0 {
                        let mut any_filled = 0u32;
                        // First try colored hearts (1-6)
                        for color_idx in 1..7usize {
                            let hc = HeartColor::from_index(color_idx);
                            let avail = *remaining.hearts.get(&hc).unwrap_or(&0);
                            if avail > 0 && any_filled < any_needed {
                                let fill = avail.min(any_needed - any_filled);
                                allocations.push(Allocation {
                                    target_idx: live_idx,
                                    target_name: card.name.clone(),
                                    source_type: "stage".into(),
                                    source_name: "Stage hearts".into(),
                                    source_slot: None,
                                    wildcard: false,
                                    color: color_idx,
                                    amount: fill,
                                    is_bonus: false,
                                });
                                *remaining.hearts.entry(hc).or_insert(0) -= fill;
                                any_filled += fill;
                            }
                        }
                        // Then try remaining Heart00 wildcards
                        if any_filled < any_needed {
                            let avail = *remaining.hearts.get(&HeartColor::Heart00).unwrap_or(&0);
                            let fill = avail.min(any_needed - any_filled);
                            if fill > 0 {
                                allocations.push(Allocation {
                                    target_idx: live_idx,
                                    target_name: card.name.clone(),
                                    source_type: "stage".into(),
                                    source_name: "Stage hearts".into(),
                                    source_slot: None,
                                    wildcard: false,
                                    color: 0,
                                    amount: fill,
                                    is_bonus: false,
                                });
                                *remaining.hearts.entry(HeartColor::Heart00).or_insert(0) -= fill;
                                any_filled += fill;
                            }
                        }
                    }
                }
            }
        }

        // Check each live card's requirement (with modifiers applied for heart reduction)
        let any_requirement_failed = live_card_ids.iter().any(|&lc_id| {
            card_db.get_card(lc_id).map_or(false, |card| {
                let nh = match card.need_heart.as_ref() {
                    Some(nh) => {
                        let mut adjusted = nh.clone();
                        if let Some(card_mods) = need_heart_modifiers.get(&lc_id) {
                            for (color, delta) in card_mods {
                                let entry = adjusted.hearts.entry(*color).or_insert(0);
                                *entry = (*entry as i32 + delta).max(0) as u32;
                            }
                        }
                        adjusted
                    }
                    None => return false,
                };
                !nh.hearts.is_empty()
                    && !crate::card::Card::need_heart_satisfied(&nh, &owned_hearts)
            })
        });
        if any_requirement_failed {
            eprintln!("[LIVE] Heart requirement not met — discarding all live cards");
            player.live_card_zone.cards.clear();
        }

        let revealed_ids: Vec<i16> = resolution_zone.cards.iter().copied().collect();
        player.last_resolution_cards = revealed_ids.clone();

        for card_id in resolution_zone.cards.drain(..) {
            player.waitroom.add_card(card_id);
        }

        LivePerformanceData {
            yell_count: total_blade,
            note_icons: cheer_icon_count,
            revealed_ids,
            member_contributions,
            yell_cards,
            total_hearts: total_hearts_arr,
            allocations,
            heart_sources,
            blade_sources,
            draw_effects_occurred: true, // Flag to trigger auto abilities after draw effects
            live_card_ids: live_card_ids.clone(),
        }
    }
}

// ============== SNAPSHOT BUILDING ==============

/// Process the ability_applications recorded during the live performance
/// and populate MemberContribution ability bonuses, score lines, etc.
pub fn enrich_from_applications(
    member_contributions: &mut Vec<MemberContribution>,
    breakdown: &mut crate::types::Breakdown,
    triggered_abilities: &mut Vec<crate::types::TriggeredAbility>,
    applications: &[crate::types::AbilityApplication],
    card_db: &crate::card::CardDatabase,
) {
    let mut seen = std::collections::HashSet::new();
    for app in applications {
        if let Some(mc) = member_contributions
            .iter_mut()
            .find(|m| m.source_id == app.target_card_id)
        {
            match app.effect_type.as_str() {
                "heart_bonus" => {
                    let source_name = card_db
                        .get_card(app.source_card_id)
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| format!("#{}", app.source_card_id));
                    mc.ability_heart_bonuses.push(crate::types::AbilityBonus {
                        source: format!("Ability: {}", source_name),
                        amount: app.amount.unsigned_abs(),
                        color: app.heart_color,
                        ability_text: app.ability_text.clone(),
                    });
                }
                "blade_bonus" => {
                    let source_name = card_db
                        .get_card(app.source_card_id)
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| format!("#{}", app.source_card_id));
                    mc.ability_blade_bonuses.push(crate::types::AbilityBonus {
                        source: format!("Ability: {}", source_name),
                        amount: app.amount.unsigned_abs(),
                        color: app.heart_color,
                        ability_text: app.ability_text.clone(),
                    });
                }
                _ => {}
            }
        }
        match app.effect_type.as_str() {
            "score_bonus" | "score_set" => {
                breakdown.scores.push(crate::types::ScoreLine {
                    source: app.ability_text.clone(),
                    value: app.amount.unsigned_abs(),
                });
            }
            "transform" => {
                breakdown.transforms.push(crate::types::EffectEntry {
                    source: app.ability_text.clone(),
                    value: format!("→ color {}", app.heart_color.map_or(0, |c| c)),
                    desc: format!(
                        "All hearts become type {}",
                        app.heart_color.map_or(0, |c| c)
                    ),
                });
            }
            _ => {}
        }
        let key = (app.source_card_id, &app.ability_text);
        if seen.insert(key) && !app.ability_text.is_empty() {
            let card = card_db.get_card(app.source_card_id);
            triggered_abilities.push(crate::types::TriggeredAbility {
                source_card_id: app.source_card_id,
                name: format!("Ability #{}", triggered_abilities.len() + 1),
                card_name: card.map(|c| c.name.clone()).unwrap_or_default(),
                effect_text: app.ability_text.clone(),
                condition_text: None,
                is_public: true,
            });
        }
    }
}

pub fn build_snapshot(
    turn: u32,
    player_id: &str,
    perf: &LivePerformanceData,
    card_db: &CardDatabase,
    note_icons: u32,
) -> crate::types::PerformanceSnapshot {
    let mut lives = Vec::new();
    // Use perf.live_card_ids (captured before heart check cleared the zone)
    for &lc_id in &perf.live_card_ids {
        let score = card_db.get_card(lc_id).map(|c| c.get_score()).unwrap_or(0);
        let card_no = card_db
            .get_card(lc_id)
            .map(|c| c.card_no.clone())
            .unwrap_or_default();
        lives.push(crate::types::LiveCardResult {
            passed: false,
            score,
            spare: empty_h7(),
            required: empty_h7(),
            filled: empty_h7(),
            adjustments: Vec::new(),
            card_id: lc_id,
            card_no,
        });
    }

    crate::types::PerformanceSnapshot {
        turn,
        player_id: player_id.to_string(),
        lives,
        member_contributions: perf.member_contributions.clone(),
        yell_cards: perf.yell_cards.clone(),
        total_hearts: perf.total_hearts,
        total_score: 0,
        success: false,
        note_icons,
        yell_count: perf.yell_count,
        breakdown: crate::types::Breakdown {
            hearts: perf.heart_sources.clone(),
            blades: perf.blade_sources.clone(),
            allocations: perf.allocations.clone(),
            requirements: Vec::new(),
            transforms: Vec::new(),
            scores: Vec::new(),
        },
        triggered_abilities: {
            let mut seen = std::collections::HashSet::new();
            let mut tas = Vec::new();
            for mc in &perf.member_contributions {
                for ab in mc
                    .ability_heart_bonuses
                    .iter()
                    .chain(mc.ability_blade_bonuses.iter())
                {
                    if !seen.insert(&ab.ability_text) {
                        continue;
                    }
                    let card = card_db.get_card(mc.source_id);
                    tas.push(crate::types::TriggeredAbility {
                        source_card_id: mc.source_id,
                        name: ab.source.clone(),
                        card_name: card.map(|c| c.name.clone()).unwrap_or_default(),
                        effect_text: ab.ability_text.clone(),
                        condition_text: None,
                        is_public: true,
                    });
                }
            }
            if perf.draw_effects_occurred {
                tas.push(crate::types::TriggeredAbility {
                    source_card_id: -1,
                    name: "Draw Effect".to_string(),
                    card_name: String::new(),
                    effect_text: "カードを引く効果が発動しました".to_string(),
                    condition_text: None,
                    is_public: true,
                });
            }
            tas
        },
        p0_wins: false,
        p1_wins: false,
    }
}

fn card_name_by_no(card_db: &CardDatabase, card_no: &str) -> String {
    if card_no.is_empty() {
        return "?".to_string();
    }
    card_db
        .get_card_by_no(card_no)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| card_no.to_string())
}

fn fmt_player_id(id: &str) -> String {
    let digits: String = id.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        "?".to_string()
    } else {
        format!("P{}", digits)
    }
}

fn fmt_hearts(arr: &[u32; 7]) -> String {
    arr.iter()
        .enumerate()
        .filter(|(_, &v)| v > 0)
        .map(|(i, v)| {
            format!(
                "{}:{}",
                crate::card::HeartColor::from_index(i).short_label(),
                v
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn fmt_heart_vec(arr: &[u32; 7]) -> String {
    arr.iter()
        .enumerate()
        .map(|(i, v)| {
            format!(
                "{}:{}",
                crate::card::HeartColor::from_index(i).short_label(),
                v
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn snapshot_to_rule_log(
    snap: &crate::types::PerformanceSnapshot,
    card_db: &CardDatabase,
) -> Vec<String> {
    let player = fmt_player_id(&snap.player_id);
    let heart_labels = ["h00", "h01", "h02", "h03", "h04", "h05", "h06"];
    let mut lines = Vec::new();

    lines.push(format!("[Turn {}] ── {} Performance ──", snap.turn, player));

    // ── Stage members ──
    let mut _stage_total_blades = 0u32;
    let mut stage_total_hearts = [0u32; 7];
    for mc in &snap.member_contributions {
        let name = card_name_by_no(card_db, &mc.card_no);
        let total_blade = mc.base_blades + mc.bonus_blades;
        _stage_total_blades += total_blade;
        for i in 0..7 {
            stage_total_hearts[i] += mc.base_hearts[i] + mc.bonus_hearts[i];
        }

        // Base hearts
        let base_h = fmt_hearts(&mc.base_hearts);
        let blade_str = if mc.bonus_blades > 0 {
            format!("★{} (+{} from abilities)", total_blade, mc.bonus_blades)
        } else {
            format!("★{}", total_blade)
        };
        if base_h.is_empty() {
            lines.push(format!("  Stage: {}  {}", name, blade_str));
        } else {
            lines.push(format!("  Stage: {}  {}  ♥[{}]", name, blade_str, base_h));
        }

        // Ability bonuses
        for ab in &mc.ability_heart_bonuses {
            let color_str = ab
                .color
                .map(|c| heart_labels[c].to_string())
                .unwrap_or_default();
            lines.push(format!(
                "    Ability: {}  ♥{}+{}",
                ab.source, color_str, ab.amount
            ));
        }
        for ab in &mc.ability_blade_bonuses {
            lines.push(format!("    Ability: {}  ★+{}", ab.source, ab.amount));
        }
    }

    // ── Yell cards ──
    let mut yell_total_hearts = [0u32; 7];
    if snap.yell_count > 0 {
        lines.push(format!("  Yell ({} cards):", snap.yell_count));
        for yc in &snap.yell_cards {
            let name = card_name_by_no(card_db, &yc.card_no);
            let bh = fmt_hearts(&yc.blade_hearts);
            for i in 0..7 {
                yell_total_hearts[i] += yc.blade_hearts[i];
            }
            if bh.is_empty() {
                lines.push(format!(
                    "    {}  ♪{}  ⎋{}",
                    name, yc.note_icons, yc.draw_icons
                ));
            } else {
                lines.push(format!(
                    "    {}  ♥[{}]  ♪{}  ⎋{}",
                    name, bh, yc.note_icons, yc.draw_icons
                ));
            }
        }
    }

    // ── Total hearts breakdown ──
    lines.push(format!("  Hearts breakdown:"));
    lines.push(format!(
        "    Base hearts:      [{}]",
        fmt_heart_vec(&stage_total_hearts)
    ));
    if yell_total_hearts.iter().any(|&v| v > 0) {
        lines.push(format!(
            "    Yell hearts:      [{}]",
            fmt_heart_vec(&yell_total_hearts)
        ));
    }
    if !snap.breakdown.hearts.is_empty() {
        for hs in &snap.breakdown.hearts {
            let hv = fmt_heart_vec(&hs.value);
            if hv.chars().any(|c| {
                c != '0'
                    && c != ':'
                    && c != 'h'
                    && c != '0'
                    && c != '1'
                    && c != '2'
                    && c != '3'
                    && c != '4'
                    && c != '5'
                    && c != '6'
            }) {
                // has non-zero values
                lines.push(format!("    {}: [{}]", hs.source, hv));
            }
        }
    }
    let total_h = fmt_heart_vec(&snap.total_hearts);
    lines.push(format!("    Total hearts:     [{}]", total_h));

    // ── Live cards ──
    for live in &snap.lives {
        let name = card_name_by_no(card_db, &live.card_no);
        let need_h = fmt_hearts(&live.required);
        let filled_h = fmt_hearts(&live.filled);
        let spare_h = fmt_hearts(&live.spare);
        let result = if live.passed { "PASS" } else { "FAIL" };
        lines.push(format!(
            "  Live: {}  need[{}]  filled[{}]  spare[{}]  score +{}  → {}",
            name, need_h, filled_h, spare_h, live.score, result
        ));
    }

    // ── Score ──
    let pass_str = if snap.success { "PASS" } else { "FAIL" };
    lines.push(format!("  Score: {}  {}", snap.total_score, pass_str));

    lines
}
