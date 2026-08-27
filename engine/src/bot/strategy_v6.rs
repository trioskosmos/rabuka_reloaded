//! Strategy bot v6 — aggressive, tempo-first development with binomial-aware
//! live sets.
//!
//! DIAGNOSIS (2026-08-27): v5 (and its v4 Main delegate) silently "did
//! nothing" on many turns. Root cause: `choose_action_v4` scores
//! `Pass` (end Main phase) at ~0, and because the selection uses a STRICT `>` the
//! bot keeps the first action it saw when values tie. A freshly drawn member
//! that does not immediately raise `passable`/`ammo` counts (e.g. a waiting
//! member, or one whose blades only matter at the yell flip) is scored ≈0 and
//! loses the tie to an earlier `Pass`, so the bot ends Main
//! having played nothing despite affordable members on board. Net effect:
//! boards never grew past a few blades, live checks never became passable,
//! and the game dragged through long no-progress stretches.
//!
//! v6 doctrine (informed by the guide sources cited in docs/BOT_STRATEGY.md):
//! - MAIN PHASE IS FOR TEMPO. Develop the board every turn. Any legal member
//!   deploy is worth more than ending the phase; `Pass` (end Main phase) is
//!   chosen ONLY when there is genuinely nothing useful to deploy or dig.
//!   Baton touches
//!   (discounted upgrades to the power piece) are prioritized.
//! - LIVE SET is binomial-aware: set the highest-scoring portfolio that PASSES
//!   the yell check (Binomial(blades, density), not the mean). Concede clearly
//!   lost checks (温存) but never fold a free win, and at match point accept
//!   longer odds. Always trade dead hand cards for fresh draws (junk filter).
//! - No hidden information is used: only own hand/deck and the public board.

use crate::bot::strategy_v4::{
    alloc, hand_lives, heart_pool, lives_in_hand, passable_count,
};
use crate::bot::strategy_v5::{best_portfolio_scored, nearest_miss_life};
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

/// MAIN PHASE: aggressive tempo-first development.
pub fn choose_action_v6(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }
    let dbg = std::env::var("V6_DEBUG").is_ok();
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

        // Doctrine 1: passable lives (placements-in-waiting).
        let d_pass = passable_count(&sim, me, db) as f64 - base_passable as f64;
        val += 60.0 * d_pass;
        if d_pass != 0.0 {
            parts.push(format!("pass{:+}", d_pass));
        }

        // Doctrine 2: ammo — lives in hand are future placements.
        let ammo_after = lives_in_hand(my_sim, db);
        let d_ammo = ammo_after as f64 - base_ammo as f64;
        val += 25.0 * d_ammo;
        if d_ammo != 0.0 {
            parts.push(format!("ammo{:+}", d_ammo));
        }
        if ammo_after == 0 && base_ammo > 0 {
            val -= 120.0; // never burn the whole arsenal
            parts.push("BURN".into());
        }

        // Development in HEARTS and BLADES. Blades are the engine of the yell
        // flip: every extra blade is a fresh Binomial trial that can supply the
        // hearts a check needs, so board growth compounds into live-set power.
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

        // Baton touch: discounted upgrade of the power piece (guide curve
        // 4->9->13). Strongly favored.
        if a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true) {
            val += 45.0;
            parts.push("baton+45".into());
        }

        // Hand reserve: a 1-card hand can neither set lives nor pay costs.
        if my_sim.hand.cards.len() <= 1 {
            val -= 60.0;
        }

        // Life acquisition when starved: drawing digs toward the next life;
        // milling to waitroom banks lives for retrieval engines.
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

        // No-op breaker: an action that changes nothing useful is worthless.
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

    // v6 fix: `Pass` ends the development phase. It is chosen ONLY when there
    // is no USEFUL deploy available. A "useful" deploy is any non-Pass action
    // with value > 0 (adds hearts/blades, raises a passable life, banks ammo,
    // or draws). This kills the do-nothing turns without clogging the 5-slot
    // stage with zero-contribution waiting members (which the old flat -2000
    // penalty wrongly forced us to play).
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
            "V6D t{} phase{:?} hand={} pass={} ammo={} blades={} CONFIRM={}\n{}",
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

/// LIVE SET: binomial-aware, score-maximizing among passers, with free-win and
/// gamble fallbacks and the junk-draw filter.
pub fn choose_live_set_v6(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let me = if gs.active_player().id == gs.player1.id { 0u8 } else { 1u8 };
    let (my, opp) = player_ref(gs, me);
    let my_succ = my.success_live_card_zone.cards.len() as i32;
    let opp_succ = opp.success_live_card_zone.cards.len() as i32;
    let mut desired = best_portfolio(gs, me, db);
    let my_score: i32 = desired
        .iter()
        .filter_map(|&hi| my.hand.cards.get(hi).copied())
        .filter_map(|cid| db.get_card(cid))
        .map(|c| c.score.unwrap_or(0) as i32)
        .sum();

    if desired.is_empty() {
        // Free win (8.4.3.2): second attacker, opponent zone still empty — any
        // sole passer places regardless of score.
        if gs.current_phase == Phase::LiveCardSetSecondAttacker && opp.live_card_zone.cards.is_empty()
        {
            if let Some(hi) = cheapest_deterministic_life(gs, me, db) {
                desired.push(hi);
                return emit(gs, actions, &desired);
            }
        }

        // Gamble: one near-miss life chosen by binomial pass probability, not
        // paper deficit. Longer odds accepted at opponent match point (folding
        // there loses outright).
        let p_floor = if opp_succ >= 2 { 0.10 } else { 0.25 };
        if let Some((p, _deficit, hi)) = nearest_miss_life(gs, me, db) {
            if p >= p_floor {
                desired.push(hi);
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
        }
        if std::env::var("V6_TRACE").is_ok() {
            let n_lives = desired
                .iter()
                .filter(|&&hi| {
                    my.hand.cards.get(hi).copied().map_or(false, |cid| {
                        db.get_card(cid).map_or(false, |c| c.card_type == CardType::Live)
                    })
                })
                .count();
            eprintln!(
                "V6L t{} me{} EMPTY->{} lives={} junk={} (my_succ={} opp_succ={})",
                gs.turn_number,
                me,
                if desired.is_empty() { "FOLD" } else { "GAMBLE" },
                n_lives,
                desired.len() - n_lives,
                my_succ,
                opp_succ
            );
        }
    } else if std::env::var("V6_TRACE").is_ok() {
        eprintln!(
            "V6L t{} me{} SET n={} score={}",
            gs.turn_number,
            me,
            desired.len(),
            my_score
        );
    }

    emit(gs, actions, &desired)
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

fn best_portfolio(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<usize> {
    best_portfolio_scored(gs, me, db).0
}

fn emit(gs: &GameState, actions: &[Action], desired: &[usize]) -> Action {
    crate::bot::strategy_common::emit_live_set(gs, actions, desired)
}

pub fn choose_mulligan_v6(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    crate::bot::strategy_v4::choose_mulligan_v4(gs, actions, db)
}
