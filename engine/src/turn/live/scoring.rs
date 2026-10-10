use crate::card::{BaseHeart, BladeColor, CardDatabase, HeartColor};
use crate::core::game_modifiers::ModifierEntry;
use crate::turn::TurnEngine;
use crate::types::{
    AllocPhase, Allocation, LivePerformanceData, SourceName, SourceType,
};
use crate::HashMap;
use smallvec::SmallVec;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

pub(crate) const EMPTY_H8: [u8; 8] = [0u8; 8];

/// Effective heart need for a live card during allocation.
struct CardNeed {
    name: crate::types::ArcStr,
    need: [u8; 8],
}

/// Suffix demand bounds for the allocation backtracking search.
///
/// `total[i]` is the sum of every card's total heart requirement from
/// card `i` onward; `color[i][c]` is the same for a single color.
/// The search only ever fills requirements from the pool, one pool
/// heart per requirement unit, and a colored note can only be filled
/// by its own color or by icon_all — so a node whose pool no longer
/// covers these bounds cannot reach a solution. Checking the bounds
/// prunes whole composition subtrees without changing which solution
/// the depth-first search finds first.
struct SuffixDemand {
    total: Vec<u16>,
    color: Vec<[u16; 8]>,
}

impl SuffixDemand {
    fn new(card_needs: &[CardNeed]) -> Self {
        let n = card_needs.len();
        let mut total = vec![0u16; n + 1];
        let mut color = vec![[0u16; 8]; n + 1];
        for i in (0..n).rev() {
            let card_total: u16 = card_needs[i].need.iter().map(|&x| u16::from(x)).sum();
            total[i] = total[i + 1] + card_total;
            for c in 0..8 {
                color[i][c] = color[i + 1][c] + u16::from(card_needs[i].need[c]);
            }
        }
        Self { total, color }
    }

    /// Necessary condition for the subtree at card `idx` to contain a
    /// solution, given the pool and the current card's fills so far.
    fn satisfied(&self, pool: &[u8; 8], filled: [u8; 8], idx: usize) -> bool {
        let avail: u16 = pool.iter().map(|&x| u16::from(x)).sum();
        let filled_total: u16 = filled.iter().map(|&x| u16::from(x)).sum();
        if avail + filled_total < self.total[idx] {
            return false;
        }
        for c in 1..7 {
            if u16::from(pool[c]) + u16::from(pool[7]) + u16::from(filled[c])
                < self.color[idx][c]
            {
                return false;
            }
        }
        true
    }
}

/// One allocation of stage hearts onto a live card's requirement.
///
/// Every phase in `greedy_allocate` pushes the same eleven-field record, and
/// ten of those fields are the same every time: what varies is which source the
/// hearts came from, whether that source is a wildcard, which colour bucket
/// they land in, how many, and which phase it was. Spelling the constant ten
/// out five times made any change to them a five-site edit, and the phase field
/// — the one thing a reader most wants to see — was buried in the middle.
fn stage_alloc(
    live_idx: usize,
    card_name: &crate::types::ArcStr,
    source_name: crate::types::SourceName,
    wildcard: bool,
    color: u8,
    amount: u8,
    phase: crate::types::AllocPhase,
) -> crate::types::Allocation {
    crate::types::Allocation {
        target_idx: u8::try_from(live_idx).unwrap(),
        target_name: card_name.clone(),
        source_type: crate::types::SourceType::Stage,
        source_name,
        source_slot: None,
        wildcard,
        color,
        amount,
        is_bonus: false,
        phase,
    }
}
pub fn blade_color_to_heart(bc: BladeColor) -> HeartColor {
    match bc {
        BladeColor::Peach => HeartColor::Heart01,
        BladeColor::Red => HeartColor::Heart02,
        BladeColor::Yellow => HeartColor::Heart03,
        BladeColor::Green => HeartColor::Heart04,
        BladeColor::Blue => HeartColor::Heart05,
        BladeColor::Purple => HeartColor::Heart06,
        // ALL blade = "any one color heart" (rule 2.1.1.3), i.e. icon_all
        // (HeartColor::All, index 7) — NOT colorless Heart00.
        BladeColor::All => HeartColor::All,
    }
}

