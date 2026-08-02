# Phase C generator: move the Step::Play arm body out of the bin into
# src/game.rs as `play_step(PlayState, keys)`, replacing the 32-field
# Step::Play tuple with a PlayState struct.

import os

ROOT = os.path.dirname(os.path.abspath(__file__))
BIN = os.path.join(ROOT, "src", "bin", "rabuka_3ds.rs")
GAME = os.path.join(ROOT, "src", "game.rs")
STEPS = os.path.join(ROOT, "src", "steps.rs")
SETUP = os.path.join(ROOT, "src", "setup.rs")
LIB = os.path.join(ROOT, "src", "lib.rs")

with open(BIN, "r", encoding="utf-8", newline="") as f:
    lines = f.readlines()

# File line N -> lines[N-1].
ARM_START = 206  # file line of `Step::Play(`
BODY_START = 240  # file line of first body statement
RECON_START = 4514  # file line of `Step::Play(` reconstruction
BODY_END = 4547  # file line of `)` closing the reconstruction
ARM_END = 4548  # file line of `}` closing the arm

# --- Verify boundaries so stale edits never silently mis-extract. ---
assert "Step::Play(" in lines[ARM_START - 1], "arm start missing"
assert lines[BODY_START - 1].lstrip().startswith("// Web server pattern"), (
    "body start mismatch: %r" % lines[BODY_START - 1]
)
assert lines[BODY_END - 1].strip() == ")", (
    "reconstruction close mismatch: %r" % lines[BODY_END - 1]
)
assert lines[ARM_END - 1].strip() == "}", "arm close mismatch"

# --- Rewrite the final Step::Play(...) reconstruction into PlayState{...}. ---
recon = lines[RECON_START - 1 : BODY_END]
fields = []
for ln in recon[1:-1]:
    t = ln.strip()
    if not t.endswith(","):
        raise SystemExit("unexpected recon line: %r" % ln)
    t = t[:-1].strip()
    if t == "atlas.clone()":
        fields.append("atlas: atlas.clone(),")
    elif t == "*vs_ai":
        fields.append("vs_ai: *vs_ai,")
    elif t == "*ai_vs_ai":
        fields.append("ai_vs_ai: *ai_vs_ai,")
    else:
        fields.append(t + ",")
new_recon = ["                Step::Play(PlayState {\n"]
for fl in fields:
    new_recon.append("                    " + fl + "\n")
new_recon.append("                })\n")

body_new = "".join(lines[BODY_START - 1 : RECON_START - 1]) + "".join(new_recon)

DESTRUCT = """pub fn play_step(p: PlayState, keys: u32) -> Step {
    let PlayState {
        gs: mut gs,
        cur: mut cur,
        acts_cache: mut acts_cache,
        dirty: mut dirty,
        redraw: mut redraw,
        atlas: ref atlas,
        vs_ai: ref vs_ai,
        ai_vs_ai: ref ai_vs_ai,
        cli_mode: mut cli_mode,
        detail_mode: mut detail_mode,
        choice_image_mode: mut choice_image_mode,
        choice_subview: mut choice_subview,
        text_page: mut text_page,
        choice_grid_offset: mut choice_grid_offset,
        list_scroll: mut list_scroll,
        detail_scroll_y: mut detail_scroll_y,
        hand_offset: mut hand_offset,
        hand_offset_p2: mut hand_offset_p2,
        touch_tap_count: mut touch_tap_count,
        viewing_card: mut viewing_card,
        zone_viewer: mut zone_viewer,
        zone_viewer_offset: mut zone_viewer_offset,
        was_touching: mut was_touching,
        is_multiplayer: is_multiplayer,
        is_host: is_host,
        waiting_for_opponent: mut waiting_for_opponent,
        overlay: mut overlay,
        pending_client_action: mut pending_client_action,
        last_client_action_seq: mut last_client_action_seq,
        next_action_seq: mut next_action_seq,
        dbg_tx_bytes: mut dbg_tx_bytes,
        dbg_rx_bytes: mut dbg_rx_bytes,
    } = p;
"""

