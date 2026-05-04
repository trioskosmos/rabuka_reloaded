impl GameState {

    pub fn trigger_auto_ability(&mut self, ability_id: String, trigger_type: AbilityTrigger, player_id: String, source_card_id: Option<String>, explicit_card_id: Option<i16>) {
        use crate::ability_queue::{AbilityQueueEntry, AbilityId};

        if let Some(ref card_no) = source_card_id {
            let (card, card_id) = if let Some(cid) = explicit_card_id {
                (self.card_database.get_card(cid).cloned(), Some(cid))
            } else {
                self.find_card_by_number_for_player(card_no, &player_id)
            };
            if let Some(card) = card {
                for (ability_index, ability) in card.abilities.iter().enumerate() {
                    if ability_id.contains(&ability.full_text) {
                        let entry = AbilityQueueEntry {
                            id: AbilityId::new(card_no, ability_index, &format!("{:?}", trigger_type)),
                            card_no: card_no.clone(),
                            player_id,
                            ability: ability.clone(),
                            ability_index,
                            card_id,
                            trigger_type,
                            completed: false,
                            pending_choice_result: None,
                            choice_card_no: None,
                            conditional_choice: None,
                            execution_context: None,
                        };

                        self.ability_queue.enqueue(entry);
                        break;
                    }
                }
            }
        }
    }

    /// Search for a card in the specified player's zones first, fall back to the other player.
    fn find_card_by_number_for_player(&self, card_no: &str, player_id: &str) -> (Option<crate::card::Card>, Option<i16>) {
        let preferred = if player_id == self.player1.id || player_id == "p1" { &self.player1 } else { &self.player2 };
        let other = if std::ptr::eq(preferred, &self.player1) { &self.player2 } else { &self.player1 };
        let result = self.search_player_zones_for_card(card_no, preferred);
        if result.0.is_some() { return result; }
        self.search_player_zones_for_card(card_no, other)
    }

    fn search_player_zones_for_card(&self, card_no: &str, player: &Player) -> (Option<crate::card::Card>, Option<i16>) {
        for id in &player.hand.cards {
            if let Some(card) = self.card_database.get_card(*id) {
                if card.card_no == card_no { return (Some(card.clone()), Some(*id)); }
            }
        }
        for stage_card_id in &player.stage.stage {
            if *stage_card_id != -1 {
                if let Some(card) = self.card_database.get_card(*stage_card_id) {
                    if card.card_no == card_no { return (Some(card.clone()), Some(*stage_card_id)); }
                }
            }
        }
        for waitroom_card_id in &player.waitroom.cards {
            if let Some(card) = self.card_database.get_card(*waitroom_card_id) {
                if card.card_no == card_no { return (Some(card.clone()), Some(*waitroom_card_id)); }
            }
        }
        for live_card_id in &player.live_card_zone.cards {
            if let Some(card) = self.card_database.get_card(*live_card_id) {
                if card.card_no == card_no { return (Some(card.clone()), Some(*live_card_id)); }
            }
        }
        for success_card_id in &player.success_live_card_zone.cards {
            if let Some(card) = self.card_database.get_card(*success_card_id) {
                if card.card_no == card_no { return (Some(card.clone()), Some(*success_card_id)); }
            }
        }
        (None, None)
    }

    pub fn process_pending_auto_abilities(&mut self, _active_player_id: &str) {
        loop {
            if !self.ability_queue.is_idle() {
                break;
            }
            if !self.ability_queue.start_next() {
                break;
            }
            self.process_current_ability();
            // If a choice is pending, stop processing and wait for player input
            if self.pending_choice.is_some() {
                break;
            }
        }
    }

    fn process_current_ability(&mut self) {
        if let Some(entry) = self.ability_queue.current_entry().cloned() {
            self.activating_card = entry.card_id;

            let (choice, looked_at, ctx, rev, result) = {
                let mut resolver = crate::ability_resolver::AbilityResolver::new(self);
                let result = resolver.resolve_ability(&entry.ability, entry.card_id, entry.ability_index);
                let choice = resolver.get_pending_choice().cloned();
                let looked_at = resolver.take_looked_at();
                let ctx = resolver.execution_context.clone();
                let rev = std::mem::take(&mut resolver.revealed_cost_cards);
                (choice, looked_at, ctx, rev, result)
            };
            self.looked_at_cards = looked_at;
            self.revealed_cost_cards = rev;

            if let Err(e) = result {
                eprintln!("Failed to resolve ability: {}", e);
                self.ability_queue.complete_current();
                return;
            }

            if let Some(c) = choice {
                if let Some(e) = self.ability_queue.current_entry_mut() {
                    e.execution_context = Some(ctx);
                }
                self.ability_queue.pause_for_choice(c);
            } else {
                self.ability_queue.complete_current();
                self.activating_card = None;
            }
        }
    }

    pub fn get_pending_choice(&self) -> Option<&crate::ability_resolver::Choice> {
        self.ability_queue.is_waiting_for_choice()
    }

    pub fn entry_effect(&self) -> Option<&crate::card::AbilityEffect> {
        self.ability_queue.current_entry().and_then(|e| e.ability.effect.as_ref())
    }

    pub fn entry_cost(&self) -> Option<&crate::card::AbilityCost> {
        self.ability_queue.current_entry().and_then(|e| e.ability.cost.as_ref())
    }

    pub fn entry_characters(&self) -> Option<&Vec<String>> {
        self.entry_cost().and_then(|c| c.characters.as_ref())
    }

    pub fn entry_destination(&self) -> Option<&str> {
        self.entry_effect().and_then(|e| e.destination.as_deref())
    }

    pub fn entry_choice_card_no(&self) -> Option<String> {
        self.ability_queue.current_entry().and_then(|e| e.choice_card_no.clone())
    }

    pub fn entry_conditional_choice(&self) -> Option<String> {
        self.ability_queue.current_entry().and_then(|e| e.conditional_choice.clone())
    }

    /// Resolve which player "self" refers to based on the ability master's player_id.
    /// The ability queue entry stores which player activated this ability.
    fn ability_master_id(&self) -> Option<String> {
        self.ability_queue.current_entry().map(|e| e.player_id.clone())
    }

    pub fn resolve_target_player_mut(&mut self, target: &str) -> &mut Player {
        let master = self.ability_master_id();
        match (target, master.as_deref()) {
            ("self", Some("player2") | Some("p2")) => &mut self.player2,
            ("self", _) => &mut self.player1,
            ("opponent", Some("player2") | Some("p2")) => &mut self.player1,
            ("opponent", _) => &mut self.player2,
            ("both", _) => {
                eprintln!("WARN: resolve_target_player_mut called with 'both' — returning player1, use execute_for_targets instead");
                &mut self.player1
            }
            _ => &mut self.player1,
        }
    }

    pub fn resolve_target_player(&self, target: &str) -> &Player {
        let master = self.ability_master_id();
        match (target, master.as_deref()) {
            ("self", Some("player2") | Some("p2")) => &self.player2,
            ("self", _) => &self.player1,
            ("opponent", Some("player2") | Some("p2")) => &self.player1,
            ("opponent", _) => &self.player2,
            _ => &self.player1,
        }
    }

    pub fn check_victory(&self) -> GameResult {
        let p1_success = self.player1.success_live_card_zone.len();
        let p2_success = self.player2.success_live_card_zone.len();

        let p1_wins = p1_success >= 3 && p2_success <= 2;
        let p2_wins = p2_success >= 3 && p1_success <= 2;

        if p1_success >= 3 && p2_success >= 3 {
            GameResult::Draw
        } else if p1_wins && !p2_wins {
            GameResult::FirstAttackerWins
        } else if p2_wins && !p1_wins {
            GameResult::SecondAttackerWins
        } else {
            GameResult::Ongoing
        }
    }

    pub fn resolve_target<'a>(&'a self, target: &str, perspective_player: &'a Player) -> Vec<&'a Player> {
        match target {
            "self" | "自分" => {
                vec![perspective_player]
            }
            "opponent" | "相手" => {
                if std::ptr::eq(perspective_player, &self.player1) {
                    vec![&self.player2]
                } else {
                    vec![&self.player1]
                }
            }
            "both" | "両方" => {
                vec![&self.player1, &self.player2]
            }
            "either" | "どちらか" => {
                vec![&self.player1, &self.player2]
            }
            _ => vec![],
        }
    }

    pub fn resolve_target_mut(&mut self, target: &str, perspective_player_id: &str) -> Vec<&mut Player> {
        match target {
            "self" | "自分" => {
                if perspective_player_id == self.player1.id {
                    vec![&mut self.player1]
                } else {
                    vec![&mut self.player2]
                }
            }
            "opponent" | "相手" => {
                if perspective_player_id == self.player1.id {
                    vec![&mut self.player2]
                } else {
                    vec![&mut self.player1]
                }
            }
            "both" | "両方" => {
                vec![&mut self.player1, &mut self.player2]
            }
            "either" | "どちらか" => {
                vec![&mut self.player1, &mut self.player2]
            }
            _ => vec![],
        }
    }

    pub fn get_player(&self, player_id: &str) -> Option<&Player> {
        if self.player1.id == player_id {
            Some(&self.player1)
        } else if self.player2.id == player_id {
            Some(&self.player2)
        } else {
            None
        }
    }

    pub fn get_player_mut(&mut self, player_id: &str) -> Option<&mut Player> {
        if self.player1.id == player_id {
            Some(&mut self.player1)
        } else if self.player2.id == player_id {
            Some(&mut self.player2)
        } else {
            None
        }
    }

    pub fn should_trigger_debut(&self, _player: &Player, card: &crate::card::Card) -> bool {
        card.is_member()
    }

    pub fn should_trigger_live_start(&self, _player: &Player) -> bool {
        self.current_phase == Phase::FirstAttackerPerformance
            || self.current_phase == Phase::SecondAttackerPerformance
    }

    pub fn should_trigger_live_success(&self, _player: &Player) -> bool {
        self.current_phase == Phase::LiveVictoryDetermination
    }

    pub fn can_place_card_in_zone(&self, card_id: i16, zone: &str, _player_id: &str) -> bool {
        if let Some(card) = self.card_database.get_card(card_id) {
            for ability in &card.abilities {
                if ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::CONSTANT)) {
                    if let Some(ref effect) = ability.effect {
                        if effect.action == "restriction"
                            && effect.restriction_type.as_deref() == Some("cannot_place")
                            && (effect.restricted_destination.as_deref() == Some(zone)
                                || effect.restricted_destination.as_deref() == Some("live_card_zone") && zone == "success_live_zone"
                                || effect.restricted_destination.as_deref() == Some("success_live_zone") && zone == "live_card_zone")
                        {
                            eprintln!("Card {} cannot be placed in {} due to constant ability restriction", card.card_no, zone);
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    pub fn enforce_constant_ability_restrictions(&mut self) {
        let p1_id = self.player1.id.clone();
        let p2_id = self.player2.id.clone();
        let p1_cards: Vec<(usize, i16)> = self.player1.live_card_zone.cards.iter().enumerate().map(|(i, &id)| (i, id)).collect();
        let p2_cards: Vec<(usize, i16)> = self.player2.live_card_zone.cards.iter().enumerate().map(|(i, &id)| (i, id)).collect();

        let mut cards_to_remove: Vec<(&str, usize)> = Vec::new();
        for (index, card_id) in p1_cards {
            if !self.can_place_card_in_zone(card_id, "live_card_zone", &p1_id) {
                cards_to_remove.push((&p1_id, index));
            }
        }
        for (index, card_id) in p2_cards {
            if !self.can_place_card_in_zone(card_id, "live_card_zone", &p2_id) {
                cards_to_remove.push((&p2_id, index));
            }
        }

        for (player_id, index) in cards_to_remove {
            let player = if *player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 };
            let card = player.live_card_zone.cards.remove(index);
            player.waitroom.cards.push(card);
            if let Some(card_data) = self.card_database.get_card(card) {
                eprintln!("Removed card {} from live_card_zone due to constant ability restriction", card_data.card_no);
            }
        }
    }

    pub fn get_triggerable_abilities<'a>(
        &self,
        card: &'a crate::card::Card,
        trigger: AbilityTrigger,
        player: &Player,
    ) -> Vec<&'a crate::card::Ability> {
        card.abilities.iter().filter(|ability| {
            match trigger {
                AbilityTrigger::Activation => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::ACTIVATION));
                    trigger_match
                }
                AbilityTrigger::Debut => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::DEBUT) || t.contains(crate::triggers::DEBUT_EN));
                    let should_trigger = trigger_match && self.should_trigger_debut(player, card);
                    should_trigger
                }
                AbilityTrigger::LiveStart => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::LIVE_START));
                    let should_trigger = trigger_match && self.should_trigger_live_start(player);
                    should_trigger
                }
                AbilityTrigger::LiveSuccess => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::LIVE_SUCCESS));
                    let should_trigger = trigger_match && self.should_trigger_live_success(player);
                    should_trigger
                }
                AbilityTrigger::Constant => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::CONSTANT));
                    trigger_match
                }
                AbilityTrigger::Auto => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::AUTO));
                    trigger_match
                }
            }
        }).collect()
    }

    pub fn add_temporary_effect(
        &mut self,
        effect_type: String,
        duration: Duration,
        target_player_id: String,
        description: String,
    ) {
        let order = self.effect_creation_counter;
        self.effect_creation_counter += 1;
        self.temporary_effects.push(TemporaryEffect {
            effect_type,
            duration,
            created_turn: self.turn_number,
            created_phase: self.current_phase.clone(),
            target_player_id,
            description,
            creation_order: order,
            effect_data: None,
        });
    }

    pub fn get_temporary_effects_in_order(&self) -> Vec<&TemporaryEffect> {
        let mut effects = self.temporary_effects.iter().collect::<Vec<_>>();
        effects.sort_by_key(|e| e.creation_order);
        effects
    }

    pub fn check_expired_effects(&mut self) {
        let mut expired_indices = Vec::new();

        for (i, effect) in self.temporary_effects.iter().enumerate() {
            let is_expired = match effect.duration {
                Duration::LiveEnd => {
                    self.current_turn_phase != TurnPhase::Live
                }
                Duration::ThisTurn => {
                    self.turn_number > effect.created_turn
                }
                Duration::ThisLive => {
                    self.current_turn_phase != TurnPhase::Live
                }
                Duration::Permanent => false,
                Duration::AsLongAs => {
                    self.current_turn_phase != TurnPhase::Live
                }
            };

            if is_expired {
                expired_indices.push(i);
            }
        }

        for i in expired_indices.into_iter().rev() {
            let effect = self.temporary_effects.remove(i);
            match effect.effect_type.as_str() {
                "activation_cost_increase" => {
                    self.prohibition_effects.retain(|p| !p.contains(&effect.effect_type));
                }
                "activation_cost_decrease" => {
                    self.prohibition_effects.retain(|p| !p.contains(&effect.effect_type));
                }
                "gain_resource_blade" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(cards) = data.as_array() {
                            for card_data in cards {
                                if let Some(card_id) = card_data.get("card_id").and_then(|v| v.as_i64()) {
                                    if let Some(amount) = card_data.get("amount").and_then(|v| v.as_i64()) {
                                        self.remove_blade_modifier(card_id as i16, amount as i32);
                                        eprintln!("Reverted {} blades from card {}", amount, card_id);
                                    }
                                }
                            }
                        } else if let Some(card_data) = data.as_object() {
                            if let Some(card_id) = card_data.get("card_id").and_then(|v| v.as_i64()) {
                                if let Some(amount) = card_data.get("amount").and_then(|v| v.as_i64()) {
                                    self.remove_blade_modifier(card_id as i16, amount as i32);
                                    eprintln!("Reverted {} blades from card {}", amount, card_id);
                                }
                            }
                        }
                    }
                }
                "gain_resource_heart" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(cards) = data.as_array() {
                            for card_data in cards {
                                if let Some(card_id) = card_data.get("card_id").and_then(|v| v.as_i64()) {
                                    if let Some(amount) = card_data.get("amount").and_then(|v| v.as_i64()) {
                                        self.remove_heart_modifier(card_id as i16, crate::card::HeartColor::Heart01, amount as i32);
                                        eprintln!("Reverted {} hearts from card {}", amount, card_id);
                                    }
                                }
                            }
                        }
                    }
                }
                "heart_override" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(card_id) = data.get("card_id").and_then(|v| v.as_i64()) {
                            self.heart_override.remove(&(card_id as i16));
                            eprintln!("Removed heart override for card {}", card_id);
                        }
                    }
                }
                _ => {
                    eprintln!("Expired effect: {}", effect.description);
                }
            }
        }
    }

    pub fn get_active_effects_for_player(&self, player_id: &str) -> Vec<&TemporaryEffect> {
        self.temporary_effects
            .iter()
            .filter(|e| e.target_player_id == player_id)
            .collect()
    }

    pub fn add_replacement_effect(
        &mut self,
        card_id: i16,
        player_id: String,
        original_event: String,
        replacement_effects: Vec<crate::card::AbilityEffect>,
        is_choice_based: bool,
    ) {
        self.replacement_effects.push(ReplacementEffect {
            card_id,
            player_id,
            original_event,
            replacement_effects,
            is_choice_based,
            applied_this_event: false,
        });
    }

    pub fn remove_replacement_effects_for_card(&mut self, card_id: i16) {
        self.replacement_effects.retain(|e| e.card_id != card_id);
    }

    pub fn get_replacement_effects_for_event(&self, event: &str) -> Vec<&ReplacementEffect> {
        self.replacement_effects
            .iter()
            .filter(|e| e.original_event == event && !e.applied_this_event)
            .collect()
    }

    pub fn reset_replacement_effect_flags(&mut self) {
        for effect in &mut self.replacement_effects {
            effect.applied_this_event = false;
        }
    }

    pub fn mark_replacement_effect_applied(&mut self, card_id: i16) {
        if let Some(effect) = self.replacement_effects.iter_mut().find(|e| e.card_id == card_id) {
            effect.applied_this_event = true;
        }
    }

    pub fn set_opponent_live_success(&mut self, no_excess_heart: bool) {
        self.opponent_live_success_this_turn = true;
        self.opponent_live_no_excess_heart_this_turn = no_excess_heart;
    }

    pub fn set_formation_change_occurred(&mut self) {
        self.formation_change_occurred_this_turn = true;
    }

    pub fn reset_change_flags(&mut self) {
        self.position_change_occurred_this_turn = false;
        self.formation_change_occurred_this_turn = false;
        self.opponent_live_success_this_turn = false;
        self.opponent_live_no_excess_heart_this_turn = false;
    }

    pub fn check_permanent_loop(&mut self) -> bool {
        let state_hash = self.generate_state_hash();

        if self.game_state_history.contains(&state_hash) {
            self.loop_detected = true;
            return true;
        }

        self.game_state_history.push(state_hash);

        if self.game_state_history.len() > self.max_state_history_size {
            self.game_state_history.remove(0);
        }

        false
    }

    fn generate_state_hash(&self) -> String {
        format!(
            "t{}_p{}_tp{}_p1h{}_p1e{}_p1w{}_p1l{}_p1su{}_p1st{:?}_p2h{}_p2e{}_p2w{}_p2l{}_p2su{}_p2st{:?}_oe{}_pro{}_tmp{}_rps{:?}",
            self.turn_number,
            self.current_phase.to_string(),
            self.current_turn_phase.to_string(),
            self.player1.hand.cards.len(),
            self.player1.energy_zone.cards.len(),
            self.player1.waitroom.cards.len(),
            self.player1.live_card_zone.cards.len(),
            self.player1.success_live_card_zone.cards.len(),
            self.player1.stage.stage,
            self.player2.hand.cards.len(),
            self.player2.energy_zone.cards.len(),
            self.player2.waitroom.cards.len(),
            self.player2.live_card_zone.cards.len(),
            self.player2.success_live_card_zone.cards.len(),
            self.player2.stage.stage,
            self.orientation_modifiers.len(),
            self.prohibition_effects.len(),
            self.temporary_effects.len(),
            self.rps_winner
        )
    }

    pub fn reset_loop_detection(&mut self) {
        self.game_state_history.clear();
        self.loop_detected = false;
    }

    pub fn is_loop_detected(&self) -> bool {
        self.loop_detected
    }

    pub fn save_state(&mut self) {
        self.future.clear();
        self.history.push(self.clone());

        if self.history.len() > self.max_history_size {
            self.history.drain(..1);
        }
    }

    pub fn undo(&mut self) -> Result<(), String> {
        if self.history.is_empty() {
            return Err("No history to undo".to_string());
        }

        self.future.push(self.clone());

        let previous = self.history.pop().unwrap();
        *self = previous;

        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), String> {
        if self.future.is_empty() {
            return Err("No future to redo".to_string());
        }

        self.history.push(self.clone());

        let next = self.future.pop().unwrap();
        *self = next;

        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
}