pub fn blade_type_override_for_stage(
    stage: &[i16],
    blade_type_modifiers: &HashMap<i16, BladeColor>,
) -> Option<BladeColor> {
    stage.iter().find_map(|card_id| {
        if *card_id == -1 {
            None
        } else {
            blade_type_modifiers.get(card_id).copied()
        }
    })
}
impl TurnEngine {
    pub fn compute_allocations(
        owned_hearts: &BaseHeart,
        live_card_ids: &[i16],
        card_db: &CardDatabase,
        need_heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
    ) -> Vec<Allocation> {
        // Build normalized card needs array + future demand
        let card_needs = Self::build_card_needs(live_card_ids, card_db, need_heart_modifiers);
        let future_demand = Self::compute_future_demand(&card_needs);

        // Initialize pool as array for deterministic access
        let mut pool = [0u8; 8];
        for (color, count) in &owned_hearts.hearts {
            pool[color.index()] += count;
        }

        // Try smart greedy first
        let mut pool_copy = pool;
        let greedy = Self::greedy_allocate(&mut pool_copy, &card_needs, &future_demand);
        if Self::allocations_pass(&greedy, &card_needs) {
            return greedy;
        }

        // Greedy failed — backtrack over Phase 3a + icon_all choices
        let pool_arr = pool;
        if let Some(bt) = Self::backtrack_allocate(&pool_arr, &card_needs) {
            return bt;
        }

        // Fallback: return greedy result even if some cards failed
        greedy
    }

    fn build_card_needs(
        live_card_ids: &[i16],
        card_db: &CardDatabase,
        need_heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
    ) -> SmallVec<[CardNeed; 4]> {
        let mut needs = SmallVec::new();
        for &lc_id in live_card_ids {
            if let Some(card) = card_db.get_card(lc_id) {
                let mut need = [0u8; 8];
                if let Some(ref nh) = card.need_heart {
                    // Start from the card's base requirements for every color.
                    for (color, count) in &nh.hearts {
                        need[color.index()] = *count;
                    }
                    if let Some(card_mods) = need_heart_modifiers.get(&lc_id) {
                        crate::core::game_modifiers::apply_need_heart_modifiers(
                            &mut need,
                            card_mods,
                        );
                    }
                }
                needs.push(CardNeed {
                    name: crate::types::ArcStr::from(card.name.as_ref()),
                    need,
                });
            }
        }
        needs
    }

    /// Compute future demand per card: for cards i+1..N, sum of non-heart00 needs.
    fn compute_future_demand(card_needs: &[CardNeed]) -> SmallVec<[[u8; 8]; 4]> {
        let n = card_needs.len();
        let mut demand: SmallVec<[[u8; 8]; 4]> =
            (0..n).map(|_| [0u8; 8]).collect();
        let mut running = [0u8; 8];
        for i in (0..n).rev() {
            if i + 1 < n {
                demand[i][1..7].copy_from_slice(&running[1..7]);
            }
            for (c, r) in running.iter_mut().enumerate().skip(1).take(6) {
                *r += card_needs[i].need[c];
            }
        }
        demand
    }

