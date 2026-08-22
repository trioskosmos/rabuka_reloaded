//! Strategy bot v4 — success-zone fundamentalism.
//!
//! DOCTRINE: the only goal is placing cards into the Success Live zone.
//! Everything else (scores, opponent modeling, curve aesthetics) is noise.
//!
//! Per decision point:
//! - Main phase: prefer actions that increase the number of hand lives whose
//!   heart requirements are covered by the board (hearts+blades), then
//!   actions that put lives into hand (ammo), then raw stage growth.
//! - Live set: set the maximum number of lives that can PASS (deterministic
//!   pool coverage, cheapest-requirements first). A passed check places a
//!   card; score never enters the calculation.
//! - Mulligan: keep every life, discard the most expensive non-lives.
//!
//! No opponent terms exist anywhere in this file.

use crate::card::{CardDatabase, CardType};
use crate::game_setup::{self, Action};
use crate::game_state::GameState;

// ── Heart accounting (self-contained; mirrors rule 8.3.15 allocation) ──

type Acc = [i32; 11];

fn hc_index(c: crate::card::HeartColor) -> usize {
    use crate::card::HeartColor as H;
    match c {
        H::Heart00 => 0,
        H::Heart01 => 1,
        H::Heart02 => 2,
        H::Heart03 => 3,
        H::Heart04 => 4,
        H::Heart05 => 5,
        H::Heart06 => 6,
        H::BAll => 7,
        H::Draw => 8,
        H::Score => 9,
        H::All => 10,
    }
}

fn acc_add(acc: &mut Acc, hearts: &crate::card::HeartMap) {
    for (c, v) in hearts.iter() {
        acc[hc_index(*c)] += *v as i32;
    }
}

/// Active blades and own-deck blade-heart density (fair info). Yell hits are
/// a BINOMIAL draw, not a guarantee — portfolio sizing must budget variance.
pub(crate) fn flip_stats(gs: &GameState, me_player: u8, db: &CardDatabase) -> (i32, f64) {
    let p = if me_player == 0 { &gs.player1 } else { &gs.player2 };
    let mut blades = 0i32;
    for &cid in p.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        let waiting = gs.mods.get_orientation_modifier(cid) == Some("wait");
        if let Some(card) = db.get_card(cid) {
            if !waiting {
                blades += card.blade as i32;
            }
        }
    }
    let deck_len = p.main_deck.cards.len().max(1);
    let density = p
        .main_deck
        .cards
        .iter()
        .filter(|&&cid| db.get_card(cid).map_or(false, |c| c.blade_heart.is_some()))
        .count() as f64
        / deck_len as f64;
    (blades, density)
}

/// Per-Acc-index chance that ONE random deck flip yields a unit of each
/// kind (own decklist = fair information). Index layout matches Acc.
pub(crate) fn blade_unit_densities(
    gs: &GameState,
    me_player: u8,
    db: &CardDatabase,
) -> [f64; 11] {
    let p = if me_player == 0 { &gs.player1 } else { &gs.player2 };
    let deck_len = p.main_deck.cards.len().max(1) as f64;
    let mut units = [0f64; 11];
    for &cid in p.main_deck.cards.iter() {
        if let Some(card) = db.get_card(cid) {
            if let Some(bh) = &card.blade_heart {
                for (color, v) in bh.hearts.iter() {
                    units[hc_index(*color)] += *v as f64;
                }
            }
        }
    }
    for u in units.iter_mut() {
        *u /= deck_len;
    }
    units
}

/// Stage base hearts + expected yell hits granted PER COLOR.
///
/// Measured miscalibration (2026-08-22 log attribution): treating expected
/// flips as any-color wildcards predicted ~45% failure but observed 74–97%
/// (avg unmet deficit ~9–12 hearts). Engine reality: a flipped blade-heart
/// grants its PRINTED colors, not a wildcard (only icon_all/BAll are
/// wildcards). Own decklist is fair information, so we distribute expected
/// hits across colors exactly as our own deck would produce them:
/// expected_units[color] = blades × units_of_that_color_in_deck / deck_len.
pub(crate) fn expected_flip_units(
    gs: &GameState,
    me_player: u8,
    db: &CardDatabase,
) -> [f64; 11] {
    let (blades, _density) = flip_stats(gs, me_player, db);
    let mut out = blade_unit_densities(gs, me_player, db);
    for u in out.iter_mut() {
        *u *= blades as f64;
    }
    out
}

