use super::stage_has_any_member;
use super::ConditionContext;
use crate::ability::debug::AbDebug;
use crate::card::Condition;
#[cfg(feature = "no_std")]
use alloc::boxed::Box;

impl<'a> ConditionContext<'a> {
    pub(crate) fn evaluate_condition_list(
        &self,
        conditions: &[Box<Condition>],
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
        let master = self.game_state.resolve_target_player("self");
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
                log::debug!("Unknown any_of condition type: {}", condition_type);
                false
            }
        }
    }

    pub(crate) fn evaluate_compound_condition(&self, condition: &Condition) -> bool {
        if let Some(conditions) = condition.get_conditions() {
            let mut dbg = AbDebug::new();
            dbg.p(
                "COMPOUND",
                format_args!(
                    "{} sub-conditions, operator={}",
                    conditions.len(),
                    condition.get_operator().unwrap_or("and")
                ),
            );
            let op = condition.get_operator().unwrap_or("and");
            #[cfg(not(feature = "no_std"))]
            let before = crate::ability::log::buffer_len();
            let (passed_count, result) = self.evaluate_condition_list(conditions, op);
            #[cfg(not(feature = "no_std"))]
            let children = crate::ability::log::drain_verdicts_since(before);
            dbg.p(
                "COMPOUND",
                format_args!(
                    "→ {}/{} passed = {}",
                    passed_count,
                    conditions.len(),
                    if result { "PASS" } else { "FAIL" }
                ),
            );
            #[cfg(not(feature = "no_std"))]
            super::push_cond_verdict(
                condition,
                &format!("{}/{}", passed_count, conditions.len()),
                result,
                children,
            );
            result
        } else {
            log::debug!("[COMPOUND] no conditions array!");
            #[cfg(not(feature = "no_std"))]
            super::push_cond_verdict(condition, "no conditions", true, vec![]);
            true
        }
    }

    pub(crate) fn evaluate_or_condition(&self, condition: &Condition) -> bool {
        if let Some(conditions) = condition.get_conditions() {
            #[cfg(not(feature = "no_std"))]
            let before = crate::ability::log::buffer_len();
            #[cfg_attr(feature = "no_std", allow(unused_variables))]
            let (cnt, result) = self.evaluate_condition_list(conditions, "or");
            #[cfg(not(feature = "no_std"))]
            let children = crate::ability::log::drain_verdicts_since(before);
            #[cfg(not(feature = "no_std"))]
            super::push_cond_verdict(
                condition,
                &format!("{}/{} any", cnt, conditions.len()),
                result,
                children,
            );
            result
        } else {
            true
        }
    }

    pub(crate) fn evaluate_any_of_condition(&self, condition: &Condition) -> bool {
        if let Some(any_of) = condition.get_any_of() {
            any_of
                .iter()
                .any(|condition_type| self.any_of_matches(condition_type))
        } else {
            true
        }
    }
}
