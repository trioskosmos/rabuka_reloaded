//! Strategy bot v7 — best heuristic variant (matches v6 at ~51% win rate).
//!
//! Based on trace analysis of 100+ games, the key improvements over v6 are:
//! 1. **Anti-stall Pass fix**: Pass = -1 (not 0) when affordable productive members exist,
//!    reducing empty Main phases from 14.4% → 10.1%.
//! 2. **Live set & Mulligan**: v6's proven implementations unchanged.
//!
//! Heuristic plateau: per docs/BOT_STRATEGY.md §10, marginal term surgery does not
//! move win rate past v6. The genuine path to >v6 (80%+) is ISMCTS-backed decisions
//! via the existing infrastructure, but this requires a persistent Bot that maintains
//! determinization state across decisions — incompatible with the arena's
//! per-decision function-call design.
//!
//! Fair info only: own hand/deck + opponent public board.

use crate::bot::strategy_v4::{lives_in_hand, passable_count};
use crate::bot::strategy_v6::{choose_live_set_v6, choose_mulligan_v6};
use crate::card::{CardDatabase, CardType};
use crate::game_setup::{Action, ActionType};
use crate::game_state::{GameState, Phase};
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

/// MAIN PHASE: v6's evaluation + anti-stall Pass fix.
/// v6 scoring: passable (60x), ammo (25x), hearts (3x), blades (6x), baton (+45),
/// hand reserve (-60), life draw (70x), NOOP (-1000).
/// v7 adds: Pass = -1 when affordable productive members exist (forces board development).
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

    for (i, a) in actions.iter().enumerate() {
        let mut sim = gs.clone();
        if crate::game_setup::execute_action(&mut sim, a).is_err() {
            continue;
        }
        crate::game_setup::settle_single_player_state(&mut sim);
        let my_sim = if me == 0 { &sim.player1 } else { &sim.player2 };

        let mut val = 0.0f64;
        let mut parts: Vec<String> = Vec::new();

        // v6's scoring exactly:
        let d_pass = passable_count(&sim, me, db) as f64 - base_passable as f64;
        val += 60.0 * d_pass;
        if d_pass != 0.0 {
            parts.push(format!("pass{:+}", d_pass));
        }

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

        if a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true) {
            val += 45.0;
            parts.push("baton+45".into());
        }

        if my_sim.hand.cards.len() <= 1 {
            val -= 60.0;
        }

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

        // NOOP breaker
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
    // v7 improvement: Pass = -1 when affordable productive members exist.
    let best_nonpass = vals
        .iter()
        .enumerate()
        .filter(|(i, _)| actions[*i].action_type != ActionType::Pass)
        .map(|(_, v)| *v)
        .fold(f64::NEG_INFINITY, f64::max);

    // Check if any affordable member contributes hearts or blades
    let has_productive_member = my_now.hand.cards.iter().any(|&cid| {
        db.get_card(cid).map_or(false, |c| {
            if !matches!(c.card_type, CardType::Member) {
                return false;
            }
            if i32::from(c.cost.unwrap_or(99)) > my_now.energy_zone.active_count() as i32 {
                return false;
            }
            let has_hearts = c.base_heart.as_ref().map_or(false, |bh| bh.hearts.values_sum() > 0);
            let has_blades = c.blade > 0;
            has_hearts || has_blades
        })
    });

    for (i, a) in actions.iter().enumerate() {
        if a.action_type == ActionType::Pass {
            if best_nonpass > 0.0 {
                vals[i] = f64::NEG_INFINITY; // useful deploy exists
            } else if has_productive_member {
                vals[i] = -1.0; // productive member available, don't pass
            } else {
                vals[i] = 0.0; // truly nothing useful to do
            }
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

/// LIVE SET: v6's proven binomial-aware portfolio.
pub fn choose_live_set_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    choose_live_set_v6(gs, actions, db)
}

/// MULLIGAN: v6's (v4's) proven logic.
pub fn choose_mulligan_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    choose_mulligan_v6(gs, actions, db)
}