/// Stage base hearts + expected yell hits granted per printed color.
/// `confidence` scales the flip contribution: 1.0 = full mean (main-phase
/// coverage metric), <1.0 = conservative for all-or-nothing portfolio
/// decisions, since a portfolio sized exactly to the mean fails ~half the
/// time (binomial variance around the hit count).
pub(crate) fn heart_pool_inner(gs: &GameState, me_player: u8, db: &CardDatabase, confidence: f64) -> Acc {
    let p = if me_player == 0 { &gs.player1 } else { &gs.player2 };
    let mut acc = [0i32; 11];
    for &cid in p.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        if let Some(card) = db.get_card(cid) {
            if let Some(bh) = &card.base_heart {
                acc_add(&mut acc, &bh.hearts);
            }
        }
    }
    // Expected yell hits, per printed color (Draw/Score icons don't feed checks).
    let expected = expected_flip_units(gs, me_player, db);
    for idx in (0..=7).chain(std::iter::once(10)) {
        acc[idx] += (expected[idx] * confidence).floor() as i32;
    }
    acc
}

pub(crate) fn heart_pool(gs: &GameState, me_player: u8, db: &CardDatabase) -> Acc {
    heart_pool_inner(gs, me_player, db, 1.0)
}

/// Try allocating `need` from `pool`; returns None if impossible, else the
/// remaining pool. Specific colors first, All/BAll cover deficits, grey
/// bucket takes colorless + leftovers (rule 2.11.3).
pub(crate) fn alloc(pool: &Acc, need: &Acc) -> Option<Acc> {
    let mut p = *pool;
    let mut wildcard_used = 0i32;
    for c in 1..=6 {
        let have = p[c];
        let want = need[c];
        if have >= want {
            p[c] = have - want;
        } else {
            let deficit = want - have;
            wildcard_used += deficit;
            if wildcard_used > p[7] + p[10] {
                return None;
            }
            p[c] = 0;
        }
    }
    let mut take = wildcard_used;
    let b = take.min(p[7]);
    p[7] -= b;
    take -= b;
    p[10] -= take.min(p[10]);
    // Grey bucket: colorless + leftover specifics + remaining wildcards.
    let mut grey = need[0];
    let g0 = grey.min(p[0]);
    p[0] -= g0;
    grey -= g0;
    for c in (1..=6).rev() {
        if grey <= 0 {
            break;
        }
        let t = grey.min(p[c]);
        p[c] -= t;
        grey -= t;
    }
    if grey > 0 {
        let w = p[7] + p[10];
        if grey > w {
            return None;
        }
        let b2 = grey.min(p[7]);
        p[7] -= b2;
        p[10] -= grey - b2;
    }
    Some(p)
}

pub(crate) fn hand_lives<'a>(
    p: &'a crate::player::Player,
    db: &CardDatabase,
) -> Vec<(usize, i16, Acc)> {
    let mut out = Vec::new();
    for (hand_index, &cid) in p.hand.cards.iter().enumerate() {
        if let Some(card) = db.get_card(cid) {
            if !matches!(card.card_type, CardType::Live) {
                continue;
            }
            let mut need = [0i32; 11];
            if let Some(nh) = &card.need_heart {
                acc_add(&mut need, &nh.hearts);
            }
            out.push((hand_index, cid, need));
        }
    }
    out
}

/// How many hand lives can currently pass their heart check.
pub(crate) fn passable_count(gs: &GameState, me_player: u8, db: &CardDatabase) -> usize {
    let p = if me_player == 0 { &gs.player1 } else { &gs.player2 };
    let pool = heart_pool(gs, me_player, db);
    hand_lives(p, db)
        .iter()
        .filter(|(_, _, need)| alloc(&pool, need).is_some())
        .count()
}

