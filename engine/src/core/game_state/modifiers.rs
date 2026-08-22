use super::GameState;
use crate::ability::enums::Zone;
use crate::core::types::{Duration, TemporaryEffect};
use crate::{HashMap, HashSet};
use smallvec::SmallVec;

extern "C" {
    // Outputs to both debug console (svcOutputDebugString) AND top screen (consoleSelect+printf)
    fn _3ds_tdbg(msg: *const u8);
}

#[cfg(feature = "3ds")]
macro_rules! tdbg {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s = format!("{}\0", msg);
        unsafe { _3ds_tdbg(s.as_ptr()); }
    }};
}
#[cfg(not(feature = "3ds"))]
macro_rules! tdbg {
    ($($arg:tt)*) => {};
}
impl GameState {
    /// Compute the opponent-front targets for a constant "正面のエリア" (front area)
    /// ability. Given the activating card's stage slot, mirrors to the opponent's
    /// slot via MemberArea::front_area (your left ↔ opp right, center ↔ center,
    /// right ↔ opp left) and applies the effect's card filter (cost_limit etc.).
    /// Returns an empty vec when no qualifying member occupies the front slot.
    fn constant_front_targets(
        &self,
        cid: i16,
        effect: &crate::card::AbilityEffect,
    ) -> Vec<i16> {
        use crate::zones::MemberArea;
        let (is_p1, area) = if let Some(pos) =
            self.player1.stage.stage.iter().position(|&x| x == cid)
        {
            (true, MemberArea::from_index(pos))
        } else if let Some(pos) = self.player2.stage.stage.iter().position(|&x| x == cid) {
            (false, MemberArea::from_index(pos))
        } else {
            return Vec::new();
        };
        let Some(area) = area else {
            return Vec::new();
        };
        let opp_area = area.front_area();
        let opp_stage = if is_p1 {
            &self.player2.stage.stage
        } else {
            &self.player1.stage.stage
        };
        let opp_cid = opp_stage[opp_area.to_index()];
        if opp_cid == -1 {
            return Vec::new();
        }
        let filter = effect.filter_subset();
        if filter.matches(&self.card_database, opp_cid, false) {
            vec![opp_cid]
        } else {
            Vec::new()
        }
    }

    /// Grant blade to host members from constant under-card abilities. See call site.
    fn grant_under_card_constant_blades(&mut self, exp_blade: &mut HashMap<i16, i16>) {
        for (under_cid, host) in self
            .player1
            .stage
            .under_cards_with_hosts()
            .into_iter()
            .chain(self.player2.stage.under_cards_with_hosts().into_iter())
        {
            let card = match self.card_database.get_card(under_cid) {
                Some(c) => c,
                None => continue,
            };
            for (_ability_idx, ar) in card.abilities.iter().enumerate() {
                if !GameState::ability_matches_trigger(
                    &ar.resolve(),
                    &crate::game_state::AbilityTrigger::Constant,
                ) {
                    continue;
                }
                let ability = ar.resolve();
                let Some(ref effect) = ability.effect else {
                    continue;
                };
                if effect.action != crate::ability::enums::ActionType::GainResource
                    || !matches!(
                        effect.resource_any().as_deref(),
                        Some("blade") | Some("ブレード")
                    )
                {
                    continue;
                }
                let Some(ref cond) = effect.condition else {
                    continue;
                };
                // Only under-member-scoped grants route to the host; a generic
                // constant gain on an under-card has no such meaning.
                if cond.get_location() != Some("under_member") {
                    continue;
                }
                let Some(groups) = cond.get_group_names() else {
                    continue;
                };
                // Condition met iff this card is under a host of the group.
                if !crate::ability::util::card_matches_any_group(
                    &self.card_database,
                    host,
                    groups,
                ) {
                    continue;
                }
                let count = effect
                    .resource_icon_count_any()
                    .unwrap_or(effect.count_any().unwrap_or(1))
                    as i16;
                *exp_blade.entry(host).or_insert(0) += count;
            }
        }
    }

    /// Apply the accumulated constant-effect scratch results onto the live
    /// modifier state: clear old constant-derived bonuses and re-apply the new
    /// ones (blade, score, per-player score bonus, heart, prohibition, and
    /// global need_heart).
    fn commit_constant_results(
        &mut self,
        exp_blade: HashMap<i16, i16>,
        exp_score: HashMap<i16, i16>,
        exp_heart: HashMap<i16, HashMap<String, i16>>,
        exp_prohibition: Vec<String>,
        exp_global_need_heart: Vec<(i16, String, i16)>,
        p1_constant_score_bonus: i32,
        p2_constant_score_bonus: i32,
    ) {
        // Blade
        tdbg!("RC:7 BLADE");
        let old_blade = core::mem::take(&mut self.mods.constant_blade_bonuses);
        for (cid, val) in &old_blade {
            self.mods.remove_blade_modifier(*cid, *val as i16);
        }
        for (&cid, &val) in &exp_blade {
            self.mods.add_blade_modifier(cid, val as i16);
        }
        self.mods.constant_blade_bonuses = exp_blade;
        self.scratch_exp_blade = old_blade;

        // Score
        tdbg!("RC:9 SCORE");
        let old_score = core::mem::take(&mut self.mods.constant_score_bonuses);
        for (cid, val) in &old_score {
            self.mods.remove_score_modifier(*cid, *val as i16);
        }
        for (&cid, &val) in &exp_score {
            self.mods.add_score_modifier(cid, val as i16);
        }
        self.mods.constant_score_bonuses = exp_score;
        self.scratch_exp_score = old_score;

        // Per-player global score bonus (from GainAbility modify_score)
        self.mods.p1_constant_total_score_bonus = p1_constant_score_bonus as i16;
        self.mods.p2_constant_total_score_bonus = p2_constant_score_bonus as i16;

        // Heart — clear old constant heart modifiers first, then re-apply new ones.
        tdbg!("RC:10 HEART");
        // Must drain the OLD map so bonuses from cards that left the stage are removed.
        {
            let old_heart = core::mem::take(&mut self.mods.constant_heart_bonuses);
            for (cid, cols) in &old_heart {
                for (color_str, &delta) in cols {
                    let hc = crate::card::parse_heart_color(color_str);
                    self.mods.remove_heart_modifier(*cid, hc, delta as i16);
                }
            }
            self.scratch_exp_heart = old_heart;
        }
        for (cid, cols) in &exp_heart {
            for (color_str, delta) in cols {
                let hc = crate::card::parse_heart_color(color_str);
                self.mods.add_heart_modifier(*cid, hc, *delta as i16);
            }
        }
        self.mods.constant_heart_bonuses = exp_heart;

        tdbg!("RC:11 PROHIBITION");
        // Apply restriction effects from constant abilities.
        // Use "const_restriction:" prefix to distinguish from debut/live ability restrictions
        // so we can safely clear and re-add constant restrictions on each recalculate call.
        self.prohibition_effects
            .retain(|p| !p.starts_with("const_restriction:"));
        for p in &exp_prohibition {
            self.prohibition_effects.push(p.clone());
        }

        tdbg!("RC:12 GLOBAL_NEED_HEART");
        // Clear old constant global need_heart modifiers, then re-apply new ones.
        let old_global_nh = core::mem::take(&mut self.mods.constant_global_need_heart);
        for (card_id, color_str, delta) in &old_global_nh {
            let hc = crate::card::parse_heart_color(color_str);
            self.mods
                .add_need_heart_modifier(*card_id, hc, -*delta as i16);
        }
        for (card_id, color_str, delta) in &exp_global_need_heart {
            let hc = crate::card::parse_heart_color(color_str);
            self.mods
                .add_need_heart_modifier(*card_id, hc, *delta as i16);
        }
        self.mods.constant_global_need_heart = exp_global_need_heart;
        tdbg!("RC:12b GLOBAL_NEED_HEART_DONE");
    }

