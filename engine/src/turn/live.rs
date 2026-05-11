use crate::card::{BladeColor, CardDatabase, HeartColor};
use crate::game_state::GameState;
use crate::types::{
    Allocation, BladeSource, HeartSource, LivePerformanceData, MemberContribution, YellCardResult,
};
use std::collections::HashMap;

fn heart_color_index(color: &HeartColor) -> usize {
    match color {
        HeartColor::Heart00 => 0,
        HeartColor::Heart01 => 1,
        HeartColor::Heart02 => 2,
        HeartColor::Heart03 => 3,
        HeartColor::Heart04 => 4,
        HeartColor::Heart05 => 5,
        HeartColor::Heart06 => 6,
        _ => 0,
    }
}

fn empty_h7() -> [u32; 7] { [0u32; 7] }

impl super::TurnEngine {
    fn score_delta_since(current: &HashMap<i16, i32>, snapshot: &HashMap<i16, i32>, zone_cards: &[i16]) -> u32 {
        let mut total = 0i32;
        for &cid in zone_cards {
            let cur = current.get(&cid).copied().unwrap_or(0);
            let prev = snapshot.get(&cid).copied().unwrap_or(0);
            total += (cur - prev).max(0);
        }
        total as u32
    }

    pub fn execute_live_victory_determination(game_state: &mut GameState) {
        game_state.player1.stage_hearts = Some(game_state.player1.calculate_stage_hearts(&game_state.card_database));
        game_state.player2.stage_hearts = Some(game_state.player2.calculate_stage_hearts(&game_state.card_database));

        let player1_id = game_state.player1.id.clone();
        let player2_id = game_state.player2.id.clone();

        let p1_extra;
        let p2_extra;
        if game_state.live_success_triggered_this_turn {
            p1_extra = 0;
            p2_extra = 0;
        } else {
            game_state.live_success_triggered_this_turn = true;

            let pre_p1 = game_state.mods.score_modifiers.clone();
            Self::trigger_live_success_abilities(game_state, &player1_id);
            game_state.process_pending_auto_abilities(&player1_id);
            if game_state.pending_choice.is_some() { return; }
            p1_extra = Self::score_delta_since(&game_state.mods.score_modifiers, &pre_p1, &game_state.player1.live_card_zone.cards);

            let pre_p2 = game_state.mods.score_modifiers.clone();
            Self::trigger_live_success_abilities(game_state, &player2_id);
            game_state.process_pending_auto_abilities(&player2_id);
            if game_state.pending_choice.is_some() { return; }
            p2_extra = Self::score_delta_since(&game_state.mods.score_modifiers, &pre_p2, &game_state.player2.live_card_zone.cards);
        }

        // Determine winner
        let player1_score = game_state.player1.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player1_cheer_blade_heart_count, game_state.player1.stage_hearts.as_ref(), Some(&game_state.mods.need_heart_modifiers)) + p1_extra;
        let player2_score = game_state.player2.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player2_cheer_blade_heart_count, game_state.player2.stage_hearts.as_ref(), Some(&game_state.mods.need_heart_modifiers)) + p2_extra;
        let player1_has_cards = !game_state.player1.live_card_zone.cards.is_empty();
        let player2_has_cards = !game_state.player2.live_card_zone.cards.is_empty();

        let (player1_won, player2_won) = if !player1_has_cards && !player2_has_cards { (false, false) }
        else if player1_has_cards && !player2_has_cards { (true, false) }
        else if !player1_has_cards && player2_has_cards { (false, true) }
        else {
            (player1_score > player2_score, player2_score > player1_score)
        };

        if player2_won { game_state.set_opponent_live_success(true); }

        // Finalize snapshots for both players
        for snap in game_state.performance_snapshots.iter_mut() {
            let player = if snap.player_id == player1_id { &game_state.player1 } else { &game_state.player2 };
            // Determine pass/fail for each live card
            for i in 0..player.live_card_zone.cards.len().min(snap.lives.len()) {
                let lc_id = player.live_card_zone.cards[i];
                if let Some(card) = game_state.card_database.get_card(lc_id) {
                    snap.lives[i].card_id = lc_id;
                    if let Some(ref nh) = card.need_heart {
                        let mut filled = empty_h7();
                        let mut spare = empty_h7();
                        // Build filled array from heart allocations targeting this live
                        for alloc in &snap.breakdown.allocations {
                            if alloc.target_idx == i {
                                filled[alloc.color] += alloc.amount;
                            }
                        }
                        let mut passed = true;
                        let mut required_arr = empty_h7();
                        for (color, needed) in &nh.hearts {
                            let idx = heart_color_index(color);
                            required_arr[idx] = *needed;
                            // Wildcard (heart00) can fill deficit if specfic color is insufficient
                            let deficit = if filled[idx] >= *needed { 0 } else { *needed - filled[idx] };
                            if deficit > 0 && filled[0] >= deficit {
                                filled[0] -= deficit;
                                filled[idx] += deficit;
                            }
                            if filled[idx] < *needed { passed = false; }
                        }
                        // Remaining hearts are spares
                        for (color, amount) in &player.stage_hearts.as_ref().map(|sh| sh.hearts.clone()).unwrap_or_default() {
                            let idx = heart_color_index(color);
                            if idx < 7 { spare[idx] = *amount; }
                        }
                        snap.lives[i].required = required_arr;
                        snap.lives[i].filled = filled;
                        snap.lives[i].spare = spare;
                        snap.lives[i].passed = passed;
                    } else {
                        snap.lives[i].passed = true;
                    }
                    snap.lives[i].score = card.get_score();
                }
            }

            let is_first = snap.player_id == player1_id;
            snap.p0_wins = is_first && player1_won || !is_first && !player2_won && player1_won;
            snap.p1_wins = !is_first && player2_won || is_first && !player1_won && player2_won;
            snap.total_score = if is_first { player1_score } else { player2_score };
            // Success: at least one live card passed + total_score > 0
            snap.success = snap.lives.iter().any(|l| l.passed) && snap.total_score > 0;
        }

        // Push snapshots to rule log
        for snap in &game_state.performance_snapshots {
            for line in snapshot_to_rule_log(snap) {
                game_state.rule_log.push(line);
            }
        }

        let card_db = game_state.card_database.clone();
        Self::move_restricted_cards_to_discard(&mut game_state.player1, &card_db);
        Self::move_restricted_cards_to_discard(&mut game_state.player2, &card_db);
        Self::move_live_to_success_and_handle_wins(game_state, player1_won, player2_won);
    }

    fn move_restricted_cards_to_discard(player: &mut crate::player::Player, card_db: &CardDatabase) {
        let mut cards_to_remove = Vec::new();
        for (index, card_id) in player.live_card_zone.cards.iter().enumerate() {
            if let Some(card) = card_db.get_card(*card_id) {
                let has_restriction = card.abilities.iter().any(|ability| {
                    if let Some(ref effect) = ability.effect {
                        let restricted_dest = effect.restricted_destination.as_deref().or_else(|| effect.destination.as_deref());
                        effect.action == "restriction" && effect.restriction_type.as_deref() == Some("cannot_place")
                            && (restricted_dest == Some("success_live_zone") || restricted_dest == Some("live_card_zone"))
                    } else { false }
                });
                if has_restriction { cards_to_remove.push(index); }
            }
        }
        for &idx in cards_to_remove.iter().rev() {
            if idx < player.live_card_zone.cards.len() {
                let card_id = player.live_card_zone.cards.remove(idx);
                player.waitroom.add_card(card_id);
            }
        }
    }

    fn move_live_to_success_and_handle_wins(game_state: &mut GameState, player1_won: bool, player2_won: bool) {
        let p1_id = game_state.player1.id.clone();
        let p2_id = game_state.player2.id.clone();

        let p1_cards = game_state.player1.live_card_zone.cards.len();
        let p2_cards = game_state.player2.live_card_zone.cards.len();
        let p1_must_skip = player1_won && player2_won && p1_cards >= 2;
        let p2_must_skip = player1_won && player2_won && p2_cards >= 2;

        if player1_won && !p1_must_skip && p1_cards > 0 {
            let card_id = game_state.player1.live_card_zone.cards.remove(p1_cards - 1);
            eprintln!("[LIVE] REMOVED from p1 lcz: {} (p1_cards={})", card_id, p1_cards);
            if game_state.can_place_card_in_zone(card_id, "success_live_zone", &p1_id) {
                game_state.player1.success_live_card_zone.cards.push(card_id);
            } else {
                game_state.player1.waitroom.cards.push(card_id);
            }
            while !game_state.player1.live_card_zone.cards.is_empty() {
                game_state.player1.waitroom.add_card(game_state.player1.live_card_zone.cards.remove(0));
            }
        } else {
            while !game_state.player1.live_card_zone.cards.is_empty() {
                game_state.player1.waitroom.add_card(game_state.player1.live_card_zone.cards.remove(0));
            }
        }

        if player2_won && !p2_must_skip && p2_cards > 0 {
            let card_id = game_state.player2.live_card_zone.cards.remove(p2_cards - 1);
            if game_state.can_place_card_in_zone(card_id, "success_live_zone", &p2_id) {
                game_state.player2.success_live_card_zone.cards.push(card_id);
            } else {
                game_state.player2.waitroom.cards.push(card_id);
            }
            while !game_state.player2.live_card_zone.cards.is_empty() {
                game_state.player2.waitroom.add_card(game_state.player2.live_card_zone.cards.remove(0));
            }
        } else {
            while !game_state.player2.live_card_zone.cards.is_empty() {
                game_state.player2.waitroom.add_card(game_state.player2.live_card_zone.cards.remove(0));
            }
        }
    }

    pub fn player_perform_live(
        player: &mut crate::player::Player,
        resolution_zone: &mut crate::zones::ResolutionZone,
        _player_id: &str, card_db: &CardDatabase,
        blade_modifiers: &HashMap<i16, i32>,
        heart_override: &HashMap<i16, (HeartColor, u32)>,
        heart_modifiers: &HashMap<i16, HashMap<HeartColor, i32>>,
        blade_type_modifiers: &HashMap<i16, BladeColor>,
    ) -> LivePerformanceData {
        let total_blade = player.stage.total_blades(card_db, blade_modifiers);

        // Capture member contributions (base values + modifiers)
        let mut member_contributions = Vec::new();
        for i in 0..3 {
            let cid = player.stage.stage[i];
            if cid == -1 { continue; }
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
                        let idx = heart_color_index(color);
                        if idx < 7 { base_h[idx] += count; }
                    }
                }
                if let Some(ref sh) = card.special_heart {
                    for (color, count) in &sh.hearts {
                        if *color == HeartColor::Draw { draw_icons += count; }
                    }
                }
            }

            let bl_mod = blade_modifiers.get(&cid).copied().unwrap_or(0);
            let bonus_blades = if bl_mod > 0 { bl_mod as u32 } else { 0u32 };

            if let Some(mods) = heart_modifiers.get(&cid) {
                for (color, delta) in mods {
                    let idx = heart_color_index(color);
                    if idx < 7 && *delta > 0 {
                        bonus_h[idx] += *delta as u32;
                    }
                }
            }

            // Check for heart override
            if let Some(&(override_color, override_count)) = heart_override.get(&cid) {
                base_h = empty_h7();
                let idx = heart_color_index(&override_color);
                if idx < 7 { base_h[idx] = override_count; }
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
        let mut owned_hearts = player.stage.get_available_hearts(card_db, heart_override, heart_modifiers);

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
        let override_color = (0..3).filter_map(|i| {
            let cid = player.stage.stage[i];
            if cid == -1 { None } else { blade_type_modifiers.get(&cid).copied().map(blade_to_heart) }
        }).next();

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
            let idx = heart_color_index(color);
            if idx < 7 { stage_heart_arr[idx] += count; }
        }
        heart_sources.push(HeartSource { source_type: "stage".into(), source: stage_heart_str, value: stage_heart_arr });
        for (color, count) in &owned_hearts.hearts {
            let idx = heart_color_index(color);
            if idx < 7 { total_hearts_arr[idx] += count; }
        }

        // Add stage blade source
        blade_sources.push(BladeSource { source_type: "stage".into(), source: "Stage members".into(), value: total_blade });

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
                        } else if effective_color == HeartColor::Draw {
                            draw_icons += count;
                            for _ in 0..*count {
                                if let Some(new_card) = player.main_deck.draw() {
                                    player.hand.add_card(new_card);
                                }
                            }
                        } else if effective_color == HeartColor::Score {
                            note_icons += count;
                            cheer_icon_count += count;
                        } else {
                            let idx = heart_color_index(&effective_color);
                            if idx < 7 {
                                *owned_hearts.hearts.entry(effective_color).or_insert(0) += count;
                                bh_arr[idx] += count;
                                total_hearts_arr[idx] += count;
                            }
                            cheer_icon_count += count;
                        }
                    }
                }

                yell_cards.push(YellCardResult {
                    card_id: *card_id,
                    blade_hearts: bh_arr,
                    note_icons,
                    draw_icons,
                });
            }
        }

        // Yell heart source
        let mut yell_heart_arr = empty_h7();
        for yc in &yell_cards {
            for i in 0..7 { yell_heart_arr[i] += yc.blade_hearts[i]; }
        }
        if yell_heart_arr.iter().any(|&v| v > 0) {
            heart_sources.push(HeartSource { source_type: "yell".into(), source: "Yell cards".into(), value: yell_heart_arr });
        }
        blade_sources.push(BladeSource { source_type: "yell".into(), source: format!("{} blades", total_blade), value: total_blade });

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

        // Build heart allocations (direct fill per live card)
        let live_card_ids: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();
        for live_idx in 0..live_card_ids.len() {
            if let Some(card) = card_db.get_card(live_card_ids[live_idx]) {
                if let Some(ref nh) = card.need_heart {
                    let mut remaining = owned_hearts.clone();
                    for (color, needed) in &nh.hearts {
                        let c_idx = heart_color_index(color);
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
                            // tracked by Allocation count
                        }
                    }
                    // Fill remaining gaps with wildcard
                    let wildcard_avail = *remaining.hearts.get(&HeartColor::Heart00).unwrap_or(&0);
                    if wildcard_avail > 0 {
                        let mut wildcard_used = 0u32;
                        for (color, needed) in &nh.hearts {
                            let already = *remaining.hearts.get(color).unwrap_or(&0);
                            let needed_after = if already >= *needed { 0 } else { *needed - already };
                            if needed_after > 0 && wildcard_avail > wildcard_used {
                                let fill = needed_after.min(wildcard_avail - wildcard_used);
                                let c_idx = heart_color_index(color);
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
                    }
                }
            }
        }

        // Check each live card's requirement
        let any_requirement_failed = live_card_ids.iter().any(|&lc_id| {
            card_db.get_card(lc_id).map_or(false, |card| {
                card.need_heart.as_ref().map_or(false, |nh| {
                    !nh.hearts.is_empty() && !card.satisfies_heart_requirement(&owned_hearts)
                })
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
        }
    }
}

// ============== SNAPSHOT BUILDING ==============

pub fn build_snapshot(
    turn: u32,
    player_id: &str,
    perf: &LivePerformanceData,
    card_db: &CardDatabase,
    player: &crate::player::Player,
    note_icons: u32,
) -> crate::types::PerformanceSnapshot {
    let mut lives = Vec::new();
    for &lc_id in &player.live_card_zone.cards {
        let score = card_db.get_card(lc_id).map(|c| c.get_score()).unwrap_or(0);
        lives.push(crate::types::LiveCardResult {
            passed: false,
            score,
            spare: empty_h7(),
            required: empty_h7(),
            filled: empty_h7(),
            adjustments: Vec::new(),
            card_id: lc_id,
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
        triggered_abilities: Vec::new(),
        p0_wins: false,
        p1_wins: false,
    }
}

pub fn snapshot_to_rule_log(snap: &crate::types::PerformanceSnapshot) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("[PF] ── Player {} Performance (Turn {}) ──", snap.player_id, snap.turn));
    for mc in &snap.member_contributions {
        let name = format!("#{} (slot {})", mc.source_id, mc.slot);
        let bh = format!("{:?}", mc.base_hearts);
        lines.push(format!("[PF]   Stage: {} base ♥{} base ★{}", name, bh, mc.base_blades));
        if mc.bonus_hearts.iter().any(|&v| v > 0) || mc.bonus_blades > 0 {
            lines.push(format!("[PF]     bonus ♥{:?} ★+{}", mc.bonus_hearts, mc.bonus_blades));
        }
    }
    lines.push(format!("[PF]   Yell: {} cards yelled", snap.yell_count));
    for yc in &snap.yell_cards {
        let name = format!("#{}", yc.card_id);
        lines.push(format!("[PF]     Card {} ♥{:?} ♪{} ⎋{}", name, yc.blade_hearts, yc.note_icons, yc.draw_icons));
    }
    lines.push(format!("[PF]   Total hearts: {:?}", snap.total_hearts));
    for live in &snap.lives {
        let result = if live.passed { "PASS" } else { "FAIL" };
        lines.push(format!("[PF]   Live card #{}: need {:?} filled {:?} spare {:?} score {} → {}", live.card_id, live.required, live.filled, live.spare, live.score, result));
    }
    let outcome = if snap.p0_wins { "Player 1 wins" } else if snap.p1_wins { "Player 2 wins" } else { "No winner" };
    lines.push(format!("[PF]   Score: {} {} → {}", snap.total_score, if snap.success { "PASS" } else { "FAIL" }, outcome));
    lines
}