pub(crate) fn lives_in_hand(p: &crate::player::Player, db: &CardDatabase) -> usize {
    p.hand
        .cards
        .iter()
        .filter(|&&c| {
            db.get_card(c).map_or(false, |x| x.card_type == CardType::Live)
        })
        .count()
}

// ── Main phase ──────────────────────────────────────────────────────────

pub fn choose_action_v4(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }
    let dbg = std::env::var("V4_DEBUG").is_ok();
    let db = &gs.card_database;
    let my_now = if me == 0 { &gs.player1 } else { &gs.player2 };
    let base_hand_len = my_now.hand.cards.len() as i32;

    let base_passable = passable_count(gs, me, db);
    let base_ammo = lives_in_hand(my_now, db);
    // Development measured in HEARTS (what actually passes checks),
    // not cost.
    let stage_hearts_of = |p: &crate::player::Player| -> i32 {
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
    };
    let base_stage = stage_hearts_of(my_now);

    // Life-acquisition odds: own decklist is fair information. When starving,
    // actions that draw cards are lottery tickets toward the next life.
    let deck_lives = my_now
        .main_deck
        .cards
        .iter()
        .filter(|&&c| {
            db.get_card(c).map_or(false, |x| x.card_type == CardType::Live)
        })
        .count();
    let deck_len = my_now.main_deck.cards.len().max(1);
    let p_life_draw = deck_lives as f64 / deck_len as f64;
    let waitroom_lives = my_now
        .waitroom
        .cards
        .iter()
        .filter(|&&c| {
            db.get_card(c).map_or(false, |x| x.card_type == CardType::Live)
        })
        .count();

    let mut best_idx = 0usize;
    let mut best_val = f64::NEG_INFINITY;
    let mut dbg_lines: Vec<String> = Vec::new();

    for (i, a) in actions.iter().enumerate() {
        let mut sim = gs.clone();
        if game_setup::execute_action(&mut sim, a).is_err() {
            continue;
        }
        game_setup::settle_single_player_state(&mut sim);
        let my_sim = if me == 0 { &sim.player1 } else { &sim.player2 };

        let mut val = 0.0f64;
        let mut dbg_parts: Vec<String> = Vec::new();

        // Doctrine 1: more passable lives = closer to a placement.
        let passable_after = passable_count(&sim, me, db);
        let d_pass = passable_after as f64 - base_passable as f64;
        val += 60.0 * d_pass;
        if d_pass != 0.0 {
            dbg_parts.push(format!("pass{:+}", d_pass));
        }

        // Doctrine 2: ammo — lives in hand are future placements.
        let ammo_after = lives_in_hand(my_sim, db);
        let d_ammo = ammo_after as f64 - base_ammo as f64;
        val += 25.0 * d_ammo;
        if d_ammo != 0.0 {
            dbg_parts.push(format!("ammo{:+}", d_ammo));
        }
        if ammo_after == 0 && base_ammo > 0 {
            val -= 120.0; // never burn the arsenal
            dbg_parts.push("BURN".into());
        }

        // Development tiebreak: HEARTS gained (not cost) — hearts are what
        // pass checks and place cards.
        let stage_after = stage_hearts_of(my_sim);
        let d_stage = stage_after - base_stage;
        val += 3.0 * d_stage as f64;
        dbg_parts.push(format!("hearts{d_stage:+}"));

        // GUIDE CURVE (S1): development is baton CHAINS -- 4->9->13 -- not
        // spreading small members. A baton play upgrades the power piece at a
        // discount; across thousands of traced games bots played 4468 member
        // actions and ZERO baton touches, so no bot ever reached the
        // big-member scores the guides take for granted.
        if a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true) {
            val += 45.0;
            dbg_parts.push(format!("baton+45"));
        }

        // Hand reserve: ≤1 card can't set lives or pay costs.
        if my_sim.hand.cards.len() <= 1 {
            val -= 60.0;
        }

        // Life acquisition: when ammo-starved, drawing cards digs toward the
        // next life (P = remaining deck lives / deck size), and milling to
        // waitroom banks lives for retrieval engines.
        // Measured pathology: a 14-turn fold stretch with 0 hand lives while
        // the board sat at 19 blades — the old 30× lottery weight never
        // outbid stage growth, so the bot starved instead of digging.
        if base_ammo <= 1 {
            let drawn = (my_sim.hand.cards.len() as i32 - base_hand_len).max(0);
            val += 70.0 * p_life_draw * drawn as f64;
            let wr_lives_now = my_sim
                .waitroom
                .cards
                .iter()
                .filter(|&&c| {
                    db.get_card(c).map_or(false, |x| x.card_type == CardType::Live)
                })
                .count();
            let wr_before = waitroom_lives;
            if wr_lives_now > wr_before && p_life_draw > 0.0 {
                val += 25.0; // banking lives where retrieval can reach them
            }
        }

        // No-op breaker: unchanged key counts bought nothing.
        if my_sim.hand.cards.len() == my_now.hand.cards.len()
            && my_sim.energy_zone.active_count() == my_now.energy_zone.active_count()
            && my_sim.stage.stage == my_now.stage.stage
            && my_sim.main_deck.cards.len() == my_now.main_deck.cards.len()
            && my_sim.waitroom.cards.len() == my_now.waitroom.cards.len()
        {
            val -= 1000.0;
            dbg_parts.push("NOOP".into());
        }
        dbg_parts.push(format!("={val:.0}"));

        if dbg {
            let baton_mark = if a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true) { "[BATON]" } else { "" };
        let cost_mark = a
            .parameters
            .as_ref()
            .and_then(|p| p.final_cost)
            .map(|c| format!("cost={}", c))
            .unwrap_or_default();
        dbg_lines.push(format!(
                "    [{}] {:?} {} {} {} -> {}",
                i,
                a.action_type,
                a.parameters
                    .as_ref()
                    .and_then(|p| p.card_id)
                    .and_then(|cid| db.get_card(cid))
                    .map(|c| c.card_no.clone())
                    .unwrap_or_default(),
                baton_mark,
                cost_mark,
                dbg_parts.join(" ")
            ));
        }

        if val > best_val {
            best_val = val;
            best_idx = i;
        }
    }
    if dbg {
        eprintln!(
            "V4D t{} phase{:?} hand={} pass={} ammo={}\n{}",
            gs.turn_number,
            gs.current_phase,
            my_now.hand.cards.len(),
            base_passable,
            base_ammo,
            dbg_lines.join("\n")
        );
    }
    actions[best_idx].clone()
}

