use crate::core::constants::CountCast;
use super::{GameState, RevealedCardMeta};
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
    if crate::core::runtime_flags::logging_enabled() {
        s.to_string()
    } else {
        String::new()
    }
}

/// UI-only per-source attribution accumulator. Sits beside [`ui_text`], which
/// already made the *strings* free when logs are off; this makes the
/// *containers* free too.
///
/// Every `*_sources` field on `GameModifiers` is read only by `game/display.rs`
/// (plus one display-only push in `turn/phases.rs`) — never by `ability/`,
/// `core/`, `turn/` or `bot/`. They exist so the UI can show WHERE a bonus came
/// from. But they were rebuilt on every `recalculate_constants`, which runs
/// twice per action: a `Vec` allocation plus growth reallocs each, for data no
/// engine code reads. `runtime_flags::action_display_enabled()` is already
/// consulted in 16 places — all in `game/game_setup.rs`; the layers that do the
/// re-derivation never asked.
///
/// A no-op `push` rather than nine `if` guards, so every call site reads exactly
/// as it did before. Off means never allocates, and `into_vec` yields empty.
///
/// **Do not move a reader for one of these fields into `core/` or `ability/`.**
/// That makes the gate unsound and the compiler will not catch it.
enum DisplaySources {
    On(Vec<crate::core::game_modifiers::BonusSource>),
    Off,
}

impl DisplaySources {
    #[inline]
    fn new() -> Self {
        if crate::core::runtime_flags::action_display_enabled() {
            DisplaySources::On(Vec::new())
        } else {
            DisplaySources::Off
        }
    }

    #[inline]
    fn push(&mut self, s: crate::core::game_modifiers::BonusSource) {
        if let DisplaySources::On(v) = self {
            v.push(s);
        }
    }

    #[inline]
    fn into_vec(self) -> Vec<crate::core::game_modifiers::BonusSource> {
        match self {
            DisplaySources::On(v) => v,
            DisplaySources::Off => Vec::new(),
        }
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
            crate::ability::util::activation_position_index(requirement) == Some(usize::from(s))
    })
}

/// Every accumulator the 常時 registration pass builds.
///
/// These were a dozen separate locals threaded through one ~550-line loop, which is what
/// blocked extracting the accumulation half of `recalculate_constants`: a helper needs ten
/// `&mut` arguments, and ten `&mut` arguments plus `&mut self` do not borrow-check. One
/// local struct keeps `&mut self` and `&mut acc` disjoint, so the accumulation can become
/// a real method taking both.
///
/// Grouping only — no behaviour change. Three of the buffers come off `self`'s scratch
/// pool so the steady state still allocates nothing; the other four never had a scratch
/// buffer and still allocate only when first pushed, exactly as the locals did. The
/// round-trip deliberately lives in `take_from` rather than a field initialiser because
/// the scratch fields are `mem::take`-shaped and must come back even on an early return.
struct ConstantAccum {
    exp_blade: HashMap<i16, i16>,
    exp_score: HashMap<i16, i16>,
    exp_heart: HashMap<i16, HashMap<String, i16>>,
    exp_prohibition: Vec<String>,
    exp_delayed_gained_effects: SmallVec<[(i16, crate::card::AbilityEffect); 2]>,
    expected_gained_texts: HashMap<i16, Vec<String>>,
    exp_global_need_heart: Vec<(i16, String, i16)>,
    exp_blade_sources: DisplaySources,
    exp_heart_sources: DisplaySources,
    exp_global_nh_sources: DisplaySources,
    p1_constant_score_bonus: i16,
    p2_constant_score_bonus: i16,
    jyouji_statuses: Vec<crate::types::ConstantAbilityStatus>,
}

impl ConstantAccum {
    /// Take every accumulator off the shared scratch pool, cleared.
    fn take_from(gs: &mut GameState) -> Self {
        gs.constant_cannot_activate_members.clear();
        Self {
            exp_blade: core::mem::take(&mut gs.scratch_exp_blade),
            exp_score: core::mem::take(&mut gs.scratch_exp_score),
            exp_heart: core::mem::take(&mut gs.scratch_exp_heart),
            // No scratch buffer exists for these; they allocate only when first pushed,
            // exactly as the locals they replace did.
            exp_prohibition: Vec::new(),
            exp_delayed_gained_effects: SmallVec::new(),
            expected_gained_texts: HashMap::default(),
            exp_global_need_heart: Vec::new(),
            exp_blade_sources: DisplaySources::new(),
            exp_heart_sources: DisplaySources::new(),
            exp_global_nh_sources: DisplaySources::new(),
            p1_constant_score_bonus: 0,
            p2_constant_score_bonus: 0,
            jyouji_statuses: Vec::new(),
        }
        .cleared()
    }

