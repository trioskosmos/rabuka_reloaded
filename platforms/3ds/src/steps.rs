#![cfg(feature = "3ds")]

// Shared Step/SetupPhase/Overlay types plus debug macros used by both the bin
// and setup.rs. Moved here from src/bin/rabuka_3ds.rs during the Phase B refactor.

use std::sync::Arc;

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::deck_parser::DeckList;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::GameState;

#[allow(unused_imports)] // needed for the _3ds_* names in the dprintln!/tprintln! bodies
use crate::ffi::*;
use crate::ui::card_atlas::CardAtlas;

// dprintln! — game output on BOTTOM screen (action list).
// Also sends to debug console via svcOutputDebugString (3dslink).
#[macro_export]
macro_rules! dprintln {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s = format!("{}\n\0", msg);
        unsafe { _3ds_debug_print(s.as_ptr()); }
        unsafe { _3ds_text_add_bot(s.as_ptr()); }
    }};
}

// tprintln! — debug output on TOP screen (timing/status).
// Appends to top text buffer, rendered in _3ds_swap_buffers().
#[macro_export]
#[allow(unused_macros)]
macro_rules! tprintln {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s = format!("{}\n\0", msg);
        unsafe { _3ds_debug_print(s.as_ptr()); }
        unsafe { _3ds_text_add_top(s.as_ptr()); }
    }};
}

#[derive(Clone)]
pub enum SetupPhase {
    PickMode(usize), // cursor: 0=sandbox, 1=vsAI, 2=AIvsAI, 3=tests, 4=localMP
    PickDeck(usize, bool, bool), // cursor, vs_ai flag, is_multiplayer
    PickDeck2(usize, usize, bool), // cursor, p1_idx, vs_ai
    Loading(usize, usize, bool), // p1_idx, p2_idx, vs_ai
    #[allow(dead_code)]
    Testing, // On-device test suite
    // Multiplayer lobby phases
    MultiplayerDeck(usize), // cursor, selecting deck for multiplayer
    MultiplayerPickRole(usize, usize), // deck_idx, role_cursor (0=Host, 1=Client)
    MultiplayerHostWait(usize), // p1_idx: host waiting for client to connect
    MultiplayerClientScan(usize, u32), // p1_idx, frames_until_rescan
    MultiplayerClientHostSelect(usize, Vec<u16>, usize), // p1_idx, host_node_ids, cursor
    MultiplayerSyncDeck(usize, usize, bool), // p1_idx, p2_idx, is_host
    MultiplayerLoading(usize, usize, bool, Option<Vec<u8>>, u64), // p1_idx, p2_idx, is_host, deck_sync_bytes, seed
    QrScan(usize),          // QR code scanning (usize = context pointer, 0=not started)
    QrResult(Vec<String>),  // QR scan result, user can confirm
    QrNotDeck(String, u32), // QR scanned but not a valid deck, shows decoded text, countdown frames
    ControlGuide(usize),    // Help/control guide overlay (usize = page index)
    DeckViewer(
        Vec<i16>,          // card_ids (resolved to i16, same as zone_viewer)
        usize,             // cursor
        usize,             // offset
        bool,              // vs_ai
        bool,              // is_multiplayer
        Option<i16>,       // viewing_card (same as zone_viewer)
        Arc<CardDatabase>, // card_db
        CardAtlas,         // atlas
    ),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Overlay {
    None,
    StartMenu(usize),
    GameLog(usize, usize),                   // offset (from end), cursor
    PerfStats(Option<usize>, usize),         // detail snapshot index, cursor
    RevealedCards(bool, usize, Option<i16>), // show_self, flat cursor, viewing card id
}

#[derive(Clone)]
pub enum Step {
    ReadCardsBin,
    ParseCards(Vec<u8>),
    Setup(Arc<Vec<Card>>, Vec<DeckList>, SetupPhase, bool),
    Play(
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
    Done(Result<(), String>),
}

pub fn step_name(s: &Step) -> &'static str {
    match s {
        Step::ReadCardsBin => "ReadCards",
        Step::ParseCards(_) => "ParseCards",
        Step::Setup(_, _, _, _) => "Setup",
        Step::Play(
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
        ) => "Play",
        Step::Done(_) => "Done",
    }
}
