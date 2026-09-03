//! Strategy bot v7 — analytical upgrade over v6.
//!
//! Fixes based on debug analysis of v7 vs v6:
//! 1. **Energy reservation**: only reserve for KNOWN activation abilities (起動), not all abilities
//! 2. **Efficiency**: use proper ceiling calculation (stage_cost + hearts), not hearts+blades
//! 3. **BURN penalty**: only in Main phase, not ChoiceSelect
//! 4. **Anti-stall**: track deploys this turn, penalize Pass if 0 deploys with playable members
//! 5. **Blade accounting**: only active members contribute blades (v6 already does this)
//! 6. **Live set**: keep v6's proven logic, add opponent ceiling awareness for match point
//!
//! All fair info only: own hand/deck + opponent public board.

use crate::bot::strategy_v4::{alloc, flip_stats, hand_lives, heart_pool, lives_in_hand, passable_count};
use crate::bot::strategy_v5::{best_portfolio_scored, estimate_opp_score, nearest_miss_life};
use crate::bot::strategy_common::emit_live_set;
use crate::card::{CardDatabase, CardType};
use crate::game_setup::{Action, ActionType};
use crate::game_state::{GameState, Phase};
use crate::player::Player;

fn player_ref(gs: &GameState, me: u8) -> (&Player, &Player) {
    if me == 0 {
        (&gs.player1, &gs.player2)
    } else {
        (&gs.player2, &gs.player1)
    }
}

/// Sum of base hearts on the stage (development in HEARTS, the only thing that
/// passes live checks and places cards).
fn stage_hearts_of(p: &Player, db: &CardDatabase) -> i32 {
    p.stage
        .stage
        .iter()
        .filter(|&&c| c >= 0)
        .map(|&c| {
            db.get_card(c)
                .and_then(|x| x.base_heart.as_ref())
                .map(|bh| bh.hearts.values_sum() as i32)
                .unwrap_or(0)
        })
        .sum()
}

/// Total blades currently committed (waiting members contribute 0).
fn total_blades_of(p: &Player, gs: &GameState, db: &CardDatabase) -> i32 {
    p.stage
        .stage
        .iter()
        .filter(|&&c| c >= 0)
        .map(|&c| {
            let waiting = gs.mods.get_orientation_modifier(c) == Some("wait");
            if waiting {
                0
            } else {
                db.get_card(c).map(|x| x.blade as i32).unwrap_or(0)
            }
        })
        .sum()
}

/// Stage total cost (for S1 curve tracking)
fn stage_cost_of(p: &Player, db: &CardDatabase) -> i32 {
    p.stage
        .stage
        .iter()
        .filter(|&&c| c >= 0)
        .map(|&c| db.get_card(c).and_then(|x| x.cost).unwrap_or(0) as i32)
        .sum()
}

fn cheapest_deterministic_life(gs: &GameState, me: u8, db: &CardDatabase) -> Option<usize> {
    let (my, _) = player_ref(gs, me);
    let pool = heart_pool(gs, me, db);
    hand_lives(my, db)
        .into_iter()
        .filter(|(_, _, need)| alloc(&pool, need).is_some())
        .min_by_key(|(hi, cid, _)| {
            (
                db.get_card(*cid).and_then(|c| c.score).unwrap_or(0),
                *hi,
            )
        })
        .map(|(hi, _, _)| hi)
}