HEADER = """#![cfg(feature = "3ds")]
// Play state machine (Phase C): the Step::Play handler, moved verbatim from the
// bin (see extract_play.py). PlayState replaces the old 32-field tuple.

use std::collections::HashMap;

use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn;

use crate::dprintln;
use crate::ffi::*;
use crate::i18n;
use crate::i18n::Lang;
use crate::lang::{current_lang, set_lang, tl};
use crate::net::{execute_received_action, mp_can_act, route_authoritative_action};
use crate::steps::{Overlay, Step};
use crate::uds;
use crate::ui::card_atlas::CardAtlas;
use crate::ui::colors::*;
use crate::ui::grid::{card_grid_input, render_card_detail, render_card_grid, GridAction};
use crate::ui::hint::render_hint_bar;
use crate::ui::text::*;
use crate::util::{cn_or_empty, heart_color_index, tl_area};

/// Full gameplay state carried by `Step::Play`.
pub struct PlayState {
    pub gs: GameState,
    pub cur: usize,
    pub acts_cache: Vec<game_setup::Action>,
    pub dirty: bool,
    pub redraw: bool,
    pub atlas: CardAtlas,
    pub vs_ai: bool,
    pub ai_vs_ai: bool,
    pub cli_mode: bool,
    pub detail_mode: bool,
    pub choice_image_mode: bool,
    pub choice_subview: bool,
    pub text_page: usize,
    pub choice_grid_offset: usize,
    pub list_scroll: usize,
    pub detail_scroll_y: f32,
    pub hand_offset: usize,
    pub hand_offset_p2: usize,
    pub touch_tap_count: u32,
    pub viewing_card: Option<i16>,
    pub zone_viewer: Option<(String, Vec<i16>)>,
    pub zone_viewer_offset: usize,
    pub was_touching: bool,
    pub is_multiplayer: bool,
    pub is_host: bool,
    pub waiting_for_opponent: bool,
    pub overlay: Overlay,
    pub pending_client_action: Option<Vec<u8>>,
    pub last_client_action_seq: u32,
    pub next_action_seq: u32,
    pub dbg_tx_bytes: u32,
    pub dbg_rx_bytes: u32,
}

"""


def extract_fn_body(text, marker):
    """Return the inner body of a fn whose signature line starts with marker."""
    i = text.index(marker)
    brace = text.index("{", i)
    start = brace + 1
    depth = 1
    j = start
    while j < len(text) and depth > 0:
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
        j += 1
    return text[start : j - 1]


# Helpers: file lines 4580..4613.
helper_text = "".join(lines[4580 - 1 : 4613])
find_body = extract_fn_body(helper_text, "fn find_card_zone_slot")
vis_body = extract_fn_body(helper_text, "fn visible_hand_slots")

game_content = (
    HEADER
    + DESTRUCT
    + body_new
    + "}\n"
    + "\n/// Locate a card's zone slot (zone, index, is-opponent) for board rendering.\n"
    + "fn find_card_zone_slot(gs: &GameState, cid: i16, my_player_idx: usize) -> Option<(i32, i32, bool)> {\n"
    + find_body.strip()
    + "\n}\n"
    + "\nfn visible_hand_slots() -> usize {\n"
    + vis_body.strip()
    + "\n}\n"
)

with open(GAME, "w", encoding="utf-8", newline="") as f:
    f.write(game_content)
print("wrote", GAME)

# --- Patch the bin. ---
arm_replacement = "            Step::Play(p) => play_step(p, keys),\n"
new_lines = lines[: ARM_START - 1] + [arm_replacement] + lines[ARM_END:]

NEW_IMPORTS = """use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use rabuka_engine::card::Card;
use rabuka_engine::deck_parser::DeckParser;

#[cfg(feature = "3ds")]
use rabuka_3ds::dprintln;
#[cfg(feature = "3ds")]
use rabuka_3ds::ffi::*;
#[cfg(feature = "3ds")]
use rabuka_3ds::game::play_step;
#[cfg(feature = "3ds")]
use rabuka_3ds::i18n;
#[cfg(feature = "3ds")]
use rabuka_3ds::lang::{tl, tl_fmt};
#[cfg(feature = "3ds")]
use rabuka_3ds::setup::setup_step;
#[cfg(feature = "3ds")]
use rabuka_3ds::steps::{step_name, SetupPhase, Step};
#[cfg(feature = "3ds")]
use rabuka_3ds::util::{ticks_to_ms, YieldReader};
"""
assert "use std::collections::HashMap;" in new_lines[30]
new_lines = new_lines[:30] + [NEW_IMPORTS] + new_lines[71:]

# Delete the moved helper fns (locate by content; line numbers shifted).
hstart = None
for i, ln in enumerate(new_lines):
    if (
        ln.strip() == '#[cfg(feature = "3ds")]'
        and i + 1 < len(new_lines)
        and "fn find_card_zone_slot" in new_lines[i + 1]
    ):
        hstart = i
        break
