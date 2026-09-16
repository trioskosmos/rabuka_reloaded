//! Option-D execution tracer for C parity replay.
//!
//! Env-gated: set `RABUKA_TRACE_DIR` to a directory; each `TestGame` appends
//! to `<dir>/<test-name>.trace` (libtest thread name, sanitized). Unset =
//! zero overhead, existing tests untouched.
//!
//! The trace is a runnable script AND the oracle at once. Both sides print
//! byte-identical streams; `diff` (ignoring `#` lines) is the parity check:
//! - `A <op>` action lines, echoed verbatim by the C runner on execute.
//! - `S` state blocks (card_no-canonical, occurrence refs `card_no#k`).
//!   The C runner LOADs `S` blocks with `PD none` (resync) and verifies the
//!   `PD` line of pending ones (resolver internals are engine behavior and
//!   must be reproduced, not imported).
//! - `CHECK <n> <pass|FAIL> <actual> <expected>` from hooked assert helpers.
//!
//! Occurrence order (FIXED, must match replay.c `trace_resolve` exactly):
//! p1stage0..2, p2stage0..2, p1hand, p2hand, p1deck, p2deck, p1waitroom,
//! p2waitroom, p1live, p2live, p1success, p2success, p1energy, p2energy,
//! p1under0..2, p2under0..2. Empty stage slots (`-1`) are skipped.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use rabuka_engine::card::HeartColor;

use super::TestGame;

pub struct Trace {
    path: Option<String>,
    buf: RefCell<Vec<String>>,
    check_n: Cell<usize>,
    quiet: Cell<bool>,
}

impl Trace {
    pub fn new() -> Self {
        let path = std::env::var("RABUKA_TRACE_DIR").ok().map(|d| {
            let t = std::thread::current()
                .name()
                .unwrap_or("unknown")
                .to_string();
            let safe: String = t
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect();
            format!("{}/{}.trace", d.trim_end_matches('/'), safe)
        });
        Self {
            path,
            buf: RefCell::new(Vec::new()),
            check_n: Cell::new(0),
            quiet: Cell::new(false),
        }
    }

    pub fn live(&self) -> bool {
        self.path.is_some()
    }

    /// Suppress per-step lines (used by drain loops that emit one summary).
    pub fn set_quiet(&self, q: bool) {
        self.quiet.set(q);
    }

    pub fn emit(&self, line: String) {
        if self.path.is_some() && !self.quiet.get() {
            self.buf.borrow_mut().push(line);
        }
    }

    pub fn flush(&self) {
        if let Some(p) = &self.path {
            let lines = self.buf.borrow().join("\n");
            if lines.is_empty() {
                return;
            }
            let prev = std::fs::read_to_string(p).unwrap_or_default();
            let mut s = prev;
            if !s.is_empty() && !s.ends_with('\n') {
                s.push('\n');
            }
            s.push_str(&lines);
            s.push('\n');
            let _ = std::fs::write(p, s);
            self.buf.borrow_mut().clear();
        }
    }
}

