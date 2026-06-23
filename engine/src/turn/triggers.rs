use crate::game_state::{AbilityTrigger, GameState};
use crate::types::LogEntry;

impl super::TurnEngine {
    pub(crate) fn trigger_debut_abilities(
        game_state: &mut GameState,
        player_id: &str,
        card_no: &str,
        _cost_paid: u32,
        baton_touch_used: bool,
    ) {
        let player_id_clone = player_id.to_string();
        let card_no_clone = card_no.to_string();
        let mut abilities_to_trigger: Vec<(String, String, i16)> = Vec::new();

        let _played_card_cost = {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            let areas = [
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            ];
            let mut found_cost = None;
            for area in areas {
                if let Some(card_id) = player.stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        if card.card_no == card_no_clone {
                            found_cost = Some(card.cost);
                            break;
                        }
                    }
                }
            }
            found_cost
        };

        {
            let player = if player_id_clone == game_state.player1.id {
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
                if let Some(card_id) = player.stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        log::debug!(
                            "[DEBUT_TRIG_DBG] stage card_id={} card_no={} card_no_clone={}",
                            card_id,
                            card.card_no,
                            card_no_clone
                        );
                        if card.card_no == card_no_clone {
                            for ability in &card.abilities {
                                let trigger_match = ability.triggers.as_ref().is_some_and(|t| {
                                    t.contains(crate::triggers::DEBUT)
                                        || t.contains(crate::triggers::DEBUT_EN)
                                });
                                log::debug!(
                                    "[DEBUT_TRIG_DBG]   ability={} triggers={:?} match={}",
                                    ability.full_text,
                                    ability.triggers,
                                    trigger_match
                                );
                                if trigger_match {
                                    // Skip abilities that require baton touch if baton touch wasn't used
                                    if !baton_touch_used
                                        && ability
                                            .effect
                                            .as_ref()
                                            .and_then(|e| e.condition.as_ref())
                                            .is_some_and(|c| c.baton_touch_trigger.unwrap_or(false))
                                    {
                                        continue;
                                    }
                                    let card_name = card.name.clone();
                                    let pp = player_id_clone.clone();
                                    game_state.structured_log.push(LogEntry {
                                        text: format!(
                                            "{pp} {card_name} [ステージ]: 能力確認 [登場]"
                                        ),
                                        turn: game_state.turn_number,
                                        player_label: pp,
                                        source_card_id: Some(card_id),
                                        source_card_name: Some(card_name),
                                        category: "trigger_evaluation".to_string(),
                                        metadata: Some(serde_json::json!({
                                            "trigger": "debut",
                                            "zone": "stage",
                                            "result": "pending"
                                        })),
                                    });
                                    let ability_id =
                                        format!("{}_{}", card_no_clone, ability.full_text);
                                    abilities_to_trigger.push((
                                        ability_id,
                                        card_no_clone.clone(),
                                        card_id,
                                    ));
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }

        let moved_snapshot = game_state.recently_moved_cards.clone();
        for (ability_id, card_no, stage_card_id) in abilities_to_trigger {
            game_state.trigger_auto_ability(
                ability_id,
                AbilityTrigger::Debut,
                player_id_clone.clone(),
                Some(card_no),
                Some(stage_card_id),
                moved_snapshot.clone(),
                None,
            );
        }
    }

    /// Count how many stage members have an ability with a trigger containing the given substring.
    pub fn count_stage_members_with_trigger(
        game_state: &GameState,
        player_id: &str,
        trigger_substring: &str,
    ) -> u32 {
        let player = if player_id == game_state.player1.id {
            &game_state.player1
        } else {
            &game_state.player2
        };
        let mut count = 0u32;
        for &cid in &player.stage.stage {
            if cid == -1 {
                continue;
            }
            if let Some(card) = game_state.card_database.get_card(cid) {
                for ability in &card.abilities {
                    if ability
                        .triggers
                        .as_ref()
                        .is_some_and(|t| t.contains(trigger_substring))
                    {
                        count += 1;
                        break; // count each member once
                    }
                }
            }
        }
        count
    }

    /// Trigger "each_time" abilities on live cards that watch for the given trigger keyword.
    /// For each matching each_time ability, enqueue it once per stage member that has
    /// an ability with the matching trigger type.
    ///
    /// Trigger types handled (matched by trigger_substring):
    ///   LIVE_START  — "ライブ開始時" (each_time: live start ability resolves)
    ///   LIVE_SUCCESS — "ライブ成功時" (each_time: live success ability resolves)
    ///   Future: DEBUT, AREA_MOVE, STATE_CHANGE, DISCARD, ENERGY, YELL
    ///
    /// Called from phase and performance transitions in phases.rs and live.rs.
    /// When `specific_member_id` is Some, enqueues once for that specific member.
    /// When None, counts all stage members with matching triggers and enqueues once per member.
    pub fn trigger_each_time_abilities(
        game_state: &mut GameState,
        player_id: &str,
        trigger_substring: &str,
        specific_member_id: Option<i16>,
    ) {
        let player_id_clone = player_id.to_string();
        let member_ids: Vec<i16> = if let Some(mid) = specific_member_id {
            vec![mid]
        } else {
            let mut ids = Vec::new();
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            for &cid in &player.stage.stage {
                if cid == -1 {
                    continue;
                }
                if let Some(card) = game_state.card_database.get_card(cid) {
                    for ability in &card.abilities {
                        if ability
                            .triggers
                            .as_ref()
                            .is_some_and(|t| t.contains(trigger_substring))
                        {
                            ids.push(cid);
                            break;
                        }
                    }
                }
            }
            ids
        };
        if member_ids.is_empty() {
            return;
        }
        let mut abilities: Vec<(String, String, i16, Option<i16>)> = Vec::new();
        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            for &card_id in &player.live_card_zone.cards {
                if let Some(card) = game_state.card_database.get_card(card_id) {
                    for ability in &card.abilities {
                        if ability.triggers.as_deref() != Some(crate::triggers::AUTO) {
                            continue;
                        }
                        let effect = match &ability.effect {
                            Some(e) => e,
                            None => continue,
                        };
                        if effect.trigger_type.as_deref() != Some("each_time") {
                            continue;
                        }
                        let watch_text = match &effect.condition {
                            Some(c) => &c.text,
                            None => &effect.text,
                        };
                        if !watch_text.contains(trigger_substring) {
                            continue;
                        }
                        // Evaluate condition before enqueuing.
                        // For example, an each_time:discard ability with
                        // "このメンバーがステージから控え室に置かれたとき" must
                        // verify via recently_moved_cards that a card actually
                        // moved to discard before this each_time fires.
                        if let Some(ref cond) = effect.condition {
                            let ctx = crate::ability::condition::ConditionContext::new(game_state);
                            if !ctx.evaluate_condition(cond) {
                                continue;
                            }
                        }
                        for &member_id in &member_ids {
                            let aid = format!("{}_{}", card.card_no, ability.full_text);
                            abilities.push((aid, card.card_no.clone(), card_id, Some(member_id)));
                        }
                    }
                }
            }
        }
        let label = crate::game_state::AbilityTrigger::Auto;
        for (aid, card_no, cid, member_id) in abilities {
            game_state.trigger_auto_ability(
                aid,
                label.clone(),
                player_id_clone.clone(),
                Some(card_no),
                Some(cid),
                None,
                member_id,
            );
        }
    }

    fn is_trigger_suppressed(game_state: &GameState, player_id: &str, trigger_name: &str) -> bool {
        let player = if player_id == game_state.player1.id {
            &game_state.player1
        } else {
            &game_state.player2
        };
        let check_card = |card_id: i16| -> bool {
            if card_id == -1 {
                return false;
            }
            if let Some(card) = game_state.card_database.get_card(card_id) {
                for ability in &card.abilities {
                    if let Some(ref effect) = ability.effect {
                        if effect.action == "suppress_ability_trigger" {
                            if effect.suppressed_trigger.as_deref() == Some(trigger_name) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        };
        for &card_id in &player.stage.stage {
            if check_card(card_id) {
                return true;
            }
        }
        for &card_id in &player.live_card_zone.cards {
            if check_card(card_id) {
                return true;
            }
        }
        false
    }

    pub fn trigger_live_start_abilities(game_state: &mut GameState, player_id: &str) {
        if Self::is_trigger_suppressed(game_state, player_id, "live_start") {
            log::debug!(
                "[LIVE_START_SUPPRESSED] live_start abilities suppressed for player {}",
                player_id
            );
            return;
        }

        let player_id_clone = player_id.to_string();
        let mut abilities_to_trigger: Vec<(String, String, Option<i16>)> = Vec::new();
        // Track (card_id, ability_idx) to prevent the same card's ability from
        // firing twice when the card is both a stage member AND a live card.
        let mut seen: std::collections::HashSet<(i16, usize)> = std::collections::HashSet::new();

        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            for card_id in &player.live_card_zone.cards {
                if game_state.negated_abilities.contains(card_id) {
                    continue;
                }
                if let Some(card) = game_state.card_database.get_card(*card_id) {
                    for (aidx, ability) in card.abilities.iter().enumerate() {
                        if ability
                            .triggers
                            .as_ref()
                            .is_some_and(|t| t.contains(crate::triggers::LIVE_START))
                        {
                            if seen.insert((*card_id, aidx)) {
                                let card_name = card.name.clone();
                                let pp = player_id_clone.clone();
                                game_state.structured_log.push(LogEntry {
                                    text: format!(
                                        "{pp} {card_name} [ライブ置場]: 能力確認 [ライブ開始時]"
                                    ),
                                    turn: game_state.turn_number,
                                    player_label: pp,
                                    source_card_id: Some(*card_id),
                                    source_card_name: Some(card_name),
                                    category: "trigger_evaluation".to_string(),
                                    metadata: Some(serde_json::json!({
                                        "trigger": "live_start",
                                        "zone": "live_card_zone",
                                        "result": "pending"
                                    })),
                                });
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                abilities_to_trigger.push((
                                    ability_id,
                                    card.card_no.clone(),
                                    Some(*card_id),
                                ));
                            }
                        }
                    }
                }
            }
            for &card_id in &player.stage.stage {
                if card_id != -1 && !game_state.negated_abilities.contains(&card_id) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        for (aidx, ability) in card.abilities.iter().enumerate() {
                            if ability
                                .triggers
                                .as_ref()
                                .is_some_and(|t| t.contains(crate::triggers::LIVE_START))
                            {
                                if seen.insert((card_id, aidx)) {
                                    let card_name = card.name.clone();
                                    let pp = player_id_clone.clone();
                                    game_state.structured_log.push(LogEntry {
                                        text: format!(
                                            "{pp} {card_name} [ステージ]: 能力確認 [ライブ開始時]"
                                        ),
                                        turn: game_state.turn_number,
                                        player_label: pp,
                                        source_card_id: Some(card_id),
                                        source_card_name: Some(card_name),
                                        category: "trigger_evaluation".to_string(),
                                        metadata: Some(serde_json::json!({
                                            "trigger": "live_start",
                                            "zone": "stage",
                                            "result": "pending"
                                        })),
                                    });
                                    let ability_id =
                                        format!("{}_{}", card.card_no, ability.full_text);
                                    abilities_to_trigger.push((
                                        ability_id,
                                        card.card_no.clone(),
                                        Some(card_id),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        log::debug!(
            "[LIVE_START_TRIGGER] triggering {} abilities for player {}",
            abilities_to_trigger.len(),
            player_id
        );
        for (ability_id, card_no, explicit_card_id) in abilities_to_trigger {
            log::debug!(
                "[LIVE_START_TRIGGER]   ability={} card_no={}",
                ability_id,
                card_no
            );
            game_state.trigger_auto_ability(
                ability_id,
                AbilityTrigger::LiveStart,
                player_id_clone.clone(),
                Some(card_no),
                explicit_card_id,
                None,
                None,
            );
        }
    }

    pub fn trigger_auto_abilities_for_player(game_state: &mut GameState, player_id: &str) {
        log::debug!("[AUTO_TRIGGER] checking stage for player {}", player_id);
        // Delegate to GameState's method, which handles the scan + enqueue
        game_state.trigger_auto_abilities_for_player(player_id);
    }

    pub fn trigger_and_process_auto_abilities(game_state: &mut GameState, player_id: &str) {
        Self::trigger_auto_abilities_for_player(game_state, player_id);
        game_state.process_pending_auto_abilities(player_id);
    }

    pub fn trigger_live_success_abilities(game_state: &mut GameState, player_id: &str) {
        // Evaluate constant modify_required_hearts abilities on cards in the
        // success_live_card_zone before checking live success conditions.
        game_state.evaluate_success_zone_heart_reductions();

        // Restore performance-time need_heart_modifiers that were cleared above.
        // This preserves modifications from live_start triggers and other non-constant
        // sources, ensuring should_trigger_live_success uses the correct requirements.
        // IMPORTANT: use a set to deduplicate (cid,color) pairs — the same global
        // modifier may appear in multiple players' snapshots, causing double-counting.
        let mut restored: std::collections::HashSet<(i16, crate::card::HeartColor)> =
            std::collections::HashSet::new();
        for snap in &game_state.performance_snapshots {
            for (cid, colors) in &snap.performance_need_heart_modifiers {
                for (color, entry) in colors {
                    if !restored.insert((*cid, *color)) {
                        continue;
                    }
                    let target = game_state
                        .mods
                        .need_heart_modifiers
                        .entry(*cid)
                        .or_default()
                        .entry(*color)
                        .or_insert(crate::core::game_modifiers::ModifierEntry::default());
                    if entry.set != 0 && target.set == 0 {
                        target.set = entry.set;
                    }
                    target.additive += entry.additive;
                }
            }
        }

        let player_id_clone = player_id.to_string();
        let mut abilities_to_trigger: Vec<(String, String, i16)> = Vec::new();
        let mut seen: std::collections::HashSet<(i16, usize)> = std::collections::HashSet::new();

        // LiveSuccess only triggers when the live card's need_heart is satisfied
        if !game_state.should_trigger_live_success(if player_id_clone == game_state.player1.id {
            &game_state.player1
        } else {
            &game_state.player2
        }) {
            return;
        }

        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            let skip_negated =
                |gs: &GameState, id: i16| -> bool { gs.negated_abilities.contains(&id) };
            for (_area, index) in [
                (crate::zones::MemberArea::LeftSide, 0),
                (crate::zones::MemberArea::Center, 1),
                (crate::zones::MemberArea::RightSide, 2),
            ] {
                let card_id = player.stage.stage[index];
                if card_id != -1 && !skip_negated(game_state, card_id) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        for (aidx, ability) in card.abilities.iter().enumerate() {
                            if ability
                                .triggers
                                .as_ref()
                                .is_some_and(|t| t == crate::triggers::LIVE_SUCCESS)
                            {
                                if !seen.insert((card_id, aidx)) {
                                    continue;
                                }
                                let card_name = card.name.clone();
                                let pp = player_id_clone.clone();
                                game_state.structured_log.push(LogEntry {
                                    text: format!(
                                        "{pp} {card_name} [ステージ]: 能力確認 [ライブ成功時]"
                                    ),
                                    turn: game_state.turn_number,
                                    player_label: pp,
                                    source_card_id: Some(card_id),
                                    source_card_name: Some(card_name),
                                    category: "trigger_evaluation".to_string(),
                                    metadata: Some(serde_json::json!({
                                        "trigger": "live_success",
                                        "zone": "stage",
                                        "result": "pending"
                                    })),
                                });
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                abilities_to_trigger.push((
                                    ability_id,
                                    card.card_no.clone(),
                                    card_id,
                                ));
                            }
                        }
                    }
                }
            }
            for card_id in &player.live_card_zone.cards {
                if let Some(card) = game_state.card_database.get_card(*card_id) {
                    for (aidx, ability) in card.abilities.iter().enumerate() {
                        let trigger_match = ability.triggers.as_ref().is_some_and(|t| {
                            t == crate::triggers::LIVE_SUCCESS
                                || t.contains(crate::triggers::LIVE_SUCCESS_EN)
                        });
                        if !trigger_match {
                            continue;
                        }
                        if !seen.insert((*card_id, aidx)) {
                            continue;
                        }
                        let card_name = card.name.clone();
                        let pp = player_id_clone.clone();
                        game_state.structured_log.push(LogEntry {
                            text: format!("{pp} {card_name} [ライブ置場]: 能力確認 [ライブ成功時]"),
                            turn: game_state.turn_number,
                            player_label: pp,
                            source_card_id: Some(*card_id),
                            source_card_name: Some(card_name),
                            category: "trigger_evaluation".to_string(),
                            metadata: Some(serde_json::json!({
                                "trigger": "live_success",
                                "zone": "live_card_zone",
                                "result": "pending"
                            })),
                        });
                        let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                        abilities_to_trigger.push((ability_id, card.card_no.clone(), *card_id));
                    }
                }
            }
        }

        for (ability_id, card_no, source_card_id) in abilities_to_trigger {
            game_state.trigger_auto_ability(
                ability_id,
                AbilityTrigger::LiveSuccess,
                player_id_clone.clone(),
                Some(card_no),
                Some(source_card_id),
                None,
                None,
            );
        }
    }
}
