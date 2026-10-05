use crate::bot::strategy_common::{emit_live_set, Acc, Pool};
use crate::card::{BaseHeart, CardDatabase, CardType, HeartMap};
use crate::game_setup::Action;
use crate::game_state::{GameState, Phase};

pub const SAMPLES: usize = 256;

#[derive(Clone, Debug, PartialEq)]
pub struct LiveCandidate {
    pub hand_indices: Vec<usize>,
    /// `Acc` and `Pool`, not `[i32; 11]` / `[i32; 8]`. Candidates are held in a
    /// `Vec` and sorted by score, so shrinking the struct shrinks the sort's
    /// working set: `LiveCandidate` drops from 108 to 62 bytes on these two
    /// fields alone.
    pub need: Acc,
    pub board: Pool,
    pub blades: usize,
    pub sampled_draws: usize,
    pub samples: usize,
    pub board_pass: bool,
    pub heart_pass_probability: f64,
    pub printed_score: i32,
    pub pass_score_proxy: f64,
    pub placement_upper_bound: f64,
}

pub fn feasible(pool: &Pool, need: &Acc) -> bool {
    if need[8] > 0 || need[9] > 0 {
        return false;
    }
    // `saturating_*` throughout: every term is a per-colour heart total, and a
    // wrapping addition here would turn a shortfall into a surplus and report
    // an unsatisfiable live set as passable.
    let mut wild = pool[7];
    let mut bucket = pool[0];
    for color in 1..=6 {
        let surplus = pool[color].saturating_sub(need[color]);
        if surplus >= 0 {
            bucket = bucket.saturating_add(surplus);
        } else {
            wild = wild.saturating_add(surplus);
            if wild < 0 {
                return false;
            }
        }
    }
    bucket.saturating_add(wild)
        >= need[0].saturating_add(need[7]).saturating_add(need[10])
}

pub fn sample_pools(mut deck: Vec<Pool>, blades: usize, board: Pool) -> Vec<Pool> {
    deck.sort_unstable();
    let draws = blades.min(deck.len());
    if draws == 0 {
        return vec![board];
    }
    let mut rng = 0x6a09e667f3bcc909u64;
    let mut pools = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let mut sampled = deck.clone();
        let mut pool = board;
        for k in 0..draws {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let offset = usize::try_from(rng % (deck.len() - k) as u64)
                .expect("sample offset fits deck length");
            sampled.swap(k, k + offset);
            for (have, extra) in pool.iter_mut().zip(sampled[k]) {
                *have = have.saturating_add(extra);
            }
        }
        pools.push(pool);
    }
    pools
}

pub fn candidate_metrics(gs: &GameState, db: &CardDatabase) -> Vec<LiveCandidate> {
    let my = gs.active_player();
    let max_slots = usize::from(3u8.saturating_sub(my.live_card_set_limit_reduction));
    if max_slots == 0 || gs.cannot_live_players.contains(&my.id) {
        return Vec::new();
    }
    let mut board: Pool = [0; 8];
    let hearts = my.stage.get_available_hearts(
        db, &gs.mods.heart_override, &gs.mods.heart_modifiers,
        &gs.mods.heart_color_multiplier, &gs.mods.heart_copy,
    );
    for (color, count) in &hearts.hearts {
        if color.index() < board.len() {
            board[color.index()] = board[color.index()].saturating_add(i16::from(*count));
        }
    }
    let blades = usize::from(my.stage.total_blades(
        db, &gs.mods.blade_modifiers, &gs.mods.orientation_modifiers, false,
    ));
    let override_color = my.stage.stage.iter().find_map(|cid| {
        gs.mods.blade_type_modifiers.get(cid).copied()
            .map(crate::turn::live::blade_color_to_heart)
    });
    let deck: Vec<Pool> = my.main_deck.cards.iter().map(|&cid| {
        // `process_yell_revealed_card_icons` writes `[u8; 8]`; widen once, here.
        let mut total = [0u8; 8];
        if let Some(card) = db.get_card(cid) {
            let mut scratch = BaseHeart { hearts: HeartMap::new() };
            let mut cheer = 0;
            crate::turn::live::process_yell_revealed_card_icons(
                card, override_color, &mut scratch, &mut total, &mut cheer,
            );
        }
        total.map(i16::from)
    }).collect();
    let sampled_draws = blades.min(deck.len());
    let pools = sample_pools(deck, blades, board);
    let lives: Vec<(usize, i32, Acc)> = my.hand.cards.iter().enumerate()
        .filter_map(|(hi, &cid)| {
            let card = db.get_card(cid)?;
            if card.card_type != CardType::Live {
                return None;
            }
            let mut need: Acc = [0; 11];
            if let Some(effective) = crate::core::stats_pipeline::effective_need_heart(
                card.need_heart.as_ref(), cid, &gs.mods.need_heart_modifiers,
            ) {
                for (color, count) in &effective.hearts {
                    need[color.index()] =
                        need[color.index()].saturating_add(i16::from(*count));
                }
            }
            Some((hi, i32::from(card.score.unwrap_or(0)), need))
        }).collect();
    let mut sets = Vec::new();
    for a in 0..lives.len() {
        sets.push(vec![a]);
        if max_slots < 2 {
            continue;
        }
        for b in a + 1..lives.len() {
            sets.push(vec![a, b]);
            if max_slots >= 3 {
                for c in b + 1..lives.len() {
                    sets.push(vec![a, b, c]);
                }
            }
        }
    }
    sets.into_iter().map(|indices| {
        let mut need = [0; 11];
        let mut printed_score = 0;
        let mut hand_indices = Vec::with_capacity(indices.len());
        for index in indices {
            let (hi, score, requirements) = lives[index];
            hand_indices.push(hi);
            printed_score += score;
            for (total, extra) in need.iter_mut().zip(requirements) {
                *total += extra;
            }
        }
        let hits = pools.iter().filter(|pool| feasible(pool, &need)).count();
        let p = hits as f64 / pools.len() as f64;
        LiveCandidate {
            hand_indices, need, board, blades, sampled_draws, samples: pools.len(),
            board_pass: feasible(&board, &need), heart_pass_probability: p,
            printed_score, pass_score_proxy: p * f64::from(printed_score),
            placement_upper_bound: p,
        }
    }).collect()
}

