use super::stage_has_any_member;
use super::ConditionContext;
use crate::ability::debug::AbDebug;
use crate::card::Condition;

impl<'a> ConditionContext<'a> {
    pub(crate) fn evaluate_condition_list(
        &self,
        conditions: &[Condition],
        operator: &str,
    ) -> (usize, bool) {
        let mut passed = 0usize;
        let mut all_pass = true;
        let mut any_pass = false;
        for condition in conditions {
            let result = self.evaluate_condition(condition);
            if result {
                passed += 1;
            } else {
                all_pass = false;
            }
            any_pass |= result;
        }
        let final_result = match operator {
            "and" => all_pass,
            "or" => any_pass,
            _ => true,
        };
        (passed, final_result)
    }

    pub(crate) fn any_of_matches(&self, condition_type: &str) -> bool {
        let master = &*self.game_state.resolve_target_player("self");
        match condition_type {
            "has_member" => stage_has_any_member(master),
            "has_energy" => !master.energy_zone.cards.is_empty(),
            "has_hand" => !master.hand.cards.is_empty(),
            "has_blade_heart" => master.stage.stage.iter().any(|&id| {
                id != crate::constants::EMPTY_SLOT
                    && self
                        .game_state
                        .card_database
                        .get_card(id)
                        .map(|c| c.has_blade_heart())
                        .unwrap_or(false)
            }),
            "has_live_card" => !master.live_card_zone.cards.is_empty(),
            "is_active_phase" => matches!(
                self.game_state.current_phase,
                crate::game_state::Phase::Active
            ),
            "is_main_phase" => matches!(
                self.game_state.current_phase,
                crate::game_state::Phase::Main
            ),
            _ => {
                eprintln!("Unknown any_of condition type: {}", condition_type);
                false
            }
        }
    }

    pub(crate) fn evaluate_compound_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            let mut dbg = AbDebug::new();
            dbg.p(
                "COMPOUND",
                format_args!(
                    "{} sub-conditions, operator={}",
                    conditions.len(),
                    condition.operator.as_deref().unwrap_or("and")
                ),
            );
            let op = condition.operator.as_deref().unwrap_or("and");
            let (passed_count, all_pass) = self.evaluate_condition_list(conditions, op);
            dbg.p(
                "COMPOUND",
                format_args!(
                    "→ {}/{} passed = {}",
                    passed_count,
                    conditions.len(),
                    if all_pass { "PASS" } else { "FAIL" }
                ),
            );
            all_pass
        } else {
            eprintln!("[COMPOUND] no conditions array!");
            true
        }
    }

    pub(crate) fn evaluate_or_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            self.evaluate_condition_list(conditions, "or").1
        } else {
            true
        }
    }

    pub(crate) fn evaluate_any_of_condition(&self, condition: &Condition) -> bool {
        if let Some(ref any_of) = condition.any_of {
            any_of
                .iter()
                .any(|condition_type| self.any_of_matches(condition_type))
        } else {
            true
        }
    }
}
