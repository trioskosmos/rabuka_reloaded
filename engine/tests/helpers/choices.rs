use rabuka_engine::ability::types::Choice;
use rabuka_engine::game_setup::{generate_possible_actions, Action, ActionType};
use rabuka_engine::turn::TurnEngine;

use super::{trace, TestGame};

pub fn answer_choice(game: &mut TestGame, idx: usize) {
    match game.get_pending_choice() {
        Choice::SelectTarget { target, .. }
            if target == "position|destination" || target == "area_select" =>
        {
            game.select_generated(idx);
        }
        Choice::SelectPosition { .. } => game.select_generated(idx),
        Choice::SelectHeartColor { .. } => game.select_option(idx as i16),
        _ => game.select_indices(&[idx]),
    }
}

pub(super) fn choice_type(choice: &Choice) -> &'static str {
    match choice {
        Choice::SelectCard { .. } => "SelectCard",
        Choice::SelectTarget { .. } => "SelectTarget",
        Choice::SelectPosition { .. } => "SelectPosition",
        Choice::SelectHeartColor { .. } => "SelectHeartColor",
        Choice::SelectHeartType { .. } => "SelectHeartType",
        Choice::SelectAutoAbility { .. } => "SelectAutoAbility",
        Choice::SelectLiveSuccess { .. } => "SelectLiveSuccess",
    }
}

#[allow(dead_code)]
impl TestGame {
    pub fn has_pending_choice(&self) -> bool {
        self.state.has_pending_choice()
    }

    pub fn get_pending_choice(&self) -> &Choice {
        self.state.get_pending_choice().expect("No pending choice")
    }

    pub fn select_indices(&mut self, indices: &[usize]) {
        self.do_select_indices(indices)
            .expect("select_indices failed");
    }

    pub fn try_select_indices(&mut self, indices: &[usize]) -> Result<(), String> {
        self.do_select_indices(indices)
    }

    /// ONE selection core: trace + resume + dump. `select_indices` expects
    /// success, `try_select_indices` returns the result.
    fn do_select_indices(&mut self, indices: &[usize]) -> Result<(), String> {
        self.trace_selection(indices);
        let r = self.resume_indices(indices);
        trace::dump_state(self, &self.trace);
        r
    }

    fn trace_selection(&self, indices: &[usize]) {
        if self.trace.live() {
            let s = indices
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            self.trace.emit(format!("A select {}", s));
        }
    }

    fn resume_indices(&mut self, indices: &[usize]) -> Result<(), String> {
        if self.select_via_generated(indices).is_err() {
            TurnEngine::resume_with_choice(&mut self.state, None, Some(indices.to_vec()))
        } else {
            Ok(())
        }
    }

    fn select_via_generated(&mut self, indices: &[usize]) -> Result<(), String> {
        let is_select_card = self
            .state
            .get_pending_choice()
            .is_some_and(|c| matches!(c, Choice::SelectCard { .. }));
        if !is_select_card {
            return Err("Not a SelectCard choice".into());
        }
        let actions = generate_possible_actions(&self.state);
        if indices.is_empty() {
            let skip = actions
                .iter()
                .find(|a| a.action_type == ActionType::ChoiceSkip)
                .ok_or("No skip action available")?;
            let p = skip
                .parameters
                .as_ref()
                .ok_or("Skip action has no params")?;
            TurnEngine::resume_with_choice(&mut self.state, p.card_id, p.card_indices.clone())
        } else if indices.len() == 1 {
            let action = actions
                .iter()
                .find(|a| {
                    a.action_type == ActionType::ChoiceSelect
                        && a.parameters
                            .as_ref()
                            .and_then(|p| p.card_indices.as_deref())
                            == Some(indices)
                })
                .ok_or_else(|| {
                    format!(
                        "No ChoiceSelect action with card_indices={:?}. Available: {:?}",
                        indices,
                        actions
                            .iter()
                            .filter(|a| a.action_type == ActionType::ChoiceSelect)
                            .map(|a| a
                                .parameters
                                .as_ref()
                                .and_then(|p| p.card_indices.as_deref()))
                            .collect::<Vec<_>>()
                    )
                })?;
            let p = action.parameters.as_ref().ok_or("Action has no params")?;
            TurnEngine::resume_with_choice(&mut self.state, p.card_id, p.card_indices.clone())
        } else {
            Err("Multi-index not supported via action path".into())
        }
    }