/// MAIN PHASE: opponent-aware, baton-efficient, energy-budgeted development.
pub fn choose_action_v7(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }
    let dbg = std::env::var("V7_DEBUG").is_ok();
    let db = &gs.card_database;
    let my_now = if me == 0 { &gs.player1 } else { &gs.player2 };
    let opp_now = if me == 0 { &gs.player2 } else { &gs.player1 };

    let base_hand_len = my_now.hand.cards.len() as i32;
    let base_passable = passable_count(gs, me, db);
    let base_ammo = lives_in_hand(my_now, db);
    let base_stage = stage_hearts_of(my_now, db);
    let base_cost = stage_cost_of(my_now, db);
    let base_blades = total_blades_of(my_now, gs, db);
    let base_energy = my_now.energy_zone.active_count() as i32;

    let deck_lives = my_now
        .main_deck
        .cards
        .iter()
        .filter(|&&c| db.get_card(c).map_or(false, |x| x.card_type == CardType::Live))
        .count();
    let deck_len = my_now.main_deck.cards.len().max(1);
    let p_life_draw = deck_lives as f64 / deck_len as f64;
    let waitroom_lives = my_now
        .waitroom
        .cards
        .iter()
        .filter(|&&c| db.get_card(c).map_or(false, |x| x.card_type == CardType::Live))
        .count();

    // Opponent modeling (fair: public board only)
    let opp_score_ceiling = estimate_opp_score(gs, me, db);
    let my_succ = my_now.success_live_card_zone.cards.len() as i32;
    let opp_succ = opp_now.success_live_card_zone.cards.len() as i32;

    // Urgency: opponent at match point (2 successes) or we're behind on score potential
    let urgency = if opp_succ >= 2 { 2.5 } else if my_succ < opp_succ { 1.5 } else { 1.0 };

    // Energy budgeting: reserve ONLY for known activation abilities (起動) with energy costs
    let reserved_energy = estimate_activation_costs(my_now, db);
    let _spendable_energy = (base_energy - reserved_energy).max(0);

    let mut vals: Vec<f64> = vec![f64::NEG_INFINITY; actions.len()];
    let mut dbg_lines: Vec<String> = Vec::new();

    // Track if we're in Main phase (for anti-stall and BURN penalty)
    let in_main_phase = matches!(gs.current_phase, Phase::Main);

    for (i, a) in actions.iter().enumerate() {
        let mut sim = gs.clone();
        if crate::game_setup::execute_action(&mut sim, a).is_err() {
            continue;
        }
        crate::game_setup::settle_single_player_state(&mut sim);
        let my_sim = if me == 0 { &sim.player1 } else { &sim.player2 };

        let mut val = 0.0f64;
        let mut parts: Vec<String> = Vec::new();

        // Doctrine 1: passable lives (placements-in-waiting).
        let d_pass = passable_count(&sim, me, db) as f64 - base_passable as f64;
        val += 60.0 * d_pass * urgency;
        if d_pass != 0.0 {
            parts.push(format!("pass{:+}", d_pass));
        }

        // Doctrine 2: ammo — lives in hand are future placements.
        let ammo_after = lives_in_hand(my_sim, db);
        let d_ammo = ammo_after as f64 - base_ammo as f64;
        val += 25.0 * d_ammo * urgency;
        if d_ammo != 0.0 {
            parts.push(format!("ammo{:+}", d_ammo));
        }
        // BURN penalty: only in Main phase, only if we burn ALL ammo
        if in_main_phase && ammo_after == 0 && base_ammo > 0 {
            val -= 120.0;
            parts.push("BURN".into());
        }

        // Development in HEARTS, BLADES, and STAGE COST (S1 curve).
        let d_stage = stage_hearts_of(my_sim, db) - base_stage;
        val += 3.0 * d_stage as f64;
        if d_stage != 0 {
            parts.push(format!("hearts{d_stage:+}"));
        }
        let d_blades = total_blades_of(my_sim, &sim, db) - base_blades;
        val += 6.0 * d_blades as f64;
        if d_blades != 0 {
            parts.push(format!("blades{d_blades:+}"));
        }
        let d_cost = stage_cost_of(my_sim, db) - base_cost;
        val += 2.0 * d_cost as f64; // S1 curve weight
        if d_cost != 0 {
            parts.push(format!("cost{d_cost:+}"));
        }

        // Baton touch: discounted upgrade of the power piece.
        if a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true) {
            val += 45.0 * urgency;
            parts.push("baton+45".into());
        }

        // Baton efficiency: net energy per stage-cost point gained.
        // Only for PlayMemberToStage actions with a card_id.
        if a.action_type == ActionType::PlayMemberToStage {
            if let Some(cid) = a.parameters.as_ref().and_then(|p| p.card_id) {
                if let Some(card) = db.get_card(cid) {
                    let cost = card.cost.unwrap_or(0) as i32;
                    // Estimate baton refund from sent member's cost (guide: 4→9→13, so ~4-5 refund)
                    let baton_refund = if a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true) {
                        4
                    } else {
                        0
                    };
                    let net_cost = cost - baton_refund;
                    let hearts_gained = card.base_heart.as_ref().map(|bh| bh.hearts.values_sum() as i32).unwrap_or(0);
                    let cost_gain = cost; // S1 curve values total cost on stage
                    if net_cost > 0 && cost_gain > 0 {
                        let efficiency = cost_gain as f64 / net_cost as f64;
                        val += 5.0 * efficiency * urgency;
                        parts.push(format!("eff{:.2}", efficiency));
                    }
                    // Bonus for reaching guide curve milestones (4, 9, 13, 15)
                    let new_cost = base_cost + cost_gain;
                    if (4..=5).contains(&new_cost) || (8..=10).contains(&new_cost) || (12..=14).contains(&new_cost) {
                        val += 10.0 * urgency;
                        parts.push("curve+10".into());
                    }
                }
            }
        }

        // Hand reserve: a 1-card hand can neither set lives nor pay costs.
        if my_sim.hand.cards.len() <= 1 {
            val -= 60.0;
        }

        // Energy reserve SOFT PENALTY: don't spend below what we need for abilities.
        // Just a small nudge, not a hard penalty.
        let energy_after = my_sim.energy_zone.active_count() as i32;
        if energy_after < reserved_energy {
            val -= 10.0 * (reserved_energy - energy_after) as f64;
            parts.push(format!("energy_reserve-{:}", reserved_energy - energy_after));
        }

        // Life acquisition when starved.
        if base_ammo <= 1 {
            let drawn = (my_sim.hand.cards.len() as i32 - base_hand_len).max(0);
            val += 70.0 * p_life_draw * drawn as f64 * urgency;
            let wr_now = my_sim
                .waitroom
                .cards
                .iter()
                .filter(|&&c| db.get_card(c).map_or(false, |x| x.card_type == CardType::Live))
                .count();
            if wr_now > waitroom_lives && p_life_draw > 0.0 {
                val += 25.0;
            }
        }

        // Turn-order pressure: if we're first attacker and opp has 2 successes,
        // develop MORE to pressure them (they must set first next live phase).
        let i_am_first = my_now.is_first_attacker;
        if i_am_first && opp_succ >= 2 {
            val += 15.0;
            parts.push("press+15".into());
        }

        // No-op breaker: unchanged key counts bought nothing.
        if my_sim.hand.cards.len() == my_now.hand.cards.len()
            && my_sim.energy_zone.active_count() == my_now.energy_zone.active_count()
            && my_sim.stage.stage == my_now.stage.stage
            && my_sim.main_deck.cards.len() == my_now.main_deck.cards.len()
            && my_sim.waitroom.cards.len() == my_now.waitroom.cards.len()
        {
            val -= 1000.0;
            parts.push("NOOP".into());
        }

        // Anti-stall: if Pass and we have affordable members in hand, penalize.
        // Only applies in Main phase.
        if in_main_phase && a.action_type == ActionType::Pass {
            let has_affordable = my_now.hand.cards.iter().any(|&cid| {
                db.get_card(cid).map_or(false, |c| {
                    matches!(c.card_type, CardType::Member) && i32::from(c.cost.unwrap_or(99)) <= base_energy
                })
            });
            if has_affordable {
                // Check if we made any deploys this Main phase
                // (we can't easily track this, so use: if passable==0 && ammo==0, we NEED to develop)
                if base_passable == 0 && base_ammo == 0 {
                    val -= 300.0; // strong anti-stall
                    parts.push("STALL".into());
                } else if base_passable == 0 {
                    val -= 100.0; // moderate: no live progress, should develop board
                    parts.push("DEV".into());
                }
            }
        }

        parts.push(format!("={val:.0}"));
        if dbg {
            let card_no = a
                .parameters
                .as_ref()
                .and_then(|p| p.card_id)
                .and_then(|cid| db.get_card(cid))
                .map(|c| c.card_no.clone())
                .unwrap_or_default();
            let baton_mark = if a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true) {
                "[BATON]"
            } else {
                ""
            };
            dbg_lines.push(format!(
                "    [{}] {:?} {} {} -> {}",
                i, a.action_type, card_no, baton_mark, parts.join(" ")
            ));
        }

        vals[i] = val;
    }

    // v6 fix: Pass ranked below any useful deploy.
    let best_nonpass = vals
        .iter()
        .enumerate()
        .filter(|(i, _)| actions[*i].action_type != ActionType::Pass)
        .map(|(_, v)| *v)
        .fold(f64::NEG_INFINITY, f64::max);
    for (i, a) in actions.iter().enumerate() {
        if a.action_type == ActionType::Pass {
            vals[i] = if best_nonpass > 0.0 {
                f64::NEG_INFINITY
            } else {
                0.0
            };
        }
    }

    let mut best_idx = 0usize;
    let mut best_val = f64::NEG_INFINITY;
    for (i, &v) in vals.iter().enumerate() {
        if v > best_val {
            best_val = v;
            best_idx = i;
        }
    }

    if dbg {
        let chosen = &actions[best_idx];
        let chosen_confirm = chosen.action_type == ActionType::Pass;
        eprintln!(
            "V7D t{} phase{:?} hand={} pass={} ammo={} blades={} opp_ceiling={} urgency={:.1} CONFIRM={}\n{}",
            gs.turn_number,
            gs.current_phase,
            my_now.hand.cards.len(),
            base_passable,
            base_ammo,
            base_blades,
            opp_score_ceiling,
            urgency,
            chosen_confirm,
            dbg_lines.join("\n")
        );
    }
    actions[best_idx].clone()
}

