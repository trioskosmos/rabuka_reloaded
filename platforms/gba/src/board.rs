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
use rabuka_engine::game_state::GameState;

/// Cards shown per hand window.
pub const HAND_VISIBLE: usize = 6;

/// One drawable card slot on the board.
#[derive(Clone)]
pub struct Slot {
    /// Card number, or None for an empty slot.
    pub card_no: Option<String>,
    /// The card is referenced by one of the currently available actions.
    pub actionable: bool,
}

impl Slot {
    fn empty() -> Slot {
        Slot {
            card_no: None,
            actionable: false,
        }
    }
}

/// Everything needed to draw one board frame.
pub struct BoardFrame {
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
    pub p2_stage: [Slot; 3],
    pub p1_stage: [Slot; 3],
    /// Visible hand window.
    pub hand: Vec<Slot>,
    /// True when more hand cards exist to the right of the window.
    pub hand_more: bool,
    /// First hand card index of the visible window (for cursor mapping).
    pub hand_offset_col: usize,
    /// Hand-cursor position within the window (None if the hand is empty).
    pub hand_cursor: Option<usize>,
    /// Card number under the hand cursor (None if empty slot / empty hand).
    pub focused_card: Option<String>,
}

/// Board state: the hand cursor (absolute index into the full hand) and the
/// scroll window keeping it visible.
pub struct Board {
    hand_cursor: usize,
    hand_offset: usize,
}

impl Board {
    pub fn new() -> Self {
        Board {
            hand_cursor: 0,
            hand_offset: 0,
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

    pub fn build(
        &mut self,
        gs: &GameState,
        actionable: &[String],
        action_line: &str,
        action_index: usize,
        action_total: usize,
    ) -> BoardFrame {
        let me = gs.active_player();
        let you = if me.id == gs.player1.id {
            &gs.player2
        } else {
            &gs.player1
        };
        let is_actionable =
            |card_no: &Option<String>| -> bool {
                match card_no {
                    Some(cn) => actionable.iter().any(|a| a == cn),
                    None => false,
                }
            };

        let stage_slot = |cid: i16| -> Slot {
            if cid == EMPTY_SLOT {
                Slot::empty()
            } else {
                Slot {
                    card_no: gs
                        .card_database
                        .get_card(cid)
                        .map(|c| c.card_no.to_string()),
                    actionable: false,
                }
            }
        };
        let mut p2_stage: Vec<Slot> = (0..STAGE_SIZE)
            .map(|i| stage_slot(you.stage.stage[i]))
            .collect();
        let mut p1_stage: Vec<Slot> = (0..STAGE_SIZE)
            .map(|i| stage_slot(me.stage.stage[i]))
            .collect();
        for s in p2_stage.iter_mut() {
            s.actionable = is_actionable(&s.card_no);
        }
        for s in p1_stage.iter_mut() {
            s.actionable = is_actionable(&s.card_no);
        }

        let hand_cards: Vec<Option<String>> = me
            .hand
            .cards
            .iter()
            .map(|&cid| gs.card_database.get_card(cid).map(|c| c.card_no.to_string()))
            .collect();
        if self.hand_cursor >= hand_cards.len().max(1) {
            self.hand_cursor = 0;
            self.hand_offset = 0;
        }
        let start = self.hand_offset.min(hand_cards.len());
        let end = (start + HAND_VISIBLE).min(hand_cards.len());
        let mut hand: Vec<Slot> = (start..end)
            .map(|i| Slot {
                card_no: hand_cards[i].clone(),
                actionable: is_actionable(&hand_cards[i]),
            })
            .collect();
        while hand.len() < HAND_VISIBLE {
            hand.push(Slot::empty());
        }

        let focused_card = if hand_cards.is_empty() {
            None
        } else {
            hand_cards[self.hand_cursor].clone()
        };
        let hand_cursor = (!hand_cards.is_empty())
            .then(|| self.hand_cursor - start)
            .filter(|&w| w < HAND_VISIBLE);

        BoardFrame {
            header: format!(
                "T{} {:?} {}",
                gs.turn_number,
                gs.current_phase,
                if me.id == gs.player1.id { "P1>" } else { "P2>" }
            ),
            action_count: format!("{}/{}", action_index + 1, action_total),
            action_line: action_line.to_string(),
            p2_info: [
                format!(
                    "P2 h{} e{}",
                    you.hand.cards.len(),
                    you.energy_zone.active_count()
                ),
                format!(
                    "d{} w{} s{}",
                    you.main_deck.cards.len(),
                    you.waitroom.cards.len(),
                    you.success_live_card_zone.cards.len()
                ),
            ],
            p1_info: [
                format!(
                    "P1 h{} e{}",
                    me.hand.cards.len(),
                    me.energy_zone.active_count()
                ),
                format!(
                    "d{} w{} s{}",
                    me.main_deck.cards.len(),
                    me.waitroom.cards.len(),
                    me.success_live_card_zone.cards.len()
                ),
            ],
            p2_stage: [p2_stage[0].clone(), p2_stage[1].clone(), p2_stage[2].clone()],
            p1_stage: [p1_stage[0].clone(), p1_stage[1].clone(), p1_stage[2].clone()],
            hand,
            hand_more: end < hand_cards.len(),
            hand_offset_col: start,
            hand_cursor,
            focused_card,
        }
    }
}