    /// Smart greedy allocation: demand-aware Phase 3a + icon_all-last Phase 4.
    fn greedy_allocate(
        pool: &mut [u8; 8],
        card_needs: &[CardNeed],
        future_demand: &[[u8; 8]],
    ) -> Vec<Allocation> {
        let mut allocs = Vec::new();
        for (live_idx, cn) in card_needs.iter().enumerate() {
            let need = cn.need;
            // Track per-color totals for this card (direct + wildcard already assigned)
            let mut filled = [0u8; 8];
            let card_name = &cn.name;

            // Phase 1a: matching colored hearts → specific color req
            for c in 1..7 {
                if need[c] > 0 && pool[c] > 0 {
                    let take = pool[c].min(need[c]);
                    allocs.push(stage_alloc(
                        live_idx,
                        card_name,
                        SourceName::StageHearts,
                        false,
                        u8::try_from(c).unwrap(),
                        take,
                        AllocPhase::Colored,
                    ));
                    pool[c] -= take;
                    filled[c] += take;
                }
            }

            // NOTE: There is deliberately NO phase where Heart00 (COLORLESS)
            // hearts fill a specific color requirement. A colorless heart
            // (e.g. from b_heart07) can ONLY replace heart0 requirements
            // (rule 2.1.1.2), never a heart01-heart06 note (rule 2.11.3).

            // Phase 3a: total remaining deficit = total_required - total_filled_so_far.
            // Need[0] is the "any" portion, but the total must also be met.
            let total_filled_so_far: u8 = filled.iter().sum();
            let total_required: u8 = need.iter().sum();
            let h00_deficit = total_required.saturating_sub(total_filled_so_far);
            if h00_deficit > 0 {
                // Demand-aware: sort colors by (pool - future_demand) descending
                // so colors with most surplus vs. future cards get consumed first.
                let mut surplus_colors: Vec<usize> = (1..7).collect();
                surplus_colors.sort_by(|&a, &b| {
                    let score_a = pool[a] as i32 - future_demand[live_idx][a] as i32;
                    let score_b = pool[b] as i32 - future_demand[live_idx][b] as i32;
                    score_b.cmp(&score_a)
                });
                let mut filled_h00 = 0u8;
                for &c in &surplus_colors {
                    if filled_h00 >= h00_deficit {
                        break;
                    }
                    if pool[c] > 0 {
                        let take = pool[c].min(h00_deficit - filled_h00);
                        allocs.push(stage_alloc(
                            live_idx,
                            card_name,
                            SourceName::StageHearts,
                            false,
                            u8::try_from(c).unwrap(),
                            take,
                            AllocPhase::ColoredSurplus,
                        ));
                        pool[c] -= take;
                        filled_h00 += take;
                        filled[c] += take;
                    }
                }

                // Phase 3b: Heart00 (COLORLESS) → remaining Heart00 deficit.
                // Colorless hearts (e.g. b_heart07) go ONLY into the heart0
                // bucket — never a specific color.
                if filled_h00 < h00_deficit && pool[0] > 0 {
                    let take = pool[0].min(h00_deficit - filled_h00);
                    allocs.push(stage_alloc(
                        live_idx,
                        card_name,
                        SourceName::StageHearts,
                        false,
                        0,
                        take,
                        AllocPhase::H00,
                    ));
                    pool[0] -= take;
                    filled_h00 += take;
                    let _ = filled_h00;
                }
            }

            // Phase 4: icon_all → ANY remaining deficit (color deficits first, then heart00)
            if pool[7] > 0 {
                // Color deficits first (otherwise they'd need heart00 which we might not have)
                for c in 1..7 {
                    if need[c] > filled[c] && pool[7] > 0 {
                        let deficit = need[c] - filled[c];
                        let take = pool[7].min(deficit);
                        allocs.push(stage_alloc(
                            live_idx,
                            card_name,
                            SourceName::AllHeartIconAll,
                            true,
                            u8::try_from(c).unwrap(),
                            take,
                            AllocPhase::AllCleanup,
                        ));
                        pool[7] -= take;
                        filled[c] += take;
                    }
                }
                // Remaining icon_all → heart00 deficit
                let total_colored: u8 = filled[1..7].iter().sum();
                let h00_remaining = need[0].saturating_sub(total_colored);
                if h00_remaining > 0 && pool[7] > 0 {
                    // Also include any previous filled[0] from Phase 3b
                    let already_filled_h00 = filled[0];
                    let h00_still_needed = h00_remaining.saturating_sub(already_filled_h00);
                    if h00_still_needed > 0 && pool[7] > 0 {
                        let take = pool[7].min(h00_still_needed);
                        allocs.push(stage_alloc(
                            live_idx,
                            card_name,
                            SourceName::AllHeartIconAll,
                            false,
                            7,
                            take,
                            AllocPhase::AllCleanup,
                        ));
                        pool[7] -= take;
                        filled[0] += take;
                        let _ = filled;
                    }
                }
            }
        }
        allocs
    }

