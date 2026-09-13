//! Board layout + cursor for the GBA turn screen.
//!
//! Pure data/logic (no agb drawing): given a `GameState` it produces a
//! [`BoardFrame`] describing every on-screen slot (card number + actionable
//! flag), the info/header lines, the selected action for the bottom bar, and
//! the hand-cursor focus. The tiled rendering happens in `display.rs`. This
//! mirrors the 3DS board (`docs/3ds/VISUAL_DESIGN.md`) adapted to a 240x160
//! GBA screen.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use rabuka_engine::core::constants::{EMPTY_SLOT, STAGE_SIZE};
use rabuka_engine::game_state::{GameState, Phase};

/// Live-set cards stay face-down until the performance phase — the 3DS
/// `live_hidden` rule (`platforms/3ds/src/game/mod.rs`: presence visible,
/// identity hidden) plus the web hidden-card filter. Scored success cards
/// are public and always shown. Shared by the board renderer and the Start
/// menu zone viewer so the two never disagree.
pub fn live_set_hidden(phase: &Phase) -> bool {
    !matches!(
        phase,
        Phase::FirstAttackerPerformance
            | Phase::SecondAttackerPerformance
            | Phase::LiveVictoryDetermination
    )
}

/// Cards shown per hand window — badge on card saves gap, so pitch = width.
const SCREEN_COLS: i32 = 30;
const HAND_PITCH_TILES: i32 = 3;
pub const HAND_VISIBLE: usize = (SCREEN_COLS / HAND_PITCH_TILES) as usize; // 10

/// One drawable card slot on the board.
///
/// `card_no` borrows from the game state's card database: a frame is built
/// and rendered in the same call (never stored), so per-frame `String`
/// churn (an alloc per card per frame fragmenting the 256KB heap over a
/// long match) becomes zero. Only the 3 formatted text lines still allocate.
#[derive(Clone, Copy)]
pub struct Slot<'a> {
    /// Card number, or None for an empty slot.
    pub card_no: Option<&'a str>,
    /// The card is referenced by one of the currently available actions.
    pub actionable: bool,
    /// Card is in wait state (tapped 90° on 3DS). On GBA we render a
    /// wait-state indicator instead of rotating (tile grid is fixed).
    pub waited: bool,
    /// Face-down (identity hidden, presence visible). Used for live-set
    /// cards before the performance phase, mirroring the 3DS card-back
    /// display and the web hidden-card filter.
    pub hidden: bool,
}

impl<'a> Slot<'a> {
    fn empty() -> Slot<'a> {
        Slot {
            card_no: None,
            actionable: false,
            waited: false,
            hidden: false,
        }
    }
}