    /// Empty every buffer. The three scratch-backed maps come off the pool with stale
    /// contents — the locals they replace cleared them immediately after `mem::take`, and
    /// that has to keep happening or a previous game's constants leak into this one.
    fn cleared(mut self) -> Self {
        self.exp_blade.clear();
        self.exp_score.clear();
        self.exp_heart.clear();
        self
    }
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
        exp_blade_sources: &mut DisplaySources,
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
                    amount: count,
                    color: None,
                    kind: ui_kind("blade"),
                });
            }
        }
    }

    /// How many cards `counted` is worth when a card counted by `counting_card`'s
    /// effect is counted — normally 1.
    ///
    /// PL!-pb2-041-L 春情ロマンティック: 「自分の『lily white』のカードの効果によって、
    /// 自分の成功ライブカード置き場にあるカードの枚数を数えるとき、このカードは2枚と
    /// して扱う」. The weight applies only to effects in a listed group, and only while
    /// the rule's own condition holds (the card is in the success live zone — which,
    /// when counting that zone, it is by definition).
    pub fn count_as_multiple_weight(&self, counted: i16, counting_card: Option<i16>) -> u32 {
        // "Which card is counting" is the card whose ability is running. Read it from
        // the ability queue first: `activating_card` is frequently unset while a
        // condition is evaluated, and the count belongs to the effect, not to
        // whatever card happens to be marked active.
        let counting_card = counting_card
            .filter(|&c| c != -1)
            .or_else(|| {
                self.ability_queue
                    .current_entry()
                    .and_then(|e| e.card_id)
                    .filter(|&c| c != -1)
            })
            .or(self.activating_card)
            .filter(|&c| c != -1);
        let Some(counting) = counting_card else {
            return 1;
        };
        let card = match self.card_database.get_card(counted) {
            Some(c) => c,
            None => return 1,
        };
        for ar in &card.abilities {
            let ability = ar.resolve();
            if !ability
                .triggers
                .as_ref()
                .is_some_and(|t| t.contains(crate::triggers::CONSTANT))
            {
                continue;
            }
            let Some(effect) = ability.effect.as_ref() else {
                continue;
            };
            if effect.action != crate::ability::enums::ActionType::CountAsMultiple {
                continue;
            }
            // The rule's own condition — for this card, "is in the success live zone".
            let cond_met = {
                use crate::ability::condition::ConditionContext;
                let owner = self.owner_of_card(counted);
                let ctx = ConditionContext::new_with_self(self, owner);
                effect
                    .condition
                    .as_ref()
                    .is_none_or(|c| ctx.evaluate_condition(c))
            };
            if !cond_met {
                continue;
            }
            // `counted_by` is the list of groups whose effects SEE the weight. An
            // empty list means any effect sees it. Borrowed, not cloned: this runs
            // once per card in the zone on the count path, and the deep copy
            // (a Vec plus one String per group) was pure waste — `effect` borrows
            // `card_database`, not `self`, so the borrow lives for the whole block.
            let seen_by: &[String] = match effect.counted_by_any() {
                Some(v) => v.as_slice(),
                None => &[],
            };
            if !seen_by.is_empty() {
                let ok = seen_by
                    .iter()
                    .any(|g| crate::ability::util::card_matches_group_str(
                        &self.card_database,
                        counting,
                        Some(g.as_str()),
                    ));
                if !ok {
                    continue;
                }
            }
            return u32::from(effect.count_as_any().unwrap_or(2).max(1));
        }
        1
    }

    /// Count of a zone, honouring any `count_as_multiple` weights on the cards in it.
    ///
    /// `counting_card` is the card whose effect is asking — the weight only applies to
    /// the groups that card belongs to. `group_names` filters which cards are counted
    /// at all, exactly as the unweighted path does.
    pub fn weighted_zone_count(
        &self,
        cards: &[i16],
        card_type: Option<&str>,
        group_names: Option<&[String]>,
        counting_card: Option<i16>,
    ) -> u32 {
        let card_db = &self.card_database;
        // An empty card_type means "any", not "matches the empty type" — passing
        // Some("") through would match nothing and silently count zero cards.
        let ct = card_type.filter(|c| !c.is_empty());
        cards
            .iter()
            .filter(|&&cid| {
                if cid == -1 {
                    return false;
                }
                if let Some(ct) = ct {
                    if !crate::ability::util::card_matches_type(card_db, cid, Some(ct)) {
                        return false;
                    }
                }
                if let Some(groups) = group_names {
                    if !groups.is_empty()
                        && !groups.iter().any(|g| {
                            crate::ability::util::card_matches_group_str(card_db, cid, Some(g))
                        })
                    {
                        return false;
                    }
                }
                true
            })
            .map(|&cid| self.count_as_multiple_weight(cid, counting_card))
            .sum()
    }

    /// Does an on-stage `replacement_appear_wait` rule hold `incoming` in wait as it
    /// appears in `area_idx`?
    ///
    /// PL!-pb2-002-PP 絢瀬絵里 — 「このメンバーの正面のエリアには、元々持つハートの数が
    /// 4つ以下のメンバーは、ウェイト状態で登場する」. It is a REPLACEMENT rule, so it is
    /// resolved once here at placement rather than in the constant scan: a constant
    /// would re-apply on every recalculation and the member could never leave wait.
    ///
    /// 「正面のエリア」 is read as the CENTRE area for every stage slot, which is the
    /// reading recorded in docs/PBLL02_ABILITY_REVIEW.md — the card does not say
    /// which area 正面 is for a member standing in a side slot, and this is the
    /// conservative reading of the printed text.
    ///
    /// The rule applies to ANY member appearing there, on either side, so both stages
    /// are scanned.
    pub fn replacement_appear_wait_applies(&self, incoming: i16, area_idx: usize) -> bool {
        let Some(card) = self.card_database.get_card(incoming) else {
            return false;
        };
        if !card.is_member() {
            return false;
        }
        let printed_hearts: u8 = card
            .base_heart
            .as_ref()
            .map(|bh| bh.hearts.values().fold(0u8, |a, n| a.saturating_add(*n)))
            .unwrap_or(0);
        // 正面 resolves to the centre area for every stage slot. See the doc comment.
        const FRONT_SLOT: usize = 1;
        let _ = FRONT_SLOT;

        for player in [&self.player1, &self.player2] {
            for &stage_id in player.stage.stage.iter() {
                if stage_id == -1 {
                    continue;
                }
                // The card being placed cannot be its own host. Without this, the
                // host's own rule matched her on the very turn she arrived (she is on
                // the stage by the time the check runs) and she entered in wait —
                // which then disabled her rule for every other member afterwards.
                // A member is never in front of itself.
                if stage_id == incoming {
                    continue;
                }
                let Some(host) = self.card_database.get_card(stage_id) else {
                    continue;
                };
                for ar in &host.abilities {
                    let ability = ar.resolve();
                    if !ability
                        .triggers
                        .as_ref()
                        .is_some_and(|t| t.contains(crate::triggers::CONSTANT))
                    {
                        continue;
                    }
                    let Some(effect) = ability.effect.as_ref() else {
                        continue;
                    };
                    if effect.action
                        != crate::ability::enums::ActionType::ReplacementAppearWait
                    {
                        continue;
                    }
                    // Host must still be active: a waited host's rule does not apply.
                    if self.mods.get_orientation_modifier(stage_id) == Some("wait") {
                        continue;
                    }
                    if let Some(pos) = effect.position_any() {
                        let pos_str = pos.get_position().unwrap_or("?");
                        let Some(slot) = crate::ability::util::stage_position_index(pos_str)
                        else {
                            continue;
                        };
                        // The rule governs the AREA the incoming member is entering.
                        // Comparing the host's own position against the host's own
                        // position is always true, so without comparing it to
                        // `area_idx` the rule applied to every area.
                        if slot != area_idx {
                            continue;
                        }
                    }
                    if let Some(ct) = effect.card_type_any() {
                        if *ct != crate::card::CardType::Member {
                            continue;
                        }
                    }
                    let limit = effect.heart_total_limit_any();
                    if let Some(limit) = limit {
                        let op = effect.heart_total_operator_any().unwrap_or("<=");
                        if !crate::ability::util::compare_counts(
                            Some(op),
                            printed_hearts,
                            limit,
                        ) {
                            continue;
                        }
                    } else if effect.blade_limit_any().is_some() {
                        // A blade-bounded variant is not modelled; do not guess.
                        continue;
                    }
                    return true;
                }
            }
        }
        false
    }

    fn collect_reusable_constant_stage_effect_ids(&self, ids: &mut Vec<(i16, usize)>) {        for cid in self
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
                    // `peek()` first: this is a read-only predicate over ~6 stage
                    // cards x ~3 abilities on every `recalculate_constants`, and
                    // `resolve()` would cost an `Arc::clone` (an atomic RMW on a
                    // line shared across all worker threads) per ability. The
                    // `resolve()` arm is the fallback for a not-yet-decoded slot
                    // on std, and the only path on `no_std` (no slot cache).
                    //
                    // Both arms compute the boolean rather than binding a
                    // reference: the `Arc` from `resolve()` is a temporary that
                    // dies at the end of the `let`, so `&*ar.resolve()` would
                    // dangle.
                    let constant_ability = match ar.peek() {
                        Some(a) => {
                            Self::ability_matches_trigger(
                                a,
                                &crate::game_state::AbilityTrigger::Constant,
                            ) && a.effect.is_some()
                        }
                        None => {
                            let a = ar.resolve();
                            Self::ability_matches_trigger(
                                &a,
                                &crate::game_state::AbilityTrigger::Constant,
                            ) && a.effect.is_some()
                        }
                    };
                    if constant_ability {
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

    /// The position + condition gate for one 常時 entry during registration.
    ///
    /// Split out of `recalculate_constants`, which was 782 lines and doing four separable
    /// things. This half only READS state, so it stands alone; the accumulation half is not
    /// extracted because it needs `activating_card` still set for target resolution and
    /// `resolve_dynamic_count`, which is why the caller saves and restores that field
    /// around BOTH halves.
    fn constant_entry_gate_met(
        &mut self,
        card_id: i16,
        ability_idx: usize,
        effect: &crate::card::AbilityEffect,
        entry_positions: &HashMap<i16, Option<u8>>,
    ) -> bool {
        let self_player = if self.player1.stage.stage.contains(&card_id) {
            Some(&self.player1)
        } else {
            Some(&self.player2)
        };
        let mut ctx = crate::ability::condition::ConditionContext::new_with_self(self, self_player);
        // Constant abilities register regardless of the current phase — the phase gate only
        // matters at trigger evaluation time, not during constant registration.
        ctx.skip_phase_gate = true;

        // `activation_position` ("左サイド,右サイド" etc.) is the AUTHORITATIVE gate when
        // present — 鬼塚夏美 SP-bp7-009 prints 「（この能力は左サイド/右サイドエリアにいる
        // 場合のみ発動する）」 and the parser encodes BOTH slots there. Checking only
        // `position` (which may name a single slot) broke the second listed side: moving
        // her right never re-granted heart02.
        let card_pos = entry_positions.get(&card_id).copied().flatten();
        let pos_ok = if let Some(act) = effect.activation_position_any() {
            // Any one of the listed sides satisfies the condition.
            act.split(',')
                .map(|p| p.trim())
                .any(|p| slot_named(p, card_pos))
        } else {
            // Otherwise the targeting rule, naming at most one slot. "front" and an
            // unstated position are not slot constraints.
            match effect.position_any().and_then(|p| p.get_position()) {
                Some(pos) if pos != "front" => slot_named(pos, card_pos),
                _ => true,
            }
        };
        if !pos_ok {
            return false;
        }
        let cond_met = effect
            .condition
            .as_ref()
            .is_none_or(|c| ctx.evaluate_condition(c));
        log::debug!(
            "[CONSTANT_CONDITION] zone=stage source={} ability={} action={} passes={}",
            card_id,
            ability_idx,
            effect.action,
            cond_met
        );
        cond_met
    }

    /// Apply the accumulated constant-effect scratch results onto the live
    /// modifier state: clear old constant-derived bonuses and re-apply the new
    /// ones (blade, score, per-player score bonus, heart, prohibition, and
    /// global need_heart).
    #[allow(clippy::too_many_arguments)]
fn commit_constant_results(
        &mut self,
        exp_blade: HashMap<i16, i16>,
        exp_score: HashMap<i16, i16>,
        exp_heart: HashMap<i16, HashMap<String, i16>>,
        exp_prohibition: Vec<String>,
        exp_global_need_heart: Vec<(i16, String, i16)>,
        exp_blade_sources: DisplaySources,
        exp_heart_sources: DisplaySources,
        exp_global_nh_sources: DisplaySources,
        p1_constant_score_bonus: i16,
        p2_constant_score_bonus: i16,
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
        self.mods.constant_blade_sources = exp_blade_sources.into_vec();
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
            && (self.mods.p1_constant_total_score_bonus != p1_constant_score_bonus
                || self.mods.p2_constant_total_score_bonus != p2_constant_score_bonus)
        {
            log::debug!(
                "[CONSTANT_SCORE] live-total bonus: p1 {} -> {}, p2 {} -> {}",
                self.mods.p1_constant_total_score_bonus, p1_constant_score_bonus,
                self.mods.p2_constant_total_score_bonus, p2_constant_score_bonus
            );
        }
        // Plain assignment. This read `i16::try_from(p1_constant_score_bonus).unwrap()`
        // on a value that was ALREADY an `i16` into an `i16` field, so the conversion
        // could not fail and the `unwrap` implied a narrowing that never happens.
        self.mods.p1_constant_total_score_bonus = p1_constant_score_bonus;
        self.mods.p2_constant_total_score_bonus = p2_constant_score_bonus;

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
        self.mods.constant_heart_sources = exp_heart_sources.into_vec();

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
        self.mods.constant_need_heart_sources = exp_global_nh_sources.into_vec();
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
    ///
    /// A fast path skipping this pass when no 常時 is derivable was attempted three times
    /// and reverted each time. The approach is RIGHT — the gate has to be built from this
    /// pass's own discovery, because `gain_ability` grants 常時 that never appear in
    /// `card.abilities` — and the failures narrowed as the discovery set narrowed:
    ///
    ///   * a `CardDatabase` index over PRINTED abilities -> 68 failures: it missed gained
    ///     常時 entirely, and the success live card zone;
    ///   * `collect_reusable_constant_stage_effect_ids` (printed stage AND gained in one
    ///     call) + success zone + `collect_constant_hand_effect_ids` -> 5 failures, and
    ///     still 5 after the success zone stopped consulting the index.
    ///
    /// The two survivors are PLAY-COST reductions no collector above names:
    /// `pb2_optional_two_distinct_printemps_wait_costs_two_less` and
    /// `hand_card_cost_reduced_by_hand_count`. Both are consumed by
    /// `recalculate_constant_cost_modifiers_with_ids`, whose only inputs are `stage_ids`
    /// and `hand_ids` — so either a 常時 reaches it by a path bypassing both, or
    /// `collect_constant_hand_effect_ids` does not see a 常時 whose effect merely READS the
    /// hand count. Whoever finishes this starts from those two tests and reads outward
    /// from `recalculate_constant_cost_modifiers_with_ids`, not from the card database.
    /// That mistake was made twice here.
    /// Does any of `cards` carry a 常時 ability?
    ///
    /// **Two O(1) lookups per card, not an ability walk.** The soundness question and the
    /// speed question are separate here:
    ///
    /// * Soundness: a printed-ability index alone is WRONG, because `gain_ability` grants
    ///   常時 that never appear in `card.abilities`. An index-only gate failed 68 tests.
    ///   So this asks the index OR `gained_constant_for`, which covers both — that pairing
    ///   is what makes it sound, and it is why the index is used as a *term* here rather
    ///   than as the answer.
    /// ── No skip gate, and that is now a MEASURED decision rather than a guess ──
    ///
    /// Five attempts, all recorded, all failing on the same root cause until the last
    /// one: **under-counting the sources.** `recalculate_constants` derives from FIVE
    /// independent sources —
    ///
    /// 1. printed 常時 on a stage slot      \
    /// 2. gained 常時 (`gain_ability`)      } -> `entries`, which walks stage SLOTS only
    /// 3. hand-resident cost modifiers       -> `hand_ids`
    /// 4. the live and success zones        -> Part B below
    /// 5. a card UNDER a stage member       -> in no zone collection at all
    ///
    /// — and each omitted source produced failures that named itself. Attempt 2 missed
    /// (4) and cost 17 failures; attempt 4 missed (5) and cost 7; and attempt 5, the
    /// first to check all five exhaustively, passed the whole suite on its second
    /// correction (the flag also has to include (5), or a departing host skips the pass
    /// that removes its blade — `under_liella_host_blade_leaving_removes_blade`).
    ///
    /// Attempt 5 is sound, its gate is free (six emptiness tests on already
    /// materialised slices), and its skip rate is **13.4%**. It measures at **+0.2% —
    /// noise.** So the correct gate is worth nothing, and that is the useful result:
    ///
    /// **this pass is already cheap when there is nothing to do.**
    ///
    /// The obvious estimate said otherwise — 13.4% of ~3.5 us of "skippable work" is
    /// ~2.6% of engine time — but the estimate assumed the empty case still costs 3.5 us.
    /// It does not. `recalculate_constant_cost_modifiers_with_ids` iterates
    /// `stage_ids.chain(hand_ids)`, so on the very inputs the gate tests for emptiness it
    /// is already doing nothing; its 1.52 us average is carried by the 86% of calls that
    /// are NOT empty, which a gate cannot touch. What the gate skips is the clearing and
    /// the bookkeeping, and those are cheap.
    ///
    /// So the fat is only in the NON-empty case, and the only lever that reaches it is
    /// making the per-常時 evaluation cheaper — which is the same work whether or not a
    /// card is on the board. An incremental constant-source count (O(1) and complete by
    /// construction) was the remaining idea and is unnecessary: the gate it would feed is
    /// already worth nothing.
    ///
    /// Measurement note: trust the skip RATE, never gps. It is deterministic and
    /// per-mechanism; gps is not comparable across builds because a faster engine fits
    /// more games into the same warmup window.
    /// Measurement note, and the reason the phase table above is not the whole story:
    /// **each `Timer::start` in this function costs ~0.44 us.** Measured by adding two
    /// more and watching the pass go from 5.53 us to 6.40 us — +0.87 us for two timers.
    ///
    /// That is the same order as most of the phases it is measuring: a pass at ~5-6 us
    /// is roughly one timer. So the per-phase figures are each inflated by about that
    /// amount (`cost_mods` 1.48 -> ~1.04 true, `main_loop` 0.54 -> ~0.10 true), the
    /// residual "unaccounted" gap is largely the timers themselves, and adding
    /// instrumentation to find the remaining cost costs more than what it finds.
    ///
    /// It also explains a pattern worth keeping: eleven separate micro-optimisations of
    /// this pass measured between -5% and +5%, i.e. nothing, while the profile kept
    /// showing a 20% hotspot. They were all being measured through an instrument that
    /// costs ~0.44 us per reading, on a function with ~0.5 us of real work outside them.
    ///
    /// What to trust is the skip RATE and the end-to-end `mean_aps`, both of which are
    /// read outside this function. gps is not comparable across builds because a faster
    /// engine fits more games into the same warmup window.
    #[inline(never)]
    pub fn recalculate_constants(&mut self) {
        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("recalculate_constants");
        #[cfg(feature = "alloc_tracker")]
        let _alloc_trace = crate::alloc_counter::maybe_arm("recalculate_constants");
        tdbg!("RC:0 ENTERED");
        // Measures the PROLOGUE — scratch takes, the `entry_positions` map, and the
        // `mods.constant_*` clears — which the `recalc::collect` timer starts after and so
        // never accounted for. Those three phases summed to 2.21 us of a 4.51 us pass,
        // leaving roughly half the function unmeasured; this closes that gap.
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
        // All accumulators for the 常時 pass, in one struct. These were a dozen
        // separate locals threaded through one ~550-line loop, which is what blocked
        // extracting the accumulation half: any helper needs ten `&mut` arguments.
        // A single local struct keeps `&mut self` and `&mut acc` disjoint.
        let mut acc = ConstantAccum::take_from(self);
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
                // Gate only: position + condition. The accumulation below still needs
                // `activating_card` set (target resolution and `resolve_dynamic_count` read
                // it), so `activating_card` is saved and restored by the CALLER around both
                // halves — which is why this half is extracted and the other is not.
                let cond_met = self.constant_entry_gate_met(
                    card_id,
                    ability_idx,
                    effect,
                    &entry_positions,
                );

                if cond_met {
                        // Record jyouji status for this card (lazily capture
                        // name/owner only now that the condition passed).
                        // Skipped under `headless` — display-only summary data.
                        #[cfg(not(feature = "headless"))]
                        if crate::core::runtime_flags::logging_enabled() {
                            let status_card_name = card_db
                                .get_card(card_id)
                                .map(|c| c.name.to_string())
                                .unwrap_or_default();
                            let status_owner = if self.player1.stage.stage.contains(&card_id) {
                                self.player1.id.clone()
                            } else {
                                self.player2.id.clone()
                            };
                            acc.jyouji_statuses.push(crate::types::ConstantAbilityStatus {
                                card_id,
                                card_name: status_card_name.clone(),
                                owner: status_owner.to_string(),
                                zone: "stage".to_string(),
                                ability_text: ui_text(&effect.text),
                                all_conditions_met: cond_met,
                                conditions: vec![crate::types::ConditionResult {
                                    text: "条件".to_string(),
                                    passed: cond_met,
                                }],
                            });
                        }
                        // A `delayed` (「次のターン」) restriction belongs to the
                        // runtime arm in effects/restriction.rs — log that this
                        // 常時 pass deliberately did NOT accumulate it.
                        if matches!(effect.action, crate::ability::enums::ActionType::Restriction)
                            && effect.delayed_any().unwrap_or(false)
                        {
                            log::debug!(
                                "[CONSTANT_SKIP] delayed restriction rt={:?} — runtime arm owns it",
                                effect.restriction_type_any()
                            );
                        }
                        match effect.action {
                            crate::ability::enums::ActionType::GainResource => {
                                match effect.resource_any().unwrap_or("") {
                                    "blade" => {
                                        // `i16` end to end. This used to be
                                        // computed as an i32 — including a
                                        // `units * base` product — only to be
                                        // narrowed by a `try_from(..).unwrap()`
                                        // a few lines later, which would have
                                        // panicked rather than clamped on a
                                        // count a card could legally produce.
                                        let n: i16 = if let Some(dc) = effect.dynamic_count_any() {
                                            i16::from(self.resolve_dynamic_count(
                                                dc,
                                                &[],
                                                &[],
                                                0,
                                                Some(card_id),
                                            ))
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
                                                1u8
                                            } else {
                                                effect
                                                    .resource_icon_count_any()
                                                    .unwrap_or(effect.count_any().unwrap_or(1))
                                            };
                                            units.saturating_mul(i16::from(base))
                                        } else {
                                            i16::from(
                                                effect
                                                    .resource_icon_count_any()
                                                    .unwrap_or(
                                                        effect.count_any().unwrap_or(1),
                                                    ),
                                            )
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
                                        // `saturating_mul`: a per-unit count
                                        // large enough to overflow should clamp,
                                            // not panic the way the `try_from`
                                            // this replaces did.
                                        let delta = n.saturating_mul(sign_mult);
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
                                                *acc.exp_blade.entry(tid).or_insert(0) += delta;
                                                acc.exp_blade_sources.push(crate::core::game_modifiers::BonusSource {
                                                    source_card_id: card_id,
                                                    ability_text: ui_text(&effect.text),
                                                    target_card_id: tid,
                                                    amount: delta,
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
                                                *acc.exp_blade.entry(mid).or_insert(0) += delta;
                                                acc.exp_blade_sources.push(crate::core::game_modifiers::BonusSource {
                                                    source_card_id: card_id,
                                                    ability_text: ui_text(&effect.text),
                                                    target_card_id: mid,
                                                    amount: delta,
                                                    color: None,
                                                    kind: ui_kind("blade"),
                                                });
                                            }
                                        } else {
                                            *acc.exp_blade.entry(card_id).or_insert(0) += delta;
                                            acc.exp_blade_sources.push(crate::core::game_modifiers::BonusSource {
                                                source_card_id: card_id,
                                                ability_text: ui_text(&effect.text),
                                                target_card_id: card_id,
                                                amount: delta,
                                                color: None,
                                                kind: ui_kind("blade"),
                                            });
                                        }
                                    }
                                    "heart" => {
                                        let n: i16 = if let Some(dc) = effect.dynamic_count_any() {
                                            // Unified dynamic_count resolution (dynamic_count.rs).
                                            // The constant path has no resolver step context, so
                                            // pass empty moved/selected and 0 draw count.
                                            i16::from(self.resolve_dynamic_count(
                                                dc,
                                                &[],
                                                &[],
                                                0,
                                                Some(card_id),
                                            ))
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
                                            i16::from(effect.count_any().unwrap_or(1))
                                        };
                                        if crate::ability::util::is_all_heart_type(effect) {
                                            *acc.exp_heart
                                                .entry(card_id)
                                                .or_default()
                                                .entry(crate::ability::util::HEART_ALL_KEY.to_string())
                                                .or_insert(0) += i16::try_from(n).unwrap();
                                            acc.exp_heart_sources.push(crate::core::game_modifiers::BonusSource {
                                                source_card_id: card_id,
                                                ability_text: ui_text(&effect.text),
                                                target_card_id: card_id,
                                                amount: n,
                                                color: Some(crate::ability::util::HEART_ALL_KEY.to_string()),
                                                kind: ui_kind("heart"),
                                            });
                                        } else {
                                            let hc_list = effect.heart_colors_any();
                                            let per_entry = n / hc_list.len().max(1).max(1) as i16;
                                            for hc in hc_list {
                                                *acc.exp_heart
                                                    .entry(card_id)
                                                    .or_default()
                                                    .entry(hc.clone())
                                                    .or_insert(0) += per_entry;
                                                acc.exp_heart_sources.push(crate::core::game_modifiers::BonusSource {
                                                    source_card_id: card_id,
                                                    ability_text: ui_text(&effect.text),
                                                    target_card_id: card_id,
                                                    amount: per_entry,
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
                                // `i16` throughout: the value is a modifier, and
                                // it feeds the same i16 accumulators as every
                                // other constant score delta. It was widened to
                                // i32 here and narrowed again three lines down.
                                let sv = i16::from(effect.value_any().unwrap_or(0));
                                if sv != 0 {
                                    self.mods.constant_score_sources.push((
                                        card_id,
                                        ui_text(&effect.text),
                                        sv,
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
                                        acc.p1_constant_score_bonus += sv;
                                    } else {
                                        acc.p2_constant_score_bonus += sv;
                                    }
                                } else {
                                    *acc.exp_score.entry(card_id).or_insert(0) += sv;
                                }
                            }
                            // `delayed` restrictions (「次のターン」) are transient: the
                            // runtime arm in effects/restriction.rs stores them
                            // (delayed_prohibition_effects / mods.delayed_cannot_active).
                            // Accumulating them here — the 常時 pass — would turn a
                            // one-turn flag into a standing prohibition
                            // (constant_cannot_activate_members / exp_prohibition).
                            // Today's 常時-only collector means no corpus card reaches
                            // this arm delayed; the guard keeps a future
                            // constant+delayed shape honest (falls to `_ => {}`).
                            crate::ability::enums::ActionType::Restriction
                                if !effect.delayed_any().unwrap_or(false) =>
                            {
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
                                        acc.exp_prohibition.push(format!(
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
                                    *acc.exp_heart
                                        .entry(card_id)
                                        .or_default()
                                        .entry("all".to_string())
                                        .or_insert(0) += 1i16;
                                    acc.exp_heart_sources.push(crate::core::game_modifiers::BonusSource {
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
                                        &mut acc.p1_constant_score_bonus
                                    } else {
                                        &mut acc.p2_constant_score_bonus
                                    };

                                    let texts = acc.expected_gained_texts.entry(card_id).or_default();
                                    if !texts.iter().any(|text| text == gain_text) {
                                        texts.push(ui_text(gain_text));
                                    }

                                    // Use gained_effect if available (structured data from parser)
                                    if let Some(gained) = effect.gained_effect_any() {
                                        let action = gained.action;
                                        if action
== crate::ability::enums::ActionType::ModifyScore
                                    {
                                        let val = i16::from(gained.value_any().unwrap_or(0));
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
                                            acc.exp_delayed_gained_effects
                                                .push((card_id, gained.clone()));
                                        }
                                    } else {
                                        // Fallback: parse value from text (legacy path)
                                        if let Some(val) =
                                            gain_text.split('+').nth(1).and_then(|s| {
                                                s.chars()
                                                    .take_while(|c| c.is_ascii_digit())
                                                    .collect::<String>()
                                                    .parse::<i16>()
                                                    .ok()
                                            })
                                        {
                                            *bonus_target += val;
                                            if val != 0 {
                                                self.mods.constant_score_sources.push((
                                                    card_id,
                                                    ui_text(gain_text),
                                                    val,
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
                                let value = i16::from(effect.value_or_count(1));
                                let op_str = effect.operation_any().unwrap_or("increase");
                                let op = op_str;
                                // `saturating_neg` so a "decrease" of a value
                                // that cannot be negated in i16 clamps rather
                                // than wrapping into a positive delta.
                                let delta = match op {
                                    "increase" => value,
                                    "decrease" => value.saturating_neg(),
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
                                        acc.exp_global_need_heart.push((
                                            *card_id,
                                            color.to_string(),
                                            delta,
                                        ));
                                        acc.exp_global_nh_sources.push(
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
                if let Some(actions) = effect.actions() {
                    // A nested condition needs its own context: the one
                                    // inside `constant_entry_gate_met` is gone by the time
                                    // the accumulation half runs. Same construction as that
                                    // helper — `skip_phase_gate` for the same reason.
                                    let self_player = if self.player1.stage.stage.contains(&card_id)
                                    {
                                        Some(&self.player1)
                                    } else {
                                        Some(&self.player2)
                                    };
                                    let mut ctx = crate::ability::condition::ConditionContext::new_with_self(self, self_player);
                                    ctx.skip_phase_gate = true;
for sub in actions.iter().map(|b| b.as_ref()) {
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
                                                    let n = i16::from(
                                                        sub.resource_icon_count_any()
                                                            .unwrap_or(sub.count.unwrap_or(1)),
                                                    );
                                                    *acc.exp_blade.entry(card_id).or_insert(0) += n;
                                                    acc.exp_blade_sources.push(crate::core::game_modifiers::BonusSource {
                                                        source_card_id: card_id,
                                                        ability_text: ui_text(&effect.text),
                                                        target_card_id: card_id,
                                                        amount: n,
                                                        color: None,
                                                        kind: ui_kind("blade"),
                                                    });
                                                }
                                                "heart" => {
                                                    let n = i16::from(sub.count.unwrap_or(1));
                                                    let hc_list = sub.heart_colors_any();
                                                    // Split the count evenly across the
                                                    // named colours. `i32` for the
                                                    // division only: a colour list
                                                    // can be long enough that the
                                                    // quotient must not itself be
                                                    // the thing that saturates.
                                                    let per_color = crate::constants::saturate_i16(
                                                        n.i32_count() / hc_list.len().max(1).i32_count(),
                                                    );
                                                    for hc in hc_list {
                                                        *acc.exp_heart
                                                            .entry(card_id)
                                                            .or_default()
                                                            .entry(hc.clone())
                                                            .or_insert(0) += per_color;
                                                        acc.exp_heart_sources.push(crate::core::game_modifiers::BonusSource {
                                                            source_card_id: card_id,
                                                            ability_text: ui_text(&effect.text),
                                                            target_card_id: card_id,
                                                            amount: per_color,
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

            // Restore the previous activating_card
            self.activating_card = prev_activating;
        }
        let previous_gained_texts = core::mem::take(&mut self.constant_gained_abilities);
        for (card_id, texts) in previous_gained_texts {
            for text in texts {
                if acc.expected_gained_texts
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
        for (card_id, texts) in acc.expected_gained_texts {
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
        let _jyouji_len = acc.jyouji_statuses.len();
        self.constant_ability_statuses = acc.jyouji_statuses.into();
tdbg!("RC:6 MAIN_LOOP_DONE jyouji={}", _jyouji_len);
        // `Option` + `take` rather than a plain `let _t = Timer::start(..)` binding: a
        // guard dropped at end of scope NESTS, so three of them report as
        // `collect > main_loop > cost_mods` and every row reads as inclusive of the rest.
        // Dropping each one at its own boundary makes the three disjoint, which is the
        // only way the report says anything.
        #[cfg(not(feature = "no_std"))]


        // Under-card constant blade abilities ("常時：このカードが『X』のメンバーの
        // 下に置かれているかぎり、そのメンバーはブレードを得る"). The blade is
        // granted to the HOST member the card is stacked under, not the card itself
        // (which isn't on stage).
        // This is the FIFTH 常時 source and it lives here, outside every other phase
        // timer — it walks `stage.under_cards` directly and is in no zone collection.
        // Four earlier attempts to gate this pass missed it, which is what
        // `under_liella_member_constant_host_blade_*` kept reporting.
        self.grant_under_card_constant_blades(&mut acc.exp_blade, &mut acc.exp_blade_sources);

        // Takes TEN collections by value, so each call both moves ~10 Vec/SmallVec/HashMap
        // headers into `mods` and drops the displaced ones — a deallocation per populated
        // accumulator. That is the other half of what the phase timers did not cover.
        self.commit_constant_results(
            acc.exp_blade,
            acc.exp_score,
            acc.exp_heart,
            acc.exp_prohibition,
            acc.exp_global_need_heart,
            acc.exp_blade_sources,
            acc.exp_heart_sources,
            acc.exp_global_nh_sources,
            acc.p1_constant_score_bonus,
            acc.p2_constant_score_bonus,
        );

        // Also recalculate cost modifiers from hand cards (hand-based cost reductions)
        // Pass pre-collected stage effects to avoid re-scanning the stage
        tdbg!("RC:13 COST_MODIFIERS_WITH_ENTRIES");
        let hand_ids = self.collect_constant_hand_effect_ids();
        self.recalculate_constant_cost_modifiers_with_ids(&entries, &hand_ids);
        self.scratch_constant_effect_ids = entries;
        // Dropped HERE, not at end of scope. A `let _t = Timer::start(..)` guard nests, so
        // leaving this one alive would fold `recalc::success_zone` into `recalc::cost_mods`
        // and report it as 2.46 us when it is ~2.13. Every phase timer in this function is
        // dropped at its own boundary for that reason.
        // Evaluate constant abilities from success live card zone (e.g. Love wing bell)
        tdbg!("RC:14 SUCCESS_ZONE");
        self.evaluate_success_zone_constant_modifiers();
        tdbg!("RC:14b SUCCESS_ZONE_DONE");
        self.refresh_yell_sources();
        self.delayed_gained_effects = acc.exp_delayed_gained_effects;
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
                        // Plain `HashMap::default()`, deliberately. Ping-ponging these two maps through
        // scratch buffers (reusing last call's `expected` as this call's scratch, and
        // recycling the displaced `old_*` maps) measured at +0.7% — noise — because the
        // maps are EMPTY on almost every call: `HashMap::default()` does not allocate
        // until first push, and a board rarely has a cost-modifying 常時. The phase's
        // 1.52 us is per-ability work in the loop below, not memory.
        let mut expected: HashMap<i16, i16> = HashMap::default();
        let mut expected_set: HashMap<i16, i16> = HashMap::default();
        // Per-source attribution for the committed cost bonuses.
        let mut cost_sources: Vec<crate::core::game_modifiers::BonusSource> = Vec::new();
        // Whether `cid` is in this player's stage, hand or energy zone.
        //
        // This used to be a `HashSet<i16>` built per `recalculate_constants`
        // call and consulted once per ability. Those zones hold about 22 ids
        // between them, so the set cost two heap allocations and ~44 hashes to
        // answer one question a linear scan answers in ~44 comparisons and no
        // allocation — and this function runs a couple of times per action.
        fn owns(p: &crate::player::Player, id: i16) -> bool {
            p.stage.stage.contains(&id)
                || p.hand.cards.contains(&id)
                || p.energy_zone.cards.contains(&id)
        }
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
                let owner_in_p1 = owns(&self.player1, cid);
                let owner_in_p2 = owns(&self.player2, cid);
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
                    // `i16`: a cost modifier, the same width `ModifierEntry`
                    // stores. The per-unit branch below multiplies by a unit
                    // COUNT, and that product is clamped here rather than
                    // wrapping.
                    let mut value = i16::from(effect.value_any().unwrap_or(0));

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
                        // Saturating: a per-unit cost modifier is a count
                        // product, and a large count should clamp rather than
                        // wrap into a small (or negative) cost.
                        value = value.saturating_mul((effective / per_unit_count) as i16);
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
                            *expected.entry(cid).or_insert(0) += value;
                            // Display-only attribution (constant_cost_sources
                            // is read solely by game/display.rs).
                            if crate::core::runtime_flags::action_display_enabled() {
                                cost_sources.push(crate::core::game_modifiers::BonusSource {
                                    source_card_id: cid,
                                    ability_text: ui_text(&effect.text),
                                    target_card_id: cid,
                                    amount: value,
                                    color: None,
                                    kind: ui_kind("cost"),
                                });
                            }
                        }
                        "subtract" => {
                            *expected.entry(cid).or_insert(0) -=
                                crate::constants::saturate_i16(i32::from(value));
                            if crate::core::runtime_flags::action_display_enabled() {
                                cost_sources.push(crate::core::game_modifiers::BonusSource {
                                    source_card_id: cid,
                                    ability_text: ui_text(&effect.text),
                                    target_card_id: cid,
                                    amount: value.saturating_neg(),
                                    color: None,
                                    kind: ui_kind("cost"),
                                });
                            }
                        }
                        "set" => {
                            expected_set.insert(cid, value);
                            if crate::core::runtime_flags::action_display_enabled() {
                                cost_sources.push(crate::core::game_modifiers::BonusSource {
                                    source_card_id: cid,
                                    ability_text: ui_text(&effect.text),
                                    target_card_id: cid,
                                    amount: value,
                                    color: None,
                                    kind: ui_kind("cost_set"),
                                });
                            }
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

    /// Record a stage-area→stage-area position change: the cards that swapped
    /// slots, the resulting events, and the movement bookkeeping.
    ///
    /// Three branches of `execute_position_change` (an explicit
    /// `source_position`, a named `target_member`, and the `this_member` case)
    /// each wrote this out in full. The sequencing is load-bearing, which is
    /// the reason it is one function rather than three copies:
    ///
    /// 1. the two `PositionChangeEvent`s are pushed BEFORE `push_movement_event`,
    ///    because that call consumes the recently-moved snapshot and arms the
    ///    opponent-caused watcher trigger;
    /// 2. the source card is pushed unconditionally while the target is
    ///    guarded, because the source slot is checked non-empty by the caller
    ///    while the target slot may legitimately be empty.
    ///
    /// `source_id` / `target_id` are `-1` when that slot holds no member. Only
    /// real cards are recorded as moved or given events — the `source_position`
    /// branch used to push its target id unguarded, so an empty target slot
    /// recorded a literal `-1` into `cards_moved_this_turn`.
    pub fn record_stage_position_swap(
        &mut self,
        source_id: i16,
        source_index: u8,
        target_id: i16,
        target_index: u8,
    ) {
        if target_id != -1 {
            self.record_card_movement(target_id);
        }
        if source_id != -1 {
            self.record_card_movement(source_id);
        }
        let mover_pid = self
            .ability_queue
            .current_entry()
            .map(|e| e.player_id.clone())
            .unwrap_or_default();
        if source_id != -1 {
            self.position_change_events
                .push(crate::types::PositionChangeEvent {
                    moved_card_id: source_id,
                    old_position: source_index,
                    new_position: target_index,
                    cause_card_id: self.activating_card,
                    cause_player_id: mover_pid.as_str().into(),
                    effect_only: true,
                });
        }
        if target_id != -1 {
            self.position_change_events
                .push(crate::types::PositionChangeEvent {
                    moved_card_id: target_id,
                    old_position: target_index,
                    new_position: source_index,
                    cause_card_id: self.activating_card,
                    cause_player_id: mover_pid.as_str().into(),
                    effect_only: true,
                });
        }
        self.push_movement_event(
            source_id,
            "stage",
            "stage",
            self.activating_card,
            &mover_pid,
            true,
        );
        if target_id != -1 {
            self.push_movement_event(
                target_id,
                "stage",
                "stage",
                self.activating_card,
                &mover_pid,
                true,
            );
        }
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
        // `cause_player_id` is the larger per-movement allocation, not
        // `recently_moved_from_zone`: it is a `String`, and the event built here is
        // cloned twice below (batch_movements, turn_movements) plus once more for
        // turn_area_movements, so each movement allocates and frees that player id
        // three or four times. The clones would be free if the field were the
        // codebase's existing `compat::ArcStr`, which is refcounted - the `==`
        // comparisons and the two `.as_str()` readers already work unchanged.
        //
        // Not done here: `cause_player_id` appears at 29 sites across game_state,
        // ability and core, so this is a mechanical type change rather than a local
        // fix, and it has no correctness impact - only allocation count. Recorded
        // so it is not rediscovered as if it were new.
        let event = crate::types::MovementEvent {
            moved_card_id,
            source_zone: src_id,
            dest_zone: dst_id,
            cause_card_id,
            // `ArcStr`, not `String`. This runs once per card movement and the event built here is
        // cloned into `batch_movements` and `turn_movements` (plus `turn_area_movements` for
        // a position change), so a `String` field allocated and freed that player id three
        // or four times per movement. `ArcStr` makes every one of those a refcount bump.
        // `ArcStr` already has `From<&str>`, `PartialEq<&str>`, `AsRef<str>` and `Display`,
        // so the comparisons and the two `.as_str()` readers are unchanged.
        cause_player_id: cause_player_id.into(),
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
        // This runs once per movement, and a batch almost always moves cards out of
        // the same zone, so the unconditional `to_string()` re-allocated for a value
        // that was already stored. Compared before assigning: the write is identical,
        // the allocation is not.
        if self.recently_moved_from_zone.as_deref() != Some(source_zone) {
            self.recently_moved_from_zone = Some(source_zone.to_string());
        }
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
                && event.cause_player_id != self.player1.id.as_str())
                || (self.player2.contains_card(moved_card_id)
                    && event.cause_player_id != self.player2.id.as_str());
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
        // Same guard as push_movement_event: this is the batch-level writer, called
        // once per batch rather than once per card, so the allocation matters far
        // less here — kept for consistency rather than for the profile.
        if let Some(z) = from_zone {
            if self.recently_moved_from_zone.as_deref() != Some(z) {
                self.recently_moved_from_zone = Some(z.to_string());
            }
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
        // 「このターン中に…」 is a per-turn history, so it turns over with the other
        // per-turn trackers. Left uncleared, an activation from a previous turn would
        // keep satisfying PL!-pb2-010-P＋ and PL!-pb2-040-L forever.
        self.cards_activated_this_turn.clear();
        // Same turnover for PL!-pb2-021-N: 「コストを支払う」 is a per-turn event, and a cost
        // paid on a previous turn must not keep firing her 自動.
        self.cards_paid_cost_this_turn.clear();
        // `card_appearance_source` must turn over with `cards_appeared_this_turn`:
        // the appearance-source condition reads the two as a pair
        // (`ability/condition/card.rs`, `get_card_appearance_source` compared
        // against the expected 登場元). It was never cleared, and
        // `get_card_appearance_source` returns the FIRST match — so once a card
        // had appeared from one zone anywhere in the game, that zone's source
        // answered the condition forever, and a later appearance from a
        // different zone could never match. The C port clears it
        // (`engine_c_wip/src/core/tracking.c`); Rust never did.
        self.card_appearance_source.clear();
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

    /// Remove the card at `index` together with its meta entry, keeping the
    /// two vectors prefix-aligned. Returns `None` when out of bounds.
    pub fn remove_revealed_at(&mut self, index: usize) -> Option<i16> {
        if index >= self.revealed_cards.len() {
            return None;
        }
        let card_id = self.revealed_cards.remove(index);
        if index < self.revealed_card_meta.len() {
            self.revealed_card_meta.remove(index);
        }
        Some(card_id)
    }

    /// Remove every occurrence of `card_id` from the revealed pool, moving the
    /// meta entries out in lockstep. Returns the meta of the FIRST removed
    /// occurrence so a caller that puts the card back can re-attach it;
    /// `None` when the card was absent or had no tracked meta.
    pub fn remove_revealed_card_all(&mut self, card_id: i16) -> Option<RevealedCardMeta> {
        let mut first_meta = None;
        let mut index = 0;
        while index < self.revealed_cards.len() {
            if self.revealed_cards[index] == card_id {
                self.revealed_cards.remove(index);
                if index < self.revealed_card_meta.len() {
                    let meta = self.revealed_card_meta.remove(index);
                    if first_meta.is_none() {
                        first_meta = Some(meta);
                    }
                }
            } else {
                index += 1;
            }
        }
        first_meta
    }

    /// Give every revealed card a meta slot by appending owner-less entries
    /// for the untracked tail (raw pushes). Run before re-adding cards so the
    /// appended meta lands next to its own card.
    pub fn pad_revealed_meta_tail(&mut self) {
        while self.revealed_card_meta.len() < self.revealed_cards.len() {
            self.revealed_card_meta.push(RevealedCardMeta {
                source: None,
                source_name: None,
                is_private: false,
                owner: None,
                reveal_type: String::new(),
            });
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
    ///
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

        // Collect all (cid, player_index, effect) pairs upfront to avoid borrow conflicts.
        //
        // BOTH live zones are scanned, not just the success one. 「自分のライブカード
        // 置き場**か**成功ライブカード置き場にあり」 (PL!-pb2-038-L) is a single
        // disjunction, and the live card zone half had no scanner at all, so a card
        // being PERFORMED contributed nothing while the same card in the success zone
        // did. `live_card_zone` holds the card currently performing, so the same
        // effect must be evaluated there too.
        //
        // The resolved `Arc<Ability>` is stored instead of a cloned
        // `AbilityEffect`: the effect is then borrowed from the Arc in
        // the application loop below, which is disjoint from the `&mut
        // self` that applies the bonus, so the borrow that forced the
        // collect-up-front still holds — but a refcount bump replaces a
        // deep `AbilityEffect` clone (whose `compound`/`effect_steps`
        // Box/Vec fields allocate when present), and a `SmallVec` keeps
        // the common ≤4-entry board on the stack instead of allocating
        // a `Vec` buffer every pass.
        let mut entries: SmallVec<[(i16, usize, crate::Arc<crate::card::Ability>); 4]> =
            SmallVec::new();
        for (player_idx, zone_cards) in [
            (0usize, &self.player1.live_card_zone.cards),
            (0usize, &self.player1.success_live_card_zone.cards),
            (1, &self.player2.live_card_zone.cards),
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
                    if ability.effect.is_some() {
                        entries.push((*cid, player_idx, ability));
                    }
                }
            }
        }

        for (cid, player_idx, ability) in &entries {
            let Some(effect) = ability.effect.as_ref() else {
                continue;
            };
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
                let amount = i16::from(
                    effect
                        .resource_icon_count_any()
                        .unwrap_or(effect.count_or(1)),
                );
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
                                self.mods
                                    .add_blade_modifier(target_id, amount);
                            *self
                                .mods
                                .success_zone_blade_bonuses
                                .entry(target_id)
                                    .or_insert(0) += amount;
                            // Display-only attribution (read solely by
                            // game/display.rs). The bonus HashMaps above are
                            // the engine-readable data; the source Vec is not,
                            // so skip the push — and its Vec growth — when the
                            // UI is off, exactly as the stage-constant path's
                            // DisplaySources already does.
                            if crate::core::runtime_flags::action_display_enabled() {
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
                    }
                    "heart" => {
                        let heart_colors = effect.heart_colors_any();
                        let per_color = amount / heart_colors.len().max(1) as i16;
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
                                // Display-only; see the blade arm above.
                                // Gating also skips the per-color
                                // `color_str.to_string()` allocation.
                                if crate::core::runtime_flags::action_display_enabled() {
                                    self.mods.success_zone_heart_sources.push(
                                        crate::core::game_modifiers::BonusSource {
                                            source_card_id: cid,
                                            ability_text: ui_text(&effect.text),
                                            target_card_id: target_id,
                                            amount: per_color,
                                            color: Some(color_str.to_string()),
                                            kind: ui_kind("heart"),
                                        },
                                    );
                                }
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
                // 「ライブの合計スコアを＋１する」 on a card in a live zone modifies the
                // OWNER'S live total, not any one card. This arm previously had only
                // `self_target` (per-card, how Angelic Angel works) and "target the
                // cards in the live set zone", so a `live_total` effect fell into the
                // second case and — with an empty live set zone, which is the normal
                // state for a card sitting in the SUCCESS zone — targeted nothing and
                // silently contributed 0. The stage-constant path has had a
                // `live_total` case all along (see the `belongs_to_p1` routing in
                // recalculate_constants); this is the same routing for live zones.
                if effect.target_any() == Some("live_total") {
                    let delta = if op == "set" { value } else { value };
                    if player_idx == 0 {
                        self.mods.p1_constant_total_score_bonus =
                            self.mods.p1_constant_total_score_bonus.saturating_add(delta);
                    } else {
                        self.mods.p2_constant_total_score_bonus =
                            self.mods.p2_constant_total_score_bonus.saturating_add(delta);
                    }
                    self.mods.constant_score_sources.push((
                        cid,
                        ui_text(&effect.text),
                        delta,
                    ));
                    return;
                }
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
                            // Display-only attribution; see the blade arm.
                            if crate::core::runtime_flags::action_display_enabled() {
                                self.mods.success_zone_score_sources.push(
                                    crate::core::game_modifiers::BonusSource {
                                        source_card_id: cid,
                                        ability_text: ui_text(&effect.text),
                                        target_card_id: target_id,
                                        amount: value,
                                        color: None,
                                        kind: ui_kind("score_set"),
                                    },
                                );
                            }
                        }
                        _ => {
                            self.mods.add_score_modifier(target_id, value);
                            *self
                                .mods
                                .success_zone_score_bonuses
                                .entry(target_id)
                                .or_insert(0) += value;
                            // Display-only attribution; see the blade arm.
                            if crate::core::runtime_flags::action_display_enabled() {
                                self.mods.success_zone_score_sources.push(
                                    crate::core::game_modifiers::BonusSource {
                                        source_card_id: cid,
                                        ability_text: ui_text(&effect.text),
                                        target_card_id: target_id,
                                        amount: value,
                                        color: None,
                                        kind: ui_kind("score"),
                                    },
                                );
                            }
                        }
                    }
                }
            }
            ActionType::Sequential => {
                if let Some(actions) = effect.actions() {
                    for sub in actions.iter().map(|b| b.as_ref()) {
                        self.apply_success_zone_effect(cid, player_idx, sub);
                    }
                }
            }
            _ => {}
        }
    }
}