    /// Re-evaluate all constant (常時) abilities on all stage members.
    /// Handles gain_resource(blade, heart), modify_score, modify_cost.
    /// Clears old constant-derived values and re-applies those whose conditions pass.
    ///
    /// NOTE: deliberately runs unconditionally (no staleness gating). Constant
    /// ability *conditions* read live state (energy counts, positions, success
    /// zone) that mutates on paths a dirty-flag scheme cannot see (e.g. paying
    /// energy costs); gating breaks 51 tests (wien dynamic energy, ruby front
    /// blade, ayumu/ayumu-style zone-leave constants).
    #[inline(never)]
    pub fn recalculate_constants(&mut self) {
        tdbg!("RC:0 ENTERED");
        // HANG WORKAROUND (3DS ARMv6K): AtomicBool::load uses 8-bit atomics
        // that may deadlock via Mutex fallback. Use a plain bool on GameState
        // OR skip the debug check entirely on 3DS.
        #[cfg(not(feature = "3ds"))]
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!("[SZ_DEBUG] recalculate_constants ENTERED");
        }
        tdbg!("RC:1 ATOMIC_LOAD_OK");
        let entries = self.collect_constant_stage_effect_ids();
        tdbg!("RC:2 COLLECT_EFFECTS_OK len={}", entries.len());
        self.mods.constant_score_sources.clear();

        // Clone the Arc once (cheap: atomic increment) — all effect lookups
        // go through this local reference instead of self.card_database,
        // avoiding 152B × N AbilityEffect clones per recalculation.
        let card_db = self.card_database.clone();

        // Reuse pre-allocated scratch buffers to avoid allocation storm
        let mut exp_blade = core::mem::take(&mut self.scratch_exp_blade);
        exp_blade.clear();
        let mut exp_score = core::mem::take(&mut self.scratch_exp_score);
        exp_score.clear();
        let mut exp_heart = core::mem::take(&mut self.scratch_exp_heart);
        exp_heart.clear();
        let mut exp_prohibition: Vec<String> = Vec::new();
        self.constant_cannot_activate_members.clear();
        let mut exp_global_need_heart: Vec<(i16, String, i16)> = Vec::new();
        let mut p1_constant_score_bonus: i32 = 0;
        let mut p2_constant_score_bonus: i32 = 0;
        let mut jyouji_statuses: Vec<crate::types::ConstantAbilityStatus> = Vec::new();
        tdbg!("RC:3 VEC_HASHMAP_INIT_OK");

        let mut entry_positions = core::mem::take(&mut self.scratch_entry_positions);
        entry_positions.clear();
        for (pos, &cid) in self.player1.stage.stage.iter().enumerate() {
            if cid != -1 {
                entry_positions.insert(cid, Some(pos as u8));
            }
        }
        for (pos, &cid) in self.player2.stage.stage.iter().enumerate() {
            if cid != -1 {
                entry_positions.entry(cid).or_insert(Some(pos as u8));
            }
        }
        tdbg!("RC:4 ENTRY_POSITIONS_DONE count={}", entry_positions.len());