pub fn select_candidate(candidates: &[LiveCandidate], free: bool, contested: bool, floor: f64, gamble: f64) -> Option<usize> {
    if free {
        if let Some((index, _)) = candidates.iter().enumerate()
            .filter(|(_, c)| c.hand_indices.len() == 1 && c.board_pass)
            .min_by_key(|(_, c)| (c.printed_score, c.hand_indices[0])) {
            return Some(index);
        }
    }
    let mut eligible: Vec<usize> = candidates.iter().enumerate()
        .filter(|(_, c)| c.heart_pass_probability >= floor && (contested || c.hand_indices.len() == 1))
        .map(|(i, _)| i).collect();
    if eligible.is_empty() {
        eligible = candidates.iter().enumerate()
            .filter(|(_, c)| c.hand_indices.len() == 1 && c.heart_pass_probability >= gamble)
            .map(|(i, _)| i).collect();
    }
    eligible.into_iter().max_by(|&a, &b| {
        let a = &candidates[a];
        let b = &candidates[b];
        let primary = if contested {
            a.pass_score_proxy.total_cmp(&b.pass_score_proxy)
        } else {
            a.heart_pass_probability.total_cmp(&b.heart_pass_probability)
        };
        primary.then_with(|| a.heart_pass_probability.total_cmp(&b.heart_pass_probability))
            .then_with(|| b.hand_indices.len().cmp(&a.hand_indices.len()))
            .then_with(|| a.printed_score.cmp(&b.printed_score))
            .then_with(|| b.hand_indices.cmp(&a.hand_indices))
    })
}

pub fn choose_live_set(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let my = gs.active_player();
    let opp = if my.id == gs.player1.id { &gs.player2 } else { &gs.player1 };
    let second = gs.current_phase == Phase::LiveCardSetSecondAttacker;
    let free = second && opp.live_card_zone.cards.is_empty();
    let contested = !free && ((second && !opp.live_card_zone.cards.is_empty())
        || opp.success_live_card_zone.cards.len() >= 2);
    let floor = if opp.success_live_card_zone.cards.len() >= 2 { 0.35 }
        else if my.success_live_card_zone.cards.len() >= 2 { 0.60 } else { 0.45 };
    let gamble = if opp.success_live_card_zone.cards.len() >= 2 { 0.10 } else { 0.25 };
    let candidates = candidate_metrics(gs, db);
    let selected = select_candidate(&candidates, free, contested, floor, gamble);
    let mut desired = selected.map(|i| candidates[i].hand_indices.clone()).unwrap_or_default();
    let max_slots = usize::from(3u8.saturating_sub(my.live_card_set_limit_reduction));
    if !gs.cannot_live_players.contains(&my.id)
        && !(free && selected.is_some_and(|i| candidates[i].board_pass))
        && my.main_deck.cards.iter().any(|&cid| db.get_card(cid).is_some_and(|c| c.card_type == CardType::Live)) {
        let mut junk: Vec<(usize, u8)> = my.hand.cards.iter().enumerate()
            .filter_map(|(hi, &cid)| {
                let card = db.get_card(cid)?;
                (card.card_type != CardType::Live).then_some((hi, card.cost.unwrap_or(0)))
            }).collect();
        junk.sort_by_key(|&(hi, cost)| (std::cmp::Reverse(cost), hi));
        for (hi, _) in junk {
            if desired.len() >= max_slots { break; }
            desired.push(hi);
        }
    }
    for (index, candidate) in candidates.iter().enumerate() {
        log::debug!(
            "v7_live candidate turn={} player={} second={} opponent_set_count={} contested={} floor={} selected={} metrics={:?} model=static_hearts_without_refresh_or_trigger_projection score_model=printed_only win_probability=unmodeled",
            gs.turn_number, my.id, second, opp.live_card_zone.cards.len(), contested,
            floor, selected == Some(index), candidate,
        );
    }
    log::debug!("v7_live decision turn={} player={} selected={:?} set_indices={:?} max_placements=1", gs.turn_number, my.id, selected, desired);
    emit_live_set(gs, actions, &desired)
}
