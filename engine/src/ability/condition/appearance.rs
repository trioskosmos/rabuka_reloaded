use super::ConditionContext;
use crate::ability::enums::Zone;
use crate::ability::util;
use crate::card::{CardProperty, Condition};
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl<'a> ConditionContext<'a> {
    /// Does this condition narrow WHICH cards count, as opposed to merely
    /// asking whether a card appeared at all?
    ///
    /// With no card-targeting filter the condition can only be 「このメンバーが
    /// 登場」, so the activating card must genuinely have appeared. With one, the
    /// condition describes a set and any appearance among them counts.
    fn has_card_targeting_filters(condition: &Condition) -> bool {
        condition.get_group_names().is_some_and(|g| !g.is_empty())
            || condition.get_cost_limit().is_some()
            || condition.get_card_type().is_some()
            || condition.get_characters().is_some_and(|c| !c.is_empty())
    }

    /// Did `cid` appear in the CURRENT movement batch?
    ///
    /// An appearance is a discrete event, so the CURRENT event is the identity —
    /// not "appeared at some point this turn". The current event is identified by
    /// two sources, in order:
    ///
    ///   1. The live batch signals (`recently_appeared_cards`, `moved_cards`,
    ///      `recently_moved_cards`).
    ///   2. The enqueue-time appearance snapshot (`entry_trigger_appeared_cards`),
    ///      the appearance mirror of `entry_trigger_moved_cards`. The effect-time
    ///      condition re-check runs after the live appearance batch is consumed, so
    ///      without this fallback a self-referential 登場 condition would have no
    ///      subject and Coco's draw would never fire.
    ///
    /// The turn-scoped `has_card_appeared_this_turn` fallback is deliberately
    /// gone: it let a self-scoped 自動 re-fire on a LATER, unrelated appearance in
    /// the same turn (e.g. きな子 gained ブレード when a *different* きな子 debuted,
    /// because the batch and the enqueue snapshot both named the other member, not
    /// her). See docs/JIDOU_COMBINATION_WORK.md §6.
    fn appeared_in_current_batch(&self, cid: i16) -> bool {
        if self.game_state.recently_appeared_cards.contains(&cid)
            || self.moved_cards.contains(&cid)
            || self
                .game_state
                .recently_moved_cards
                .as_ref()
                .is_some_and(|v| v.contains(&cid))
        {
            return true;
        }
        if let Some(enq) = self.game_state.entry_trigger_appeared_cards() {
            return enq.contains(&cid);
        }
        false
    }

    /// Cost-limit gate: does any card on the stage have a cost satisfying the
    /// condition's `cost_limit` under its `operator`? (e.g. 「コスト10のメンバー」)
    ///
    /// An operator this engine does not know leaves the gate open, matching the
    /// cost-threshold checks elsewhere: a parser gap must not silently disable
    /// the ability.
    fn stage_satisfies_cost_limit(&self, condition: &Condition, stage_ids: &[i16]) -> bool {
        let Some(cost_limit) = condition.get_cost_limit() else {
            return true;
        };
        let operator = condition.get_operator().unwrap_or("=");
        let met = stage_ids.iter().any(|&cid| {
            self.game_state
                .card_database
                .get_card(cid)
                .and_then(|c| c.cost)
                .and_then(|cost| util::compare_with_operator(operator, cost, cost_limit))
                .unwrap_or(false)
        });
        if !met {
            log::debug!(
                "[APPEARANCE] cost_limit unmet limit={} operator={:?} stage={:?}",
                cost_limit,
                operator,
                stage_ids
            );
        }
        met
    }

    /// Card-type gate: does any card on the stage have the required type?
    /// (e.g. 「メンバー」 → member_card)
    fn stage_has_card_type(&self, condition: &Condition, stage_ids: &[i16]) -> bool {
        let Some(ref card_type) = condition.get_card_type() else {
            return true;
        };
        stage_ids.iter().any(|&cid| {
            util::card_matches_type(
                &self.game_state.card_database,
                cid,
                Some(card_type.as_str()),
            )
        })
    }

    /// Is appearance tracking still populated? Once it has been cleared the
    /// ability was queued by the trigger event, so the "did this card appear
    /// this turn" refinements are skipped.
    fn tracks_appearances(&self) -> bool {
        !self.game_state.cards_appeared_this_turn.is_empty()
    }

    /// Does any card on the stage that matches `matches_group` and appeared this
    /// turn satisfy `exclude_activating`?
    fn some_appeared_matching(
        &self,
        matches_group: &dyn Fn(i16) -> bool,
        stage_ids: &[i16],
        exclude_activating: bool,
    ) -> bool {
        stage_ids.iter().any(|&cid| {
            matches_group(cid)
                && (!exclude_activating
                    || (self
                        .activating_card_id != Some(cid)))
                && self.game_state.has_card_appeared_this_turn(cid)
        })
    }

    /// Group gate for an appearance condition.
    ///
    /// Three things are checked, in order: some card on the stage is in the
    /// group (every slot, when the condition demands all areas); the matching
    /// card itself appeared this turn, because the first check only proves SOME
    /// stage card is in the group; and, when the condition excludes self, the
    /// match is not the activating card alone on its own appearance. Baton-touch
    /// appearances legitimately trigger on the activating card, so the last two
    /// are skipped then.
    fn stage_satisfies_groups(
        &self,
        condition: &Condition,
        stage_ids: &[i16],
        baton_touch_trigger: bool,
        push_rich: &dyn Fn(&str, bool),
    ) -> bool {
        let Some(groups) = condition.get_group_names() else {
            return true;
        };
        if groups.is_empty() {
            return true;
        }
        let card_db = &self.game_state.card_database;
        let matches_group = |cid: i16| -> bool {
            groups.iter().any(|g| {
                crate::ability::util::card_matches_group_str(card_db, cid, Some(g))
            })
        };
        let met = if condition.get_all_areas().unwrap_or(false) {
            stage_ids.iter().all(|&cid| matches_group(cid))
        } else {
            stage_ids.iter().any(|&cid| matches_group(cid))
        };
        if !met {
            push_rich(&format!("グループ不一致: {:?}", groups), false);
            return false;
        }
        if baton_touch_trigger || !self.tracks_appearances() {
            return true;
        }
        if !self.some_appeared_matching(&matches_group, stage_ids, false) {
            push_rich("該当グループ未登場", false);
            return false;
        }
        if condition.get_exclude_self().unwrap_or(false)
            && !self.some_appeared_matching(&matches_group, stage_ids, true)
        {
            push_rich("自カードのみ登場", false);
            return false;
        }
        true
    }

    /// Position-and-name gate: every 「<name>@<position>」 entry must name the
    /// card actually standing in that slot.
    fn stage_satisfies_positioned_characters(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        push_rich: &dyn Fn(&str, bool),
    ) -> bool {
        let Some(pos_chars) = condition.get_positions_characters() else {
            return true;
        };
        log::debug!(
            "[POSCHARS] checking {} entries: {:?}",
            pos_chars.len(),
            pos_chars
                .iter()
                .map(|p| format!("{}@{}", p.character, p.position))
                .collect::<Vec<_>>()
        );
        for pc in pos_chars {
            // stage_position_index is the single place that knows which
            // spellings mean which slot, so this reads positions exactly the
            // way the `position` gate does.
            let Some(pos_idx) = util::stage_position_index(&pc.position) else {
                push_rich(&format!("不明な位置: {}", pc.position), false);
                return false;
            };
            let Some(&card_id) = player.stage.stage.get(pos_idx) else {
                push_rich(&format!("不明な位置: {}", pc.position), false);
                return false;
            };
            if card_id == -1 {
                push_rich(&format!("{}にカードなし", pc.position), false);
                return false;
            }
            let card_name = self
                .game_state
                .card_database
                .get_card(card_id)
                .map(|c| c.name_key.to_string());
            // `pc.character` comes from `positions_characters`, squashed by the
            // parser — no normalisation needed on the needle side either.
            let wanted = pc.character.as_str();
            match card_name {
                Some(ref name) if name.contains(wanted) => {}
                _ => {
                    push_rich(
                        &format!(
                            "{}に{}不在(実際={:?})",
                            pc.position, pc.character, card_name
                        ),
                        false,
                    );
                    return false;
                }
            }
        }
        true
    }

    /// The cost of the first card on the stage whose name contains `name`.
    fn stage_cost_of_name(&self, stage_ids: &[i16], name: &str) -> Option<u8> {
        stage_ids.iter().find_map(|&cid| {
            let card = self.game_state.card_database.get_card(cid)?;
            card.name_key.contains(name).then_some(card.cost)?
        })
    }

    /// Character gate: every named character must be on the stage. When the
    /// condition also names a reference character, the two costs are compared.
    fn stage_satisfies_characters(
        &self,
        condition: &Condition,
        stage_ids: &[i16],
        push_rich: &dyn Fn(&str, bool),
    ) -> bool {
        let chars = condition.get_characters().unwrap_or_default();
        if chars.is_empty() {
            let r = !stage_ids.is_empty();
            push_rich(&format!("ステージ在籍={}", stage_ids.len()), r);
            return r;
        }
        let stage_card_names: Vec<String> = stage_ids
            .iter()
            .filter_map(|&cid| {
                self.game_state
                    .card_database
                    .get_card(cid)
                    .map(|c| c.name_key.to_string())
            })
            .collect();
        log::debug!("[APPEARANCE] stage card names: {:?}", stage_card_names);
        // `chars` is squashed at generation time, so it compares raw against the
        // already-canonical `name_key`.
        let present = chars
            .iter()
            .all(|name| stage_card_names.iter().any(|cname| cname.contains(name)));
        log::debug!("[APPEARANCE] result={}", present);
        if !present {
            push_rich(
                &format!(
                    "キャラ不在: 期待={:?}, 在籍=[{}]",
                    chars,
                    stage_card_names.join(", ")
                ),
                false,
            );
            return false;
        }
        match condition.get_cost_reference_character() {
            Some(ref_char) => {
                let subject = chars[0].as_str();
                let subject_cost = self.stage_cost_of_name(stage_ids, subject);
                let ref_cost = self.stage_cost_of_name(stage_ids, ref_char);
                let op = condition
                    .get_cost_reference_operator()
                    .map(|o| o.as_str())
                    .unwrap_or(">");
                // A missing cost on either side is not a comparison that failed.
                let ok = match (subject_cost, ref_cost) {
                    (Some(sc), Some(rc)) => {
                        util::compare_with_operator(op, sc, rc).unwrap_or(false)
                    }
                    _ => false,
                };
                log::debug!(
                    "[APPEARANCE] cost_compare: subject={} cost={:?} ref={} cost={:?} op={} ok={}",
                    subject,
                    subject_cost,
                    ref_char,
                    ref_cost,
                    op,
                    ok
                );
                push_rich(
                    &format!(
                        "{}({}) {} {}({}) → {}",
                        subject,
                        subject_cost.unwrap_or(0),
                        if ok { "成立" } else { "不成立" },
                        ref_char,
                        ref_cost.unwrap_or(0),
                        if ok { "成立" } else { "不成立" }
                    ),
                    ok,
                );
                ok
            }
            None => {
                push_rich(&format!("全キャラ在籍: {:?}", chars), true);
                true
            }
        }
    }

    /// Card-property gate over the cards moved this turn, honouring the
    /// condition's `negation`. Only meaningful when the stage is occupied.
    fn stage_satisfies_card_property(&self, condition: &Condition) -> bool {
        if self.moved_cards.is_empty() {
            return true;
        }
        let Some(ref prop) = condition.get_card_property() else {
            return true;
        };
        let has_prop = self.moved_cards.iter().any(|&cid| {
            self.game_state
                .card_database
                .get_card(cid)
                .is_some_and(|c| match *prop {
                    CardProperty::HasBladeHeart => c.has_blade_heart(),
                    CardProperty::HasScoreIcon => c.has_score_icon(),
                    CardProperty::HasAllBlade => c.has_all_blade(),
                    CardProperty::HasDrawIcon => c.has_draw_icon(),
                })
        });
        let negated = condition.get_negation().unwrap_or(false);
        negated != has_prop
    }

    fn evaluate_appearance_stage(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        stage_ids: &[i16],
        push_rich: &dyn Fn(&str, bool),
    ) -> bool {
        if stage_ids.is_empty() {
            push_rich("ステージ空", false);
            return false;
        }
        let has_card_filters = Self::has_card_targeting_filters(condition);
        let baton_touch_trigger = condition.get_baton_touch_trigger().unwrap_or(false);
        if !baton_touch_trigger && !has_card_filters {
            if self.game_state.cards_appeared_this_turn.is_empty() {
                push_rich("今ターン未登場", false);
                return false;
            }
            let self_appeared = self
                .activating_card_id
                .is_some_and(|cid| self.appeared_in_current_batch(cid) && stage_ids.contains(&cid));
            if !self_appeared {
                push_rich("自カード未登場", false);
                return false;
            }
        }
        // Generalized self-trigger guard: when cost_limit or card_type filters
        // are set AND exclude_self is true, prevent the activating card from
        // triggering on its own appearance (the same logic as the group_names
        // guard below).
        if !baton_touch_trigger
            && !self.game_state.cards_appeared_this_turn.is_empty()
            && condition.get_exclude_self().unwrap_or(false)
            && (condition.get_cost_limit().is_some() || condition.get_card_type().is_some())
        {
            let has_other_appeared = stage_ids.iter().any(|&cid| {
                (self.activating_card_id != Some(cid))
                    && self.game_state.has_card_appeared_this_turn(cid)
            });
            if !has_other_appeared {
                push_rich("自カードのみ登場", false);
                return false;
            }
        }
        let filled_count = player.stage.stage.iter().filter(|&&id| id != -1).count();
        if condition.get_all_areas().unwrap_or(false) && filled_count != 3 {
            push_rich(&format!("全エリア未充足({}/3)", filled_count), false);
            return false;
        }
        // Check cost_limit if specified (e.g. コスト10のメンバー)
        if let Some(cost_limit) = condition.get_cost_limit() {
            if !self.stage_satisfies_cost_limit(condition, stage_ids) {
                push_rich(&format!("コスト不一致(limit={})", cost_limit), false);
                return false;
            }
        }
        if let Some(ref ct) = condition.get_card_type() {
            if !self.stage_has_card_type(condition, stage_ids) {
                push_rich(&format!("カード種別不一致(type={})", ct.as_str()), false);
                return false;
            }
        }
        if !self.stage_satisfies_groups(condition, stage_ids, baton_touch_trigger, push_rich) {
            return false;
        }
        // activation_position: independent position requirement for the
        // activating card itself (e.g. "center" means ability only works
        // when the card is at center).  This is distinct from the `position`
        // field, which may be a cross-comparison reference (with position_compare).
        if let Some(ref act_pos) = condition.get_activation_position() {
            let card_id = self.activating_card_id;
            let passes = act_pos.split(',').any(|p| {
                let Some(idx) = crate::ability::util::activation_position_index(p) else {
                    return false;
                };
                idx < player.stage.stage.len()
                    && card_id.is_some()
                    && player.stage.stage[idx] == card_id.unwrap()
            });
            if !passes {
                push_rich(&format!("位置不一致: {}", act_pos), false);
                return false;
            }
        }

        if let Some(ref pos) = condition.get_position() {
            // When position_compare is set, `position` is a cross-comparison
            // reference (e.g. compare cost at left_side vs right_side), not
            // a requirement that the activating card be at that position.
            // Skip the card-own-position check in that case.
            if condition.get_position_compare().is_some() {
                // position is for cross-comparison, not card positioning
            } else {
                log::debug!(
                    "[POS_CHECK] pos={:?} get_position={:?}",
                    pos,
                    pos.get_position()
                );
                let pos_str = pos.get_position();
                // stage_position_index is the single place that knows which
                // spellings mean which slot. It also accepts the Japanese forms,
                // which this ladder did not — and the extra arms it used to
                // carry ("leftside", "rightside", "centre") could never match:
                // the parser maps 「{{leftside.png|左サイド}}」 to "left_side" in
                // parser_fields.py, so those spellings never reach the engine.
                let pos_idx = match util::stage_position_index(pos_str.unwrap_or("")) {
                    Some(idx) => idx,
                    None => {
                        log::debug!("[APPEARANCE] unknown position: {:?}", pos_str);
                        push_rich(&format!("不明な位置: {:?}", pos_str), false);
                        return false;
                    }
                };

                let expected = self.activating_card_id;
                if pos_idx >= player.stage.stage.len()
                    || expected.is_none()
                    || player.stage.stage[pos_idx] != expected.unwrap()
                {
                    push_rich(&format!("位置不一致(idx={})", pos_idx), false);
                    return false;
                }
            } // end else (position without position_compare)
        }
        log::debug!(
            "[PCOND] position={:?} pc_some={} chars={:?} type={:?}",
            condition.get_position(),
            condition.get_positions_characters().is_some(),
            condition.get_characters().map(|v| v.len()),
            condition.condition_type()
        );
        if !self.stage_satisfies_positioned_characters(condition, player, push_rich) {
            return false;
        }
        // Appearance-source validation runs BEFORE both branches: a condition
        // may name characters AND pin a source (「○○が手札以外から登場」). The
        // check used to live only in the no-characters else-branch, so a
        // combined shape skipped source validation entirely.
        if let Some(expected_source) = condition.get_appearance_source() {
            // WHO appeared is not always the activating card. For a 「登場」
            // ability the two coincide, but a watcher (自動 — 中須かすみ
            // 「自分の控え室からメンバーが登場したとき」) resolves on ANOTHER
            // card's arrival: `activating_card_id` is the watcher, which never
            // moved, so reading its source always failed and the draw never
            // fired. Test every card that arrived on this stage instead — for
            // the self-referential shape the activating card is on the stage
            // and carries its own source, so both spellings land here.
            //
            // `non_hand` is the inverse spelling (HS-PR-038 ab#0
            // 「手札以外から登場したとき」): a member played from the hand
            // ALSO appears (turn/phases.rs records its origin as "hand"), so
            // it must NOT satisfy this. Only an arrival recorded with a
            // different origin counts; the hand placements are the two
            // paths that record "hand" explicitly, everything else is
            // effect-driven (source = the moved-from zone) or unrecorded.
            let ok = stage_ids.iter().any(|&cid| {
                let source = self.game_state.get_card_appearance_source(cid);
                if expected_source == "non_hand" {
                    self.game_state.has_card_appeared_this_turn(cid)
                        && source != Some("hand")
                } else {
                    source == Some(expected_source)
                }
            });
            if !ok {
                push_rich(&format!("登場元不一致: 期待={}", expected_source), false);
                return false;
            }
        }
        if condition.get_characters().is_some() {
            log::debug!(
                "[APPEARANCE] checking characters: {:?} against stage_ids={:?}",
                condition.get_characters(),
                stage_ids
            );
            self.stage_satisfies_characters(condition, stage_ids, push_rich)
        } else {
            let names = stage_ids
                .iter()
                .filter_map(|&cid| {
                    self.game_state
                        .card_database
                        .get_card(cid)
                        .map(|c| c.name.to_string())
                })
                .collect::<Vec<_>>()
                .join(", ");
            push_rich(&format!("在籍キャラ: [{}]", names), true);
            let stage_occupied = !stage_ids.is_empty();
            if stage_occupied && !self.stage_satisfies_card_property(condition) {
                let prop = condition.get_card_property().unwrap();
                push_rich(&format!("card_property={} unmet", prop.as_str()), false);
                return false;
            }
            stage_occupied
        }
    }

    pub(crate) fn evaluate_appearance_condition(&self, condition: &Condition) -> bool {
        // NOTE: 「〜していないかぎり」 negation is applied generically at the end
        // of ConditionContext::evaluate_condition (get_negation) — do NOT
        // invert here or the two flips cancel out.
        self.evaluate_appearance_condition_inner(condition)
    }

    fn evaluate_appearance_condition_inner(&self, condition: &Condition) -> bool {
        let appearance = condition.get_appearance().unwrap_or(false);
        let location = condition.get_location().unwrap_or("");
        let target = condition.get_target().unwrap_or("self");
        let baton_touch_trigger = condition.get_baton_touch_trigger().unwrap_or(false);
        let player = self.resolve_condition_player(target);

        // Helper to push enriched verdict with character/cost data
        #[cfg(not(feature = "no_std"))]
        let push_rich = |actual: &str, ok: bool| {
            super::push_cond_verdict(condition, actual, ok, vec![]);
        };
        #[cfg(feature = "no_std")]
        let push_rich = |_actual: &str, _ok: bool| {};

        if baton_touch_trigger {
            if let Some(ref _activating_card) = self.game_state.activating_card {
                let player_id = player.id.as_str();
                let bt_count = self.game_state.get_baton_touch_count(player_id);
                if bt_count == 0 {
                    push_rich("バトンタッチ未実行", false);
                    return false;
                }
                if let Some(min_count) = condition.get_min_baton_touch_count() {
                    if bt_count < min_count {
                        push_rich(&format!("バトンタッチ回数不足({})", bt_count), false);
                        return false;
                    }
                }
            } else {
                push_rich("起動カードなし", false);
                return false;
            }
        }

        if !baton_touch_trigger && !appearance {
            push_rich("不在条件", false);
            return false;
        }

        if appearance {
            match Zone::from_str(location) {
                Some(Zone::Stage) => {
                    let stage_ids: Vec<i16> = player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect();
                    self.evaluate_appearance_stage(condition, player, &stage_ids, &push_rich)
                }
                Some(Zone::Hand) => {
                    let r = !player.hand.cards.is_empty();
                    push_rich(&format!("手札枚数={}", player.hand.cards.len()), r);
                    r
                }
                Some(Zone::Discard) => {
                    let r = !player.waitroom.cards.is_empty();
                    push_rich(&format!("控え室枚数={}", player.waitroom.cards.len()), r);
                    r
                }
                _ => {
                    push_rich("他ゾーン", true);
                    true
                }
            }
        } else {
            match Zone::from_str(location) {
                Some(Zone::Stage) => {
                    let r = player.stage.stage[0] == -1
                        && player.stage.stage[1] == -1
                        && player.stage.stage[2] == -1;
                    push_rich(
                        if r {
                            "不在=全エリア空"
                        } else {
                            "不在≠全エリア空"
                        },
                        r,
                    );
                    r
                }
                Some(Zone::Hand) => {
                    let r = player.hand.cards.is_empty();
                    push_rich(&format!("手札={}", player.hand.cards.len()), r);
                    r
                }
                Some(Zone::Discard) => {
                    let r = player.waitroom.cards.is_empty();
                    push_rich(&format!("控え室={}", player.waitroom.cards.len()), r);
                    r
                }
                _ => {
                    push_rich("他ゾーン不在", true);
                    true
                }
            }
        }
    }

}