assert hstart is not None, "find_card_zone_slot cfg line not found"
hend = None
for i in range(hstart + 2, len(new_lines)):
    if "fn visible_hand_slots" in new_lines[i]:
        hend = i
        break
assert hend is not None, "visible_hand_slots not found"
j = hend
while new_lines[j].strip() != "}":
    j += 1
new_lines = new_lines[:hstart] + new_lines[j + 1 :]

with open(BIN, "w", encoding="utf-8", newline="") as f:
    f.writelines(new_lines)
print("patched", BIN)

# --- Patch steps.rs: Step::Play now carries PlayState. ---
with open(STEPS, "r", encoding="utf-8", newline="") as f:
    st = f.read()

old_play = """    Play(
        GameState,
        usize, // cursor
        Vec<game_setup::Action>,
        bool, // dirty
        bool, // redraw
        CardAtlas,
        bool,                       // vs_ai (human vs AI)
        bool,                       // ai_vs_ai (spectator: both AI)
        bool,                       // cli_mode
        bool,                       // detail_mode
        bool,                       // choice_image_mode
        bool,                       // choice_subview (false=choices grid, true=text overlay)
        usize,                      // text_page (current page index in text subview)
        usize,                      // choice_grid_offset (scroll offset for choice image grid)
        usize,                      // list_scroll (stable scroll offset for action list)
        f32,                        // detail_scroll_y (scroll offset for card detail text)
        usize,                      // hand_offset (P1)
        usize,                      // hand_offset_p2
        u32,                        // touch_tap_count
        Option<i16>,                // viewing_card_id
        Option<(String, Vec<i16>)>, // zone_viewer (label, card_ids)
        usize,                      // zone_viewer_offset
        bool,                       // was_touching (edge detect for touch screen)
        bool,                       // is_multiplayer
        bool,                       // is_host (true = P1/host, false = P2/client)
        bool,                       // waiting_for_opponent
        Overlay,                    // overlay (start menu, game log, perf stats, revealed)
        Option<Vec<u8>>, // either side: my last action bytes, retransmitted until the opponent ACKs
        u32,             // last action_seq received from the opponent (dedup)
        u32,             // my next action_seq to send
        u32,             // packet debug counter: bytes sent
        u32,             // packet debug counter: bytes received
    ),
"""
assert old_play in st, "old Play tuple not found in steps.rs"
st = st.replace(old_play, "    Play(PlayState),\n")

old_stepname = """        Step::Play(
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
        ) => "Play","""
assert old_stepname in st, "old step_name Play arm not found"
st = st.replace(old_stepname, '        Step::Play(_) => "Play",')

old_uses = "use crate::ffi::*;\nuse crate::ui::card_atlas::CardAtlas;"
assert old_uses in st, "steps.rs use block not found"
st = st.replace(
    old_uses,
    "use crate::ffi::*;\nuse crate::game::PlayState;\nuse crate::ui::card_atlas::CardAtlas;",
)

with open(STEPS, "w", encoding="utf-8", newline="") as f:
    f.write(st)
print("patched", STEPS)

# --- Patch setup.rs: two construction sites wrap a PlayState. ---
with open(SETUP, "r", encoding="utf-8", newline="") as f:
    su = f.read()

OLD_CONSTRUCT = """                Step::Play(
                    gs,
                    0,
                    Vec::new(),
                    true,
                    true,
                    atlas,
                    vs_ai,
                    false,  // ai_vs_ai
                    false,  // cli_mode (start in game mode)
                    false,  // detail_mode
                    true,   // choice_image_mode
                    false,  // choice_subview (false=choices grid)
                    0,      // text_page
                    0,      // choice_grid_offset
                    0,      // list_scroll
                    0.0f32, // detail_scroll_y
                    0,      // hand_offset
                    0,      // hand_offset_p2
                    0,      // touch_tap_count
                    None,   // viewing_card
                    None,   // zone_viewer
                    0,      // zone_viewer_offset
                    false,  // was_touching
                    false,  // is_multiplayer
                    false,  // is_host
                    false,  // waiting_for_opponent
                    Overlay::None,
                    None,
                    0,
                    1,
                    0,
                    0,
                )"""