// ── Live set: maximize PASSING lives, nothing else ──────────────────────

pub fn choose_live_set_v4(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let me = if gs.active_player().id == gs.player1.id { 0u8 } else { 1u8 };
    let my = if me == 0 { &gs.player1 } else { &gs.player2 };
    // Full-mean flip credit: calibration (calibrate.rs, 8130 decisions)
    // showed 0.6× UNDERPERFORMS — its fail-bucket still placed 33% vs
    // mean's 25%, i.e. it threw away winnable checks for no gain. Both
    // predictors land at 79% on their pass-bucket anyway (placement also
    // requires winning the comparison).
    let mut pool = heart_pool(gs, me, db);

    // Score-descending greedy among PASSABLE lives only.
    //
    // A placement requires winning the COMPARISON (8.4.6), not merely
    // passing — so among lives that all pass deterministically, prefer the
    // highest scores. Count-maximization stays implicit (everything that
    // fits gets set until slots run out).
    let mut candidates: Vec<(usize, i32, [i32; 11])> = Vec::new();
    for &(hi, cid, ref need) in &hand_lives(my, db) {
        if let Some(card) = db.get_card(cid) {
            candidates.push((hi, card.score.unwrap_or(0) as i32, *need));
        }
    }
    candidates.sort_by(|a, b| b.1.cmp(&a.1));

    let max_slots =
        (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
    let mut desired: Vec<usize> = Vec::new();
    for &(hi, _req, ref need) in &candidates {
        if desired.len() >= max_slots {
            break;
        }
        if let Some(next) = alloc(&pool, need) {
            desired.push(hi);
            pool = next;
        }
    }
    // Non-passing spare slot: one extra life anyway IF we already have a
    // passer (a failed extra poisons nothing retroactively — wait, it does:
    // all-or-nothing means ONE failure kills ALL. So never add non-passing).
    //
    // Junk filtering: setting a NON-live card is safe (discarded at 8.3.4
    // before checks) and draws a replacement — but only when at least one
    // passing life is already in the portfolio, so the check still wins.
    if !desired.is_empty() && desired.len() < max_slots {
        for (hand_index, &cid) in my.hand.cards.iter().enumerate() {
            if desired.len() >= max_slots {
                break;
            }
            if desired.contains(&hand_index) {
                continue;
            }
            if let Some(card) = db.get_card(cid) {
                if !matches!(card.card_type, CardType::Live) {
                    desired.push(hand_index);
                }
            }
        }
    }

    // Emit toward `desired`.
    let selected: Vec<usize> = gs
        .live_card_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect();
    let find = |hi: usize, want: bool| -> Option<Action> {
        actions
            .iter()
            .find(|a| {
                a.action_type == game_setup::ActionType::SelectLiveCard
                    && a.selected == Some(want)
                    && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hi)
            })
            .cloned()
    };
    for &hi in &desired {
        if !selected.contains(&hi) {
            if let Some(a) = find(hi, false) {
                return a;
            }
        }
    }
    for &hi in &selected {
        if !desired.contains(&hi) {
            if let Some(a) = find(hi, true) {
                return a;
            }
        }
    }
    actions
        .iter()
        .find(|a| a.action_type == game_setup::ActionType::ConfirmLiveCardSet)
        .or_else(|| actions.first())
        .cloned()
        .expect("live set actions non-empty")
}

