use crate::core::game_state::GameState;
use crate::ability::enums::Zone;
impl GameState {
    pub fn get_pending_choice(&self) -> Option<&crate::ability::types::Choice> {
        self.pending_loop_protocol
            .as_ref()
            .map(|protocol| &protocol.choice)
            .or_else(|| self.ability_queue.is_waiting_for_choice())
    }

    pub fn has_pending_choice(&self) -> bool {
        self.pending_loop_protocol.is_some() || self.ability_queue.is_waiting_for_choice().is_some()
    }

    pub fn get_pending_choice_player_id(&self) -> Option<String> {
        if self.pending_loop_protocol.is_some() {
            let active = if core::ptr::eq(self.active_player(), &self.player1) {
                &self.player2.id
            } else {
                &self.player1.id
            };
            return Some(active.to_string());
        }
        let entry = self.ability_queue.current_entry();
        // A REAL entry always carries a stamp: `pause_for_choice` applies the universal
        // default from the entry's own `player_id` before parking the choice
        // (`ability/queue.rs`, "Universal default"). So a `None` here can only be the
        // dummy entry that `pause_for_choice` synthesises when the queue is `Idle` —
        // that one is created with an empty `player_id`, so it is identifiable exactly.
        //
        // Without a stamp here the choice is not merely unlabelled: `web_server.rs`
        // DROPS the whole `pending_choice` object when `choice_player_id` is missing,
        // so in PVP the prompt is hidden from BOTH seats and the game waits for an
        // answer that can never arrive. The owner of a choice raised outside the queue
        // is whoever the game is currently waiting on — the active player.
        match entry {
            Some(e) if e.choice_player_id.is_none() && e.player_id.is_empty() => {
                // …but only when the CHOICE does not name an owner itself. A `picker` /
                // `target_player_id` of `p2` is a deliberate routing decision and is MORE
                // specific than "whoever is active"; fabricating a stamp here would
                // override it, and the observation layer reads those fields to build its
                // per-viewer view (`bot/observation.rs`), so overriding them makes an
                // opponent-only prompt visible to the wrong seat.
                //
                // Only an ABSOLUTE seat counts. `self` / `opponent` are relative and
                // resolve against the owning entry, so they identify nobody — which is
                // exactly the case this fallback exists for.
                if let Some(c) = self.get_pending_choice() {
                    let absolute = |s: &str| matches!(s, "p1" | "p2" | "player1" | "player2");
                    let names_owner = match c {
                        crate::ability::types::Choice::SelectCard {
                            picker,
                            target_player_id,
                            ..
                        } => {
                            picker.as_deref().is_some_and(absolute)
                                || target_player_id.as_deref().is_some_and(absolute)
                        }
                        crate::ability::types::Choice::SelectAutoAbility { .. }
                        | crate::ability::types::Choice::SelectLiveSuccess { .. } => true,
                        _ => false,
                    };
                    if names_owner {
                        return None;
                    }
                }
                Some(self.active_player().id.to_string())
            }
            other => other.and_then(|e| e.choice_player_id.clone()),
        }
    }

    pub fn get_pending_choice_json(&self) -> Option<serde_json::Value> {
        let choice = self.get_pending_choice()?;
        let mut json = choice.to_frontend_json()?;
        self.inject_choice_ability_context(&mut json);
        Some(json)
    }

