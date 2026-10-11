//! AZALEA deck specialist — the most-guided archetype in Loveca.
//!
//! Five public guides cover this deck, more than any other
//! archetype and the only one with four independent authors:
//!   - ミウミ@ラブカ「デッキ紹介: AZALEA三神」(note, 2025-06-27)
//!   - ミウミ@ラブカ「Aqours三神」(predecessor, same deck family)
//!   - mute_tern875「AZALEAデッキ、最初の一歩」
//!   - Macro Asamiya「新弾（ロイホ）AZALEAの話をしよう」
//!   - chic_jacana9055「8点トリコリコを投げつけろ！AZALEAデッキで遊ぶ」
//!
//! The doctrine all five share:
//!   - Turn 3: stage 「2夏美-13曜」. 渡辺曜's debut digs the
//!     top 7 cards for green-heart members; a 5-point live is
//!     the aspirational outcome (~26% all-green).
//!   - Turn 4: stage 「9-20」(三神 9-cost + 曜&夏美&瑠璃乃
//!     20-cost). トリコリコPLEASE!! + アオクハルカ = 9 points.
//!   - Turn 5: 11 points. Turn 6: 16 points with 松浦果南's
//!     yell-reveal green heart active.
//!
//! The Main phase delegates to v7, the proven strongest Main:
//! the guides' deployment doctrine (cheap green members early,
//! 2-13 on turn 3, 9-20 on turn 4) is the cost curve v7
//! already plays, and v7's buff-aware accounting already
//! values 曜's debut dig and 花丸's bounce-and-draw
//! activation. A deck-specific deploy override was tried and
//! measured — it regressed the paired win rate against v7 by
//! 21pp (arena, 400 games, seed 11), because second-guessing
//! v7's member picks discards the ability value v7's scoring
//! already carries. The deck-specific knowledge lives in the
//! live set (the guides' pricing of the three AZALEA lives)
//! and the mulligan (keep the engine pieces).

use crate::bot::strategy_common::{acc_add, emit_mulligan, Acc};
use crate::card::{CardDatabase, CardType};
use crate::game_setup::Action;
use crate::game_state::GameState;

// ── Card identity (name_key forms: whitespace stripped, ＆ folded) ──
const KANAN: &str = "松浦果南";
const DIA: &str = "黒澤ダイヤ";
const HANAMARU: &str = "国木田花丸";
const YOU: &str = "渡辺曜";
const NATSUMI: &str = "鬼塚夏美";
const RURINO: &str = "大沢瑠璃乃";
const TRINITY: &str = "渡辺曜&鬼塚夏美&大沢瑠璃乃";
const TORIKORICO: &str = "トリコリコPLEASE!!";
const AOQUHARUKA: &str = "アオクハルカ";
const THREE_BOUQUET: &str = "スリーズブーケ";
const AQAOURS_SERIES: &str = "サンシャイン";

/// The engine pieces every guide builds around: the 三神
/// trinity, 渡辺曜, 鬼塚夏美, 大沢瑠璃乃, and the
/// cross-series 曜&夏美&瑠璃乃 finisher. Any print counts.
fn is_engine_name(name_key: &str) -> bool {
    matches!(
        name_key,
        KANAN | DIA | HANAMARU | YOU | NATSUMI | RURINO | TRINITY
    )
}

/// Base heart04 (green) on stage, Aqours members only — the
/// トリコリコPLEASE!! live-start condition (10+ grants +2 score).
/// Blade hearts are excluded: the card text reads 「持つハート」
/// and the printed FAQ for the sibling 渡辺曜 dig confirms blade
/// hearts are not referenced by such filters.
fn aqours_green_hearts(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    let p = gs.seat_player(me);
    let mut acc: Acc = [0i16; 11];
    for &cid in p.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        if !card.series.contains(AQAOURS_SERIES) {
            continue;
        }
        if let Some(bh) = &card.base_heart {
            acc_add(&mut acc, &bh.hearts);
        }
    }
    i32::from(acc[4]) // Heart04 index in the Acc layout
}

/// Three's Bouquet live cards in the waitroom — the
/// アオクハルカ live-start condition (3+ grants +1 score).
/// 曜's debut dig parks the dug lives in the waitroom, which is
/// how the guides set this up by turn 4.
fn waitroom_bouquet_lives(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    let p = gs.seat_player(me);
    let mut count = 0i32;
    for &cid in p.waitroom.cards.iter() {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        if card.card_type == CardType::Live
            && card.unit.as_deref() == Some(THREE_BOUQUET)
        {
            count += 1;
        }
    }
    count
}

/// Main phase: v7's proven Main, unchanged.
///
/// The guides' deployment doctrine (cheap green members
/// early, turn 3 「2夏美-13曜」, turn 4 「9-20」) is the
/// cost curve v7 already plays, and v7's buff-aware
/// accounting already values 曜's debut dig and 花丸's
/// bounce-and-draw activation. A deck-specific deploy
/// override was tried and measured: it regressed the
/// paired win rate against v7 by 21pp (arena, 400 games,
/// seed 11) because second-guessing v7's member picks
/// discards the ability value v7's scoring already
/// carries. The deck-specific knowledge is confined to
/// the live set and the mulligan, where the guides
/// actually differ from the general bots.
pub fn choose_action(gs: &GameState, actions: &[Action], me: u8) -> Action {
    crate::bot::strategy_v7::choose_action(gs, actions, me)
}