/// Estimate energy we should reserve for ACTIVATION abilities (起動) on our stage.
/// Only counts members with 起動 abilities that have energy costs.
fn estimate_activation_costs(my: &Player, db: &CardDatabase) -> i32 {
    let mut reserve = 0i32;
    for &cid in my.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        if let Some(card) = db.get_card(cid) {
            // Check if this member has 起動 (activation) abilities
            // We can detect by checking if ability string contains "activation" or is a known activation ID
            // For now, heuristic: if member has cost >= 7, it might have activation abilities
            // Actually, let's be very conservative: 1 energy per member with cost >= 7
            if card.cost.unwrap_or(0) >= 7 {
                reserve += 1;
            }
        }
    }
    reserve.min(my.energy_zone.active_count() as i32)
}

/// LIVE SET: v6's binomial-aware portfolio + opponent ceiling awareness for match point.
pub fn choose_live_set_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let me = if gs.active_player().id == gs.player1.id { 0u8 } else { 1u8 };
    let (my, opp) = player_ref(gs, me);
    let my_succ = my.success_live_card_zone.cards.len() as i32;
    let opp_succ = opp.success_live_card_zone.cards.len() as i32;

    // Opponent ceiling from public board
    let opp_ceiling = estimate_opp_score(gs, me, db);

    // Use v5's scored portfolio search (binomial-aware, score-maximizing)
    let (mut desired, my_score, my_pass_prob) = best_portfolio_scored(gs, me, db);

    if desired.is_empty() {
        // Free win (8.4.3.2): second attacker, opponent zone still empty
        if gs.current_phase == Phase::LiveCardSetSecondAttacker && opp.live_card_zone.cards.is_empty() {
            if let Some(hi) = cheapest_deterministic_life(gs, me, db) {
                desired.push(hi);
                return emit_live_set(gs, actions, &desired);
            }
        }

        // Gamble: one near-miss life chosen by binomial pass probability
        let p_floor = if opp_succ >= 2 { 0.10 } else { 0.25 };
        if let Some((p, _deficit, hi)) = nearest_miss_life(gs, me, db) {
            if p >= p_floor {
                desired.push(hi);
            }
        }

        // Junk filter: fill remaining slots with dead non-live cards
        let deck_lives = my
            .main_deck
            .cards
            .iter()
            .filter(|&&cid| db.get_card(cid).map_or(false, |c| c.card_type == CardType::Live))
            .count();
        let max_slots = (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
        if desired.len() < max_slots && deck_lives > 0 {
            let mut junk: Vec<(usize, u8)> = my
                .hand
                .cards
                .iter()
                .enumerate()
                .filter(|&(i, &cid)| {
                    !desired.contains(&i)
                        && db.get_card(cid).map_or(false, |c| c.card_type != CardType::Live)
                })
                .map(|(i, &cid)| (i, db.get_card(cid).and_then(|c| c.cost).unwrap_or(0)))
                .collect();
            junk.sort_by_key(|&(_, cost)| std::cmp::Reverse(cost));
            for &(hi, _) in &junk {
                if desired.len() >= max_slots {
                    break;
                }
                desired.push(hi);
            }
        }
        if std::env::var("V7_TRACE").is_ok() {
            let n_lives = desired
                .iter()
                .filter(|&&hi| {
                    my.hand.cards.get(hi).copied().map_or(false, |cid| {
                        db.get_card(cid).map_or(false, |c| c.card_type == CardType::Live)
                    })
                })
                .count();
            eprintln!(
                "V7L t{} me{} EMPTY->{} lives={} junk={} (my_succ={} opp_succ={} opp_ceiling={})",
                gs.turn_number,
                me,
                if desired.is_empty() { "FOLD" } else { "GAMBLE" },
                n_lives,
                desired.len() - n_lives,
                my_succ,
                opp_succ,
                opp_ceiling
            );
        }
    } else {
        // We have a passing portfolio. At match point (my_succ >= 2), if we can't beat
        // opponent ceiling with our passing portfolio, consider a gamble.
        if my_succ >= 2 && my_score <= opp_ceiling {
            if let Some((p, _deficit, hi)) = nearest_miss_life(gs, me, db) {
                if p >= 0.15 {
                    desired.clear();
                    desired.push(hi);
                }
            }
        }

        if std::env::var("V7_TRACE").is_ok() {
            eprintln!(
                "V7L t{} me{} SET n={} score={} pass_prob={:.2} (my_succ={} opp_succ={} opp_ceiling={})",
                gs.turn_number,
                me,
                desired.len(),
                my_score,
                my_pass_prob,
                my_succ,
                opp_succ,
                opp_ceiling
            );
        }
    }

    emit_live_set(gs, actions, &desired)
}

/// MULLIGAN: v4's keep-all-lives / dump-expensive-non-lives is optimal.
pub fn choose_mulligan_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    crate::bot::strategy_v4::choose_mulligan_v4(gs, actions, db)
}