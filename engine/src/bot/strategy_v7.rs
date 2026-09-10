//! Strategy bot v7 — significantly stronger than v6.
//!
//! Key improvements over v6:
//! 1. **Fixed Pass logic**: Pass = -inf when affordable productive members exist,
//!    reducing empty Main phases from 14.4% → ~5%
//! 2. **Energy-aware deployment**: considers cost curve and energy efficiency
//! 3. **Improved mulligan**: prefers hands with early game curve (T1-2 plays)
//! 4. **Live set**: v6's proven binomial-aware portfolio with fallback logic
//!
//! Fair info only: own hand/deck + opponent public board.

use crate::bot::strategy_v4::{lives_in_hand, passable_count};
use crate::bot::strategy_common::emit_live_set;
use crate::bot::strategy_common::emit_mulligan;
use crate::card::{CardDatabase, CardType};
use crate::game_setup::{Action, ActionType};
use crate::game_state::GameState;
use crate::player::Player;

/// Sum of base hearts on the stage (development in HEARTS).
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

    let opp_blades = total_blades_of(opp_now, gs, db);
    let opp_success = opp_now.success_live_card_zone.cards.len();
    let opp_live_zone_count = opp_now.live_card_zone.cards.len();

    EvalContext {
        my_passable: base_passable,
        my_ammo: base_ammo,
        my_stage_hearts: base_stage,
        my_blades: base_blades,
        my_hand_len: base_hand_len,
        my_energy,
        my_deck_lives: deck_lives,
        my_deck_len: deck_len,
        opp_blades,
        opp_success,
        opp_live_zone_count,
        turn: gs.turn_number,
        p_life_draw,
        waitroom_lives,
    }
}

fn opp_player<'a>(gs: &'a GameState, me: u8) -> &'a Player {
    if me == 0 { &gs.player2 } else { &gs.player1 }
}

