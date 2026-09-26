use rabuka_engine::ability::types::Choice;
use rabuka_engine::player::Player;
use rabuka_engine::zones::MemberArea;

use super::{choices::choice_type, trace, TestGame};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PlayerZoneSnapshot {
    pub hand: Vec<i16>,
    pub stage: Vec<i16>,
    pub live_card_zone: Vec<i16>,
    pub success_live_card_zone: Vec<i16>,
    pub energy_cards: Vec<i16>,
    pub main_deck: Vec<i16>,
    pub energy_deck: Vec<i16>,
    pub waitroom: Vec<i16>,
    pub exclusion: Vec<i16>,
    pub active_energy: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BoardSnapshot {
    pub p1: PlayerZoneSnapshot,
    pub p2: PlayerZoneSnapshot,
}

#[allow(dead_code)]
impl TestGame {
    pub fn board_snapshot(&self) -> BoardSnapshot {
        BoardSnapshot {
            p1: Self::player_snapshot(&self.state.player1),
            p2: Self::player_snapshot(&self.state.player2),
        }
    }

    fn player_snapshot(p: &Player) -> PlayerZoneSnapshot {
        PlayerZoneSnapshot {
            hand: p.hand.cards.to_vec(),
            stage: p.stage.stage.to_vec(),
            live_card_zone: p.live_card_zone.cards.to_vec(),
            success_live_card_zone: p.success_live_card_zone.cards.to_vec(),
            energy_cards: p.energy_zone.cards.to_vec(),
            active_energy: p.energy_zone.active_count(),
            main_deck: p.main_deck.cards.to_vec(),
            energy_deck: p.energy_deck.cards.to_vec(),
            waitroom: p.waitroom.cards.to_vec(),
            exclusion: p.exclusion_zone.cards.to_vec(),
        }
    }

    pub fn assert_board_matches(&self, expected: &BoardSnapshot, msg: &str) {
        let actual = self.board_snapshot();
        let mut diffs = Vec::new();
        for (label, exp, act) in [
            ("p1", &expected.p1, &actual.p1),
            ("p2", &expected.p2, &actual.p2),
        ] {
            for (zone, ev, av) in [
                ("hand", &exp.hand, &act.hand),
                ("stage", &exp.stage, &act.stage),
                ("live_card_zone", &exp.live_card_zone, &act.live_card_zone),
                (
                    "success_live_card_zone",
                    &exp.success_live_card_zone,
                    &act.success_live_card_zone,
                ),
                ("energy_cards", &exp.energy_cards, &act.energy_cards),
                ("main_deck", &exp.main_deck, &act.main_deck),
                ("energy_deck", &exp.energy_deck, &act.energy_deck),
                ("waitroom", &exp.waitroom, &act.waitroom),
                ("exclusion", &exp.exclusion, &act.exclusion),
            ] {
                if ev != av {
                    diffs.push(format!(
                        "{}.{}:\n  expected: [{}]\n  actual:   [{}]",
                        label,
                        zone,
                        self.fmt_ids(ev),
                        self.fmt_ids(av)
                    ));
                }
            }
            if exp.active_energy != act.active_energy {
                diffs.push(format!(
                    "{}.active_energy: expected {}, got {}",
                    label, exp.active_energy, act.active_energy
                ));
            }
        }
        assert!(diffs.is_empty(), "{}\n{}", msg, diffs.join("\n"));
    }

    fn fmt_ids(&self, ids: &[i16]) -> String {
        ids.iter()
            .map(|&id| self.name(id))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn assert_select_card(
        &self,
        expected_zone: &str,
        expected_count: usize,
        expected_allow_skip: bool,
    ) {
        let choice = self.get_pending_choice();
        match choice {
            Choice::SelectCard {
                zone,
                count,
                allow_skip,
                ..
            } => {
                assert_eq!(zone, expected_zone, "SelectCard zone mismatch");
                assert_eq!(*count, expected_count, "SelectCard count mismatch");
                assert_eq!(
                    *allow_skip, expected_allow_skip,
                    "SelectCard allow_skip mismatch"
                );
            }
            _ => panic!("Expected SelectCard, got {:?}", choice),
        }
    }

    pub fn assert_conditional_optional(&self, expected_opts: &[&str]) {
        let choice = self.get_pending_choice();
        match choice {
            Choice::SelectTarget {
                target, options, ..
            } => {
                assert_eq!(
                    target, "conditional_optional",
                    "Expected conditional_optional target"
                );
                if let Some(ref opts) = options {
                    assert_eq!(opts.len(), expected_opts.len(), "Option count mismatch");
                    for (i, expected) in expected_opts.iter().enumerate() {
                        assert_eq!(&opts[i], expected, "Option {} mismatch", i);
                    }
                }
            }
            _ => panic!(
                "Expected SelectTarget(conditional_optional), got {:?}",
                choice
            ),
        }
    }

    /// ONE trace hook for value assertions: dump + CHECK line when tracing.
    /// `assert_hand`, `assert_energy`, `assert_blade`, `assert_stage_pos` and
    /// `assert_pending_choice_type` all report through here.
    fn trace_check(&self, label: String, actual: String, expected: String) {
        if self.trace.live() {
            trace::dump_state(self, &self.trace);
            trace::check(&self.trace, label, actual, expected);
        }
    }

    pub fn assert_hand(&self, expected: usize, msg: &str) {
        let actual = self.state.player1.hand.len();
        self.trace_check("hand 1".into(), actual.to_string(), expected.to_string());
        assert_eq!(
            actual, expected,
            "{}: expected {} cards in hand, got {}",
            msg, expected, actual
        );
    }

    pub fn assert_stage_pos(&self, pos: MemberArea, card_id: i16, msg: &str) {
        let actual = self.state.player1.stage.get_area(pos);
        self.trace_check(
            format!(
                "stage 1 {} {}",
                pos.to_index(),
                trace::ref_of(self, card_id)
            ),
            actual
                .map(|id| trace::ref_of(self, id))
                .unwrap_or_else(|| "null".into()),
            trace::ref_of(self, card_id),
        );
        assert_eq!(
            actual,
            Some(card_id),
            "{}: expected {:?} at position {:?}, got {:?}",
            msg,
            self.state.card_database.get_card(card_id).map(|c| &c.name),
            pos,
            actual.and_then(|id| self.state.card_database.get_card(id).map(|c| &c.name))
        );
    }

    pub fn assert_energy(&self, expected: u32, msg: &str) {
        let actual = self.state.player1.energy_zone.active_count();
        self.trace_check(
            "energy 1".into(),
            actual.to_string(),
            (expected as u8).to_string(),
        );
        assert_eq!(
            actual, expected as u8,
            "{}: expected {} energy, got {}",
            msg, expected, actual
        );
    }

    pub fn assert_blade(&self, card_id: i16, expected: i32, msg: &str) {
        let actual = self.state.mods.get_blade_modifier(card_id);
        self.trace_check(
            format!("blade {}", trace::ref_of(self, card_id)),
            actual.to_string(),
            expected.to_string(),
        );
        assert_eq!(actual, expected, "{}", msg);
    }

    pub fn assert_choice_identity(
        &self,
        expected_card_id: i16,
        expected_card_name: &str,
        expected_player_id: &str,
    ) {
        let json = self
            .state
            .get_pending_choice_json()
            .expect("No pending choice JSON available");
        let actual_id = json["card_id"].as_i64().map(|v| v as i16);
        assert_eq!(
            actual_id,
            Some(expected_card_id),
            "choice JSON card_id: expected {:?}, got {:?}",
            expected_card_id,
            actual_id
        );
        let actual_name = json["card_name"].as_str().unwrap_or("");
        assert_eq!(
            actual_name, expected_card_name,
            "choice JSON card_name: expected '{}', got '{}'",
            expected_card_name, actual_name
        );
        let actual_pid = json["choice_player_id"].as_str().unwrap_or("");
        assert_eq!(
            actual_pid, expected_player_id,
            "choice JSON choice_player_id: expected '{}', got '{}'",
            expected_player_id, actual_pid
        );
    }

    pub fn assert_selection_contains(&self, expected_card_no: &str, expected_name: &str) {
        let cards = self.pending_selection_cards();
        let found = cards.iter().any(|c| {
            c["card_no"].as_str() == Some(expected_card_no)
                && c["name"].as_str() == Some(expected_name)
        });
        assert!(
            found,
            "selection_cards should contain card_no='{}' name='{}', but it doesn't. Cards: {:?}",
            expected_card_no, expected_name, cards
        );
    }

    pub fn assert_selection_not_contains(&self, card_no: &str) {
        let cards = self.pending_selection_cards();
        let found = cards.iter().any(|c| c["card_no"].as_str() == Some(card_no));
        assert!(
            !found,
            "selection_cards should NOT contain '{}', but it does",
            card_no
        );
    }

    /// Pin that `card_id` really is the print named `card_no`.
    ///
    /// The `similar_cards` audit exists because bp2/pb2, bp7/pb2 and SEC/P
    /// suffixes are transposed often enough that a silently mis-staged card
    /// passes a test that otherwise asserts everything. Comparing the printed
    /// card_no is the only check that notices. `card_no` is an ArcStr, so this
    /// goes through `to_string()` rather than the unstable `as_str` inherent.
    pub fn assert_card_identity(&self, card_id: i16, card_no: &str) {
        let actual = self
            .db
            .get_card(card_id)
            .unwrap_or_else(|| panic!("card id {card_id} is not in the database"))
            .card_no
            .to_string();
        assert_eq!(
            actual, card_no,
            "card id {card_id} is '{}', not the expected print '{}'",
            actual, card_no
        );
    }

    /// Pin that two instances share one printed card name ( supplementing it).
    ///
    /// 「カード名の異なる」 conditions are routinely tested by staging two copies
    /// vs. two different prints. Staging `PL!SP-bp1-013-PR` where
    /// `PL!SP-pb1-013-PR` was meant flips such a test from "distinct" to
    /// "duplicate" with no other assertion noticing.
    pub fn assert_same_card_name(&self, a: i16, b: i16, ctx: &str) {
        let an = self.card_name_of(a);
        let bn = self.card_name_of(b);
        assert_eq!(
            an, bn,
            "{}: expected the same card name on both instances, got '{}' and '{}'",
            ctx, an, bn
        );
    }

    /// Pin a card's printed TYPE (`member_card` / `live_card` / `energy_card`).
    ///
    /// The type is the filter many abilities actually read — 「メンバーカードを
    /// すべて」, 「ライブカードを1枚」 — so a fixture that drifted to the wrong
    /// type turns a positive test into a test of something else entirely, and a
    /// negative test into one that holds for the wrong reason.
    pub fn assert_card_type(&self, card_id: i16, expected: &str, ctx: &str) {
        let actual = self
            .db
            .get_card(card_id)
            .unwrap_or_else(|| panic!("card id {card_id} is not in the database"))
            .card_type
            .to_string();
        assert_eq!(
            actual, expected,
            "{}: '{}' (card id {card_id}) is a '{actual}', not a '{expected}'",
            ctx,
            self.card_name_of(card_id)
        );
    }

    /// Pin that `card_id` really belongs to `group`, using the engine's own
    /// matcher (`card_matches_group_str`) rather than a hand-rolled one.
    ///
    /// GROUP and CARD NAME are different things, and a fixture that means
    /// 「3人の『Printemps』」 can be built from three different characters — so
    /// `assert_same_card_name` is the wrong pin for it and fails on a fixture
    /// that is actually correct. Conversely a fixture that means 「同じカード名の
    /// メンバー」 can be built from three different groups. Reading the
    /// distinction off the engine also means a group_name that is `None` in
    /// cards.json (all of them are) still matches through `unit`/`series`, the
    /// same way the ability under test resolves it.
    pub fn assert_card_in_group(&self, card_id: i16, group: &str, ctx: &str) {
        let matched = rabuka_engine::ability::util::card_matches_group_str(
            &self.db,
            card_id,
            Some(group),
        );
        let name = self.card_name_of(card_id);
        assert!(
            matched,
            "{}: '{}' (card id {card_id}) is not in group '{}' — the fixture is not \
             the case this test claims",
            ctx, name, group
        );
    }

    /// Pin that every card in `cards` belongs to `group` — the premise of any
    /// 「『X』のメンバーがN人」 / 「『X』のカードと名前が異なる」 fixture.
    pub fn assert_all_in_group(&self, cards: &[i16], group: &str, ctx: &str) {
        for &id in cards {
            self.assert_card_in_group(id, group, ctx);
        }
    }

    /// Pin that two instances are different printed cards (not just different
    /// physical copies of one print) — the counterweight to
    /// [`Self::assert_same_card_name`] for 「カード名の異なる」 conditions.
    pub fn assert_distinct_card_names(&self, a: i16, b: i16, ctx: &str) {
        let an = self.card_name_of(a);
        let bn = self.card_name_of(b);
        assert_ne!(
            an, bn,
            "{}: expected two DIFFERENT card names, but both instances are '{}'",
            ctx, an
        );
    }

    fn card_name_of(&self, card_id: i16) -> String {
        self.db
            .get_card(card_id)
            .unwrap_or_else(|| panic!("card id {card_id} is not in the database"))
            .name
            .to_string()
    }

    /// Pin a card's printed cost.
    ///
    /// A transposed bp number very often lands on ANOTHER PRINTING OF THE SAME
    /// CHARACTER (PL!SP-pb2-006-R / PL!SP-pb1-006-R / PL!SP-bp5-006-R are all
    /// 桜小路きな子, printed at costs 2 / 9 / 11). Name-based assertions cannot
    /// see that, but cost-limited plays, 「コストN以下」 filters and energy
    /// budgets can, so cost-sensitive fixtures pin it explicitly.
    pub fn assert_card_cost(&self, card_id: i16, expected: i32) {
        let (no, actual) = self
            .db
            .get_card(card_id)
            .map(|c| (c.card_no.to_string(), c.cost))
            .unwrap_or_else(|| panic!("card id {card_id} is not in the database"));
        assert_eq!(
            actual.map(|c| c as i32),
            Some(expected),
            "card {no} (id {card_id}) should print cost {expected}, got {actual:?}"
        );
    }

    /// Pin a card's printed score.
    ///
    /// The companion to [`Self::assert_card_cost`] for the many 「スコアN以下」 /
    /// 「スコアN以上」 filters (live-card recovers, ライブ成功時 scoring). An
    /// auto-aimed single-candidate select resolves with no prompt, so the score
    /// is frequently the only thing that decides whether a fixture is eligible
    /// at all.
    pub fn assert_card_score(&self, card_id: i16, expected: i32) {
        let (no, actual) = self
            .db
            .get_card(card_id)
            .map(|c| (c.card_no.to_string(), c.score))
            .unwrap_or_else(|| panic!("card id {card_id} is not in the database"));
        assert_eq!(
            actual.map(|s| s as i32),
            Some(expected),
            "card {no} (id {card_id}) should print score {expected}, got {actual:?}"
        );
    }

    /// A refused activation/play must leave no trace of the attempt.
    ///
    /// `assert!(result.is_err())` alone cannot distinguish "refused for the
    /// printed reason" (not enough energy, wrong area, use-limit spent) from
    /// "refused for some unrelated reason", so a regression that trips a
    /// different guard would still pass. These pin the two invariants that
    /// separate them: the energy the attempt would have paid is still there,
    /// and the ターン1回 / use-limit counter did not advance.
    pub fn assert_energy_untouched_after_refusal(&self, before: i16, ctx: &str) {
        assert_eq!(
            self.state.player1.energy_zone.active_count() as i16,
            before,
            "{}: a refused action must not spend energy",
            ctx
        );
    }

    /// Assert `card_id`'s `ability_index` slot is NOT recorded in this turn's
    /// use-limit table — i.e. the refusal did not consume the ターンN回 use.
    pub fn assert_use_not_recorded(&self, card_id: i16, ability_index: usize, ctx: &str) {
        let turn = self.state.turn_number;
        assert!(
            !self
                .state
                .turn_limited_abilities_used
                .contains_key(&(card_id, ability_index, turn)),
            "{}: a refused action must not record a use-limit spend (looked up \
             (card={card_id}, ab#{ability_index}, turn={turn}))",
            ctx
        );
    }

    pub fn assert_pending_choice_type(&self, expected: &str, msg: &str) {
        self.trace_check(
            "pending".into(),
            self.pending_choice_type().unwrap_or_else(|| "none".into()),
            expected.to_string(),
        );
        if let Some(choice) = self.state.ability_queue.is_waiting_for_choice() {
            let actual = choice_type(choice);
            assert_eq!(
                actual, expected,
                "{}: expected choice type {}, got {}",
                msg, expected, actual
            );
        } else {
            panic!(
                "{}: expected pending choice of type {}, got none",
                msg, expected
            );
        }
    }

    /// ONE fetch for the pending choice's `selection_cards` array.
    /// `assert_selection_contains` / `assert_selection_not_contains` share it.
    fn pending_selection_cards(&self) -> Vec<serde_json::Value> {
        self.state
            .get_pending_choice_json()
            .expect("No pending choice JSON available")["selection_cards"]
            .as_array()
            .expect("No selection_cards in choice JSON")
            .to_vec()
    }
}
