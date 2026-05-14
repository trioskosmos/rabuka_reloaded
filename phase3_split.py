#!/usr/bin/env python3
import re

path = (
    r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\src\ability\effects.rs"
)

with open(path, encoding="utf-8") as f:
    raw = f.read()

# Step 1: Replace inline deck->energy with call
old_deck = (
    "        // Draw from energy deck and place in energy zone with state (e.g. wait)\n"
    '        if source == Some("deck") && destination == Some("energy_zone") {\n'
    "            let player = self.game_state.resolve_target_player_mut(&target);\n"
    "            for _ in 0..count {\n"
    "                if let Some(energy_id) = player.energy_deck.draw() {\n"
    "                    player.energy_zone.cards.push(energy_id);\n"
    '                    if state_change == "wait" {\n'
    "                        // Card is placed in wait (not counted as active)\n"
    '                    } else if state_change == "active" {\n'
    "                        player.energy_zone.active_energy_count += 1;\n"
    "                    }\n"
    "                }\n"
    "            }\n"
    "            return Ok(());\n"
    "        }"
)
new_deck = (
    "        // Draw from energy deck and place in energy zone with state (e.g. wait)\n"
    '        if source == Some("deck") && destination == Some("energy_zone") {\n'
    "            self.execute_energy_placement(&state_change, &target, count);\n"
    "            return Ok(());\n"
    "        }"
)
raw = raw.replace(old_deck, new_deck)

# Step 2: Replace the member state return+energy comment with a call
old_mem_end = (
    "            return Ok(());\n"
    "        }\n"
    "\n"
    "        // Energy card state change (original behavior)"
)
new_mem_end = (
    "            return Ok(());\n"
    "        }\n"
    "\n"
    "        // Energy card state change (original behavior) — delegated\n"
    "        return self.execute_energy_state_change(\n"
    "            &state_change, &target, count, max,\n"
    "            card_type_filter.as_deref(), group_filter.as_deref(),\n"
    "        );"
)
raw = raw.replace(old_mem_end, new_mem_end)

# Step 3: Find execute_modify_score position, extract energy body, insert helpers
pos = raw.find("    fn execute_modify_score(")
energy_start = raw.find("        // Energy card state change (original behavior)")

energy_body = raw[energy_start:pos]

# Build the 3 helper functions
helpers = """
    /// Place energy from deck to energy zone with specific state (wait/active).
    fn execute_energy_placement(
        &mut self, state_change: &str, target: &str, count: u32,
    ) {
        let player = self.game_state.resolve_target_player_mut(target);
        for _ in 0..count {
            if let Some(energy_id) = player.energy_deck.draw() {
                player.energy_zone.cards.push(energy_id);
                if state_change == "active" {
                    player.energy_zone.active_energy_count += 1;
                }
            }
        }
    }

    /// Change the state of energy zone cards (wait/active).
    fn execute_energy_state_change(
        &mut self,
        state_change: &str, target: &str, count: u32, max: bool,
        card_type_filter: Option<&str>, group_filter: Option<&str>,
    ) -> Result<(), String> {
        let card_db = self.card_db();
        let (wait_cards, deactivate_count) = {
            let player = self.game_state.resolve_target_player_mut(target);

            let filter = util::filter_from_parts(
                card_type_filter,
                group_filter,
                None,
                None,
                None, None, None,
            );
            let valid_indices =
                util::matching_indices(&player.energy_zone.cards, &card_db, &filter, false);

            let effective_count = if max {
                let available = match state_change {
                    "active" | "\u30a2\u30af\u30c6\u30a3\u30d6" => player
                        .energy_zone
                        .cards
                        .len()
                        .saturating_sub(player.energy_zone.active_energy_count),
                    _ => player.energy_zone.active_energy_count,
                };
                let capped = (count as usize).min(available) as u32;
                eprintln!(
                    "[ENERGY] max=true: count={} available={} effective={}",
                    count, available, capped
                );
                capped
            } else {
                eprintln!("[ENERGY] max=false: count={} effectve={}", count, count);
                count
            };

            if valid_indices.len() < effective_count as usize {
                return Err(format!(
                    "Not enough energy cards to deactivate: need {}, have {}",
                    effective_count,
                    valid_indices.len()
                ));
            }

            if !max && valid_indices.len() > effective_count as usize {
                if state_change != "active" && state_change != "\u30a2\u30af\u30c6\u30a3\u30d6" {
                    self.pending_choice = Some(Choice::SelectCard {
                        zone: "energy_zone".to_string(),
                        card_type: card_type_filter.map(|s| s.to_string()),
                        count: effective_count as usize,
                        description: format!(
                            "Select {} energy card(s) to deactivate (set to wait)",
                            effective_count
                        ),
                        allow_skip: false,
                        cost_limit: None,
                        cost_limit_operator: None,
                        group: None,
                        characters: None,
                        filtered_indices: None,
                        is_select_action: false,
            heart_colors: vec![],
            name_fragments: None,
        });
                    self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                    return Ok(());
                }
            }

            let wait_cards: Vec<i16> = valid_indices
                .iter()
                .take(effective_count as usize)
                .filter_map(|i| {
                    if *i < player.energy_zone.cards.len() {
                        Some(player.energy_zone.cards[*i])
                    } else {
                        None
                    }
                })
                .collect();

            (wait_cards, effective_count)
        };

        let active_cards: Vec<i16> = if state_change == "active" || state_change == "\u30a2\u30af\u30c6\u30a3\u30d6" {
            let player = self.game_state.resolve_target_player(target);
            let mut result = Vec::new();
            let mut active_count = 0u32;
            for i in 0..player.energy_zone.cards.len() {
                if active_count >= deactivate_count {
                    break;
                }
                if let Some(&card_id) = player.energy_zone.cards.get(i) {
                    let matches_type = card_type_filter.map_or(true, |ct| {
                        util::card_matches_type(&card_db, card_id, Some(ct))
                    });
                    let matches_grp = group_filter.map_or(true, |gf| {
                        util::card_matches_group_str(&card_db, card_id, Some(gf))
                    });
                    if matches_type && matches_grp {
                        result.push(card_id);
                        active_count += 1;
                    }
                }
            }
            result
        } else {
            vec![]
        };

        match state_change {
            "wait" | "\u30a6\u30a7\u30a4\u30c8" => {
                for card_id in &wait_cards {
                    self.game_state.mods.add_orientation_modifier(*card_id, "wait");
                }
                for _ in 0..deactivate_count {
                    let player = self.game_state.resolve_target_player_mut(target);
                    player.energy_zone.active_energy_count =
                        player.energy_zone.active_energy_count.saturating_sub(1);
                }
            }
            "active" | "\u30a2\u30af\u30c6\u30a3\u30d6" => {
                for card_id in &active_cards {
                    self.game_state.mods.add_orientation_modifier(*card_id, "active");
                }
                let player = self.game_state.resolve_target_player_mut(target);
                player.energy_zone.active_energy_count += active_cards.len();
            }
            _ => {}
        }
        Ok(())
    }

"""

# Insert helpers before execute_modify_score, remove the old energy body
new_raw = raw[:energy_start] + raw[pos:]
new_raw = (
    new_raw[: new_raw.find("    fn execute_modify_score(")]
    + helpers
    + new_raw[new_raw.find("    fn execute_modify_score(") :]
)

with open(path, "w", encoding="utf-8") as f:
    f.write(new_raw)
print("Phase 3 done: execute_change_state split into 3 functions")
