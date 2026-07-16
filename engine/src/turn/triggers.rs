use crate::game_state::{AbilityTrigger, GameState};
use crate::types::LogEntry;
use crate::HashSet;

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
                        if card.card_no.as_ref() == card_no_clone {
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
                        if card.card_no.as_ref() == card_no_clone {
                            for (ability_index, ability) in card.abilities.iter().enumerate() {
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
                                    // Position gate: skip if activation_position doesn't match this area
                                    if !crate::zones::check_effect_position(
                                        ability
                                            .effect
                                            .as_ref()
                                            .and_then(|e| e.activation_position_any()),
                                        area,
                                    ) {
                                        continue;
                                    }
                                    // Skip abilities that require baton touch if baton touch wasn't used
                                    if !baton_touch_used
                                        && ability
                                            .effect
                                            .as_ref()
                                            .and_then(|e| e.condition.as_ref())
                                            .is_some_and(|c| {
                                                c.get_baton_touch_trigger().unwrap_or(false)
                                            })
                                    {
                                        continue;
                                    }
                                    if crate::ability::debug::ABILITY_DEBUG
                                        .load(std::sync::atomic::Ordering::Relaxed)
                                    {
                                        let card_name = &card.name;
                                        let pp = player_id_clone.clone();
                                        game_state.structured_log.push(LogEntry {
                                            text: format!(
                                                "{pp} {card_name} [ステージ]: 能力確認 [登場]"
                                            ),
                                            turn: game_state.turn_number,
                                            player_label: pp,
                                            source_card_id: Some(card_id),
                                            source_card_name: Some(card_name.to_string()),
                                            category: "trigger_evaluation".to_string(),
                                            metadata: Some(serde_json::json!({
                                                "trigger": "debut",
                                                "zone": "stage",
                                                "result": "pending",
                                                "ability_index": ability_index,
                                                "ability_text": ability.full_text,
                                            })),
                                        });
                                    }
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
                            if effect.suppressed_trigger_any().as_deref() == Some(trigger_name) {
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
        if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!(
                "[TLS_ENTER] player={} phase={:?} turn_phase={:?}",
                player_id, game_state.current_phase, game_state.current_turn_phase
            );
        }
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
        let mut seen: HashSet<(i16, usize)> = HashSet::new();

        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
                eprintln!(
                    "[TLS_LIVE] player={} live_zone_cards={:?} stage={:?}",
                    player_id, player.live_card_zone.cards, player.stage.stage
                );
            }
            for card_id in &player.live_card_zone.cards {
                if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
                    eprintln!(
                        "[TLS_LIVE] checking card={} negated={}",
                        card_id,
                        game_state.negated_abilities.contains(card_id)
                    );
                }
                if game_state.negated_abilities.contains(card_id) {
                    continue;
                }
                if let Some(card) = game_state.card_database.get_card(*card_id) {
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(std::sync::atomic::Ordering::Relaxed)
                    {
                        eprintln!(
                            "[TLS_LIVE] card_id={} card_no={} abilities={}",
                            card_id,
                            card.card_no,
                            card.abilities.len()
                        );
                    }
                    for (aidx, ability) in card.abilities.iter().enumerate() {
                        if crate::ability::debug::ABILITY_DEBUG
                            .load(std::sync::atomic::Ordering::Relaxed)
                        {
                            eprintln!("[TLS_LIVE]   aidx={} triggers={:?}", aidx, ability.triggers);
                        }
                        if ability
                            .triggers
                            .as_ref()
                            .is_some_and(|t| t.contains(crate::triggers::LIVE_START))
                        {
                            if seen.insert((*card_id, aidx)) {
                                if crate::ability::debug::ABILITY_DEBUG
                                    .load(std::sync::atomic::Ordering::Relaxed)
                                {
                                    let card_name = &card.name;
                                    let pp = player_id_clone.clone();
                                    game_state.structured_log.push(LogEntry {
                                        text: format!(
                                            "{pp} {card_name} [ライブ置場]: 能力確認 [ライブ開始時]"
                                        ),
                                        turn: game_state.turn_number,
                                        player_label: pp,
                                        source_card_id: Some(*card_id),
                                        source_card_name: Some(card_name.to_string()),
                                        category: "trigger_evaluation".to_string(),
                                        metadata: Some(serde_json::json!({
                                            "trigger": "live_start",
                                            "zone": "live_card_zone",
                                            "result": "pending",
                                            "ability_index": aidx,
                                            "ability_text": ability.full_text,
                                        })),
                                    });
                                }
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                abilities_to_trigger.push((
                                    ability_id,
                                    card.card_no.to_string(),
                                    Some(*card_id),
                                ));
                            }
                        }
                    }
                }
            }
            for (stage_idx, &card_id) in player.stage.stage.iter().enumerate() {
                let card_position = match stage_idx {
                    0 => crate::zones::MemberArea::LeftSide,
                    1 => crate::zones::MemberArea::Center,
                    _ => crate::zones::MemberArea::RightSide,
                };
                if card_id != -1 && !game_state.negated_abilities.contains(&card_id) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        for (aidx, ability) in card.abilities.iter().enumerate() {
                            if !crate::zones::check_effect_position(
                                ability
                                    .effect
                                    .as_ref()
                                    .and_then(|e| e.activation_position_any()),
                                card_position,
                            ) {
                                continue;
                            }
                            if ability
                                .triggers
                                .as_ref()
                                .is_some_and(|t| t.contains(crate::triggers::LIVE_START))
                            {
                                if seen.insert((card_id, aidx)) {
                                    if crate::ability::debug::ABILITY_DEBUG
                                        .load(std::sync::atomic::Ordering::Relaxed)
                                    {
                                        let card_name = &card.name;
                                        let pp = player_id_clone.clone();
                                        game_state.structured_log.push(LogEntry {
                                            text: format!(
                                                "{pp} {card_name} [ステージ]: 能力確認 [ライブ開始時]"
                                            ),
                                            turn: game_state.turn_number,
                                            player_label: pp,
                                            source_card_id: Some(card_id),
                                            source_card_name: Some(card_name.to_string()),
                                            category: "trigger_evaluation".to_string(),
                                            metadata: Some(serde_json::json!({
                                                "trigger": "live_start",
                                                "zone": "stage",
                                                "result": "pending",
                                                "ability_index": aidx,
                                                "ability_text": ability.full_text,
                                            })),
                                        });
                                    }
                                    let ability_id =
                                        format!("{}_{}", card.card_no, ability.full_text);
                                    abilities_to_trigger.push((
                                        ability_id,
                                        card.card_no.to_string(),
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
        game_state.evaluate_success_zone_constant_abilities();

        // Restore performance-time need_heart_modifiers that were cleared above.
        // This preserves modifications from live_start triggers and other non-constant
        // sources, ensuring should_trigger_live_success uses the correct requirements.
        // IMPORTANT: use a set to deduplicate (cid,color) pairs — the same global
        // modifier may appear in multiple players' snapshots, causing double-counting.
        let mut restored: HashSet<(i16, crate::card::HeartColor)> = HashSet::new();
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
        let mut seen: HashSet<(i16, usize)> = HashSet::new();

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
            for (area, index) in [
                (crate::zones::MemberArea::LeftSide, 0),
                (crate::zones::MemberArea::Center, 1),
                (crate::zones::MemberArea::RightSide, 2),
            ] {
                let card_id = player.stage.stage[index];
                if card_id != -1 && !skip_negated(game_state, card_id) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        let card_no = card.card_no.to_string();
                        for (aidx, ability) in card.abilities.iter().enumerate() {
                            if !crate::zones::check_effect_position(
                                ability
                                    .effect
                                    .as_ref()
                                    .and_then(|e| e.activation_position_any()),
                                area,
                            ) {
                                continue;
                            }
                            if ability
                                .triggers
                                .as_ref()
                                .is_some_and(|t| &**t == crate::triggers::LIVE_SUCCESS)
                            {
                                if !seen.insert((card_id, aidx)) {
                                    continue;
                                }
                                if crate::ability::debug::ABILITY_DEBUG
                                    .load(std::sync::atomic::Ordering::Relaxed)
                                {
                                    let card_name = &card.name;
                                    let pp = player_id_clone.clone();
                                    game_state.structured_log.push(LogEntry {
                                        text: format!(
                                            "{pp} {card_name} [ステージ]: 能力確認 [ライブ成功時]"
                                        ),
                                        turn: game_state.turn_number,
                                        player_label: pp,
                                        source_card_id: Some(card_id),
                                        source_card_name: Some(card_name.to_string()),
                                        category: "trigger_evaluation".to_string(),
                                        metadata: Some(serde_json::json!({
                                            "trigger": "live_success",
                                            "zone": "stage",
                                            "result": "pending",
                                            "ability_index": aidx,
                                            "ability_text": ability.full_text,
                                        })),
                                    });
                                }
                                let ability_id = format!("{}_{}", card_no, ability.full_text);
                                abilities_to_trigger.push((ability_id, card_no.clone(), card_id));
                            }
                        }
                        // Also check gained card abilities
                        if let Some(gained_list) = game_state.gained_card_abilities.get(&card_id) {
                            for (gidx, gained_ability) in gained_list.iter().enumerate() {
                                if !crate::zones::check_effect_position(
                                    gained_ability
                                        .effect
                                        .as_ref()
                                        .and_then(|e| e.activation_position_any()),
                                    area,
                                ) {
                                    continue;
                                }
                                if gained_ability
                                    .triggers
                                    .as_ref()
                                    .is_some_and(|t| &**t == crate::triggers::LIVE_SUCCESS)
                                {
                                    if !seen.insert((card_id, 10000 + gidx)) {
                                        continue;
                                    }
                                    if crate::ability::debug::ABILITY_DEBUG
                                        .load(std::sync::atomic::Ordering::Relaxed)
                                    {
                                        let pp = player_id_clone.clone();
                                        game_state.structured_log.push(LogEntry {
                                        text: format!(
                                            "{pp} card#{card_id} [ステージ/獲得]: 能力確認 [ライブ成功時]"
                                        ),
                                        turn: game_state.turn_number,
                                        player_label: pp,
                                        source_card_id: Some(card_id),
                                        source_card_name: None,
                                        category: "trigger_evaluation".to_string(),
                                        metadata: Some(serde_json::json!({
                                            "trigger": "live_success",
                                            "zone": "stage_gained",
                                            "result": "pending",
                                            "ability_index": 10000 + gidx,
                                            "ability_text": gained_ability.full_text,
                                        })),
                                    });
                                    }
                                    let ability_id = format!("{}_gained_{}", card_no, gidx);
                                    abilities_to_trigger.push((
                                        ability_id,
                                        card_no.clone(),
                                        card_id,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            for card_id in &player.live_card_zone.cards {
                if let Some(card) = game_state.card_database.get_card(*card_id) {
                    let card_no = card.card_no.to_string();
                    for (aidx, ability) in card.abilities.iter().enumerate() {
                        let trigger_match = ability.triggers.as_ref().is_some_and(|t| {
                            &**t == crate::triggers::LIVE_SUCCESS
                                || t.contains(crate::triggers::LIVE_SUCCESS_EN)
                        });
                        if !trigger_match {
                            continue;
                        }
                        if !seen.insert((*card_id, aidx)) {
                            continue;
                        }
                        if crate::ability::debug::ABILITY_DEBUG
                            .load(std::sync::atomic::Ordering::Relaxed)
                        {
                            let card_name = &card.name;
                            let pp = player_id_clone.clone();
                            game_state.structured_log.push(LogEntry {
                                text: format!(
                                    "{pp} {card_name} [ライブ置場]: 能力確認 [ライブ成功時]"
                                ),
                                turn: game_state.turn_number,
                                player_label: pp,
                                source_card_id: Some(*card_id),
                                source_card_name: Some(card_name.to_string()),
                                category: "trigger_evaluation".to_string(),
                                metadata: Some(serde_json::json!({
                                    "trigger": "live_success",
                                    "zone": "live_card_zone",
                                    "result": "pending",
                                    "ability_index": aidx,
                                    "ability_text": ability.full_text,
                                })),
                            });
                        }
                        let ability_id = format!("{}_{}", card_no, ability.full_text);
                        abilities_to_trigger.push((ability_id, card_no.clone(), *card_id));
                    }
                    // Also check gained card abilities
                    if let Some(gained_list) = game_state.gained_card_abilities.get(card_id) {
                        for (gidx, gained_ability) in gained_list.iter().enumerate() {
                            if gained_ability
                                .triggers
                                .as_ref()
                                .is_some_and(|t| &**t == crate::triggers::LIVE_SUCCESS)
                            {
                                if !seen.insert((*card_id, 10000 + gidx)) {
                                    continue;
                                }
                                if crate::ability::debug::ABILITY_DEBUG
                                    .load(std::sync::atomic::Ordering::Relaxed)
                                {
                                    let pp = player_id_clone.clone();
                                    game_state.structured_log.push(LogEntry {
                                    text: format!(
                                        "{pp} card#{card_id} [ライブ置場/獲得]: 能力確認 [ライブ成功時]"
                                    ),
                                    turn: game_state.turn_number,
                                    player_label: pp,
                                    source_card_id: Some(*card_id),
                                    source_card_name: None,
                                    category: "trigger_evaluation".to_string(),
                                    metadata: Some(serde_json::json!({
                                        "trigger": "live_success",
                                        "zone": "live_card_zone_gained",
                                        "result": "pending",
                                        "ability_index": 10000 + gidx,
                                        "ability_text": gained_ability.full_text,
                                    })),
                                });
                                }
                                let ability_id = format!("{}_gained_{}", card_no, gidx);
                                abilities_to_trigger.push((ability_id, card_no.clone(), *card_id));
                            }
                        }
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