    /// Check whether all cards' heart requirements are satisfied by the allocations.
    /// Uses the same logic as the pass/fail check in execute_live_victory_determination.
    fn allocations_pass(allocs: &[Allocation], card_needs: &[CardNeed]) -> bool {
        if card_needs.is_empty() {
            return true;
        }
        // Build per-card filled arrays
        let num_cards = card_needs.len();
        let mut per_card_filled = vec![[0u8; 8]; num_cards];
        for a in allocs {
            let target_idx = usize::from(a.target_idx);
            if target_idx < num_cards {
                let color_idx = usize::from(a.color);
                per_card_filled[target_idx][color_idx] += a.amount;
            }
        }
        // Check each card
        for (i, cn) in card_needs.iter().enumerate() {
            let filled = per_card_filled[i];
            let req = cn.need;
            let mut ok = true;
            let total_filled: u8 = filled.iter().sum();
            let total_required: u8 = req.iter().sum();
            if total_filled < total_required {
                ok = false;
            }
            // COLORLESS hearts (filled[0], e.g. from b_heart07) count toward the
            // heart0/total bucket but can NEVER be used as a specific color.
            // Only icon_all (filled[7]) can cover a colored-note deficit.
            let mut icon_all = filled[7];
            if ok && req[0] > 0 {
                let any_hearts: u8 = filled[1..7].iter().sum::<u8>() + filled[0];
                if any_hearts + icon_all < req[0] {
                    ok = false;
                } else {
                    icon_all = icon_all.saturating_sub(req[0].saturating_sub(any_hearts));
                }
            }
            if ok {
                for (idx, (&f, &r)) in filled[1..7].iter().zip(req[1..7].iter()).enumerate() {
                    if f < r {
                        let actual_idx = idx + 1;
                        let deficit = req[actual_idx] - filled[actual_idx];
                        if icon_all >= deficit {
                            icon_all -= deficit;
                        } else {
                            ok = false;
                            break;
                        }
                    }
                }
            }
            if !ok {
                return false;
            }
        }
        true
    }

    /// Exhaustive backtracking search over Phase 3a + Phase 4 choices.
    /// Tries all valid ways to extract hearts from the pool per card.
    fn backtrack_allocate(pool: &[u8; 8], card_needs: &[CardNeed]) -> Option<Vec<Allocation>> {
        let mut allocs = Vec::new();
        let mut current_pool = *pool;
        let demand = SuffixDemand::new(card_needs);
        if Self::bt_search(&mut current_pool, card_needs, &demand, 0, &mut allocs) {
            Some(allocs)
        } else {
            None
        }
    }

    /// Recursive backtracking: try all valid allocations for card `idx` then recurse.
    fn bt_search(
        pool: &mut [u8; 8],
        card_needs: &[CardNeed],
        demand: &SuffixDemand,
        idx: usize,
        allocs: &mut Vec<Allocation>,
    ) -> bool {
        if idx >= card_needs.len() {
            return true;
        }
        // Phase 1a below is forced and preserves both bounds — it trades
        // pool hearts for matching requirements one-for-one — so this
        // entry check also covers the state after it.
        if !demand.satisfied(pool, [0u8; 8], idx) {
            return false;
        }
        let saved_pool = *pool;
        let saved_len = allocs.len();
        let cn = &card_needs[idx];
        let need = cn.need;
        let card_name = &cn.name;

        // ----- Forced phases (no choice) -----

        // Phase 1a: matching colored hearts → color req (no choice)
        let mut filled = [0u8; 8];
        for c in 1..7 {
            if need[c] > 0 && pool[c] > 0 {
                let take = pool[c].min(need[c]);
                allocs.push(Allocation {
                    target_idx: u8::try_from(idx).unwrap(),
                    target_name: card_name.clone(),
                    source_type: SourceType::Stage,
                    source_name: SourceName::StageHearts,
                    source_slot: None,
                    wildcard: false,
                    color: u8::try_from(c).unwrap(),
                    amount: take,
                    is_bonus: false,
                    phase: AllocPhase::Colored,
                });
                pool[c] -= take;
                filled[c] += take;
            }
        }

        // NOTE: There is deliberately NO phase where Heart00 (COLORLESS)
        // hearts fill a specific color requirement. A colorless heart
        // (e.g. from b_heart07) can ONLY replace heart0 requirements
        // (rule 2.1.1.2), never a heart01-heart06 note (rule 2.11.3).

        // ----- Choice phases: Phase 3a (which surplus colors → heart00) -----
        let total_filled_so_far: u8 = filled.iter().sum();
        let total_required: u8 = need.iter().sum();
        let h00_deficit = total_required.saturating_sub(total_filled_so_far);

        // Collect available surplus colors
        let mut surplus_colors: Vec<usize> = (1..7).filter(|&c| pool[c] > 0).collect();
        surplus_colors.sort();
        let total_surplus: u8 = surplus_colors.iter().map(|&c| pool[c]).sum();
        let h00_from_surplus = h00_deficit.min(total_surplus);

        let found = Self::try_surplus_compositions(
            pool,
            card_needs,
            demand,
            idx,
            &surplus_colors,
            h00_from_surplus,
            0,
            allocs,
            filled,
        );
        if found {
            return true;
        }

        // Undo: restore pool and truncate allocs
        *pool = saved_pool;
        allocs.truncate(saved_len);
        false
    }