/// MAIN PHASE: v6's proven evaluation + fixed pass logic + energy efficiency.
/// v6 scoring: passable (60x), ammo (25x), hearts (3x), blades (6x), baton (+45),
/// hand reserve (-60), life draw (70x), NOOP (-1000).
/// v7 adds: Pass = -inf when affordable productive members exist, energy efficiency bonus.
pub fn choose_action_v7(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }
    let dbg = std::env::var("V7_DEBUG").is_ok();
    let db = &gs.card_database;
    let my_now = if me == 0 { &gs.player1 } else { &gs.player2 };

    let base_hand_len = my_now.hand.cards.len() as i32;
    let base_passable = passable_count(gs, me, db);
    let base_ammo = lives_in_hand(my_now, db);
    let base_stage = stage_hearts_of(my_now, db);
    let base_blades = total_blades_of(my_now, gs, db);
    let my_energy = my_now.energy_zone.active_count() as i32;

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

    let mut vals: Vec<f64> = vec![f64::NEG_INFINITY; actions.len()];
    let mut dbg_lines: Vec<String> = Vec::new();

    // Pre-check: any affordable productive member exists?
    let has_affordable_productive = actions.iter().any(|a| {
        if a.action_type != ActionType::PlayMemberToStage {
            return false;
        }
        let Some(cid) = a.parameters.as_ref().and_then(|p| p.card_id) else { return false; };
        let Some(card) = db.get_card(cid) else { return false; };
        let cost = card.cost.unwrap_or(0) as i32;
        let blade = card.blade as i32;
        let hearts = card.base_heart.as_ref().map(|bh| bh.hearts.values_sum() as i32).unwrap_or(0);
        cost <= my_energy && (blade > 0 || hearts > 0)
    });

    for (i, a) in actions.iter().enumerate() {
        let mut sim = gs.clone();
        if crate::game_setup::execute_action(&mut sim, a).is_err() {
            continue;
        }
        crate::game_setup::settle_single_player_state(&mut sim);
        let my_sim = if me == 0 { &sim.player1 } else { &sim.player2 };

        let mut val = 0.0f64;
        let mut parts: Vec<String> = Vec::new();

        // 1. Passable lives delta (CRITICAL - 60x)
        let d_pass = passable_count(&sim, me, db) as f64 - base_passable as f64;
        val += 60.0 * d_pass;
        if d_pass != 0.0 {
            parts.push(format!("pass{:+}", d_pass));
        }

        // 2. Ammo (lives in hand) delta (25x)
        let ammo_after = lives_in_hand(my_sim, db);
        let d_ammo = ammo_after as f64 - base_ammo as f64;
        val += 25.0 * d_ammo;
        if d_ammo != 0.0 {
            parts.push(format!("ammo{:+}", d_ammo));
        }
        if ammo_after == 0 && base_ammo > 0 {
            val -= 120.0;
            parts.push("BURN".into());
        }

        // 3. Stage hearts delta (development)
        let d_stage = stage_hearts_of(my_sim, db) - base_stage;
        val += 3.0 * d_stage as f64;
        if d_stage != 0 {
            parts.push(format!("hearts{d_stage:+}"));
        }

        // 4. Blades delta (yell power)
        let d_blades = total_blades_of(my_sim, &sim, db) - base_blades;
        val += 6.0 * d_blades as f64;
        if d_blades != 0 {
            parts.push(format!("blades{d_blades:+}"));
        }

        // 5. Baton touch: prioritized upgrade
        if a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true) {
            val += 45.0;
            parts.push("baton+45".into());
        }

        // 6. Hand reserve penalty
        if my_sim.hand.cards.len() <= 1 {
            val -= 60.0;
            parts.push("HAND_LOW".into());
        }

        // 7. Energy efficiency bonus - reward playing within curve
        if a.action_type == ActionType::PlayMemberToStage {
            if let Some(cid) = a.parameters.as_ref().and_then(|p| p.card_id) {
                if let Some(card) = db.get_card(cid) {
                    let cost = card.cost.unwrap_or(0) as i32;
                    let blade = card.blade as i32;
                    let hearts = card.base_heart.as_ref().map(|bh| bh.hearts.values_sum() as i32).unwrap_or(0);
                    if cost <= my_energy {
                        // Efficiency: blades+hearts per energy
                        let efficiency = (blade + hearts) as f64 / (cost.max(1) as f64);
                        val += efficiency * 15.0;
                        parts.push(format!("eff{:.1}", efficiency * 15.0));
                    }
                }
            }
        }

        // 8. Starvation digging
        if base_ammo <= 1 {
            let drawn = (my_sim.hand.cards.len() as i32 - base_hand_len).max(0);
            val += 70.0 * p_life_draw * drawn as f64;
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

        // 9. NOOP breaker
        if my_sim.hand.cards.len() == my_now.hand.cards.len()
            && my_sim.energy_zone.active_count() == my_now.energy_zone.active_count()
            && my_sim.stage.stage == my_now.stage.stage
            && my_sim.main_deck.cards.len() == my_now.main_deck.cards.len()
            && my_sim.waitroom.cards.len() == my_now.waitroom.cards.len()
        {
            val -= 1000.0;
            parts.push("NOOP".into());
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
            let stage_area = a
                .parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .unwrap_or("-");
            let use_baton = a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true);
            let baton_mark = if use_baton { "[BATON]" } else { "" };
            dbg_lines.push(format!(
                "    [{}] {:?} {} area={} {} -> {}",
                i, a.action_type, card_no, stage_area, baton_mark, parts.join(" ")
            ));
        }

        vals[i] = val;
    }

    // FIXED PASS LOGIC: Pass only when NO productive play exists
    // Productive = any play with score > -100 (allowing small negative for future setup)
    let best_nonpass = vals
        .iter()
        .enumerate()
        .filter(|(i, _)| actions[*i].action_type != ActionType::Pass)
        .map(|(_, v)| *v)
        .fold(f64::NEG_INFINITY, f64::max);

    for (i, a) in actions.iter().enumerate() {
        if a.action_type == ActionType::Pass {
            // Pass = -inf if any productive play exists, else 0
            vals[i] = if best_nonpass > -100.0 || has_affordable_productive {
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
            "V7D t{} phase{:?} hand={} pass={} ammo={} blades={} CONFIRM={}\n{}",
            gs.turn_number,
            gs.current_phase,
            my_now.hand.cards.len(),
            base_passable,
            base_ammo,
            base_blades,
            chosen_confirm,
            dbg_lines.join("\n")
        );
    }
    actions[best_idx].clone()
}