NEW_CONSTRUCT = """                Step::Play(PlayState {
                    gs,
                    cur: 0,
                    acts_cache: Vec::new(),
                    dirty: true,
                    redraw: true,
                    atlas,
                    vs_ai,
                    ai_vs_ai: false,
                    cli_mode: false,
                    detail_mode: false,
                    choice_image_mode: true,
                    choice_subview: false,
                    text_page: 0,
                    choice_grid_offset: 0,
                    list_scroll: 0,
                    detail_scroll_y: 0.0f32,
                    hand_offset: 0,
                    hand_offset_p2: 0,
                    touch_tap_count: 0,
                    viewing_card: None,
                    zone_viewer: None,
                    zone_viewer_offset: 0,
                    was_touching: false,
                    is_multiplayer: false,
                    is_host: false,
                    waiting_for_opponent: false,
                    overlay: Overlay::None,
                    pending_client_action: None,
                    last_client_action_seq: 0,
                    next_action_seq: 1,
                    dbg_tx_bytes: 0,
                    dbg_rx_bytes: 0,
                })"""
assert su.count(OLD_CONSTRUCT) == 1, (
    "setup.rs single-player construct site != 1 (count=%d)" % su.count(OLD_CONSTRUCT)
)
su = su.replace(OLD_CONSTRUCT, NEW_CONSTRUCT)

OLD_CONSTRUCT2 = """                Step::Play(
                    gs,
                    0,
                    Vec::new(),
                    true,
                    true,
                    atlas,
                    false,    // vs_ai (this is multiplayer)
                    false,    // ai_vs_ai
                    false,    // cli_mode (start in game mode)
                    false,    // detail_mode
                    true,     // choice_image_mode
                    false,    // choice_subview (false=choices grid)
                    0,        // text_page
                    0,        // choice_grid_offset
                    0,        // list_scroll
                    0.0f32,   // detail_scroll_y
                    0,        // hand_offset
                    0,        // hand_offset_p2
                    0,        // touch_tap_count
                    None,     // viewing_card
                    None,     // zone_viewer
                    0,        // zone_viewer_offset
                    false,    // was_touching
                    true,     // is_multiplayer
                    is_host,  // is_host
                    !is_host, // waiting_for_opponent will be recalculated after settle
                    Overlay::None,
                    None,
                    0,
                    1,
                    0,
                    0,
                )"""
NEW_CONSTRUCT2 = """                Step::Play(PlayState {
                    gs,
                    cur: 0,
                    acts_cache: Vec::new(),
                    dirty: true,
                    redraw: true,
                    atlas,
                    vs_ai: false,
                    ai_vs_ai: false,
                    cli_mode: false,
                    detail_mode: false,
                    choice_image_mode: true,
                    choice_subview: false,
                    text_page: 0,
                    choice_grid_offset: 0,
                    list_scroll: 0,
                    detail_scroll_y: 0.0f32,
                    hand_offset: 0,
                    hand_offset_p2: 0,
                    touch_tap_count: 0,
                    viewing_card: None,
                    zone_viewer: None,
                    zone_viewer_offset: 0,
                    was_touching: false,
                    is_multiplayer: true,
                    is_host,
                    waiting_for_opponent: !is_host,
                    overlay: Overlay::None,
                    pending_client_action: None,
                    last_client_action_seq: 0,
                    next_action_seq: 1,
                    dbg_tx_bytes: 0,
                    dbg_rx_bytes: 0,
                })"""
assert su.count(OLD_CONSTRUCT2) == 1, "setup.rs mp construct site != 1"
su = su.replace(OLD_CONSTRUCT2, NEW_CONSTRUCT2)

old_setup_uses = "use crate::dprintln;\nuse crate::ffi::*;"
assert old_setup_uses in su, "setup.rs use block not found"
su = su.replace(
    old_setup_uses,
    "use crate::dprintln;\nuse crate::ffi::*;\nuse crate::game::PlayState;",
)

with open(SETUP, "w", encoding="utf-8", newline="") as f:
    f.write(su)
print("patched", SETUP)

# --- Patch lib.rs: add pub mod game. ---
with open(LIB, "r", encoding="utf-8", newline="") as f:
    li = f.read()
if "pub mod game;" not in li:
    assert "pub mod ffi;" in li, "lib.rs module list not found"
    li = li.replace("pub mod ffi;", "pub mod game;\npub mod ffi;")
    with open(LIB, "w", encoding="utf-8", newline="") as f:
        f.write(li)
    print("patched", LIB)
else:
    print(LIB, "already has pub mod game")

print("DONE")