    /// Recursively enumerate all compositions of `remaining` hearts from `colors[color_idx..]`.
    #[allow(clippy::too_many_arguments)]
fn try_surplus_compositions(
        pool: &mut [u8; 8],
        card_needs: &[CardNeed],
        demand: &SuffixDemand,
        idx: usize,
        colors: &[usize],
        remaining: u8,
        color_idx: usize,
        allocs: &mut Vec<Allocation>,
        filled: [u8; 8],
    ) -> bool {
        let cn = &card_needs[idx];
        let card_name = &cn.name;

        if color_idx >= colors.len() {
            if remaining > 0 {
                return false;
            }
            return Self::try_phase4(pool, card_needs, demand, idx, allocs, filled);
        }

        let saved_pool = *pool;
        let saved_len = allocs.len();
        let c = colors[color_idx];
        let max_take = pool[c].min(remaining);
        for take in 0..=max_take {
            let mut new_filled = filled;
            if take > 0 {
                allocs.push(Allocation {
                    target_idx: u8::try_from(idx).unwrap(),
                    target_name: card_name.clone(),
                    source_type: SourceType::Stage,
                    source_name: SourceName::StageHearts,
                    source_slot: None,
                    wildcard: false,
                    color: u8::try_from(c).unwrap(),
                    amount: take,
                    is_bonus: false,
                    phase: AllocPhase::ColoredSurplus,
                });
                pool[c] -= take;
                new_filled[c] += take;
            }

            // Prune: a composition that leaves the pool unable to
            // cover the remaining demand (this card plus every later
            // card) cannot lead to a solution.
            if demand.satisfied(pool, new_filled, idx) {
                let result = Self::try_surplus_compositions(
                    pool,
                    card_needs,
                    demand,
                    idx,
                    colors,
                    remaining - take,
                    color_idx + 1,
                    allocs,
                    new_filled,
                );
                if result {
                    return true;
                }
            }

            // Undo
            *pool = saved_pool;
            allocs.truncate(saved_len);
        }
        false
    }