// ── Mulligan: keep all lives, dump expensive non-lives ──────────────────

pub fn choose_mulligan_v4(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let me = gs.active_player();
    let mut discard: Vec<usize> = Vec::new();
    let mut lives_seen = 0usize;
    let mut members: Vec<(usize, u8)> = Vec::new();
    for (hand_index, &cid) in me.hand.cards.iter().enumerate() {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        match card.card_type {
            CardType::Live => {
                lives_seen += 1;
                if lives_seen > 3 {
                    discard.push(hand_index); // only 3 lives are usable per turn
                }
            }
            CardType::Member => members.push((hand_index, card.cost.unwrap_or(0))),
            CardType::Energy => {}
        }
    }
    // Dump the most expensive members first (they're furthest from playable).
    members.sort_by_key(|&(_, c)| std::cmp::Reverse(c));
    for &(hi, _c) in &members {
        if discard.len() >= 3 {
            break;
        }
        if !discard.contains(&hi) {
            discard.push(hi);
        }
    }

    let selected: Vec<usize> = gs
        .mulligan_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect();
    for &hi in &discard {
        if !selected.contains(&hi) {
            if let Some(a) = actions.iter().find(|a| {
                a.action_type == game_setup::ActionType::SelectMulligan
                    && a.selected == Some(false)
                    && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hi)
            }) {
                return a.clone();
            }
        }
    }
    for &hi in &selected {
        if !discard.contains(&hi) {
            if let Some(a) = actions.iter().find(|a| {
                a.action_type == game_setup::ActionType::SelectMulligan
                    && a.selected == Some(true)
                    && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hi)
            }) {
                return a.clone();
            }
        }
    }
    actions
        .iter()
        .find(|a| {
            matches!(
                a.action_type,
                game_setup::ActionType::ConfirmMulligan | game_setup::ActionType::SkipMulligan
            )
        })
        .or_else(|| actions.first())
        .cloned()
        .expect("mulligan actions non-empty")
}