    #[cfg(feature = "serde_support")]
    pub fn inject_choice_ability_context(&self, json: &mut serde_json::Value) {
        let entry = self.ability_queue.current_entry();
        let entry_ref = entry.as_ref();
        let existing_title = json
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(obj) = json.as_object_mut() {
            if let Some(entry) = entry_ref {
                obj.insert(
                    "card_no".into(),
                    serde_json::Value::String(entry.card_no.to_string()),
                );
                obj.insert(
                    "ability_text".into(),
                    serde_json::Value::String(entry.ability.full_text.clone()),
                );
                // Only inject prompt_en/prompt_ja if the Choice-level fields are absent.
                // Choice-level fields are set by to_frontend_json() from description_en/description_ja.
                if !obj.contains_key("prompt_en") {
                    let prompt_en = if entry.choice_effect_text.is_some() {
                        if let Some(ref effect) = entry.ability.effect {
                            crate::ability::describe::describe_effect_en(effect)
                        } else {
                            existing_title.clone()
                        }
                    } else {
                        existing_title.clone()
                    };
                    obj.insert("prompt_en".into(), serde_json::Value::String(prompt_en.clone()));
                }
                // Always derive a Japanese prompt so the UI never silently falls back to
                // English in Japanese mode. Engine is the single source of truth:
                //   1. generic instruction template translator (parameterized prompts)
                //   2. effect-backed Japanese description
                //   3. English prompt (last resort) -- and we WARN so the gap is caught.
                if !obj.contains_key("prompt_ja") {
                    let prompt_en = obj
                        .get("prompt_en")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let prompt_ja = crate::ability::describe::translate_choice_prompt_en_to_ja(&prompt_en)
                        .or_else(|| {
                            if entry.choice_effect_text.as_deref().is_some_and(|t| !t.is_empty()) {
                                entry.ability.effect.as_ref().map(|e| crate::ability::describe::describe_effect_ja(e))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| prompt_en.clone());
                    if !prompt_en.is_empty() && prompt_ja == prompt_en {
                        log::warn!(
                            "[i18n] choice prompt has no Japanese translation (English shown): {:?}",
                            prompt_en
                        );
                    }
                    obj.insert("prompt_ja".into(), serde_json::Value::String(prompt_ja));
                }
                obj.insert(
                    "trigger_type".into(),
                    serde_json::Value::String(format!("{:?}", entry.trigger_type)),
                );
                if let Some(cid) = entry.card_id {
                    obj.insert(
                        "card_id".into(),
                        serde_json::Value::Number(serde_json::Number::from(cid as i64)),
                    );
                    if let Some(card) = self.card_database.get_card(cid) {
                        obj.insert(
                            "card_name".into(),
                            serde_json::Value::String(card.name.to_string()),
                        );
                    }
                }
                let raw_pid = entry.choice_player_id.as_ref().unwrap_or(&entry.player_id);
                let normalized = match raw_pid.as_str() {
                    "player1" => "p1",
                    "player2" => "p2",
                    _ => raw_pid.as_str(),
                };
                obj.insert(
                    "choice_player_id".into(),
                    serde_json::Value::String(normalized.to_string()),
                );
                // Inject selection_cards for SelectCard choices so the frontend can render options.
                if let Some(crate::ability::types::Choice::SelectCard {
                    ref zone,
                    ref card_type,
                    cost_limit,
                    ref cost_limit_operator,
                    ref target_player_id,
                    ref group,
                    ref characters,
                    ref filtered_indices,
                    ..
                }) = self.ability_queue.is_waiting_for_choice()
                {
                        let target = target_player_id.as_deref().unwrap_or("self");
                        let player = self.resolve_target_player(target);
                        let card_ids: Vec<i16> = match Zone::from_str(zone) {
                            Some(Zone::Hand) => player.hand.cards.iter().copied().collect(),
                            Some(Zone::Discard) | Some(Zone::Waitroom) => {
                                player.waitroom.cards.iter().copied().collect()
                            }
                            Some(Zone::Stage) => player
                                .stage
                                .stage
                                .iter()
                                .copied()
                                .filter(|&id| id != -1)
                                .collect(),
                            Some(Zone::Energy) | Some(Zone::EnergyZone) => {
                                player.energy_zone.cards.iter().copied().collect()
                            }
                            Some(Zone::LookedAt) => self.looked_at_cards.to_vec(),
                            Some(Zone::RevealedCards) => self.revealed_cards.to_vec(),
                            Some(Zone::Deck) => player.main_deck.cards.iter().copied().collect(),
                            Some(Zone::SelectedCards) => entry
                                .resolver
                                .as_ref()
                                .map(|r| r.selection.cards.to_vec())
                                .unwrap_or_default(),
                            _ => Vec::new(),
                        };
                        // When filtered_indices is set (look_and_select with greyed-out cards),
                        // include ALL cards in selection_cards -- filtered_indices restricts
                        // selection on the frontend. Otherwise, filter by choice criteria.
                        let filtered: Vec<i16> = if filtered_indices.is_some() {
                            card_ids
                        } else {
                            let card_db = &self.card_database;
                            card_ids
                                .into_iter()
                                .filter(|&cid| {
                                    let type_ok = match card_type.as_deref() {
                                        Some("member_card") => card_db
                                            .get_card(cid)
                                            .map(|c| c.is_member())
                                            .unwrap_or(false),
                                        Some("live_card") => card_db
                                            .get_card(cid)
                                            .map(|c| c.is_live())
                                            .unwrap_or(false),
                                        Some("energy_card") => card_db
                                            .get_card(cid)
                                            .map(|c| c.is_energy())
                                            .unwrap_or(false),
                                        None => true,
                                        _ => true,
                                    };
                                    let group_ok = match group.as_ref() {
                                        Some(g) => crate::ability::util::card_matches_group_str(
                                            card_db,
                                            cid,
                                            Some(g),
                                        ),
                                        None => true,
                                    };
                                    let chars_ok = match characters.as_ref() {
                                        Some(chars) => {
                                            crate::ability::util::card_matches_characters(
                                                card_db,
                                                cid,
                                                Some(chars),
                                            )
                                        }
                                        None => true,
                                    };
                                    let cost_ok = if let Some(lim) = cost_limit {
                                        crate::ability::util::card_matches_cost_limit_op(
                                            card_db,
                                            cid,
                                            Some(*lim),
                                            cost_limit_operator.as_deref(),
                                        )
                                    } else {
                                        true
                                    };
                                    type_ok && group_ok && chars_ok && cost_ok
                                })
                                .collect()
                        };
                        let sel: Vec<serde_json::Value> = filtered.iter().map(|&cid| {
                            let card_ref = self.card_database.get_card(cid);
                            let card_type_val = card_ref.map(|c| serde_json::to_value(&c.card_type).unwrap_or_default());
                            serde_json::json!({
                                "id": cid,
                                "card_no": card_ref.map(|c| c.card_no.to_string()).unwrap_or_default(),
                                "name": card_ref.map(|c| c.name.to_string()).unwrap_or_default(),
                                "type": card_type_val.unwrap_or_default()
                            })
                        }).collect();
                        obj.insert("selection_cards".into(), serde_json::Value::Array(sel));
                    }
            } else if let Some(choice) = self.ability_queue.is_waiting_for_choice() {
                // The or-pattern has to bind through a reference, so this
                // cannot collapse into the outer `if let` (clippy asks, the
                // pattern does not type-check against `Option<&Choice>`).
                #[allow(clippy::collapsible_match)]
                if let crate::ability::types::Choice::SelectAutoAbility { ref player_id, .. }
                | crate::ability::types::Choice::SelectLiveSuccess { ref player_id, .. } = choice
                {
                    let normalized = match player_id.as_str() {
                        "player1" => "p1",
                        "player2" => "p2",
                        _ => player_id.as_str(),
                    };
                    obj.insert(
                        "choice_player_id".into(),
                        serde_json::Value::String(normalized.to_string()),
                    );
                }
            }
        }
    }
}