/// Live-card set: v8's proven placement-probability
/// lookahead, with the guides' pricing of the three
/// AZALEA lives injected as the life scorer.
///
/// The guides' pricing — トリコリコPLEASE!! is score 6,
/// or 8 once the stage carries 10+ green hearts;
/// アオクハルカ is score 2, or 3 once three Three's
/// Bouquet lives sit in the waitroom; 眩耀夜行 is its
/// printed 5 — is the deck knowledge the general bots
/// do not carry. v8's lookahead supplies the mechanism
/// (heart-pass sampling, the commitment equilibrium);
/// the scorer supplies the deck. A hand-rolled greedy
/// fill over these prices measured 12.75pp worse than
/// v7 in a paired arena run, because it discards the
/// sampling this lookahead does.
pub fn choose_live_set(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    crate::bot::v8_live::choose_live_set_v8_scored(gs, actions, db, azalea_life_score)
}

/// The azalea life scorer: the printed score plus the
/// conditional live-start bonuses — the guides' pricing.
/// The bonuses are deterministic functions of the board
/// (the stage and waitroom are fixed during the live
/// set), so they price the live at the score it will
/// actually enter the check with.
///
/// `pub` so the deck pricing is testable directly: the
/// live-set pick that consumes this score is v8's argmax
/// over candidate value, which folds in pass probability
/// and contestation, so a pick-level test cannot isolate
/// the pricing the way a direct call can.
pub fn azalea_life_score(gs: &GameState, me: u8, db: &CardDatabase, cid: i16) -> i32 {
    // Ablation switch: `AZALEA_SCORER=0` prices every life at its
    // printed score, isolating the mulligan half of the deck
    // knowledge. Off by default — the guides' pricing is the point.
    if std::env::var("AZALEA_SCORER").as_deref() == Ok("0") {
        return crate::bot::v8_model::printed_score(db, cid);
    }
    let base = crate::bot::v8_model::printed_score(db, cid);
    let Some(card) = db.get_card(cid) else {
        return base;
    };
    let score = match card.name_key.as_ref() {
        TORIKORICO => {
            let g = aqours_green_hearts(gs, me, db);
            let s = base + 2 * i32::from(g >= 10);
            if std::env::var_os("AZALEA_TRACE").is_some() {
                eprintln!(
                    "AZALEA_SCORE cid={} name={} base={} green={} score={}",
                    cid, card.name_key, base, g, s
                );
            }
            s
        }
        AOQUHARUKA => {
            let b = waitroom_bouquet_lives(gs, me, db);
            let s = base + i32::from(b >= 3);
            if std::env::var_os("AZALEA_TRACE").is_some() {
                eprintln!(
                    "AZALEA_SCORE cid={} name={} base={} bouquet={} score={}",
                    cid, card.name_key, base, b, s
                );
            }
            s
        }
        _ => base,
    };
    score
}

/// Mulligan: v7's rule — dump up to three of the
/// most expensive members, lives and energy stay —
/// with one deck-specific refinement: engine pieces
/// are never dumped.
///
/// The guides' whole plan runs on the engine pieces
/// (三神, 曜, 夏美, 瑠璃乃, the 20-cost
/// 曜&夏美&瑠璃乃 finisher), and the deck runs 25 of
/// 48 members as 曜/夏美/瑠璃乃-named. An engine
/// piece in the opening is the turn-3/4 play, not
/// dead weight, so it sorts as cost 0 here and
/// v7's most-expensive-first dump passes it by.
pub fn choose_mulligan(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    // Ablation switch: `AZALEA_KEEP=0` hands the mulligan to v7's
    // curve rule, isolating the live-scorer half of the deck
    // knowledge. Off by default — the engine-keep is the point.
    if std::env::var("AZALEA_KEEP").as_deref() == Ok("0") {
        return crate::bot::strategy_v7::choose_mulligan(gs, actions, db);
    }
    // The guides' own mulligan is a KEEP/REDRAW decision, not a
    // selective discard: an opening that holds an engine piece
    // plus a live is kept WHOLE; anything else is redrawn.
    // `AZALEA_MULLIGAN=guides` measures it against the
    // selective engine-keep below.
    if std::env::var("AZALEA_MULLIGAN").as_deref() == Ok("guides") {
        return choose_mulligan_guides(gs, actions, db);
    }
    let me = gs.active_player();
    let hand = &me.hand.cards;

    let mut members: Vec<(usize, u8, bool)> = Vec::new();
    let mut lives_seen = 0usize;
    let mut discard: Vec<usize> = Vec::new();
    for (hi, &cid) in hand.iter().enumerate() {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        match card.card_type {
            CardType::Live => {
                lives_seen += 1;
                if lives_seen > 3 {
                    discard.push(hi);
                }
            }
            CardType::Member => {
                members.push((
                    hi,
                    card.cost.unwrap_or(0),
                    is_engine_name(card.name_key.as_ref()),
                ));
            }
            CardType::Energy => {}
        }
    }

    // Most expensive first, but an engine piece is
    // never dumped: it is the turn-3/4 play, not
    // dead weight.
    members.sort_by_key(|&(_, cost, _)| std::cmp::Reverse(cost));
    for &(hi, _cost, is_engine) in &members {
        if discard.len() >= 3 {
            break;
        }
        if is_engine {
            continue;
        }
        if !discard.contains(&hi) {
            discard.push(hi);
        }
    }
    emit_mulligan(gs, actions, &discard)
}