/// Everything needed to draw one board frame.
pub struct BoardFrame<'a> {
    /// "T3 MAIN >P1" header line.
    pub header: String,
    /// Selected-action position, e.g. "3/12".
    pub action_count: String,
    /// First line of the selected action's description.
    pub action_line: String,
    /// Opponent / player count lines, wrapped to fit beside the stage rows.
    pub p2_info: [String; 2],
    pub p1_info: [String; 2],
    /// Stage slots, left to right.
    pub p2_stage: [Slot<'a>; 3],
    pub p1_stage: [Slot<'a>; 3],
    /// Live/success zone (3 slots, small) — victory condition.
    pub p2_live: [Slot<'a>; 3],
    pub p1_live: [Slot<'a>; 3],
    /// Live card set zone (3 slots) — where live cards are placed during Live phase.
    pub p2_live_set: [Slot<'a>; 3],
    pub p1_live_set: [Slot<'a>; 3],
    /// Visible hand window.
    pub hand: Vec<Slot<'a>>,
    /// True when more hand cards exist to the right of the window.
    pub hand_more: bool,
    /// First hand card index of the visible window (for cursor mapping).
    pub hand_offset_col: usize,
    /// Hand-cursor position within the window (None if the hand is empty).
    pub hand_cursor: Option<usize>,
    /// Stage cursor for own/opponent stage (0..2) when focus is on stage.
    pub own_stage_cursor: Option<usize>,
    pub opp_stage_cursor: Option<usize>,
    pub focus: Focus,
    /// Card number under the focused cursor (hand or stage).
    pub focused_card: Option<&'a str>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Hand,
    OwnStage,
    OppStage,
}

/// Board state: the hand cursor (absolute index into the full hand) and the
/// scroll window keeping it visible, plus stage focus.
pub struct Board {
    hand_cursor: usize,
    hand_offset: usize,
    pub focus: Focus,
    own_stage_cursor: usize,
    opp_stage_cursor: usize,
}

impl Board {
    pub fn new() -> Self {
        Board {
            hand_cursor: 0,
            hand_offset: 0,
            focus: Focus::Hand,
            own_stage_cursor: 0,
            opp_stage_cursor: 0,
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Hand => Focus::OwnStage,
            Focus::OwnStage => Focus::OppStage,
            Focus::OppStage => Focus::Hand,
        };
    }

    /// Reverse focus cycle (Hand -> Opp Stage -> Own Stage). Reached via R
    /// on an empty slot, so a B overshoot costs one press instead of two.
    pub fn cycle_focus_rev(&mut self) {
        self.focus = match self.focus {
            Focus::Hand => Focus::OppStage,
            Focus::OppStage => Focus::OwnStage,
            Focus::OwnStage => Focus::Hand,
        };
    }

    /// Move cursor within current focus. For Hand, scrolls hand window.
    pub fn move_focused(&mut self, delta: i32, hand_len: usize) -> bool {
        match self.focus {
            Focus::Hand => self.scroll_hand(delta, hand_len),
            Focus::OwnStage => {
                self.own_stage_cursor = ((self.own_stage_cursor as i32 + delta).rem_euclid(3)) as usize;
                true
            }
            Focus::OppStage => {
                self.opp_stage_cursor = ((self.opp_stage_cursor as i32 + delta).rem_euclid(3)) as usize;
                true
            }
        }
    }

    /// Move the hand cursor by `delta` within a hand of `hand_len` cards and
    /// keep the window on it. Returns true when the cursor moved.
    pub fn scroll_hand(&mut self, delta: i32, hand_len: usize) -> bool {
        if hand_len == 0 {
            return false;
        }
        let n = hand_len as i32;
        let cur = self.hand_cursor as i32;
        self.hand_cursor = ((cur + delta).rem_euclid(n)) as usize;
        if self.hand_cursor < self.hand_offset {
            self.hand_offset = self.hand_cursor;
        }
        if self.hand_cursor >= self.hand_offset + HAND_VISIBLE {
            self.hand_offset = self.hand_cursor + 1 - HAND_VISIBLE;
        }
        true
    }

    pub fn build<'g>(
        &mut self,
        gs: &'g GameState,
        actionable: &[String],
        action_line: &str,
        action_index: usize,
        action_total: usize,
    ) -> BoardFrame<'g> {
        let me = gs.active_player();
        let you = if me.id == gs.player1.id {
            &gs.player2
        } else {
            &gs.player1
        };
        let is_actionable =
            |card_no: &Option<&str>| -> bool {
                match card_no {
                    Some(cn) => actionable.iter().any(|a| a.as_str() == *cn),
                    None => false,
                }
            };

        let stage_slot = |cid: i16| -> Slot<'g> {
            if cid == EMPTY_SLOT {
                Slot::empty()
            } else {
                let waited = gs.mods.get_orientation_modifier(cid).as_deref() == Some("wait");
                Slot {
                    card_no: gs
                        .card_database
                        .get_card(cid)
                        .map(|c| c.card_no.as_ref()),
                    actionable: false,
                    waited,
                    hidden: false,
                }
            }
        };
        let mut p2_stage = [Slot::empty(), Slot::empty(), Slot::empty()];
        let mut p1_stage = [Slot::empty(), Slot::empty(), Slot::empty()];
        for i in 0..STAGE_SIZE {
            p2_stage[i] = stage_slot(you.stage.stage[i]);
            p1_stage[i] = stage_slot(me.stage.stage[i]);
        }
        for s in p2_stage.iter_mut() {
            s.actionable = is_actionable(&s.card_no);
        }
        for s in p1_stage.iter_mut() {
            s.actionable = is_actionable(&s.card_no);
        }

        // Live/success zone: 3 slots, empty padded
        // Live-set cards stay face-down (presence visible, identity hidden)
        // until the performance phase — see `live_set_hidden`. Scored
        // success cards are public and always shown.
        let live_hidden = live_set_hidden(&gs.current_phase);
        let live_slot = |cid: Option<i16>| -> Slot<'g> {
            match cid {
                Some(id) if id != EMPTY_SLOT => {
                    let waited = gs.mods.get_orientation_modifier(id).as_deref() == Some("wait");
                    Slot {
                        card_no: gs.card_database.get_card(id).map(|c| c.card_no.as_ref()),
                        actionable: false,
                        waited,
                        hidden: false,
                    }
                }
                _ => Slot::empty(),
            }
        };
        let live_set_slot = |cid: Option<i16>| -> Slot<'g> {
            match cid {
                Some(id) if id != EMPTY_SLOT => {
                    let waited = gs.mods.get_orientation_modifier(id).as_deref() == Some("wait");
                    Slot {
                        card_no: gs.card_database.get_card(id).map(|c| c.card_no.as_ref()),
                        actionable: false,
                        waited,
                        hidden: live_hidden,
                    }
                }
                _ => Slot::empty(),
            }
        };
        let mut p2_live = [Slot::empty(), Slot::empty(), Slot::empty()];
        let mut p1_live = [Slot::empty(), Slot::empty(), Slot::empty()];
        let mut p2_live_set = [Slot::empty(), Slot::empty(), Slot::empty()];
        let mut p1_live_set = [Slot::empty(), Slot::empty(), Slot::empty()];
        for i in 0..3 {
            let mut s = live_slot(you.success_live_card_zone.cards.get(i).copied());
            s.actionable = is_actionable(&s.card_no);
            p2_live[i] = s;
            let mut s = live_slot(me.success_live_card_zone.cards.get(i).copied());
            s.actionable = is_actionable(&s.card_no);
            p1_live[i] = s;
            let mut s = live_set_slot(you.live_card_zone.cards.get(i).copied());
            s.actionable = is_actionable(&s.card_no);
            p2_live_set[i] = s;
            let mut s = live_set_slot(me.live_card_zone.cards.get(i).copied());
            s.actionable = is_actionable(&s.card_no);
            p1_live_set[i] = s;
        }

        if self.hand_cursor >= me.hand.cards.len().max(1) {
            self.hand_cursor = 0;
            self.hand_offset = 0;
        }
        let start = self.hand_offset.min(me.hand.cards.len());
        let end = (start + HAND_VISIBLE).min(me.hand.cards.len());
        let mut hand: Vec<Slot> = (start..end)
            .map(|i| {
                let cid = me.hand.cards[i];
                let waited = gs.mods.get_orientation_modifier(cid).as_deref() == Some("wait");
                let card_no = gs.card_database.get_card(cid).map(|c| c.card_no.as_ref());
                Slot {
                    actionable: is_actionable(&card_no),
                    card_no,
                    waited,
                    hidden: false,
                }
            })
            .collect();
        while hand.len() < HAND_VISIBLE {
            hand.push(Slot::empty());
        }

        let hand_cursor_disp = (!me.hand.cards.is_empty() && self.focus == Focus::Hand)
            .then(|| self.hand_cursor.saturating_sub(start))
            .filter(|&w| w < HAND_VISIBLE);
        let own_stage_cursor_disp = if self.focus == Focus::OwnStage { Some(self.own_stage_cursor) } else { None };
        let opp_stage_cursor_disp = if self.focus == Focus::OppStage { Some(self.opp_stage_cursor) } else { None };
        let focused_card = match self.focus {
            Focus::Hand => me
                .hand
                .cards
                .get(self.hand_cursor)
                .and_then(|&cid| gs.card_database.get_card(cid))
                .map(|c| c.card_no.as_ref()),
            Focus::OwnStage => p1_stage[self.own_stage_cursor].card_no,
            Focus::OppStage => p2_stage[self.opp_stage_cursor].card_no,
        };

        // NOTE: p1_info/p2_info hearts+blade summaries used to be computed
        // here every frame, but no renderer reads them (the board bar shows
        // the action line; overlay builds its own stats). Deleted to save
        // per-frame iteration + String churn — see git history if the text
        // panes ever need them back.
        BoardFrame {
            header: format!(
                "T{} {:?} {}",
                gs.turn_number,
                gs.current_phase,
                if me.id == gs.player1.id { "P1>" } else { "P2>" }
            ),
            action_count: format!("{}/{}", action_index + 1, action_total),
            action_line: action_line.to_string(),
            // Unread by any renderer (see note above): kept as empty fields
            // for BoardFrame shape compatibility.
            p2_info: [String::new(), String::new()],
            p1_info: [String::new(), String::new()],
            p2_stage,
            p1_stage,
            p2_live,
            p1_live,
            p2_live_set,
            p1_live_set,
            hand,
            hand_more: end < me.hand.cards.len(),
            hand_offset_col: start,
            hand_cursor: hand_cursor_disp,
            own_stage_cursor: own_stage_cursor_disp,
            opp_stage_cursor: opp_stage_cursor_disp,
            focus: self.focus,
            focused_card,
        }
    }
}