        for &(card_id, ability_idx) in &entries {
            tdbg!("RC:5_LOOP card_id={}", card_id);
            // Re-lookup effect through the local Arc clone — avoids 152B clone.
            // The reference borrows card_db (not self), so there's no borrow
            // conflict with &mut self operations later in this iteration.
            let ability = match self.resolve_constant_ability(card_id, ability_idx) {
                Some(a) => a,
                None => continue,
            };
            let Some(ref effect) = ability.effect else {
                continue;
            };
            // Set activating_card so condition evaluators (e.g. exclude_self in
            // location_condition) know which card is "self" for this entry.
            let prev_activating = self.activating_card;
            self.activating_card = Some(card_id);
            // Card info and owner for jyouji status tracking are captured
            // lazily inside the `cond_met` branch below — computing them here
            // allocated per entry even when the condition failed.

            {
                let self_player = if self.player1.stage.stage.contains(&card_id) {
                    Some(&self.player1)
                } else {
                    Some(&self.player2)
                };
                let mut ctx =
                    crate::ability::condition::ConditionContext::new_with_self(self, self_player);
                // Constant abilities should register their effects regardless
                // of the current phase — the phase gate only matters at trigger
                // evaluation time, not during constant registration.
                ctx.skip_phase_gate = true;

                // Check effect-level position requirement.
                // "front" is a targeting rule (正面のエリア) not an activation gate,
                // so it does not restrict where the activating card must sit.
                //
                // activation_position ("左サイド,右サイド" etc.) is the AUTHORITATIVE
                // gate when present — cards like 鬼塚夏美 SP-bp7-009 print
                // 「（この能力は左サイド/右サイドエリアにいる場合のみ発動する）」 and the
                // parser encodes both slots there. Checking only `position` (which
                // may name a single slot) broke the second listed side: moving her
                // right never re-granted heart02.
                let card_pos = entry_positions.get(&card_id).copied().flatten();
                let pos_matches = |ps: &str, cp: Option<u8>| {
                    matches!(
                        (ps, cp),
                        ("center", Some(1))
                            | ("left" | "left_side", Some(0))
                            | ("right" | "right_side", Some(2))
                    )
                };
                let pos_ok = if let Some(act) = effect
                    .activation_position_any()
                    .map(|s| s.to_string())
                {
                    act.split(',')
                        .map(|p| p.trim())
                        .any(|p| pos_matches(p, card_pos))
                } else if let Some(ref pos) = effect.position_any() {
                    let pos_str = pos.get_position();
                    if pos_str == Some("front") {
                        true
                    } else {
                        matches!(
                            (pos_str, card_pos),
                            (Some("center"), Some(1))
                                | (Some("left") | Some("left_side"), Some(0))
                                | (Some("right") | Some("right_side"), Some(2))
                                | (None, _)
                        )
                    }
                } else {
                    true
                };

                if pos_ok {
                    let cond_met = effect
                        .condition
                        .as_ref()
                        .is_none_or(|c| ctx.evaluate_condition(c));

                    if cond_met {
                        // Record jyouji status for this card (lazily capture
                        // name/owner only now that the condition passed).
                        // Skipped under `headless` — display-only summary data.
                        #[cfg(not(feature = "headless"))]
                        {
                            let status_card_name = card_db
                                .get_card(card_id)
                                .map(|c| c.name.to_string())
                                .unwrap_or_default();
                            let status_owner = if self.player1.stage.stage.contains(&card_id) {
                                self.player1.id.clone()
                            } else {
                                self.player2.id.clone()
                            };
                            jyouji_statuses.push(crate::types::ConstantAbilityStatus {
                                card_id: card_id,
                                card_name: status_card_name.clone(),
                                owner: status_owner.clone(),
                                zone: "stage".to_string(),
                                ability_text: effect.text.to_string(),
                                all_conditions_met: pos_ok && cond_met,
                                conditions: vec![crate::types::ConditionResult {
                                    text: "条件".to_string(),
                                    passed: cond_met,
                                }],
                            });
                        }
                        match effect.action {
                            crate::ability::enums::ActionType::GainResource => {
                                match effect.resource_any().as_deref().unwrap_or("") {
                                    "blade" | "ブレード" => {
                                        let n = if let Some(ref dc) = effect.dynamic_count_any() {
                                            self.resolve_dynamic_count(
                                                dc,
                                                &[],
                                                &[],
                                                0,
                                                Some(card_id),
                                            ) as i32
                                        } else if effect.per_unit_any().unwrap_or(false) {
                                            let player =
                                                if self.player1.stage.stage.contains(&card_id) {
                                                    &self.player1
                                                } else {
                                                    &self.player2
                                                };
                                            let units = crate::ability::util::constant_per_unit_units(
                                                effect,
                                                player,
                                                &self.card_database,
                                                &self.mods.orientation_modifiers,
                                                card_id,
                                            );
                                            let base = if effect.max.unwrap_or(false) {
                                                1
                                            } else {
                                                effect
                                                    .resource_icon_count_any()
                                                    .unwrap_or(effect.count_any().unwrap_or(1))
                                            };
                                            units * base as i32
                                        } else {
                                            effect
                                                .resource_icon_count_any()
                                                .unwrap_or(effect.count_any().unwrap_or(1))
                                                as i32
                                        };
                                        // "失う" (lose) is represented as sign:negative.
                                        // Apply the sign so the modifier is negative.
                                        let sign_mult: i16 = if matches!(
                                            effect.sign_any().as_deref(),
                                            Some("negative") | Some("-")
                                        ) {
                                            -1
                                        } else {
                                            1
                                        };
                                        let delta = (n as i16) * sign_mult;
                                        // Determine blade grant targets:
                                        //   - position "front" (正面のエリア): opponent's
                                        //     mirrored slot (your left faces opp right, etc.)
                                        //   - all_any (自分のステージにいる...): all matching
                                        //     members on the ability card's side
                                        //   - otherwise: the activating card itself
                                        let is_front = effect
                                            .position_any()
                                            .as_ref()
                                            .and_then(|p| p.get_position())
                                            == Some("front");
                                        if is_front {
                                            for tid in self.constant_front_targets(card_id, effect) {
                                                *exp_blade.entry(tid).or_insert(0) += delta;
                                            }
                                        } else if effect.all_any().unwrap_or(false) {
                                            // Grant to ALL matching stage members on the
                                            // ability card's side. Optionally restricted to
                                            // members that have a member card underneath
                                            // (requires_under_card, e.g. 渡辺曜 ab#1).
                                            let player = if self.player1.stage.stage.contains(&card_id) {
                                                &self.player1
                                            } else {
                                                &self.player2
                                            };
                                            let filter = effect.filter_subset();
                                            let need_under =
                                                effect.requires_under_card_any().unwrap_or(false);
                                            for (slot, &mid) in player.stage.stage.iter().enumerate() {
                                                if mid == -1
                                                    || !filter.matches(&self.card_database, mid, true)
                                                {
                                                    continue;
                                                }
                                                if need_under {
                                                    let has_member_under = player.stage.under_cards[slot]
                                                        .iter()
                                                        .any(|&u| {
                                                            self.card_database
                                                                .get_card(u)
                                                                .map_or(false, |c| c.is_member())
                                                        });
                                                    if !has_member_under {
                                                        continue;
                                                    }
                                                }
                                                *exp_blade.entry(mid).or_insert(0) += delta;
                                            }
                                        } else {
                                            *exp_blade.entry(card_id).or_insert(0) += delta;
                                        }
                                    }
                                    "heart" | "ハート" => {
                                        let n = if let Some(ref dc) = effect.dynamic_count_any() {
                                            // Unified dynamic_count resolution (dynamic_count.rs).
                                            // The constant path has no resolver step context, so
                                            // pass empty moved/selected and 0 draw count.
                                                self.resolve_dynamic_count(
                                                    dc,
                                                    &[],
                                                    &[],
                                                    0,
                                                    Some(card_id),
                                                ) as i32
                                        } else if effect.per_unit_any().unwrap_or(false) {
                                            let player =
                                                if self.player1.stage.stage.contains(&card_id) {
                                                    &self.player1
                                                } else {
                                                    &self.player2
                                                };
                                            crate::ability::util::constant_per_unit_units(
                                                effect,
                                                player,
                                                &self.card_database,
                                                &self.mods.orientation_modifiers,
                                                card_id,
                                            )
                                        } else {
                                            effect.count_any().unwrap_or(1) as i32
                                        };
                                        if crate::ability::util::is_all_heart_type(effect) {
                                            *exp_heart
                                                .entry(card_id)
                                                .or_default()
                                                .entry(crate::ability::util::HEART_ALL_KEY.to_string())
                                                .or_insert(0) += n as i16;
                                        } else {
                                            let hc_list = effect.heart_colors_any().to_vec();
                                            let per_entry = crate::ability::util::heart_gain_per_entry(
                                                n,
                                                &hc_list,
                                            ) as i16;
                                            for hc in &hc_list {
                                                *exp_heart
                                                    .entry(card_id)
                                                    .or_default()
                                                    .entry(hc.clone())
                                                    .or_insert(0) += per_entry;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            crate::ability::enums::ActionType::ModifyScore => {
                                let sv = effect.value_any().unwrap_or(0) as i32;
                                if sv != 0 {
                                    self.mods.constant_score_sources.push((
                                        card_id,
                                        effect.text.to_string(),
                                        sv as i16,
                                    ));
                                }
                                // target="live_total" (parser-emitted for
                                // 「ライブの合計スコアを＋１する」) modifies the
                                // player's live TOTAL — route it into the same
                                // per-player accumulator used by gained 常時
                                // score abilities. Keying it under the member's
                                // card_id could never match a live card and
                                // silently no-op'd.
                                if effect.target_any() == Some("live_total") {
                                    let belongs_to_p1 =
                                        self.player1.stage.stage.contains(&card_id);
                                    if belongs_to_p1 {
                                        p1_constant_score_bonus += sv;
                                    } else {
                                        p2_constant_score_bonus += sv;
                                    }
                                } else {
                                    *exp_score.entry(card_id).or_insert(0) += sv as i16;
                                }
                            }
                            crate::ability::enums::ActionType::Restriction => {
                                if let Some(rt) = effect.restriction_type_any() {
                                    let card_name = self
                                        .card_database
                                        .get_card(card_id)
                                        .map(|c| c.name.to_string())
                                        .unwrap_or_default();
                                    // Do NOT push `cannot_activate` to prohibition_effects:
                                    // the auto-activation blocking is already handled by
                                    // constant_cannot_activate_members in phases.rs. Pushing
                                    // to prohibition_effects would incorrectly block manual
                                    // ability activation via is_action_prohibited.
                                    if rt != "cannot_activate" {
                                        exp_prohibition.push(format!(
                                            "const_restriction:{},card={},cardname={}:",
                                            rt, card_id, card_name
                                        ));
                                    }
                                    let tgt_opt = effect.target_any();
                                    let tgt = tgt_opt.unwrap_or("self");
                                    if rt == "cannot_activate_by_effect" {
                                        let resolved = self.resolve_target_player(tgt).id.clone();
                                        if !self.cannot_activate_members.contains(&resolved) {
                                            self.cannot_activate_members.push(resolved);
                                        }
                                    } else if rt == "cannot_activate" {
                                        if tgt == "self" {
                                            // Per-card: only block this specific member
                                            self.constant_cannot_activate_members
                                                .push(card_id.to_string());
                                        } else {
                                            // Player-level: block all members of the target player
                                            let resolved =
                                                self.resolve_target_player(tgt).id.clone();
                                            self.constant_cannot_activate_members.push(resolved);
                                        }
                                    }
                                    if rt == "cannot_live" {
                                        let resolved = self.resolve_target_player(tgt).id.clone();
                                        if !self.cannot_live_players.contains(&resolved) {
                                            self.cannot_live_players.push(resolved);
                                        }
                                    }
                                }
                            }
                            // ── gain_ability ──────────────────────────────────
                            // This produces a persistent effect that shows on the
                            // card via bonus_triggers texticon (in game_state_to_display).
                            // The trigger type (常時 → jyouji.png, ライブ成功時 → live_success.png)
                            // is read from ability_gain_trigger and rendered on the card.
                            //
                            // Texticon display for this action:
                            //   - All-heart case: bonus_heart "all" → icon_all.png badge
                            //   - ModifyScore via gained_effect: bonus_score → icon_score.png badge
                            //     PLUS bonus_triggers → trigger texticon (e.g. jyouji.png)
                            //   - ConditionalAlternative: deferred, no immediate texticon
                            //   - Legacy text parse: bonus_score → icon_score.png badge
                            //     PLUS bonus_triggers → trigger texticon
                            crate::ability::enums::ActionType::GainAbility => {
                                if effect.ability_gain_any().as_deref()
                                    == Some("{{icon_all.png|ハート}}")
                                    || effect
                                        .ability_gain_any()
                                        .as_deref()
                                        .is_some_and(|t| t.contains("ALL"))
                                    || effect
                                        .ability_gain_any()
                                        .as_deref()
                                        .is_some_and(|t| t.contains("【ハート】"))
                                {
                                    // All-heart: store as single "all" entry (HeartColor::All)
                                    *exp_heart
                                        .entry(card_id)
                                        .or_default()
                                        .entry("all".to_string())
                                        .or_insert(0) += 1i16;
                                } else if let Some(gain_text) = effect.ability_gain_any().as_deref()
                                {
                                    // Determine which player this card belongs to
                                    let belongs_to_p1 = self.player1.stage.stage.contains(&card_id);
                                    let bonus_target = if belongs_to_p1 {
                                        &mut p1_constant_score_bonus
                                    } else {
                                        &mut p2_constant_score_bonus
                                    };

                                    // Record the gained ability for tracking
                                    self.add_gained_ability(card_id, gain_text.to_string());

                                    // Use gained_effect if available (structured data from parser)
                                    if let Some(ref gained) = effect.gained_effect_any() {
                                        let action = gained.action;
                                        if action
                                            == crate::ability::enums::ActionType::ModifyScore
                                        {
                                            let val = gained.value_any().unwrap_or(0) as i32;
                                            *bonus_target += val;
                                            if val != 0 {
                                                self.mods.constant_score_sources.push((
                                                    card_id,
                                                    gain_text.to_string(),
                                                    val as i16,
                                                ));
                                            }
                                        } else if action
                                            == crate::ability::enums::ActionType::ConditionalAlternative
                                        {
                                            // Conditional gained effects (e.g. live_success score
                                            // based on revealed card count) can't be evaluated at
                                            // constant evaluation time.  Store them for later
                                            // evaluation during execute_live_victory_determination.
                                            self.delayed_gained_effects
                                                .push((card_id, *(*gained).clone()));
                                        }
                                    } else {
                                        // Fallback: parse value from text (legacy path)
                                        if let Some(val) =
                                            gain_text.split('+').nth(1).and_then(|s| {
                                                s.chars()
                                                    .take_while(|c| c.is_ascii_digit())
                                                    .collect::<String>()
                                                    .parse::<i32>()
                                                    .ok()
                                            })
                                        {
                                            *bonus_target += val;
                                            if val != 0 {
                                                self.mods.constant_score_sources.push((
                                                    card_id,
                                                    gain_text.to_string(),
                                                    val as i16,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            crate::ability::enums::ActionType::GainAbilityFromSource => {
                                let mut resolver = crate::ability::resolver::AbilityResolver::new(
                                    self.card_database.clone(),
                                    self.activating_card,
                                );
                                let _ = resolver.execute_gain_ability_from_source(self, effect);
                            }
                            crate::ability::enums::ActionType::ModifyRequiredHeartsGlobal => {
                                let target_name = effect.target_name();
                                let target_player = self.resolve_target_player(target_name);
                                let target_cards: Vec<i16> =
                                    target_player.live_card_zone.cards.to_vec();
                                let value = effect.value_or_count(1) as i32;
                                let op_str = effect.operation_any().unwrap_or("increase");
                                let op = op_str;
                                let delta = match op {
                                    "increase" => value,
                                    "decrease" => -value,
                                    _ => value,
                                };
                                let colors: Vec<String> = if effect.heart_colors_any().is_empty() {
                                    vec![crate::ability::util::HEART_ALL_KEY.to_string()]
                                } else {
                                    effect.heart_colors_any().to_vec()
                                };
                                for card_id in &target_cards {
                                    for color in &colors {
                                        exp_global_need_heart.push((
                                            *card_id,
                                            color.clone(),
                                            delta as i16,
                                        ));
                                    }
                                }
                            }
                            crate::ability::enums::ActionType::Sequential => {
                                if let Some(ref actions) = effect.compound.actions {
                                    for sub in actions {
                                        let sub_cond = sub
                                            .condition
                                            .as_ref()
                                            .is_none_or(|c| ctx.evaluate_condition(c));
                                        if !sub_cond {
                                            continue;
                                        }
                                        if sub.action
                                            == crate::ability::enums::ActionType::GainResource
                                        {
                                            match sub.resource_any().as_deref().unwrap_or("") {
                                                "blade" | "ブレード" => {
                                                    let n = sub
                                                        .resource_icon_count_any()
                                                        .unwrap_or(sub.count.unwrap_or(1))
                                                        as i32;
                                                    *exp_blade.entry(card_id).or_insert(0) +=
                                                        n as i16;
                                                }
                                                "heart" | "ハート" => {
                                                    let n = sub.count.unwrap_or(1) as i32;
                                                    let hc_list: Vec<String> =
                                                        sub.heart_colors_any().to_vec();
                                                    let per_color = crate::ability::util::heart_gain_per_entry(
                                                        n,
                                                        &hc_list,
                                                    ) as i16;
                                                    for hc in &hc_list {
                                                        *exp_heart
                                                            .entry(card_id)
                                                            .or_default()
                                                            .entry(hc.clone())
                                                            .or_insert(0) += per_color;
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Restore the previous activating_card
            self.activating_card = prev_activating;
        }
        // Recycle entry_positions allocation into scratch buffer
        self.scratch_entry_positions = entry_positions;
        let _jyouji_len = jyouji_statuses.len();
        self.constant_ability_statuses = jyouji_statuses.into();
        tdbg!("RC:6 MAIN_LOOP_DONE jyouji={}", _jyouji_len);

        // Under-card constant blade abilities ("常時：このカードが『X』のメンバーの
        // 下に置かれているかぎり、そのメンバーはブレードを得る"). The blade is
        // granted to the HOST member the card is stacked under, not the card itself
        // (which isn't on stage).
        self.grant_under_card_constant_blades(&mut exp_blade);

        self.commit_constant_results(
            exp_blade,
            exp_score,
            exp_heart,
            exp_prohibition,
            exp_global_need_heart,
            p1_constant_score_bonus,
            p2_constant_score_bonus,
        );

        // Also recalculate cost modifiers from hand cards (hand-based cost reductions)
        // Pass pre-collected stage effects to avoid re-scanning the stage
        tdbg!("RC:13 COST_MODIFIERS_WITH_ENTRIES");
        let hand_ids = self.collect_constant_hand_effect_ids();
        self.recalculate_constant_cost_modifiers_with_ids(&entries, &hand_ids);
        tdbg!("RC:13b COST_MODIFIERS_DONE");

        // Evaluate constant abilities from success live card zone (e.g. Love wing bell)
        tdbg!("RC:14 SUCCESS_ZONE");
        self.evaluate_success_zone_constant_modifiers();
        tdbg!("RC:14b SUCCESS_ZONE_DONE");
        self.refresh_yell_sources();
    }

    /// G8: set each player's yell source from 常時 yell_source_modifier live cards
    /// (e.g. 恋になりたいAQUARIUM "デッキの上から行う代わりにデッキの下から行う").
    /// A live card in the live/success zone whose custom effect is
    /// custom{yell_source_modifier, yell_source:deck_bottom} sets yell_from_bottom.
    fn refresh_yell_sources(&mut self) {
        let db = self.card_database.clone();
        for player in [&mut self.player1, &mut self.player2] {
            player.yell_from_bottom = false;
            let cids: Vec<i16> = player
                .live_card_zone
                .cards
                .iter()
                .chain(player.success_live_card_zone.cards.iter())
                .copied()
                .collect();
            for cid in cids {
                let Some(card) = db.get_card(cid) else { continue };
                let has_yell_bottom = card.abilities.iter().any(|ar| {
                    let a = ar.resolve();
                    a.triggers.as_ref().is_some_and(|t| {
                        t.contains(crate::triggers::CONSTANT)
                    }) && a.effect.as_ref().is_some_and(|e| {
                        e.action == crate::ability::enums::ActionType::ModifyYellSource
                            && e.yell_source_any().as_deref() == Some("deck_bottom")
                    })
                });
                if has_yell_bottom {
                    player.yell_from_bottom = true;
                    break;
                }
            }
        }
    }

    pub fn recalculate_constant_cost_modifiers(&mut self) {
        let stage_ids = self.collect_constant_stage_effect_ids();
        let hand_ids = self.collect_constant_hand_effect_ids();
        self.recalculate_constant_cost_modifiers_with_ids(&stage_ids, &hand_ids);
    }

    fn recalculate_constant_cost_modifiers_with_ids(
        &mut self,
        stage_ids: &[(i16, usize)],
        hand_ids: &[(i16, usize)],
    ) {
        let mut expected: HashMap<i16, i16> = HashMap::default();
        // Set-operation modifiers ("このカードのコストはNになる") override the cost
        // to an absolute value rather than adjusting it by a delta.
        let mut expected_set: HashMap<i16, i16> = HashMap::default();
        {
            // Chain stage and hand ability IDs, look up each effect, filter to ModifyCost
            let all_ids = stage_ids.iter().chain(hand_ids.iter());
            for &(cid, ability_idx) in all_ids {
                let Some(cost_ability) = self.resolve_constant_ability(cid, ability_idx) else {
                    continue;
                };
                let Some(ref effect) = cost_ability.effect else {
                    continue;
                };
                if effect.action != crate::ability::enums::ActionType::ModifyCost {
                    continue;
                }
                // LL-bp7-001 play-time cost (手札3枚捨てて10) is NOT a passive constant;
                // it is handled via the pre-play choice hook in phases.rs.
                // Detect by: set 10 + location hand + 3 characters + optional.
                let is_ll_bp7_play_cost = effect.operation_any().as_deref() == Some("set")
                    && effect.value_any() == Some(10)
                    && effect.location_any().as_deref() == Some("hand")
                    && effect.optional.unwrap_or(false)
                    && effect.characters_any().map(|c| c.len() == 3).unwrap_or(false);
                if is_ll_bp7_play_cost {
                    continue;
                }
                // Resolve each card's OWNER so condition evaluators ("自分の..." /
                // comparison_target: opponent) judge from the right player's
                // perspective. A shared context would evaluate every copy as if
                // it belonged to player1, wrongly applying a mirror-match ability
                // to both sides when only the side with more energy should qualify.
                let owner_in_p1 = self.player1.stage.stage.contains(&cid)
                    || self.player1.hand.cards.contains(&cid)
                    || self.player1.energy_zone.cards.contains(&cid);
                let owner_in_p2 = self.player2.stage.stage.contains(&cid)
                    || self.player2.hand.cards.contains(&cid)
                    || self.player2.energy_zone.cards.contains(&cid);
                let self_player = if owner_in_p1 {
                    Some(&self.player1)
                } else if owner_in_p2 {
                    Some(&self.player2)
                } else {
                    None
                };
                let mut ctx =
                    crate::ability::condition::ConditionContext::new_with_self(self, self_player);
                ctx.skip_phase_gate = true;
                let cond_met = effect
                    .condition
                    .as_ref()
                    .is_none_or(|c| ctx.evaluate_condition(c));
                if cond_met {
                    let mut value = effect.value_any().unwrap_or(0) as i32;

                    // Handle per_unit cost reduction (e.g. "1 per other card in hand")
                    if effect.per_unit_any().unwrap_or(false) {
                        let player = self.resolve_target_player(effect.target_name());
                        // per_unit_location overrides the counting zone when the
                        // parser determines the per-unit count targets a different
                        // zone than the effect's location (e.g. count stage members
                        // while the cost modifier itself applies to hand cards).
                        let per_unit_loc = effect.per_unit_location_any();
                        let loc2 = effect.location_any();
                        let count_zone = per_unit_loc.or(loc2).unwrap_or(Zone::Hand.to_str());
                        let count = if count_zone == "stage" && effect.group_names_any().is_some() {
                            let group_name = effect.group_name();
                            let card_db = &self.card_database;
                            let stage_ids: Vec<i16> = player
                                .stage
                                .stage
                                .iter()
                                .copied()
                                .filter(|&id| id != -1)
                                .collect();
                            log::debug!(
                                "[COST_MOD_PER_UNIT_DEBUG] stage_ids={:?} group_name={:?}",
                                stage_ids,
                                group_name
                            );
                            let matches = stage_ids
                                .iter()
                                .filter(|&&id| {
                                    crate::ability::util::card_matches_group_str(
                                        card_db, id, group_name,
                                    )
                                })
                                .count();
                            log::debug!("[COST_MOD_PER_UNIT_DEBUG] group_matches={}", matches);
                            matches as u8
                        } else {
                            let cards: Vec<i16> =
                                crate::ability::util::zone_cards(player, count_zone).to_vec();
                            cards.len() as u8
                        };
                        log::debug!(
                            "[COST_MOD_PER_UNIT] cid={} count_zone={} count={}",
                            cid,
                            count_zone,
                            count
                        );
                        let per_unit_count = effect.per_unit_count_any().unwrap_or(1);
                        let exclude_self = effect.exclude_self_any().unwrap_or(false);
                        let effective = if exclude_self {
                            count.saturating_sub(1)
                        } else {
                            count
                        };
                        value = ((effective / per_unit_count) * (value as u8)) as i32;
                        log::debug!("[COST_MOD] cid={} zone={} count={} eff={} per_unit_cnt={} val={} exclude={}",
                            cid, count_zone, count, effective, per_unit_count, value, exclude_self);
                    }
                    log::debug!(
                        "[COST_MOD] cid={} op={:?} val={}",
                        cid,
                        effect.operation_any().as_deref(),
                        value
                    );

                    let op_str = effect.operation_any().unwrap_or("add");
                    let op = op_str;
                    match op {
                        "add" => *expected.entry(cid).or_insert(0) += value as i16,
                        "subtract" => *expected.entry(cid).or_insert(0) -= value as i16,
                        "set" => {
                            expected_set.insert(cid, value as i16);
                        }
                        _ => {}
                    }
                }
            }
        }

        let old_bonuses = core::mem::take(&mut self.mods.constant_cost_bonuses);
        for (cid, old) in &old_bonuses {
            self.mods.remove_cost_modifier(*cid, *old as i16);
        }
        // Clear previously-applied set overrides that are no longer active.
        let old_sets = core::mem::take(&mut self.mods.constant_cost_set_bonuses);
        for cid in old_sets.keys() {
            self.mods.remove_cost_modifier_set(*cid);
        }
        for (&cid, &new_val) in &expected {
            self.mods.add_cost_modifier(cid, new_val as i16);
        }
        for (&cid, &new_val) in &expected_set {
            self.mods.set_cost_modifier(cid, new_val as i16);
        }
        self.mods.constant_cost_bonuses = expected;
        self.mods.constant_cost_set_bonuses = expected_set;
    }

    pub fn set_heart_override(
        &mut self,
        card_id: i16,
        color: crate::card::HeartColor,
        count: u8,
        duration: &str,
    ) {
        self.mods.set_heart_override(card_id, color, count);
        #[cfg(feature = "serde_support")]
        {
            let mut data = serde_json::Map::new();
            data.insert(
                "card_id".to_string(),
                serde_json::Value::Number(card_id.into()),
            );
            data.insert(
                "color".to_string(),
                serde_json::Value::String(format!("{:?}", color)),
            );
            data.insert("count".to_string(), serde_json::Value::Number(count.into()));
        }
        self.temporary_effects.push(TemporaryEffect {
            effect_type: "heart_override".to_string(),
            duration: match duration {
                "live_end" => Duration::LiveEnd,
                "this_turn" => Duration::ThisTurn,
                _ => Duration::ThisLive,
            },
            created_turn: self.turn_number,
            created_phase: self.current_phase.clone(),
            target_player_id: String::new(),
            description: format!("Heart override: card {} = {:?} x{}", card_id, color, count),
            creation_order: 0,
            effect_data: Some(crate::core::types::EffectData::HeartOverride {
                card_id,
                color: format!("{:?}", color),
                count,
            }),
        });
    }

    pub fn clear_area_placement_tracking(&mut self) {
        self.areas_placed_this_turn.clear();
    }

    pub fn record_card_appearance(&mut self, card_id: i16, source: &str) {
        if !self.cards_appeared_this_turn.contains(&card_id) {
            self.cards_appeared_this_turn.push(card_id);
        }
        if !self.recently_appeared_cards.contains(&card_id) {
            self.recently_appeared_cards.push(card_id);
        }
        if !source.is_empty() {
            self.card_appearance_source
                .push((card_id, source.to_string()));
        }
    }

    pub fn has_card_appeared_this_turn(&self, card_id: i16) -> bool {
        self.cards_appeared_this_turn.contains(&card_id)
    }

    pub fn get_card_appearance_source(&self, card_id: i16) -> Option<&str> {
        self.card_appearance_source
            .iter()
            .find(|(k, _)| k == &card_id)
            .map(|(_, v)| v.as_str())
    }

    pub fn clear_card_appearance_tracking(&mut self) {
        self.cards_appeared_this_turn.clear();
        self.card_appearance_source.clear();
    }

    pub fn clear_auto_ability_trigger_tracking(&mut self) {
        self.auto_ability_trigger_counts.clear();
    }

    pub fn record_baton_touch(&mut self, player_id: &str, arriving_card_id: Option<i16>) {
        if player_id == "p1" {
            self.baton_touch_count_p1 += 1;
        } else {
            self.baton_touch_count_p2 += 1;
        }
        if let Some(cid) = arriving_card_id {
            self.baton_touch_arriving_card_ids.push(cid);
        }
    }

    pub fn get_baton_touch_count(&self, player_id: &str) -> u8 {
        if player_id == "p1" {
            self.baton_touch_count_p1
        } else {
            self.baton_touch_count_p2
        }
    }

    pub fn clear_baton_touch_tracking(&mut self) {
        self.baton_touch_count_p1 = 0;
        self.baton_touch_count_p2 = 0;
        self.baton_touch_arriving_card_ids.clear();
        self.baton_touch_zero_cost = false;
        self.baton_touch_replaced_member_cost = None;
        self.baton_touch_replaced_member_id = None;
        self.baton_touch_arriving_card_id = None;
    }

    pub fn record_card_movement(&mut self, card_id: i16) {
        self.cards_moved_this_turn.push(card_id);
    }

    /// Push a MovementEvent recording the movement of a card, tracking what caused it.
    /// Also syncs `recently_moved_cards`/`recently_moved_from_zone` for backward compat.
    pub fn push_movement_event(
        &mut self,
        moved_card_id: i16,
        source_zone: &str,
        dest_zone: &str,
        cause_card_id: Option<i16>,
        cause_player_id: &str,
        effect_only: bool,
    ) {
        self.movement_event_counter = self.movement_event_counter.wrapping_add(1);
        let event = crate::types::MovementEvent {
            moved_card_id,
            source_zone: crate::types::ZoneId::from_str(source_zone),
            dest_zone: crate::types::ZoneId::from_str(dest_zone),
            cause_card_id,
            cause_player_id: cause_player_id.to_string(),
            effect_only,
            timestamp: self.movement_event_counter,
        };
        // Display-only tracking (skipped in profiling/bot mode):
        if cfg!(not(feature = "profiling")) {
            self.batch_movements.push(event.clone());
        }
        // Track turn-level ALL-zone movement for ability triggers
        self.turn_movements.push(event.clone());
        // Card left the stage → its gained abilities no longer apply
        if source_zone == "stage" && dest_zone != "stage" {
            self.clear_gained_abilities_for_card(moved_card_id);
        }
        let cards = self.recently_moved_cards.get_or_insert_with(SmallVec::new);
        cards.push(moved_card_id);
        self.recently_moved_from_zone = Some(source_zone.to_string());
        // Track turn-level area movement (stage-area-to-stage-area)
        let is_area_move = source_zone == "stage" && dest_zone == "stage";
        if is_area_move {
            self.turn_area_movements.push(event.clone());
            self.position_change_occurred_this_turn = true;
        }
        // Track in cards_moved_this_turn for fast O(1) lookups
        self.cards_moved_this_turn.push(moved_card_id);
    }

    pub fn has_card_moved_this_turn(&self, card_id: i16) -> bool {
        self.cards_moved_this_turn.iter().any(|x| x == &card_id)
    }

    pub fn clear_card_movement_tracking(&mut self) {
        self.cards_moved_this_turn.clear();
        self.turn_movements.clear();
        self.cards_appeared_this_turn.clear();
        self.turn_area_movements.clear();
    }

    pub fn remove_revealed_card(&mut self, card_id: i16) {
        self.revealed_cards.retain(|id| *id != card_id);
    }

    pub fn clear_revealed_cards(&mut self) {
        self.revealed_cards.clear();
        self.revealed_card_meta.clear();
    }

    pub fn remove_from_source_hands(&mut self, card_ids: &[i16]) {
        let mut seen = HashSet::<i16>::default();
        for &cid in card_ids {
            if !seen.insert(cid) {
                continue;
            }
            // Only remove from hand if the card was from a cost reveal
            // (tracked in revealed_cost_cards). Non-cost reveals (deck peek, etc.)
            // should NOT remove from hand.
            if !self.revealed_cost_cards.contains(&cid) {
                continue;
            }
            for player in [&mut self.player1, &mut self.player2] {
                if let Some(pos) = player.hand.cards.iter().position(|&c| c == cid) {
                    player.hand.remove_card(pos);
                    break;
                }
            }
        }
    }
    pub fn add_gained_ability(&mut self, card_id: i16, ability_type: String) {
        let list = self.gained_abilities.entry(card_id).or_default();
        // Idempotent: recalculate_constants runs on every state change and calls
        // this for the same constant gain_ability repeatedly — don't accumulate
        // duplicate entries (which previously multiplied bonus_triggers badges).
        if !list.contains(&ability_type) {
            list.push(ability_type);
        }
    }

    pub fn clear_gained_abilities_for_card(&mut self, card_id: i16) {
        self.gained_abilities.remove(&card_id);
        self.gained_card_abilities.remove(&card_id);
    }

    /// Single choke point for zone-exit cleanup (rule 4.1.4: a card that
    /// changes zones is a NEW card — all runtime state resets). Clears both
    /// the modifier tables and any runtime-gained abilities. Zone-exit paths
    /// MUST route through this instead of picking individual clears.
    pub fn on_cards_left_zones(&mut self, cards: &[i16]) {
        for &card_id in cards {
            if card_id == -1 {
                continue;
            }
            self.mods.clear_all_for_card(card_id);
            self.clear_gained_abilities_for_card(card_id);
        }
    }

    /// Evaluate all constant (常時) abilities on cards in the success_live_card_zone.
    /// Handles the following action types:
    ///   - modify_required_hearts: heart requirement reductions (existing behavior)
    ///   - gain_resource(blade): blade grants to stage members
    ///   - gain_resource(heart): heart grants to stage members
    ///   - modify_score: score bonuses to live cards
    ///   - sequential: recurses into sub-actions
    /// Uses a clear-and-re-evaluate pattern to ensure as_long_as semantics: when a
    /// card leaves the success zone, its modifier is not re-applied.
    /// Evaluate all constant (常時) abilities on cards in the success_live_card_zone.
    /// Used during the live flow (victory determination and live success triggering).
    /// Clears need_heart_modifiers first, then delegates to
    /// evaluate_success_zone_constant_modifiers for the tracked bonuses.
    pub fn evaluate_success_zone_constant_abilities(&mut self) {
        self.mods.need_heart_modifiers.clear();
        self.evaluate_success_zone_constant_modifiers();
    }

    /// Evaluate constant abilities on success zone cards for tracked bonuses
    /// (blade, heart, score). Does NOT touch need_heart_modifiers.
    /// Called from recalculate_constants on every state change, and from
    /// evaluate_success_zone_constant_abilities during the live flow.
    pub fn evaluate_success_zone_constant_modifiers(&mut self) {
        use crate::ability::condition::ConditionContext;

        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!("[SZ_DEBUG] evaluate_success_zone_constant_modifiers called");
            log::debug!(
                "[SZ_DEBUG] p1 success zone = {:?}",
                self.player1.success_live_card_zone.cards.to_vec()
            );
            log::debug!(
                "[SZ_DEBUG] p2 success zone = {:?}",
                self.player2.success_live_card_zone.cards.to_vec()
            );
        }

        // ── Clear previously-applied success zone bonuses ──
        let old_sz_blade = core::mem::take(&mut self.mods.success_zone_blade_bonuses);
        for (cid, val) in &old_sz_blade {
            self.mods.remove_blade_modifier(*cid, *val as i16);
        }
        let old_sz_heart = core::mem::take(&mut self.mods.success_zone_heart_bonuses);
        for (cid, cols) in &old_sz_heart {
            for (color_str, delta) in cols {
                let hc = crate::card::parse_heart_color(color_str);
                self.mods.remove_heart_modifier(*cid, hc, *delta);
            }
        }
        let old_sz_score = core::mem::take(&mut self.mods.success_zone_score_bonuses);
        for (cid, val) in &old_sz_score {
            self.mods.remove_score_modifier(*cid, *val as i16);
        }

        // Track non-stackable effects locally so they are reset each evaluation
        let mut local_non_stackable: HashSet<String> = HashSet::default();

        let zone_cards_p1 = self.player1.success_live_card_zone.cards.clone();
        let zone_cards_p2 = self.player2.success_live_card_zone.cards.clone();

        // Collect all (cid, player_index, effect) pairs upfront to avoid borrow conflicts
        let mut entries: Vec<(i16, usize, crate::card::AbilityEffect)> = Vec::new();
        for (player_idx, zone_cards) in [(0usize, &zone_cards_p1), (1, &zone_cards_p2)] {
            for cid in zone_cards {
                let card = match self.card_database.get_card(*cid) {
                    Some(c) => c.clone(),
                    None => continue,
                };
                for ar in &card.abilities {
                    let ability = ar.resolve();
                    let is_constant = ability
                        .triggers
                        .as_ref()
                        .is_some_and(|t| t.contains(crate::triggers::CONSTANT));
                    if !is_constant {
                        continue;
                    }
                    if let Some(effect) = ability.effect.as_ref() {
                        entries.push((*cid, player_idx, (**effect).clone()));
                    }
                }
            }
        }

        for (cid, player_idx, effect) in &entries {
            let prev_activating = self.activating_card;
            self.activating_card = Some(*cid);
            let self_player = match player_idx {
                0 => Some(&self.player1),
                1 => Some(&self.player2),
                _ => None,
            };
            let ctx = ConditionContext::new_with_self(self, self_player);
            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                log::debug!("[SZ_DEBUG] cid={} effect={}", cid, effect.action);
            }
            let cond_met = effect
                .condition
                .as_ref()
                .is_none_or(|c| ctx.evaluate_condition(c));
            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                log::debug!("[SZ_DEBUG] cond_met={}", cond_met);
            }
            if !cond_met {
                self.activating_card = prev_activating;
                continue;
            }
            if effect.non_stackable.unwrap_or(false) {
                let effect_key = format!("{}:{}", effect.action, effect.text);
                if local_non_stackable.contains(&effect_key) {
                    self.activating_card = prev_activating;
                    continue;
                }
                local_non_stackable.insert(effect_key);
            }

            self.apply_success_zone_effect(*cid, *player_idx, effect);
            self.activating_card = prev_activating;
        }
    }

    /// Apply a single success zone constant effect. Called by
    /// evaluate_success_zone_constant_modifiers and recursively for sequential sub-actions.
    fn apply_success_zone_effect(
        &mut self,
        cid: i16,
        player_idx: usize,
        effect: &crate::card::AbilityEffect,
    ) {
        use crate::ability::enums::ActionType;
        use crate::ability::resolver::AbilityResolver;

        // Resolve the correct player directly since these effects don't go through the ability queue
        let owner_player = match player_idx {
            0 => &mut self.player1,
            1 => &mut self.player2,
            _ => return,
        };

        match effect.action {
            ActionType::ModifyRequiredHearts => {
                let prev = self.activating_card;
                self.activating_card = Some(cid);
                // Set queue context so resolve_target_player("self") targets
                // the correct owner, not always player1.
                let owner_id = match player_idx {
                    0 => "player1",
                    1 => "player2",
                    _ => "player1",
                };
                self.ability_queue
                    .push_constant_context(owner_id.to_string());
                let mut resolver = AbilityResolver::new(self.card_database.clone(), Some(cid));
                let _ = resolver.execute_modify_required_hearts(self, effect);
                self.ability_queue.pop_constant_context();
                self.activating_card = prev;
            }
            ActionType::GainResource => {
                let resource_binding = effect.resource_any();
                let resource = resource_binding.unwrap_or("");
                let amount = effect
                    .resource_icon_count_any()
                    .unwrap_or(effect.count_or(1)) as i32;
                let card_db = self.card_database.clone();
                let player = match effect.target_name() {
                    "self" | "自分" => owner_player,
                    "opponent" | "相手" => match player_idx {
                        0 => &mut self.player2,
                        1 => &mut self.player1,
                        _ => return,
                    },
                    _ => owner_player,
                };
                if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
                {
                    log::debug!(
                        "[SZ_DEBUG] GainResource resource={} amount={} target={} position={:?}",
                        resource,
                        amount,
                        effect.target_name(),
                        effect.position_any()
                    );
                    log::debug!("[SZ_DEBUG] stage={:?}", player.stage.stage);
                }

                let candidates: Vec<i16> = player
                    .stage
                    .stage
                    .iter()
                    .enumerate()
                    .filter(|&(_, &idx)| idx != -1)
                    .filter(|&(pos, _)| {
                        if let Some(ref pos_req) = effect.position_any() {
                            let pos_str = pos_req.get_position();
                            match pos_str {
                                Some("center") => pos == 1,
                                Some("left") | Some("left_side") => pos == 0,
                                Some("right") | Some("right_side") => pos == 2,
                                _ => true,
                            }
                        } else {
                            true
                        }
                    })
                    .filter(|&(_, &id)| {
                        if let Some(ref groups) = effect.group_names_any() {
                            groups.iter().any(|g| {
                                crate::ability::util::card_matches_group_str(
                                    &card_db,
                                    id,
                                    Some(g.as_str()),
                                )
                            })
                        } else {
                            true
                        }
                    })
                    .map(|(_, &id)| id)
                    .collect();

                if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
                {
                    log::debug!(
                        "[SZ_DEBUG] GainResource resource={} amount={}",
                        resource,
                        amount
                    );
                    log::debug!(
                        "[SZ_DEBUG] candidates count={} ids={:?}",
                        candidates.len(),
                        candidates
                    );
                }
                match resource {
                    "blade" | "ブレード" => {
                        for &target_id in &candidates {
                            if crate::ability::debug::ABILITY_DEBUG
                                .load(core::sync::atomic::Ordering::Relaxed)
                            {
                                log::debug!(
                                    "[SZ_DEBUG] ADDING blade {} to target {}",
                                    amount,
                                    target_id
                                );
                            }
                            self.mods.add_blade_modifier(target_id, amount as i16);
                            *self
                                .mods
                                .success_zone_blade_bonuses
                                .entry(target_id)
                                .or_insert(0) += amount as i16;
                        }
                    }
                    "heart" | "ハート" => {
                        let heart_colors = if effect.heart_colors_any().is_empty() {
                            vec!["heart01".to_string()]
                        } else {
                            effect.heart_colors_any().to_vec()
                        };
                        let per_color =
                            crate::ability::util::heart_gain_per_entry(amount, &heart_colors) as i16;
                        for &target_id in &candidates {
                            for color_str in &heart_colors {
                                let hc = crate::card::parse_heart_color(color_str);
                                self.mods.add_heart_modifier(target_id, hc, per_color);
                                *self
                                    .mods
                                    .success_zone_heart_bonuses
                                    .entry(target_id)
                                    .or_default()
                                    .entry(color_str.clone())
                                    .or_insert(0) += per_color;
                            }
                        }
                    }
                    _ => {}
                }
            }
            ActionType::ModifyScore => {
                let player = match effect.target_name() {
                    "self" | "自分" => owner_player,
                    "opponent" | "相手" => match player_idx {
                        0 => &mut self.player2,
                        1 => &mut self.player1,
                        _ => return,
                    },
                    _ => owner_player,
                };
                let value = effect.value_or_count(1) as i32;
                let op_binding = effect.operation_any();
                let op = op_binding.unwrap_or("add");
                // When self_target is true, apply the score modifier to the
                // success zone card itself (e.g. Angelic Angel's +5 self buff).
                // Otherwise, target cards in the live set zone.
                let targets: Vec<i16> = if effect.is_self_target() {
                    vec![cid]
                } else {
                    player.live_card_zone.cards.to_vec()
                };
                for &target_id in &targets {
                    match op {
                        "set" => {
                            self.mods.set_score_modifier(target_id, value as i16);
                            self.mods
                                .success_zone_score_bonuses
                                .insert(target_id, value as i16);
                        }
                        _ => {
                            self.mods.add_score_modifier(target_id, value as i16);
                            *self
                                .mods
                                .success_zone_score_bonuses
                                .entry(target_id)
                                .or_insert(0) += value as i16;
                        }
                    }
                }
            }
            ActionType::Sequential => {
                if let Some(ref actions) = effect.compound.actions {
                    for sub in actions {
                        self.apply_success_zone_effect(cid, player_idx, sub);
                    }
                }
            }
            _ => {}
        }
    }
}
