use crate::core::constants::U8Count;
use super::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use crate::ability::enums::Zone;
use crate::core::types::TemporaryEffect;
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

/// UI-only provenance string: empty (no alloc) when training has logs off.
#[inline]
fn ui_text(s: &str) -> String {
    if crate::game_setup::logging_enabled() {
        s.to_string()
    } else {
        String::new()
    }
}

/// UI-only kind label for BonusSource.
#[inline]
fn ui_kind(s: &str) -> String {
    ui_text(s)
}

/// Does `requirement` name `slot`? This is only the comparison — the
/// position vocabulary itself lives in
/// `ability::util::activation_position_index`, which is its single source of
/// truth. "front", an unstated position and a card that is not on stage all
/// read as "does not name this slot"; callers decide whether that means pass.
fn slot_named(requirement: &str, slot: Option<u8>) -> bool {
    slot.is_some_and(|s| {
        crate::ability::util::activation_position_index(requirement) == Some(s as usize)
    })
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
    fn grant_under_card_constant_blades(
        &mut self,
        exp_blade: &mut HashMap<i16, i16>,
        exp_blade_sources: &mut Vec<crate::core::game_modifiers::BonusSource>,
    ) {
        for (under_cid, host) in self
            .player1
            .stage
            .under_cards_with_hosts()
            .into_iter()
            .chain(self.player2.stage.under_cards_with_hosts())
        {
            let card = match self.card_database.get_card(under_cid) {
                Some(c) => c,
                None => continue,
            };
            for ar in card.abilities.iter() {
                let ability = ar.resolve();
                if !GameState::ability_matches_trigger(
                    &ability,
                    &crate::game_state::AbilityTrigger::Constant,
                ) {
                    continue;
                }
                let Some(ref effect) = ability.effect else {
                    continue;
                };
                if effect.action != crate::ability::enums::ActionType::GainResource
                    || !matches!(effect.resource_any(), Some("blade"))
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
                exp_blade_sources.push(crate::core::game_modifiers::BonusSource {
                    source_card_id: under_cid,
                    ability_text: ui_text(&effect.text),
                    target_card_id: host,
                    amount: count as i32,
                    color: None,
                    kind: ui_kind("blade"),
                });
            }
        }
    }

    fn collect_reusable_constant_stage_effect_ids(&self, ids: &mut Vec<(i16, usize)>) {
        for cid in self
            .player1
            .stage
            .stage
            .iter()
            .chain(self.player2.stage.stage.iter())
            .copied()
            .filter(|cid| *cid != -1)
        {
            if let Some(card) = self.card_database.get_card(cid) {
                for (idx, ar) in card.abilities.iter().enumerate() {
                    let ability = ar.resolve();
                    if Self::ability_matches_trigger(
                        &ability,
                        &crate::game_state::AbilityTrigger::Constant,
                    ) && ability.effect.is_some()
                    {
                        ids.push((cid, idx));
                    }
                }
            }
            for (gidx, ability) in self
                .gained_card_abilities
                .get(&cid)
                .into_iter()
                .flatten()
                .enumerate()
            {
                if Self::ability_matches_trigger(
                    ability,
                    &crate::game_state::AbilityTrigger::Constant,
                ) && ability.effect.is_some()
                {
                    ids.push((cid, crate::ability::types::GAINED_ABILITY_INDEX_BASE + gidx));
                }
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
        exp_blade_sources: Vec<crate::core::game_modifiers::BonusSource>,
        exp_heart_sources: Vec<crate::core::game_modifiers::BonusSource>,
        exp_global_nh_sources: Vec<crate::core::game_modifiers::BonusSource>,
        p1_constant_score_bonus: i32,
        p2_constant_score_bonus: i32,
    ) {
        // Blade
        tdbg!("RC:7 BLADE");
        let old_blade = core::mem::take(&mut self.mods.constant_blade_bonuses);
        if old_blade != exp_blade {
            for (cid, val) in &old_blade {
                self.mods.remove_blade_modifier(*cid, *val);
            }
            for (&cid, &val) in &exp_blade {
                self.mods.add_blade_modifier(cid, val);
            }
        }
        self.mods.constant_blade_bonuses = exp_blade;
        self.mods.constant_blade_sources = exp_blade_sources;
        self.scratch_exp_blade = old_blade;

        // Score
        tdbg!("RC:9 SCORE");
        let old_score = core::mem::take(&mut self.mods.constant_score_bonuses);
        if old_score != exp_score {
            for (cid, val) in &old_score {
                self.mods.remove_score_modifier(*cid, *val);
            }
            for (&cid, &val) in &exp_score {
                self.mods.add_score_modifier(cid, val);
            }
        }
        if log::log_enabled!(log::Level::Debug) && old_score != exp_score {
            log::debug!("[CONSTANT_SCORE] per-card bonuses: {:?} -> {:?}", old_score, exp_score);
        }
        self.mods.constant_score_bonuses = exp_score;
        self.scratch_exp_score = old_score;

        // Per-player global score bonus (from GainAbility modify_score)
        if log::log_enabled!(log::Level::Debug)
            && (i32::from(self.mods.p1_constant_total_score_bonus) != p1_constant_score_bonus
                || i32::from(self.mods.p2_constant_total_score_bonus) != p2_constant_score_bonus)
        {
            log::debug!(
                "[CONSTANT_SCORE] live-total bonus: p1 {} -> {}, p2 {} -> {}",
                self.mods.p1_constant_total_score_bonus, p1_constant_score_bonus,
                self.mods.p2_constant_total_score_bonus, p2_constant_score_bonus
            );
        }
        self.mods.p1_constant_total_score_bonus = i16::try_from(p1_constant_score_bonus).unwrap();
        self.mods.p2_constant_total_score_bonus = i16::try_from(p2_constant_score_bonus).unwrap();

        // Heart — clear old constant heart modifiers first, then re-apply new ones.
        tdbg!("RC:10 HEART");
        // Must drain the OLD map so bonuses from cards that left the stage are removed.
        {
            let old_heart = core::mem::take(&mut self.mods.constant_heart_bonuses);
            if old_heart != exp_heart {
                for (cid, cols) in &old_heart {
                    for (color_str, &delta) in cols {
                        let hc = crate::card::parse_heart_color(color_str);
                        self.mods.remove_heart_modifier(*cid, hc, delta);
                    }
                }
                for (cid, cols) in &exp_heart {
                    for (color_str, delta) in cols {
                        let hc = crate::card::parse_heart_color(color_str);
                        self.mods.add_heart_modifier(*cid, hc, *delta);
                    }
                }
            }
            self.scratch_exp_heart = old_heart;
        }
        self.mods.constant_heart_bonuses = exp_heart;
        self.mods.constant_heart_sources = exp_heart_sources;

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
        if old_global_nh != exp_global_need_heart {
            for (card_id, color_str, delta) in &old_global_nh {
                let hc = crate::card::parse_heart_color(color_str);
                self.mods
                    .add_need_heart_modifier(*card_id, hc, -*delta);
            }
            for (card_id, color_str, delta) in &exp_global_need_heart {
                let hc = crate::card::parse_heart_color(color_str);
                self.mods
                    .add_need_heart_modifier(*card_id, hc, *delta);
            }
        }
        self.mods.constant_global_need_heart = exp_global_need_heart;
        self.mods.constant_need_heart_sources = exp_global_nh_sources;
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
        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("recalculate_constants");
        tdbg!("RC:0 ENTERED");
        // HANG WORKAROUND (3DS ARMv6K): AtomicBool::load uses 8-bit atomics
        // that may deadlock via Mutex fallback. Use a plain bool on GameState
        // OR skip the debug check entirely on 3DS.
        #[cfg(not(feature = "3ds"))]
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::trace!("[SZ_DEBUG] recalculate_constants");
        }
        tdbg!("RC:1 ATOMIC_LOAD_OK");
        let mut entries = core::mem::take(&mut self.scratch_constant_effect_ids);
        entries.clear();
        self.collect_reusable_constant_stage_effect_ids(&mut entries);
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
        let mut exp_delayed_gained_effects = SmallVec::new();
        let mut expected_gained_texts: HashMap<i16, Vec<String>> = HashMap::default();
        self.constant_cannot_activate_members.clear();
        let mut exp_global_need_heart: Vec<(i16, String, i16)> = Vec::new();
        // Per-source attribution for everything accumulated below (UI bonus
        // provenance). Mirrors the exp_* maps entry-for-entry.
        let mut exp_blade_sources: Vec<crate::core::game_modifiers::BonusSource> = Vec::new();
        let mut exp_heart_sources: Vec<crate::core::game_modifiers::BonusSource> = Vec::new();
        let mut exp_global_nh_sources: Vec<crate::core::game_modifiers::BonusSource> = Vec::new();
        let mut p1_constant_score_bonus: i32 = 0;
        let mut p2_constant_score_bonus: i32 = 0;
        let mut jyouji_statuses: Vec<crate::types::ConstantAbilityStatus> = Vec::new();
        tdbg!("RC:3 VEC_HASHMAP_INIT_OK");

        let mut entry_positions = core::mem::take(&mut self.scratch_entry_positions);
        entry_positions.clear();
        for (pos, &cid) in self.player1.stage.stage.iter().enumerate() {
            if cid != -1 {
                entry_positions.insert(cid, Some(u8::try_from(pos).unwrap()));
            }
        }
        for (pos, &cid) in self.player2.stage.stage.iter().enumerate() {
            if cid != -1 {
                entry_positions.entry(cid).or_insert(Some(u8::try_from(pos).unwrap()));
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

                // activation_position ("左サイド,右サイド" etc.) is the AUTHORITATIVE
                // gate when present — cards like 鬼塚夏美 SP-bp7-009 print
                // 「（この能力は左サイド/右サイドエリアにいる場合のみ発動する）」 and the
                // parser encodes both slots there. Checking only `position` (which
                // may name a single slot) broke the second listed side: moving her
                // right never re-granted heart02.
                let card_pos = entry_positions.get(&card_id).copied().flatten();
                let pos_ok = if let Some(act) = effect.activation_position_any() {
                    // The authoritative gate, and the parser can list several
                    // sides in it — any one of them satisfies the condition.
                    act.split(',')
                        .map(|p| p.trim())
                        .any(|p| slot_named(p, card_pos))
                } else {
                    // Otherwise the targeting rule, which names at most one slot.
                    // "front" and an unstated position are not slot constraints.
                    match effect.position_any().and_then(|p| p.get_position()) {
                        Some(pos) if pos != "front" => slot_named(pos, card_pos),
                        _ => true,
                    }
                };

                if pos_ok {
                    let cond_met = effect
                        .condition
                        .as_ref()
                        .is_none_or(|c| ctx.evaluate_condition(c));
                    log::debug!("[CONSTANT_CONDITION] zone=stage source={} ability={} action={} passes={}",
                        card_id, ability_idx, effect.action, cond_met);

                    if cond_met {
                        // Record jyouji status for this card (lazily capture
                        // name/owner only now that the condition passed).
                        // Skipped under `headless` — display-only summary data.
                        #[cfg(not(feature = "headless"))]
                        if crate::game_setup::logging_enabled() {
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
                                card_id,
                                card_name: status_card_name.clone(),
                                owner: status_owner.to_string(),
                                zone: "stage".to_string(),
                                ability_text: ui_text(&effect.text),
                                all_conditions_met: pos_ok && cond_met,
                                conditions: vec![crate::types::ConditionResult {
                                    text: "条件".to_string(),
                                    passed: cond_met,
                                }],
                            });
                        }
                        match effect.action {
                            crate::ability::enums::ActionType::GainResource => {
                                match effect.resource_any().unwrap_or("") {
                                    "blade" => {
                                        let n = if let Some(dc) = effect.dynamic_count_any() {
                                            self.resolve_dynamic_count(
                                                dc,
                                                &[],
                                                &[],
                                                0,
                                                Some(card_id),
                                            ) as i32
                                        } else if effect.per_unit_any().unwrap_or(false) {
                                            // Per-unit constants count units on a stage.
                                            // `target: "opponent"` (相手のステージにいる…)
                                            // counts the OPPOSING player's stage; anything
                                            // else defaults to the host's own side.
                                            let host_on_p1 =
                                                self.player1.stage.stage.contains(&card_id);
                                            let player = match effect.target_any() {
                                                Some("opponent") => {
                                                    if host_on_p1 {
                                                        &self.player2
                                                    } else {
                                                        &self.player1
                                                    }
                                                }
                                                _ => {
                                                    if host_on_p1 {
                                                        &self.player1
                                                    } else {
                                                        &self.player2
                                                    }
                                                }
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
                                            effect.sign_any(),
                                            Some("negative") | Some("-")
                                        ) {
                                            -1
                                        } else {
                                            1
                                        };
                                        let delta = i16::try_from(n).unwrap() * sign_mult;
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
                                                exp_blade_sources.push(crate::core::game_modifiers::BonusSource {
                                                    source_card_id: card_id,
                                                    ability_text: ui_text(&effect.text),
                                                    target_card_id: tid,
                                                    amount: delta as i32,
                                                    color: None,
                                                    kind: ui_kind("blade"),
                                                });
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
                                                                .is_some_and(|c| c.is_member())
                                                        });
                                                if !has_member_under {
                                                    continue;
                                                }
                                            }
                                                *exp_blade.entry(mid).or_insert(0) += delta;
                                                exp_blade_sources.push(crate::core::game_modifiers::BonusSource {
                                                    source_card_id: card_id,
                                                    ability_text: ui_text(&effect.text),
                                                    target_card_id: mid,
                                                    amount: delta as i32,
                                                    color: None,
                                                    kind: ui_kind("blade"),
                                                });
                                            }
                                        } else {
                                            *exp_blade.entry(card_id).or_insert(0) += delta;
                                            exp_blade_sources.push(crate::core::game_modifiers::BonusSource {
                                                source_card_id: card_id,
                                                ability_text: ui_text(&effect.text),
                                                target_card_id: card_id,
                                                amount: delta as i32,
                                                color: None,
                                                kind: ui_kind("blade"),
                                            });
                                        }
                                    }
                                    "heart" => {
                                        let n = if let Some(dc) = effect.dynamic_count_any() {
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
                                            // Same target=opponent resolution as the
                                            // blade path above.
                                            let host_on_p1 =
                                                self.player1.stage.stage.contains(&card_id);
                                            let player = match effect.target_any() {
                                                Some("opponent") => {
                                                    if host_on_p1 {
                                                        &self.player2
                                                    } else {
                                                        &self.player1
                                                    }
                                                }
                                                _ => {
                                                    if host_on_p1 {
                                                        &self.player1
                                                    } else {
                                                        &self.player2
                                                    }
                                                }
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
                                                .or_insert(0) += i16::try_from(n).unwrap();
                                            exp_heart_sources.push(crate::core::game_modifiers::BonusSource {
                                                source_card_id: card_id,
                                                ability_text: ui_text(&effect.text),
                                                target_card_id: card_id,
                                                amount: n,
                                                color: Some(crate::ability::util::HEART_ALL_KEY.to_string()),
                                                kind: ui_kind("heart"),
                                            });
                                        } else {
                                            let hc_list = effect.heart_colors_any();
                                            let per_entry = i16::try_from(
                                                n / hc_list.len().max(1) as i32,
                                            )
                                            .unwrap();
                                            for hc in hc_list {
                                                *exp_heart
                                                    .entry(card_id)
                                                    .or_default()
                                                    .entry(hc.clone())
                                                    .or_insert(0) += per_entry;
                                                exp_heart_sources.push(crate::core::game_modifiers::BonusSource {
                                                    source_card_id: card_id,
                                                    ability_text: ui_text(&effect.text),
                                                    target_card_id: card_id,
                                                    amount: per_entry as i32,
                                                    color: Some(hc.clone()),
                                                    kind: ui_kind("heart"),
                                                });
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            crate::ability::enums::ActionType::ModifyScore => {
                                let sv = i32::from(effect.value_any().unwrap_or(0));
                                if sv != 0 {
                                    self.mods.constant_score_sources.push((
                                        card_id,
                                        ui_text(&effect.text),
                                        i16::try_from(sv).unwrap(),
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
                                    *exp_score.entry(card_id).or_insert(0) += i16::try_from(sv).unwrap();
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
                                        let resolved = self.resolve_target_player(tgt).id.to_string();
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
                                                self.resolve_target_player(tgt).id.to_string();
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
                                if effect.ability_gain_any()
                                    == Some("{{icon_all.png|ハート}}")
                                    || effect
                                        .ability_gain_any()
                                        .is_some_and(|t| t.contains("ALL"))
                                {
                                    // All-heart: store as single "all" entry (HeartColor::All)
                                    *exp_heart
                                        .entry(card_id)
                                        .or_default()
                                        .entry("all".to_string())
                                        .or_insert(0) += 1i16;
                                    exp_heart_sources.push(crate::core::game_modifiers::BonusSource {
                                        source_card_id: card_id,
                                        ability_text: ui_text(&effect.text),
                                        target_card_id: card_id,
                                        amount: 1,
                                        color: Some("all".to_string()),
                                        kind: ui_kind("gained_ability"),
                                    });
                                } else if let Some(gain_text) = effect.ability_gain_any()
                                {
                                    // Determine which player this card belongs to
                                    let belongs_to_p1 = self.player1.stage.stage.contains(&card_id);
                                    let bonus_target = if belongs_to_p1 {
                                        &mut p1_constant_score_bonus
                                    } else {
                                        &mut p2_constant_score_bonus
                                    };

                                    let texts = expected_gained_texts.entry(card_id).or_default();
                                    if !texts.iter().any(|text| text == gain_text) {
                                        texts.push(ui_text(gain_text));
                                    }

                                    // Use gained_effect if available (structured data from parser)
                                    if let Some(gained) = effect.gained_effect_any() {
                                        let action = gained.action;
                                        if action
== crate::ability::enums::ActionType::ModifyScore
                                    {
                                        let val = i32::from(gained.value_any().unwrap_or(0));
                                        *bonus_target += val;
                                        if val != 0 {
                                            self.mods.constant_score_sources.push((
                                                card_id,
                                                ui_text(gain_text),
                                                i16::try_from(val).unwrap(),
                                            ));
                                        }
                                        } else if action
                                            == crate::ability::enums::ActionType::ConditionalAlternative
                                        {
                                            // Conditional gained effects (e.g. live_success score
                                            // based on revealed card count) can't be evaluated at
                                            // constant evaluation time.  Store them for later
                                            // evaluation during execute_live_victory_determination.
                                            exp_delayed_gained_effects
                                                .push((card_id, *gained.clone()));
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
                                                    ui_text(gain_text),
                                                    i16::try_from(val).unwrap(),
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
                                let value = effect.value_or_count(1) as i32;
                                let op_str = effect.operation_any().unwrap_or("increase");
                                let op = op_str;
                                let delta = match op {
                                    "increase" => value,
                                    "decrease" => -value,
                                    _ => value,
                                };
                                let heart_colors = effect.heart_colors_any();
                                let colors = heart_colors.iter().map(String::as_str).chain(
                                    heart_colors
                                        .is_empty()
                                        .then_some(crate::ability::util::HEART_ALL_KEY),
                                );
                                let host_id = card_id;
                                for card_id in &target_player.live_card_zone.cards {
                                    for color in colors.clone() {
                                        exp_global_need_heart.push((
                                            *card_id,
                                            color.to_string(),
                                            delta as i16,
                                        ));
                                        exp_global_nh_sources.push(
                                            crate::core::game_modifiers::BonusSource {
                                                source_card_id: host_id,
                                                ability_text: ui_text(&effect.text),
                                                target_card_id: *card_id,
                                                amount: delta,
                                                color: Some(color.to_string()),
                                                kind: ui_kind("need_heart"),
                                            },
                                        );
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
                                            match sub.resource_any().unwrap_or("") {
                                                "blade" => {
                                                    let n = sub
                                                        .resource_icon_count_any()
                                                        .unwrap_or(sub.count.unwrap_or(1)) as i32;
                                                    *exp_blade.entry(card_id).or_insert(0) += i16::try_from(n).unwrap();
                                                    exp_blade_sources.push(crate::core::game_modifiers::BonusSource {
                                                        source_card_id: card_id,
                                                        ability_text: ui_text(&effect.text),
                                                        target_card_id: card_id,
                                                        amount: n,
                                                        color: None,
                                                        kind: ui_kind("blade"),
                                                    });
                                                }
                                                "heart" => {
                                                    let n = i32::from(sub.count.unwrap_or(1));
                                                    let hc_list = sub.heart_colors_any();
                                                    let per_color =
                                                        (n / hc_list.len().max(1) as i32) as i16;
                                                    for hc in hc_list {
                                                        *exp_heart
                                                            .entry(card_id)
                                                            .or_default()
                                                            .entry(hc.clone())
                                                            .or_insert(0) += per_color;
                                                        exp_heart_sources.push(crate::core::game_modifiers::BonusSource {
                                                            source_card_id: card_id,
                                                            ability_text: ui_text(&effect.text),
                                                            target_card_id: card_id,
                                                            amount: per_color as i32,
                                                            color: Some(hc.clone()),
                                                            kind: ui_kind("heart"),
                                                        });
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
        let previous_gained_texts = core::mem::take(&mut self.constant_gained_abilities);
        for (card_id, texts) in previous_gained_texts {
            for text in texts {
                if expected_gained_texts
                    .get(&card_id)
                    .is_some_and(|expected| expected.contains(&text))
                {
                    self.constant_gained_abilities.entry(card_id).or_default().push(text);
                } else {
                    if let Some(gained) = self.gained_abilities.get_mut(&card_id) {
                        gained.retain(|entry| entry != &text);
                        if gained.is_empty() {
                            self.gained_abilities.remove(&card_id);
                        }
                    }
                    log::debug!(
                        "[GAINED_ABILITY] constant expired card={} source_on_stage={} text_chars={}",
                        card_id,
                        entry_positions.contains_key(&card_id),
                        text.chars().count()
                    );
                }
            }
        }
        for (card_id, texts) in expected_gained_texts {
            for text in texts {
                let gained = self.gained_abilities.entry(card_id).or_default();
                if !gained.contains(&text) {
                    log::debug!(
                        "[GAINED_ABILITY] constant registered card={} text_chars={}",
                        card_id,
                        text.chars().count()
                    );
                    gained.push(text.clone());
                    let owned = self.constant_gained_abilities.entry(card_id).or_default();
                    if !owned.contains(&text) {
                        owned.push(text);
                    }
                }
            }
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
        self.grant_under_card_constant_blades(&mut exp_blade, &mut exp_blade_sources);

        self.commit_constant_results(
            exp_blade,
            exp_score,
            exp_heart,
            exp_prohibition,
            exp_global_need_heart,
            exp_blade_sources,
            exp_heart_sources,
            exp_global_nh_sources,
            p1_constant_score_bonus,
            p2_constant_score_bonus,
        );

        // Also recalculate cost modifiers from hand cards (hand-based cost reductions)
        // Pass pre-collected stage effects to avoid re-scanning the stage
        tdbg!("RC:13 COST_MODIFIERS_WITH_ENTRIES");
        let hand_ids = self.collect_constant_hand_effect_ids();
        self.recalculate_constant_cost_modifiers_with_ids(&entries, &hand_ids);
        self.scratch_constant_effect_ids = entries;
        tdbg!("RC:13b COST_MODIFIERS_DONE");

        // Evaluate constant abilities from success live card zone (e.g. Love wing bell)
        tdbg!("RC:14 SUCCESS_ZONE");
        self.evaluate_success_zone_constant_modifiers();
        tdbg!("RC:14b SUCCESS_ZONE_DONE");
        self.refresh_yell_sources();
        self.delayed_gained_effects = exp_delayed_gained_effects;
    }

    /// G8: set each player's yell source from 常時 yell_source_modifier live cards
    /// (e.g. 恋になりたいAQUARIUM "デッキの上から行う代わりにデッキの下から行う").
    /// A live card in the live/success zone whose custom effect is
    /// custom{yell_source_modifier, yell_source:deck_bottom} sets yell_from_bottom.
    fn refresh_yell_sources(&mut self) {
        let db = self.card_database.clone();
        for player in [&mut self.player1, &mut self.player2] {
            player.yell_from_bottom = player
                .live_card_zone
                .cards
                .iter()
                .chain(player.success_live_card_zone.cards.iter())
                .copied()
                .any(|cid| {
                    let Some(card) = db.get_card(cid) else { return false };
                    card.abilities.iter().any(|ar| {
                        let ability = ar.resolve();
                        ability.triggers.as_ref().is_some_and(|trigger| {
                            trigger.contains(crate::triggers::CONSTANT)
                        }) && ability.effect.as_ref().is_some_and(|effect| {
                            effect.action
                                == crate::ability::enums::ActionType::ModifyYellSource
                                && effect.yell_source_any() == Some("deck_bottom")
                        })
                    })
                });
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
        // Per-source attribution for the committed cost bonuses.
        let mut cost_sources: Vec<crate::core::game_modifiers::BonusSource> = Vec::new();
        let mut p1_memberships: HashSet<i16> = HashSet::default();
        let mut p2_memberships: HashSet<i16> = HashSet::default();
        p1_memberships.extend(
            self.player1
                .stage
                .stage
                .iter()
                .chain(self.player1.hand.cards.iter())
                .chain(self.player1.energy_zone.cards.iter())
                .copied(),
        );
        p2_memberships.extend(
            self.player2
                .stage
                .stage
                .iter()
                .chain(self.player2.hand.cards.iter())
                .chain(self.player2.energy_zone.cards.iter())
                .copied(),
        );
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
                let is_ll_bp7_play_cost = effect.operation_any() == Some("set")
                    && effect.value_any() == Some(10)
                    && effect.location_any() == Some("hand")
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
                let owner_in_p1 = p1_memberships.contains(&cid);
                let owner_in_p2 = p2_memberships.contains(&cid);
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
                            let matches = player
                                .stage
                                .stage
                                .iter()
                                .copied()
                                .filter(|&id| id != -1)
                                .filter(|&id| {
                                    crate::ability::util::card_matches_group_str(
                                        card_db, id, group_name,
                                    )
                                })
                                .count();
                            matches.u8_count()
                        } else if Zone::from_str(count_zone) == Some(Zone::UnderMember) {
                            // UnderMember is a 2D structure that zone_cards cannot
                            // represent — flatten every stage slot's under-cards,
                            // honoring the group filter (e.g. pb2-006 桜小路きな子:
                            // 「下にある『Liella!』のメンバーカード1枚につき」).
                            let card_db = &self.card_database;
                            let group_name = effect.group_name();
                            let matches = player
                                .stage
                                .under_cards
                                .iter()
                                .flatten()
                                .copied()
                                .filter(|&id| match group_name {
                                    Some(g) => crate::ability::util::card_matches_group_str(
                                        card_db, id, Some(g),
                                    ),
                                    None => true,
                                })
                                .count();
                            log::debug!(
                                "[COST_MOD_PER_UNIT] under_member group={group_name:?} count={matches}"
                            );
                            u8::try_from(matches).unwrap()
                        } else {
                            crate::ability::util::zone_cards(player, count_zone)
                                .len()
                                .u8_count()
                        };
                        let per_unit_count = effect.per_unit_count_any().unwrap_or(1);
                        let exclude_self = effect.exclude_self_any().unwrap_or(false);
                        let effective = if exclude_self {
                            count.saturating_sub(1)
                        } else {
                            count
                        };
                        value = ((effective / per_unit_count) * u8::try_from(value).unwrap()) as i32;
                        log::debug!("[COST_MOD] cid={} zone={} count={} eff={} per_unit_cnt={} val={} exclude={}",
                            cid, count_zone, count, effective, per_unit_count, value, exclude_self);
                    }
                    log::debug!(
                        "[COST_MOD] cid={} op={:?} val={}",
                        cid,
                        effect.operation_any(),
                        value
                    );

                    let op_str = effect.operation_any().unwrap_or("add");
                    let op = op_str;
                    match op {
                        "add" => {
                            *expected.entry(cid).or_insert(0) += i16::try_from(value).unwrap();
                            cost_sources.push(crate::core::game_modifiers::BonusSource {
                                source_card_id: cid,
                                ability_text: ui_text(&effect.text),
                                target_card_id: cid,
                                amount: value,
                                color: None,
                                kind: ui_kind("cost"),
                            });
                        }
                        "subtract" => {
                            *expected.entry(cid).or_insert(0) -= value as i16;
                            cost_sources.push(crate::core::game_modifiers::BonusSource {
                                source_card_id: cid,
                                ability_text: ui_text(&effect.text),
                                target_card_id: cid,
                                amount: -(value),
                                color: None,
                                kind: ui_kind("cost"),
                            });
                        }
                        "set" => {
                            expected_set.insert(cid, value as i16);
                            cost_sources.push(crate::core::game_modifiers::BonusSource {
                                source_card_id: cid,
                                ability_text: ui_text(&effect.text),
                                target_card_id: cid,
                                amount: value,
                                color: None,
                                kind: ui_kind("cost_set"),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        let old_bonuses = core::mem::take(&mut self.mods.constant_cost_bonuses);
        let old_sets = core::mem::take(&mut self.mods.constant_cost_set_bonuses);
        if old_bonuses != expected {
            for (cid, old) in &old_bonuses {
                self.mods.remove_cost_modifier(*cid, *old);
            }
            for (&cid, &new_val) in &expected {
                self.mods.add_cost_modifier(cid, new_val);
            }
        }
        if old_sets != expected_set {
            for cid in old_sets.keys() {
                self.mods.remove_cost_modifier_set(*cid);
            }
            for (&cid, &new_val) in &expected_set {
                self.mods.set_cost_modifier(cid, new_val);
            }
        }
        self.mods.constant_cost_bonuses = expected;
        self.mods.constant_cost_set_bonuses = expected_set;
        self.mods.constant_cost_sources = cost_sources;
    }

    pub fn set_heart_override(
        &mut self,
        card_id: i16,
        color: crate::card::HeartColor,
        count: u8,
        duration: &str,
    ) {
        let Some(duration) = crate::ability::util::parse_duration(duration) else {
            log::error!(
                "unsupported heart override duration '{}' for card {}",
                duration,
                card_id
            );
            return;
        };
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
            duration,
            created_turn: self.turn_number,
            created_phase: self.current_phase,
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

    /// Reset only the PLAY-scoped baton state (replaced-member identity/cost,
    /// zero-cost flag, last arriving id). Turn-scoped history — arriving ids
    /// and per-player counts — is intentionally preserved across plays within
    /// the turn: 「このターン中にバトンタッチして登場したメンバーが2人以上」
    /// requires two separate baton plays to ACCUMULATE
    /// (PL!HS-bp2-023-L / PL!HS-bp2-025-L). Full clearing remains at the
    /// Active-phase boundary via reset_keyword_tracking.
    pub fn clear_play_scoped_baton_touch(&mut self) {
        self.baton_touch_zero_cost = false;
        self.baton_touch_replaced_member_cost = None;
        self.baton_touch_replaced_member_id = None;
        self.baton_touch_arriving_card_id = None;
    }

    pub fn record_card_movement(&mut self, card_id: i16) {
        self.cards_moved_this_turn.push(card_id);
    }

    /// Typed wrapper: takes canonical ZoneId variants so alias drift ("energy" vs
    /// "energy_zone") dies at the call boundary. Strings are still accepted via
    /// the legacy string overload below but this typed path is preferred for new code.
    pub fn push_movement_event_typed(
        &mut self,
        moved_card_id: i16,
        source_zone: crate::types::ZoneId,
        dest_zone: crate::types::ZoneId,
        cause_card_id: Option<i16>,
        cause_player_id: &str,
        effect_only: bool,
    ) {
        self.push_movement_event(
            moved_card_id,
            source_zone.as_str(),
            dest_zone.as_str(),
            cause_card_id,
            cause_player_id,
            effect_only,
        )
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
        // B2: alias drift detection — Unknown means the caller used a raw string
        // that ZoneId::from_str doesn't recognise; log loudly so watchers don't silently die.
        let src_id = crate::types::ZoneId::from_str(source_zone);
        let dst_id = crate::types::ZoneId::from_str(dest_zone);
        if src_id == crate::types::ZoneId::Unknown || dst_id == crate::types::ZoneId::Unknown {
            log::debug!(
                "[MOVEMENT_ALIAS_DRIFT] unknown zone raw src='{}' dst='{}' -> src={:?} dst={:?}",
                source_zone,
                dest_zone,
                src_id,
                dst_id
            );
        }
        self.movement_event_counter = self.movement_event_counter.wrapping_add(1);
        let event = crate::types::MovementEvent {
            moved_card_id,
            source_zone: src_id,
            dest_zone: dst_id,
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
            log::debug!(
                "[AREA_MOVE_RECORDED] card={} cause={} owner_p1={} owner_p2={}",
                moved_card_id,
                event.cause_player_id,
                self.player1.contains_card(moved_card_id),
                self.player2.contains_card(moved_card_id)
            );
            self.turn_area_movements.push(event.clone());
            self.position_change_occurred_this_turn = true;
            // Opponent-caused trigger arm: 「(対戦相手のカードの効果でも
            // 発動する。)」 — when the CAUSER is not the moved card's owner,
            // arm that owner's marked watchers right here, while the cause
            // is unambiguous and before any batch state is cleared.
            let caused_by_opponent = (self.player1.contains_card(moved_card_id)
                && event.cause_player_id != self.player1.id)
                || (self.player2.contains_card(moved_card_id)
                    && event.cause_player_id != self.player2.id);
            if caused_by_opponent {
                self.fire_opponent_cause_watchers_for_move(
                    moved_card_id,
                    &event.cause_player_id,
                );
            }
        }
        // Track in cards_moved_this_turn for fast O(1) lookups
        self.cards_moved_this_turn.push(moved_card_id);
    }

    pub fn has_card_moved_this_turn(&self, card_id: i16) -> bool {
        self.cards_moved_this_turn.iter().any(|x| x == &card_id)
    }

    /// R1 choke point: the ONLY way effect/choice code may write the
    /// recently-batch scratch views (`recently_moved_cards` /
    /// `recently_moved_from_zone`). Centralizing these writes means the
    /// planned unification (deriving them from the event log, deleting the
    /// shadow fields) touches this method alone instead of a dozen sites.
    pub fn set_recently_moved_batch(
        &mut self,
        cards: SmallVec<[i16; 4]>,
        from_zone: Option<&str>,
    ) {
        self.recently_moved_cards = Some(cards);
        if let Some(z) = from_zone {
            self.recently_moved_from_zone = Some(z.to_string());
        }
    }

    /// R1 choke point companion: clears the recently-batch scratch views.
    pub fn clear_recently_moved_batch(&mut self) {
        self.recently_moved_cards = None;
        self.recently_moved_from_zone = None;
    }

    /// Accumulate cards into the current recently-batch WITHOUT resetting the
    /// from-zone marker (any_number re-prompt pattern: each answered prompt
    /// adds its moves to the same logical batch). Duplicates are significant:
    /// identical copy ids can legitimately move more than once.
    pub fn accumulate_recently_moved(&mut self, card_ids: &[i16]) {
        let cards = self.recently_moved_cards.get_or_insert_with(SmallVec::new);
        cards.extend_from_slice(card_ids);
    }

    pub fn clear_card_movement_tracking(&mut self) {
        self.cards_moved_this_turn.clear();
        self.turn_movements.clear();
        self.cards_appeared_this_turn.clear();
        self.turn_area_movements.clear();
        // Opponent-cause watcher dedupe is turn-scoped: a given move can arm
        // a marked watcher once, and the set resets with the movement data.
        self.mods.opp_cause_fired_keys.clear();
    }

    pub fn remove_revealed_card(&mut self, card_id: i16) {
        if let Some(index) = self.revealed_cards.iter().position(|id| *id == card_id) {
            self.revealed_cards.remove(index);
            if index < self.revealed_card_meta.len() {
                self.revealed_card_meta.remove(index);
            }
        }
    }

    pub fn remove_revealed_cost_card(&mut self, card_id: i16) {
        if let Some(index) = self
            .revealed_cost_cards
            .iter()
            .position(|id| *id == card_id)
        {
            self.revealed_cost_cards.remove(index);
            if index < self.revealed_cost_card_meta.len() {
                self.revealed_cost_card_meta.remove(index);
            }
        }
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
        if let Some(owned) = self.constant_gained_abilities.get_mut(&card_id) {
            owned.retain(|text| text != &ability_type);
            if owned.is_empty() {
                self.constant_gained_abilities.remove(&card_id);
            }
        }
        let list = self.gained_abilities.entry(card_id).or_default();
        // Idempotent: recalculate_constants runs on every state change and calls
        // this for the same constant gain_ability repeatedly — don't accumulate
        // duplicate entries (which previously multiplied bonus_triggers badges).
        if !list.contains(&ability_type) {
            log::debug!(
                "[GAINED_ABILITY] registered card={} text_chars={}",
                card_id,
                ability_type.chars().count()
            );
            list.push(ability_type);
        }
    }

    pub fn clear_gained_abilities_for_card(&mut self, card_id: i16) {
        self.constant_gained_abilities.remove(&card_id);
        self.gained_abilities.remove(&card_id);
        self.gained_card_abilities.remove(&card_id);
        self.gained_ability_sources.remove(&card_id);
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

    /// Restore performance-time need_heart_modifiers that were cleared by
    /// evaluate_success_zone_constant_abilities. This preserves modifications
    /// from live_start triggers and other non-constant sources, ensuring
    /// should_trigger_live_success uses the correct requirements.
    ///
    /// Single source of truth shared by the victory-determination flow
    /// (turn/live.rs) and live-success triggering (turn/triggers.rs).
    pub fn restore_performance_need_heart_modifiers(&mut self) {
        
        // IMPORTANT: deduplicate (cid,color) pairs — the same global modifier
        // may appear in multiple players' snapshots, causing double-counting.
        let mut restored: HashSet<(i16, crate::card::HeartColor)> = HashSet::default();
        for snap in &self.performance_snapshots {
            for &(cid, color, ref entry) in &snap.performance_need_heart_modifiers {
                if !restored.insert((cid, color)) {
                    continue;
                }
                let target = self
                    .mods
                    .need_heart_modifiers
                    .entry(cid)
                    .or_default()
                    .entry(color)
                    .or_default();
                if entry.set != 0 && target.set == 0 {
                    target.set = entry.set;
                }
                target.additive += entry.additive;
            }
        }
    }

    /// Evaluate constant abilities on success zone cards for tracked bonuses
    /// (blade, heart, score). Does NOT touch need_heart_modifiers.
    /// Called from recalculate_constants on every state change, and from
    /// evaluate_success_zone_constant_abilities during the live flow.
    pub fn evaluate_success_zone_constant_modifiers(&mut self) {
        use crate::ability::condition::ConditionContext;

        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::trace!(
                "[CONSTANT_RECALC] success-zone cards: p1={:?} p2={:?}",
                self.player1.success_live_card_zone.cards,
                self.player2.success_live_card_zone.cards
            );
        }

        // ── Clear previously-applied success zone bonuses ──
        let old_sz_blade = core::mem::take(&mut self.mods.success_zone_blade_bonuses);
        for (cid, val) in &old_sz_blade {
            self.mods.remove_blade_modifier(*cid, *val);
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
            self.mods.remove_score_modifier(*cid, *val);
        }
        // Source attribution is rebuilt from scratch alongside the bonuses.
        self.mods.success_zone_blade_sources.clear();
        self.mods.success_zone_heart_sources.clear();
        self.mods.success_zone_score_sources.clear();

        // Track non-stackable effects locally so they are reset each evaluation
        let mut local_non_stackable: HashSet<String> = HashSet::default();

        // Collect all (cid, player_index, effect) pairs upfront to avoid borrow conflicts
        let mut entries: Vec<(i16, usize, crate::card::AbilityEffect)> = Vec::new();
        for (player_idx, zone_cards) in [
            (0usize, &self.player1.success_live_card_zone.cards),
            (1, &self.player2.success_live_card_zone.cards),
        ] {
            for cid in zone_cards {
                let card = match self.card_database.get_card(*cid) {
                    Some(c) => c,
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
            let cond_met = effect
                .condition
                .as_ref()
                .is_none_or(|c| ctx.evaluate_condition(c));
            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                log::debug!("[CONSTANT_CONDITION] zone=success source={} owner={} action={} passes={}",
                    cid, self_player.map(|p| p.id.as_str()).unwrap_or("unknown"), effect.action, cond_met);
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
        if log::log_enabled!(log::Level::Debug) {
            if old_sz_score != self.mods.success_zone_score_bonuses {
                log::debug!("[CONSTANT_SCORE] success-zone bonuses: {:?} -> {:?}",
                    old_sz_score, self.mods.success_zone_score_bonuses);
            }
            if old_sz_blade != self.mods.success_zone_blade_bonuses
                || old_sz_heart != self.mods.success_zone_heart_bonuses
            {
                log::debug!("[CONSTANT_RESOURCE] success-zone blades: {:?} -> {:?}; hearts: {:?} -> {:?}",
                    old_sz_blade, self.mods.success_zone_blade_bonuses,
                    old_sz_heart, self.mods.success_zone_heart_bonuses);
            }
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
                    log::trace!(
                        "[CONSTANT_RESOURCE] source={} resource={} amount={} target={} position={:?} stage={:?}",
                        cid,
                        resource,
                        amount,
                        effect.target_name(),
                        effect.position_any(),
                        player.stage.stage
                    );
                }

                let candidates: Vec<i16> = player
                    .stage
                    .stage
                    .iter()
                    .enumerate()
                    .filter(|&(_, &idx)| idx != -1)
                    .filter(|&(pos, _)| {
                        if let Some(pos_req) = effect.position_any() {
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
                        if let Some(groups) = effect.group_names_any() {
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
                    "blade" => {
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
                            self.mods.success_zone_blade_sources.push(
                                crate::core::game_modifiers::BonusSource {
                                    source_card_id: cid,
                                    ability_text: ui_text(&effect.text),
                                    target_card_id: target_id,
                                    amount,
                                    color: None,
                                    kind: ui_kind("blade"),
                                },
                            );
                        }
                    }
                    "heart" => {
                        let heart_colors = effect.heart_colors_any();
                        let per_color = (amount / heart_colors.len().max(1) as i32) as i16;
                        let colors = heart_colors
                            .iter()
                            .map(String::as_str)
                            .chain(heart_colors.is_empty().then_some("heart01"));
                        for &target_id in &candidates {
                            for color_str in colors.clone() {
                                let hc = crate::card::parse_heart_color(color_str);
                                self.mods.add_heart_modifier(target_id, hc, per_color);
                                *self
                                    .mods
                                    .success_zone_heart_bonuses
                                    .entry(target_id)
                                    .or_default()
                                    .entry(color_str.to_string())
                                    .or_insert(0) += per_color;
                                self.mods.success_zone_heart_sources.push(
                                    crate::core::game_modifiers::BonusSource {
                                        source_card_id: cid,
                                        ability_text: ui_text(&effect.text),
                                        target_card_id: target_id,
                                        amount: per_color as i32,
                                        color: Some(color_str.to_string()),
                                        kind: ui_kind("heart"),
                                    },
                                );
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
                // value_or_count is u8-bounded, so i16 storage never truncates here.
                let value = i16::from(effect.value_or_count(1));
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
                            self.mods.set_score_modifier(target_id, value);
                            self.mods
                                .success_zone_score_bonuses
                                .insert(target_id, value);
                            self.mods.success_zone_score_sources.push(
                                crate::core::game_modifiers::BonusSource {
                                    source_card_id: cid,
                                    ability_text: ui_text(&effect.text),
                                    target_card_id: target_id,
                                    amount: value as i32,
                                    color: None,
                                    kind: ui_kind("score_set"),
                                },
                            );
                        }
                        _ => {
                            self.mods.add_score_modifier(target_id, value);
                            *self
                                .mods
                                .success_zone_score_bonuses
                                .entry(target_id)
                                .or_insert(0) += value;
                            self.mods.success_zone_score_sources.push(
                                crate::core::game_modifiers::BonusSource {
                                    source_card_id: cid,
                                    ability_text: ui_text(&effect.text),
                                    target_card_id: target_id,
                                    amount: value as i32,
                                    color: None,
                                    kind: ui_kind("score"),
                                },
                            );
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
