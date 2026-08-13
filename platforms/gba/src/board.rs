//! Board layout + cursor for the GBA turn screen.
//!
//! Pure data/logic (no agb drawing): given a `GameState` it produces a set of
//! clear text lines describing the zones (with actual card numbers), the
//! focused card, and the action bar. The tiled rendering happens in
//! `display.rs`. This mirrors the 3DS board (`docs/3ds/VISUAL_DESIGN.md`)
//! adapted to a 240x160 GBA screen.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use rabuka_engine::core::constants::{EMPTY_SLOT, STAGE_SIZE};
use rabuka_engine::game_state::GameState;

/// Everything needed to draw one board frame (as text lines).
pub struct BoardFrame {
    /// Board text rendered from the top of the screen.
    pub lines: Vec<String>,
    /// Bottom action-bar lines (from the text buffer).
    pub action_lines: Vec<String>,
    /// Card_no of the focused card (None if the focus is on an empty slot).
    pub focused_card: Option<String>,
}

/// 2D cursor: row 0 = player stage, row 1 = player hand; col = slot within.
pub struct Board {
    pub row: usize,
    pub col: usize,
}

impl Board {
    pub fn new() -> Self {
        Board { row: 0, col: 0 }
    }

    /// Move the cursor with the D-pad over a grid of `col_sizes` (per row).
    pub fn update_cursor(&mut self, up: bool, down: bool, left: bool, right: bool, col_sizes: &[usize]) {
        if col_sizes.is_empty() {
            self.row = 0;
            self.col = 0;
            return;
        }
        let rows = col_sizes.len();
        if self.row >= rows {
            self.row = 0;
        }
        if up {
            self.row = (self.row + rows - 1) % rows;
        } else if down {
            self.row = (self.row + 1) % rows;
        }
        let ncols = col_sizes[self.row].max(1);
        if self.col >= ncols {
            self.col = ncols - 1;
        }
        if right {
            self.col = (self.col + 1) % ncols;
        } else if left {
            self.col = (self.col + ncols - 1) % ncols;
        }
    }

    pub fn build(
        &mut self,
        gs: &GameState,
        up: bool,
        down: bool,
        left: bool,
        right: bool,
        action_lines: Vec<String>,
    ) -> BoardFrame {
        let me = gs.active_player();
        let you = if me.id == gs.player1.id {
            &gs.player2
        } else {
            &gs.player1
        };

        let header = format!(
            "T{} {:?} {}",
            gs.turn_number,
            gs.current_phase,
            if me.id == gs.player1.id { "P1>" } else { "P2>" }
        );

        let mut lines: Vec<String> = Vec::new();
        lines.push(header);
        lines.push(format!(
            "P2 h:{} e:{} dk:{} w:{}",
            you.hand.len(),
            you.energy_zone.active_count(),
            you.main_deck.len(),
            you.waitroom.len()
        ));

        // Opponent stage (never focused).
        let opp_stage: Vec<String> = (0..STAGE_SIZE)
            .map(|i| card_label(gs, you.stage.stage[i]))
            .collect();
        lines.push(format!("P2 STG: {}", format_zone(&opp_stage, None)));

        lines.push(format!(
            "P1 h:{} e:{} dk:{} w:{}",
            me.hand.len(),
            me.energy_zone.active_count(),
            me.main_deck.len(),
            me.waitroom.len()
        ));

        // Focusable grid: row 0 = player stage (3), row 1 = player hand.
        let stage: Vec<String> = (0..STAGE_SIZE)
            .map(|i| card_label(gs, me.stage.stage[i]))
            .collect();
        let hand: Vec<String> = me
            .hand
            .cards
            .iter()
            .map(|&cid| card_label(gs, cid))
            .collect();
        let col_sizes = [stage.len(), hand.len()];
        self.update_cursor(up, down, left, right, &col_sizes);

        let stage_focus = if self.row == 0 { Some(self.col) } else { None };
        lines.push(format!("P1 STG: {}", format_zone(&stage, stage_focus)));

        let hand_focus = if self.row == 1 { Some(self.col) } else { None };
        let hand_shown: Vec<String> = hand.iter().take(6).cloned().collect();
        lines.push(format!("HAND: {}", format_zone(&hand_shown, hand_focus)));

        let focused_card = match (self.row, self.col) {
            (0, c) if c < stage.len() && !stage[c].is_empty() => Some(stage[c].clone()),
            (1, c) if c < hand.len() && !hand[c].is_empty() => Some(hand[c].clone()),
            _ => None,
        };
        lines.push(match &focused_card {
            Some(cn) => format!("> {}", cn),
            None => String::new(),
        });
        lines.push("Select: Actions  R: detail".to_string());

        BoardFrame {
            lines,
            action_lines,
            focused_card,
        }
    }
}

fn card_label(gs: &GameState, cid: i16) -> String {
    if cid == EMPTY_SLOT {
        String::new()
    } else {
        gs.card_database
            .get_card(cid)
            .map(|c| c.card_no.to_string())
            .unwrap_or_else(|| format!("#{cid}"))
    }
}

/// Render a zone's cards as " [001] [002]" with the focused card marked ">[001]".
fn format_zone(cards: &[String], focus: Option<usize>) -> String {
    let mut s = String::new();
    for (i, c) in cards.iter().enumerate() {
        let label = if c.is_empty() { "..".to_string() } else { c.clone() };
        if Some(i) == focus {
            s.push_str(&format!(">{label} "));
        } else {
            s.push_str(&format!(" {label} "));
        }
    }
    s
}