/// (card_no, occurrence) enumeration in the FIXED zone order. Returns
/// id -> "card_no#k" for every card currently in a listed zone.
fn occurrences(game: &TestGame) -> HashMap<i16, String> {
    let mut out: HashMap<i16, String> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut put = |id: i16, no: String| {
        if id < 0 || out.contains_key(&id) {
            return;
        }
        let k = counts.get(&no).copied().unwrap_or(0);
        counts.insert(no.clone(), k + 1);
        out.insert(id, format!("{}#{}", no, k));
    };
    let no = |game: &TestGame, id: i16| {
        game.db
            .get_card(id)
            .map(|c| c.card_no.to_string())
            .unwrap_or_else(|| format!("#{}", id))
    };
    for a in 0..3 {
        put(game.state.player1.stage.stage[a], no(game, game.state.player1.stage.stage[a]));
    }
    for a in 0..3 {
        put(game.state.player2.stage.stage[a], no(game, game.state.player2.stage.stage[a]));
    }
    for id in game.state.player1.hand.cards.iter().copied() {
        put(id, no(game, id));
    }
    for id in game.state.player2.hand.cards.iter().copied() {
        put(id, no(game, id));
    }
    for id in game.state.player1.main_deck.cards.iter().copied() {
        put(id, no(game, id));
    }
    for id in game.state.player2.main_deck.cards.iter().copied() {
        put(id, no(game, id));
    }
    for id in game.state.player1.waitroom.cards.iter().copied() {
        put(id, no(game, id));
    }
    for id in game.state.player2.waitroom.cards.iter().copied() {
        put(id, no(game, id));
    }
    for id in game.state.player1.live_card_zone.cards.iter().copied() {
        put(id, no(game, id));
    }
    for id in game.state.player2.live_card_zone.cards.iter().copied() {
        put(id, no(game, id));
    }
    for id in game.state.player1.success_live_card_zone.cards.iter().copied() {
        put(id, no(game, id));
    }
    for id in game.state.player2.success_live_card_zone.cards.iter().copied() {
        put(id, no(game, id));
    }
    for id in game.state.player1.energy_zone.cards.iter().copied() {
        put(id, no(game, id));
    }
    for id in game.state.player2.energy_zone.cards.iter().copied() {
        put(id, no(game, id));
    }
    for a in 0..3 {
        for id in game.state.player1.stage.get_under_cards(area_of(a)).iter().copied() {
            put(id, no(game, id));
        }
    }
    for a in 0..3 {
        for id in game.state.player2.stage.get_under_cards(area_of(a)).iter().copied() {
            put(id, no(game, id));
        }
    }
    out
}

fn area_of(a: usize) -> rabuka_engine::zones::MemberArea {
    match a {
        0 => rabuka_engine::zones::MemberArea::LeftSide,
        1 => rabuka_engine::zones::MemberArea::Center,
        _ => rabuka_engine::zones::MemberArea::RightSide,
    }
}

fn heart_variant(i: usize) -> HeartColor {
    match i {
        0 => HeartColor::Heart00,
        1 => HeartColor::Heart01,
        2 => HeartColor::Heart02,
        3 => HeartColor::Heart03,
        4 => HeartColor::Heart04,
        5 => HeartColor::Heart05,
        6 => HeartColor::Heart06,
        _ => HeartColor::BAll,
    }
}

fn phase_c_name(disp: &str) -> &str {
    match disp {
        "Main" => "Main",
        "Active" => "Active",
        "Energy" => "Energy",
        "Draw" => "Draw",
        "RPS" => "RPS",
        "LiveCardSet (1st)" | "LiveCardSet (2nd)" => "LiveCardSet",
        "Perform (1st)" | "Perform (2nd)" => "Performance",
        "Live Result" => "Victory",
        _ => "Opening",
    }
}

/// `card_no#k` ref for a copy id, or `#id` if the id is in no listed zone.
pub fn ref_of(game: &TestGame, id: i16) -> String {
    occurrences(game)
        .get(&id)
        .cloned()
        .unwrap_or_else(|| format!("#{}", id))
}