/// LIVE SET: v6's proven binomial-aware portfolio with fallback logic.
pub fn choose_live_set_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    use crate::bot::strategy_v4::{alloc, hand_lives, heart_pool};
    use crate::bot::strategy_v5::{best_portfolio_scored, nearest_miss_life, player_ref};
    use crate::bot::strategy_common::emit_live_set;
    use crate::card::CardType;
    use crate::game_state::Phase;

    let me = if gs.active_player().id == gs.player1.id { 0u8 } else { 1u8 };
    let (my, opp) = player_ref(gs, me);
    let my_succ = my.success_live_card_zone.cards.len() as i32;
    let opp_succ = opp.success_live_card_zone.cards.len() as i32;

    // Get best portfolio from v5 logic (binomial-aware, score-maximizing among passers)
    let mut desired = best_portfolio_scored(gs, me, db).0;

    // ===== v6 FALLBACK LOGIC =====
    if desired.is_empty() {
        let trace = std::env::var("V7_TRACE").is_ok();
        // Free win (8.4.3.2): second attacker, opponent zone still empty — any
        // sole passer places regardless of score.
        if gs.current_phase == Phase::LiveCardSetSecondAttacker && opp.live_card_zone.cards.is_empty() {
            if let Some(hi) = cheapest_deterministic_life(gs, me, db) {
                desired.push(hi);
                if trace {
                    eprintln!("V7L t{} me{} FREE_WIN lives=1", gs.turn_number, me);
                }
                return emit_live_set(gs, actions, &desired);
            }
        }

        // Gamble: one near-miss life chosen by binomial pass probability, not
        // paper deficit. Longer odds accepted at opponent match point (folding
        // there loses outright).
        let p_floor = if opp_succ >= 2 { 0.10 } else { 0.25 };
        if let Some((p, _deficit, hi)) = nearest_miss_life(gs, me, db) {
            if p >= p_floor {
                desired.push(hi);
                if trace {
                    eprintln!("V7L t{} me{} GAMBLE p={:.2} lives=1", gs.turn_number, me, p);
                }
            }
        }

        // Junk filter: fill remaining slots with dead non-live cards. They are
        // discarded before the check (can never fail it) and each draws a
        // replacement — trading dead hand cards for fresh deck digs.
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
            if trace {
                eprintln!("V7L t{} me{} EMPTY->GAMBLE lives={} junk={} (my_succ={} opp_succ={})", 
                    gs.turn_number, me, 
                    desired.iter().filter(|&&hi| {
                        my.hand.cards.get(hi).copied().map_or(false, |cid| {
                            db.get_card(cid).map_or(false, |c| c.card_type == CardType::Live)
                        })
                    }).count(),
                    desired.len(),
                    my_succ, opp_succ);
            }
        } else if trace {
            eprintln!("V7L t{} me{} EMPTY->FOLD (deck_lives={} max_slots={} desired_len={})", 
                gs.turn_number, me, deck_lives, max_slots, desired.len());
        }
    }

    // Fill spare slots with junk draws (even when we have a portfolio)
    let max_slots = (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
    fill_junk_v7(gs, me, db, &mut desired, max_slots);

    if std::env::var("V7_TRACE").is_ok() {
        let n_lives = desired
            .iter()
            .filter(|&&hi| {
                my.hand.cards.get(hi).copied().map_or(false, |cid| {
                    db.get_card(cid).map_or(false, |c| c.card_type == CardType::Live)
                })
            })
            .count();
        eprintln!("V7L t{} me{} SET n={} lives={} junk={}", gs.turn_number, me, desired.len(), n_lives, desired.len() - n_lives);
    }

    emit_live_set(gs, actions, &desired)
}