    pub fn select_choice_option(&mut self, idx: usize) {
        self.resume_traced(
            format!("A selopt {}", idx),
            Some(idx as i16),
            None,
            "select_choice_option failed",
        );
    }

    /// ONE traced-resume core for scalar answers: trace + resume + dump.
    /// `select_option`, `select_choice_option` and `select_generated` differ
    /// only in label and payload.
    fn resume_traced(
        &mut self,
        label: String,
        selected: Option<i16>,
        indices: Option<Vec<usize>>,
        expect_msg: &str,
    ) {
        if self.trace.live() {
            self.trace.emit(label);
        }
        TurnEngine::resume_with_choice(&mut self.state, selected, indices).expect(expect_msg);
        trace::dump_state(self, &self.trace);
    }

    pub fn select_energy_from_zone(&mut self, count: usize) {
        let idxs: Vec<usize> = (0..count).collect();
        self.select_indices(&idxs);
    }

    pub fn select_waitroom_card_filtered(&mut self, card_id: i16) {
        let pending = self.get_pending_choice().clone();
        let (fi, zone) = match &pending {
            Choice::SelectCard { filtered_indices: Some(fi), zone, .. } => (fi.clone(), zone.clone()),
            _ => panic!(
                "select_waitroom_card_filtered: expected SelectCard with filtered_indices, got {:?}",
                pending
            ),
        };
        if zone != "discard" && zone != "waitroom" {
            panic!(
                "select_waitroom_card_filtered: expected discard zone, got '{}'",
                zone
            );
        }
        let pos = self
            .state
            .player1
            .waitroom
            .cards
            .iter()
            .position(|&c| c == card_id)
            .expect("Card not found in waitroom");
        let filtered_idx = fi
            .iter()
            .position(|&p| p == pos)
            .unwrap_or_else(|| panic!("Card not in filtered_indices: pos={} fi={:?}", pos, fi));
        self.select_indices(&[filtered_idx]);
    }

    fn is_any_number_choice(&self) -> bool {
        self.state.get_pending_choice().is_some_and(|c| {
            matches!(
                c,
                Choice::SelectCard {
                    count: 0,
                    allow_skip: true,
                    ..
                }
            )
        })
    }

    pub fn select_indices_sequential(&mut self, indices: &[usize]) {
        if indices.is_empty() || !self.is_any_number_choice() {
            self.select_indices(indices);
            return;
        }
        for (i, &idx) in indices.iter().enumerate() {
            self.select_indices(&[idx]);
            if i + 1 < indices.len() {
                assert!(
                    self.state.has_pending_choice(),
                    "Expected re-prompt after selecting index {} of {}",
                    i + 1,
                    indices.len()
                );
            }
        }
        if self.state.has_pending_choice() && self.is_any_number_choice() {
            self.select_indices(&[]);
        }
    }

    pub fn select_option(&mut self, option_index: i16) {
        self.resume_traced(
            format!("A selopt {}", option_index),
            Some(option_index),
            None,
            "select_option failed",
        );
    }

    pub fn generated_actions(&self) -> Vec<Action> {
        let pending = self.get_pending_choice();
        let all = generate_possible_actions(&self.state);
        match pending {
            Choice::SelectTarget { target, .. }
                if target == "position|destination" || target == "area_select" =>
            {
                all.into_iter()
                    .filter(|a| a.action_type == ActionType::ChoicePosition)
                    .collect()
            }
            _ => all,
        }
    }

    pub fn select_generated(&mut self, nth: usize) {
        let matching = self.generated_actions();
        assert!(
            nth < matching.len(),
            "select_generated({}): only {} generated actions",
            nth,
            matching.len(),
        );
        let action = &matching[nth];
        self.resume_traced(
            format!("A selgen {}", nth),
            action.parameters.as_ref().and_then(|p| p.card_id),
            action
                .parameters
                .as_ref()
                .and_then(|p| p.card_indices.clone()),
            "select_generated failed",
        );
    }

    pub fn drain_auto_ability_choices(&mut self) {
        self.trace.set_quiet(true);
        while let Some(choice) = self.state.get_pending_choice() {
            match choice {
                Choice::SelectAutoAbility { .. } => {
                    self.select_indices(&[]);
                }
                _ => break,
            }
        }
        self.trace.set_quiet(false);
        if self.trace.live() {
            self.trace.emit("A drain".to_string());
            trace::dump_state(self, &self.trace);
        }
    }