    /// After Phase 3a choices are made, try Phase 3b (heart00 → heart00 deficit)
    /// and Phase 4 (icon_all → remaining deficits).
    fn try_phase4(
        pool: &mut [u8; 8],
        card_needs: &[CardNeed],
        demand: &SuffixDemand,
        idx: usize,
        allocs: &mut Vec<Allocation>,
        mut filled: [u8; 8],
    ) -> bool {
        let saved_pool = *pool;
        let saved_len = allocs.len();
        let cn = &card_needs[idx];
        let card_name = &cn.name;
        let need = cn.need;

        // Count all hearts allocated so far (1a + 3a)
        let total_filled_so_far: u8 = filled.iter().sum();
        let total_required: u8 = need.iter().sum();
        let h00_deficit = total_required.saturating_sub(total_filled_so_far);

        // Phase 3b: Heart00 (COLORLESS) → remaining heart0/total deficit.
        // Colorless hearts (e.g. b_heart07) go ONLY into the heart0 bucket —
        // never a specific color. Forced (no choice).
        if h00_deficit > 0 && pool[0] > 0 {
            let take = pool[0].min(h00_deficit);
            allocs.push(Allocation {
                target_idx: u8::try_from(idx).unwrap(),
                target_name: card_name.clone(),
                source_type: SourceType::Stage,
                source_name: SourceName::StageHearts,
                source_slot: None,
                wildcard: false,
                color: 0,
                amount: take,
                is_bonus: false,
                phase: AllocPhase::H00,
            });
            pool[0] -= take;
            filled[0] += take;
        }

        // Prune before enumerating Phase 4 distributions: with the
        // forced phases done, icon_all is the only source left for
        // colored deficits, so the bounds are tightest here.
        if !demand.satisfied(pool, filled, idx) {
            *pool = saved_pool;
            allocs.truncate(saved_len);
            return false;
        }

        // Now compute all remaining deficits
        let total_filled_now: u8 = filled.iter().sum();
        let total_required: u8 = need.iter().sum();
        let h00_still_needed = total_required.saturating_sub(total_filled_now);

        let mut color_deficits: Vec<(usize, u8)> = Vec::new();
        for c in 1..7 {
            if filled[c] < need[c] {
                color_deficits.push((c, need[c] - filled[c]));
            }
        }
        if h00_still_needed > 0 {
            color_deficits.push((0, h00_still_needed));
        }

        // Phase 4: icon_all → try all distributions to deficits
        let all_count = pool[7];
        if all_count == 0 && color_deficits.is_empty() {
            if Self::card_ok_with_wildcard(filled, need) {
                return Self::bt_search(pool, card_needs, demand, idx + 1, allocs);
            }
        } else if all_count == 0 {
            // No icon_all but deficits exist
            *pool = saved_pool;
            allocs.truncate(saved_len);
            return false;
        } else {
            let deficit_indices: Vec<usize> = {
                let mut v: Vec<usize> = color_deficits.iter().map(|&(c, _)| c).collect();
                v.sort();
                v
            };
            if Self::try_all_distribution(
                pool,
                card_needs,
                demand,
                idx,
                allocs,
                filled,
                &deficit_indices,
                &color_deficits,
                all_count,
                0,
            ) {
                return true;
            }
        }

        // Undo
        *pool = saved_pool;
        allocs.truncate(saved_len);
        false
    }

    /// Try all distributions of `remaining` icon_all hearts to deficit types starting at `di`.
    #[allow(clippy::too_many_arguments)]
fn try_all_distribution(
        pool: &mut [u8; 8],
        card_needs: &[CardNeed],
        demand: &SuffixDemand,
        idx: usize,
        allocs: &mut Vec<Allocation>,
        filled: [u8; 8],
        deficit_indices: &[usize],
        deficits: &[(usize, u8)],
        remaining: u8,
        di: usize,
    ) -> bool {
        let saved_pool = *pool;
        let saved_len = allocs.len();
        let cn = &card_needs[idx];
        let card_name = &cn.name;

        if di >= deficit_indices.len() {
            if remaining > 0 {
                // icon_all left but no deficits → use for heart00 surplus
                *pool = saved_pool;
                allocs.truncate(saved_len);
                return false;
            }
            if Self::card_ok_with_wildcard(filled, cn.need) {
                return Self::bt_search(pool, card_needs, demand, idx + 1, allocs);
            }
            *pool = saved_pool;
            allocs.truncate(saved_len);
            return false;
        }

        let target_color = deficit_indices[di];
        let deficit_amt = deficits
            .iter()
            .find(|&&(c, _)| c == target_color)
            .map(|&(_, d)| d)
            .unwrap_or(0);

        let max_take = remaining.min(deficit_amt);
        for take in 0..=max_take {
            let mut new_filled = filled;
            if take > 0 {
                let alloc_color = if target_color == 0 {
                    7
                } else {
                    u8::try_from(target_color).unwrap()
                };
                allocs.push(Allocation {
                    target_idx: u8::try_from(idx).unwrap(),
                    target_name: card_name.clone(),
                    source_type: SourceType::Stage,
                    source_name: SourceName::AllHeartIconAll,
                    source_slot: None,
                    wildcard: target_color == 0,
                    color: alloc_color,
                    amount: take,
                    is_bonus: false,
                    phase: AllocPhase::AllCleanup,
                });
                pool[7] -= take;
                if target_color == 0 {
                    new_filled[0] += take;
                } else {
                    new_filled[target_color] += take;
                }
            }

            // Prune: a distribution that leaves the pool unable to
            // cover the remaining demand cannot lead to a solution.
            if demand.satisfied(pool, new_filled, idx) {
                let result = Self::try_all_distribution(
                    pool,
                    card_needs,
                    demand,
                    idx,
                    allocs,
                    new_filled,
                    deficit_indices,
                    deficits,
                    remaining - take,
                    di + 1,
                );
                if result {
                    return true;
                }
            }

            *pool = saved_pool;
            allocs.truncate(saved_len);
        }
        false
    }