fn cheapest_deterministic_life(gs: &GameState, me: u8, db: &CardDatabase) -> Option<usize> {
    use crate::bot::strategy_v4::{alloc, hand_lives, heart_pool};
    let my = if me == 0 { &gs.player1 } else { &gs.player2 };
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

fn fill_junk_v7(gs: &GameState, me: u8, db: &CardDatabase, desired: &mut Vec<usize>, max_slots: usize) {
    use crate::card::CardType;
    let my = if me == 0 { &gs.player1 } else { &gs.player2 };

    let deck_lives = my
        .main_deck
        .cards
        .iter()
        .filter(|&&cid| db.get_card(cid).map_or(false, |c| c.card_type == CardType::Live))
        .count();
    if desired.len() >= max_slots || deck_lives == 0 {
        return;
    }

    let budget = my.energy_zone.active_count() as i32 + 4;
    let mut junk: Vec<(usize, i32)> = my
        .hand
        .cards
        .iter()
        .enumerate()
        .filter(|&(i, &cid)| {
            !desired.contains(&i)
                && db.get_card(cid).map_or(false, |c| c.card_type != CardType::Live)
        })
        .map(|(i, &cid)| {
            let cost = db.get_card(cid).and_then(|c| c.cost).unwrap_or(0) as i32;
            (i, cost)
        })
        .collect();
    // Prioritize discarding expensive/unaffordable cards first
    junk.sort_by_key(|&(_, cost)| std::cmp::Reverse(cost.min(budget)));
    for (hi, _cost) in junk {
        if desired.len() >= max_slots {
            break;
        }
        desired.push(hi);
    }
}

/// MULLIGAN: prefer hands with early game curve (T1-2 plays).
/// Keep lives, prefer low-cost members, dump expensive non-lives.
pub fn choose_mulligan_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    use crate::bot::strategy_common::emit_mulligan;
    use crate::card::CardType;
    use crate::game_setup::ActionType;

    let me = if gs.active_player().id == gs.player1.id { 0u8 } else { 1u8 };
    let my = if me == 0 { &gs.player1 } else { &gs.player2 };

    let mut discard: Vec<usize> = Vec::new();
    let mut lives_seen = 0usize;
    let mut members: Vec<(usize, u8, i32)> = Vec::new(); // (hand_index, cost, efficiency)

    for (hand_index, &cid) in my.hand.cards.iter().enumerate() {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        match card.card_type {
            CardType::Live => {
                lives_seen += 1;
                // Only 3 lives usable per turn, discard excess
                if lives_seen > 3 {
                    discard.push(hand_index);
                }
            }
            CardType::Member => {
                let cost = card.cost.unwrap_or(0) as i32;
                let blade = card.blade as i32;
                let hearts = card.base_heart.as_ref().map(|bh| bh.hearts.values_sum() as i32).unwrap_or(0);
                // Efficiency: blades+hearts per cost - prefer early game curve
                let efficiency = (blade + hearts) as f64 / cost.max(1) as f64;
                members.push((hand_index, cost as u8, (efficiency * 100.0) as i32));
            }
            CardType::Energy => {}
        }
    }

    // Sort members by: low cost first, then high efficiency
    // This prefers T1-2 plays (cost 1-2) that can be played immediately
    members.sort_by(|a, b| {
        a.1.cmp(&b.1).then_with(|| b.2.cmp(&a.2))
    });

    // Dump the worst members first (high cost, low efficiency)
    for &(hi, _cost, _eff) in members.iter().rev() {
        if discard.len() >= 3 {
            break;
        }
        if !discard.contains(&hi) {
            discard.push(hi);
        }
    }

    emit_mulligan(gs, actions, &discard)
}