fn zone_list(game: &TestGame, ids: &[i16]) -> String {
    ids.iter()
        .map(|id| {
            game.db
                .get_card(*id)
                .map(|c| c.card_no.to_string())
                .unwrap_or_else(|| format!("#{}", id))
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Emit a full canonical state block (`S` + section lines).
pub fn dump_state(game: &TestGame, tr: &Trace) {
    if !tr.live() {
        return;
    }
    let occ = occurrences(game);
    tr.emit("S".to_string());
    let p = [&game.state.player1, &game.state.player2];
    for (pi, pl) in p.iter().enumerate() {
        let n = pi + 1;
        tr.emit(format!("Z {} hand {}", n, zone_list(game, &pl.hand.cards)));
        tr.emit(format!("Z {} deck {}", n, zone_list(game, &pl.main_deck.cards)));
        tr.emit(format!(
            "Z {} waitroom {}",
            n,
            zone_list(game, &pl.waitroom.cards)
        ));
        tr.emit(format!(
            "Z {} live {}",
            n,
            zone_list(game, &pl.live_card_zone.cards)
        ));
        tr.emit(format!(
            "Z {} success {}",
            n,
            zone_list(game, &pl.success_live_card_zone.cards)
        ));
        tr.emit(format!(
            "Z {} energy {}",
            n,
            zone_list(game, &pl.energy_zone.cards)
        ));
        tr.emit(format!("E {} {}", n, pl.energy_zone.active_count()));
        let st: Vec<String> = (0..3)
            .map(|a| {
                let id = pl.stage.stage[a];
                if id < 0 {
                    "-".to_string()
                } else {
                    game.db
                        .get_card(id)
                        .map(|c| c.card_no.to_string())
                        .unwrap_or_else(|| format!("#{}", id))
                }
            })
            .collect();
        tr.emit(format!("ST {} {} {} {}", n, st[0], st[1], st[2]));
        for a in 0..3 {
            let u: Vec<i16> = pl.stage.get_under_cards(area_of(a)).to_vec();
            if !u.is_empty() {
                tr.emit(format!("U {} {} {}", n, a, zone_list(game, &u)));
            }
        }
        for a in 0..3 {
            let id = pl.stage.stage[a];
            if id >= 0 {
                if let Some(o) = game.state.mods.get_orientation_modifier(id) {
                    tr.emit(format!(
                        "O {} {}",
                        occ.get(&id).cloned().unwrap_or_else(|| format!("#{}", id)),
                        o
                    ));
                }
            }
        }
    }
    // Mods: absolute totals, sorted by (kind, ref, color) for determinism.
    let mut mods: Vec<(String, String, i32, i32)> = Vec::new();
    let mut ids: Vec<i16> = occ.keys().copied().collect();
    ids.sort();
    for id in ids {
        let r = &occ[&id];
        let b = game.state.mods.get_blade_modifier(id);
        if b != 0 {
            mods.push(("blade".into(), r.clone(), -1, b));
        }
        let s = game.state.mods.get_score_modifier(id);
        if s != 0 {
            mods.push(("score".into(), r.clone(), -1, s));
        }
        let c = game.state.mods.get_cost_modifier(id);
        if c != 0 {
            mods.push(("cost".into(), r.clone(), -1, c));
        }
        for color in 0..8 {
            let h = game
                .state
                .mods
                .get_heart_modifier(id, heart_variant(color));
            if h != 0 {
                mods.push(("heart".into(), r.clone(), color as i32, h));
            }
            let nh = game
                .state
                .mods
                .get_need_heart_modifier(id, heart_variant(color));
            if nh != 0 {
                mods.push(("need".into(), r.clone(), color as i32, nh));
            }
        }
    }
    mods.sort();
    for (kind, r, color, v) in mods {
        if color >= 0 {
            tr.emit(format!("M {} {} {} {}", kind, r, color, v));
        } else {
            tr.emit(format!("M {} {} {}", kind, r, v));
        }
    }
    tr.emit(format!(
        "PH {} {}",
        phase_c_name(&game.state.current_phase.to_string()),
        game.state.turn_number
    ));
    let pd = match game.pending_choice_type() {
        None => "none".to_string(),
        Some(t) => format!("{}:{}", t, game.pending_choice_count()),
    };
    tr.emit(format!("PD {}", pd));
}

/// Emit an `X` spec line plus a `CHECK` line for a hooked assert helper.
/// The C runner evaluates the `X` spec on its live state and prints its own
/// `CHECK` with the same sequence number, so `diff` compares them directly.
pub fn check(tr: &Trace, spec: String, actual: String, expected: String) {
    if !tr.live() {
        return;
    }
    let n = tr.check_n.get() + 1;
    tr.check_n.set(n);
    tr.emit(format!("X {}", spec));
    let verdict = if actual == expected { "pass" } else { "FAIL" };
    tr.emit(format!("CHECK {} {} {} {}", n, verdict, actual, expected));
}