    pub fn drain_choices_strict(&mut self, allowed: &[&str], answer: &[usize]) {
        let mut drained = 0usize;
        while let Some(choice) = self.state.get_pending_choice() {
            drained += 1;
            assert!(
                drained <= 1000,
                "drain_choices_strict: exceeded 1000 prompts — non-terminating choice loop.\n{}",
                self.pending_choice_summary()
            );
            let ty = self
                .pending_choice_type()
                .unwrap_or_else(|| "Unknown".into());
            assert!(
                allowed.contains(&ty.as_str()),
                "drain_choices_strict: unexpected pending choice {} (allowed: {:?})\n{}",
                ty,
                allowed,
                self.dbg_choice_string()
            );
            match choice {
                Choice::SelectCard { .. } | Choice::SelectAutoAbility { .. } => {
                    self.select_indices(answer);
                }
                _ => panic!(
                    "drain_choices_strict: no auto-answer for {}; answer this prompt explicitly in the test",
                    ty
                ),
            }
        }
    }

    /// Both pending-choice sources in one view: the live queue choice first,
    /// else the serialized JSON fallback. Every inspector below reads
    /// through this instead of repeating the two-source fallback chain.
    fn pending_choice_view(&self) -> PendingChoiceView<'_> {
        if let Some(choice) = self.state.ability_queue.is_waiting_for_choice() {
            PendingChoiceView::Live(choice)
        } else if let Some(pc) = self.state.get_pending_choice_json() {
            PendingChoiceView::Json(pc)
        } else {
            PendingChoiceView::None
        }
    }

    fn dbg_choice_string(&self) -> String {
        match self.pending_choice_view() {
            PendingChoiceView::Live(choice) => format!("{:#?}", choice),
            PendingChoiceView::Json(pc) => format!("{}", pc),
            PendingChoiceView::None => "(none)".into(),
        }
    }

    pub fn pending_choice_type(&self) -> Option<String> {
        match self.pending_choice_view() {
            PendingChoiceView::Live(choice) => Some(choice_type(choice).to_string()),
            PendingChoiceView::Json(pc) => Some(
                match pc["choice_type"].as_str() {
                    Some(
                        name @ ("SelectCard" | "SelectTarget" | "SelectPosition" | "SelectHeartColor"
                        | "SelectHeartType" | "SelectAutoAbility" | "SelectLiveSuccess"),
                    ) => name,
                    _ => "Unknown",
                }
                .to_string(),
            ),
            PendingChoiceView::None => None,
        }
    }

    pub fn pending_choice_count(&self) -> usize {
        match self.pending_choice_view() {
            PendingChoiceView::Live(Choice::SelectCard { count, .. }) => *count,
            PendingChoiceView::Live(_) => 0,
            PendingChoiceView::Json(pc) => pc["count"].as_u64().map_or(0, |c| c as usize),
            PendingChoiceView::None => 0,
        }
    }

    pub fn dbg_choice(&self) {
        match self.pending_choice_view() {
            PendingChoiceView::Live(choice) => eprintln!("[CHOICE] {:?}", choice),
            PendingChoiceView::Json(pc) => eprintln!("[CHOICE] (json) {:?}", pc),
            PendingChoiceView::None => eprintln!("[CHOICE] none"),
        }
    }

    pub fn pending_choice_summary(&self) -> String {
        match self.pending_choice_view() {
            PendingChoiceView::Live(choice) => match choice {
                Choice::SelectCard {
                    zone, card_type, count, allow_skip, group, filtered_indices, ..
                } => format!(
                    "SelectCard zone={} count={} allow_skip={} card_type={:?} group={:?} options={:?}",
                    zone, count, allow_skip, card_type, group,
                    filtered_indices.as_ref().map(|v| v.len())
                ),
                Choice::SelectTarget { target, .. } => format!("SelectTarget target={}", target),
                _ => choice_type(choice).to_string(),
            },
            PendingChoiceView::Json(pc) => format!("(json) {}", pc),
            PendingChoiceView::None => "none".to_string(),
        }
    }
}

/// See `TestGame::pending_choice_view`: live queue choice, JSON fallback, or absent.
enum PendingChoiceView<'a> {
    Live(&'a Choice),
    Json(serde_json::Value),
    None,
}