    /// Check if a single card's requirements are satisfied with its filled array.
    /// COLORLESS hearts (filled[0], e.g. from b_heart07) count toward the
    /// heart0/total bucket but can NEVER be used as a specific color — only
    /// icon_all (filled[7]) can cover a colored-note deficit.
    fn card_ok_with_wildcard(filled: [u8; 8], need: [u8; 8]) -> bool {
        let mut icon_all = filled[7];
        let total_filled: u8 = filled.iter().sum();
        let total_required: u8 = need.iter().sum();
        if total_filled < total_required {
            return false;
        }
        if need[0] > 0 {
            let any_hearts: u8 = filled[1..7].iter().sum::<u8>() + filled[0];
            if any_hearts + icon_all < need[0] {
                return false;
            }
            icon_all = icon_all.saturating_sub(need[0].saturating_sub(any_hearts));
        }
        for idx in 1..7 {
            if filled[idx] < need[idx] {
                let deficit = need[idx] - filled[idx];
                if icon_all >= deficit {
                    icon_all -= deficit;
                } else {
                    return false;
                }
            }
        }
        true
    }

    /// Rule 8.3.14-8.3.16: Check heart requirements, determine live success/failure,
    /// drain resolution zone. Called AFTER the 8.3.13 check timing so hearts granted
    /// by "when you yell" abilities are included in the live success check.
    /// `heart_override`/`heart_modifiers`/`heart_color_multiplier` must come from the
    /// current game state (post-ability-trigger) so ability-granted hearts are counted.
    #[allow(clippy::too_many_arguments)]
    pub fn check_live_success(
        player: &mut crate::player::Player,
        resolution_zone: &mut crate::zones::ResolutionZone,
        card_db: &CardDatabase,
        need_heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
        heart_override: &HashMap<i16, (HeartColor, u8)>,
        heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
        heart_color_multiplier: &HashMap<i16, HeartColor>,
        heart_copy: &HashMap<i16, i16>,
        mut perf: LivePerformanceData,
    ) -> LivePerformanceData {
        // `perf` arrives owned from `execute_performance_phase`, which
        // built it in `player_perform_live`. The old shape took the
        // six collection fields as slices and `.to_vec()`-cloned every
        // one back into a fresh `LivePerformanceData` here — six Vec
        // allocations plus an element-wise clone of `member_contributions`
        // (416 B/element) and `allocations` per performance, for data
        // the caller already owned and never reads again. Taking ownership
        // moves those fields straight through; only the four fields this
        // function actually recomputes (`allocations`, `revealed_ids`,
        // `moved_live_card_ids`, `draw_effects_occurred`) are written.
        //
        // `draw_effects_occurred` is read off `perf.yell_cards` up here
        // so the immutable borrow ends before the owned fields are
        // reassigned below.
        let draw_effects_occurred = perf.yell_cards.iter().any(|yc| yc.draw_icons > 0);
        // Recompute hearts from current state (includes ability-granted hearts
        // from the 8.3.13 check timing) + yell blade heart hearts.
        let mut owned_hearts = player.stage.get_available_hearts(
            card_db,
            heart_override,
            heart_modifiers,
            heart_color_multiplier,
            heart_copy,
        );
        for yc in &perf.yell_cards {
            for i in 0..8 {
                if yc.blade_hearts[i] > 0 {
                    *owned_hearts
                        .hearts
                        .entry_or_default(HeartColor::from_index(i)) += yc.blade_hearts[i];
                }
            }
        }
        // Recompute allocations from the updated pool.
        let allocations =
            Self::compute_allocations(&owned_hearts, &perf.live_card_ids, card_db, need_heart_modifiers);

        let mut per_card_filled: SmallVec<[[u8; 8]; 4]> =
            (0..perf.live_card_ids.len()).map(|_| EMPTY_H8).collect();
        for alloc in &allocations {
            if (alloc.target_idx as usize) < per_card_filled.len() {
                per_card_filled[alloc.target_idx as usize][alloc.color as usize] += alloc.amount;
            }
        }

        // Q259: Required heart check is only performed at live success judgment timing.
        // Subsequent changes do NOT retroactively fail a live.
        let any_requirement_failed = perf.live_card_ids.iter().enumerate().any(|(live_idx, &lc_id)| {
            card_db.get_card(lc_id).is_some_and(|card| {
                // Build effective need_heart by starting from base requirements,
                // then applying per-color set/additive modifiers.
                // Q115/Q127: A set modifier on one color does NOT erase other colors.
                let base_nh = match card.need_heart.as_ref() {
                    Some(nh) => nh,
                    None => return false,
                };
                if base_nh.hearts.is_empty() {
                    return false;
                }
                let mut required_arr = EMPTY_H8;
                // Populate from base card requirements.
                for (color, needed) in &base_nh.hearts {
                    required_arr[color.index()] = *needed;
                }
                // Q115/Q127: per-colour set first, then additive on top — the
                // single definition of that rule, not a third restatement.
                if let Some(card_mods) = need_heart_modifiers.get(&lc_id) {
                    crate::core::game_modifiers::apply_need_heart_modifiers(
                        &mut required_arr,
                        card_mods,
                    );
                }
                let filled = per_card_filled[live_idx];
                // COLORLESS hearts (filled[0], e.g. from b_heart07) count toward the
                // heart0/total bucket but can NEVER be used as a specific color —
                // only icon_all (filled[7]) can cover a colored-note deficit.
                let mut icon_all = filled[7];
                let mut ok = true;
                let total_filled: u8 = filled.iter().sum();
                let total_required: u8 = required_arr.iter().sum();
                if total_filled < total_required {
                    ok = false;
                }
                if ok && required_arr[0] > 0 {
                    let any_hearts: u8 = filled[1..7].iter().sum::<u8>() + filled[0];
                    if any_hearts + icon_all < required_arr[0] {
                        ok = false;
                    } else {
                        icon_all =
                            icon_all.saturating_sub(required_arr[0].saturating_sub(any_hearts));
                    }
                }
                if ok {
                    for (idx, (&f, &r)) in filled[1..7]
                        .iter()
                        .zip(required_arr[1..7].iter())
                        .enumerate()
                    {
                        if f < r {
                            let actual_idx = idx + 1;
                            let deficit = required_arr[actual_idx] - filled[actual_idx];
                            if icon_all >= deficit {
                                icon_all -= deficit;
                            } else {
                                ok = false;
                                break;
                            }
                        }
                    }
                }
                !ok
            })
        });
        let moved_live_card_ids: Vec<i16> = if any_requirement_failed {
            log::debug!("[LIVE] Heart requirement not met — sending all live cards to waitroom");
            let moved = core::mem::take(&mut player.live_card_zone.cards);
            player.waitroom.cards.extend_from_slice(&moved);
            moved.into_vec()
        } else {
            Vec::new()
        };

        let revealed_ids: Vec<i16> = resolution_zone.cards.iter().copied().collect();
        player.last_resolution_cards = revealed_ids.clone().into();
        // Q41: Yell-revealed cards go to waitroom during live victory determination,
        // after successful cards are placed in the success zone (Rule 8.4.7).
        for card_id in resolution_zone.cards.drain(..) {
            player.waitroom.add_card(card_id);
        }

        // The four fields this function recomputes; the rest of `perf`
        // (member_contributions, yell_cards, total_hearts, heart_sources,
        // blade_sources, live_card_ids, yell_count, note_icons) moves
        // through untouched — no `.to_vec()` clone of caller-owned data.
        perf.allocations = allocations;
        perf.revealed_ids = revealed_ids;
        perf.moved_live_card_ids = moved_live_card_ids;
        perf.draw_effects_occurred = draw_effects_occurred;
        perf
    }
}
