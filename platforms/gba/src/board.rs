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

use rabuka_engine::card::HeartColor;
use rabuka_engine::core::constants::{EMPTY_SLOT, STAGE_SIZE};
use rabuka_engine::game_state::GameState;

/// Cards shown per hand window — badge on card saves gap, so pitch = width.
const SCREEN_COLS: i32 = 30;
const HAND_PITCH_TILES: i32 = 3;
pub const HAND_VISIBLE: usize = (SCREEN_COLS / HAND_PITCH_TILES) as usize; // 10

/// One drawable card slot on the board.
#[derive(Clone)]
pub struct Slot {
    /// Card number, or None for an empty slot.
    pub card_no: Option<String>,
    /// The card is referenced by one of the currently available actions.
    pub actionable: bool,
    /// Card is in wait state (tapped 90° on 3DS). On GBA we render a
    /// wait-state indicator instead of rotating (tile grid is fixed).
    pub waited: bool,
}

impl Slot {
    fn empty() -> Slot {
        Slot {
            card_no: None,
            actionable: false,
            waited: false,
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
    /// Live/success zone (3 slots, small) — victory condition.
    pub p2_live: [Slot; 3],
    pub p1_live: [Slot; 3],
    /// Live card set zone (3 slots) — where live cards are placed during Live phase.
    pub p2_live_set: [Slot; 3],
    pub p1_live_set: [Slot; 3],
    /// Visible hand window.
    pub hand: Vec<Slot>,
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
    pub focused_card: Option<String>,
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
                let waited = gs.mods.get_orientation_modifier(cid).as_deref() == Some("wait");
                Slot {
                    card_no: gs
                        .card_database
                        .get_card(cid)
                        .map(|c| c.card_no.to_string()),
                    actionable: false,
                    waited,
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

        // Live/success zone: 3 slots, empty padded
        let live_slot = |cid: Option<i16>| -> Slot {
            match cid {
                Some(id) if id != EMPTY_SLOT => {
                    let waited = gs.mods.get_orientation_modifier(id).as_deref() == Some("wait");
                    Slot {
                        card_no: gs.card_database.get_card(id).map(|c| c.card_no.to_string()),
                        actionable: false,
                        waited,
                    }
                }
                _ => Slot::empty(),
            }
        };
        let p2_live_vec: Vec<Slot> = (0..3)
            .map(|i| {
                let cid = you.success_live_card_zone.cards.get(i).copied();
                let mut s = live_slot(cid);
                s.actionable = is_actionable(&s.card_no);
                s
            })
            .collect();
        let p1_live_vec: Vec<Slot> = (0..3)
            .map(|i| {
                let cid = me.success_live_card_zone.cards.get(i).copied();
                let mut s = live_slot(cid);
                s.actionable = is_actionable(&s.card_no);
                s
            })
            .collect();
        // Live card set zone (where live cards are placed before performance)
        let p2_live_set_vec: Vec<Slot> = (0..3)
            .map(|i| {
                let cid = you.live_card_zone.cards.get(i).copied();
                let mut s = live_slot(cid);
                s.actionable = is_actionable(&s.card_no);
                s
            })
            .collect();
        let p1_live_set_vec: Vec<Slot> = (0..3)
            .map(|i| {
                let cid = me.live_card_zone.cards.get(i).copied();
                let mut s = live_slot(cid);
                s.actionable = is_actionable(&s.card_no);
                s
            })
            .collect();

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
            .map(|i| {
                let cid = me.hand.cards[i];
                let waited = gs.mods.get_orientation_modifier(cid).as_deref() == Some("wait");
                Slot {
                    card_no: hand_cards[i].clone(),
                    actionable: is_actionable(&hand_cards[i]),
                    waited,
                }
            })
            .collect();
        while hand.len() < HAND_VISIBLE {
            hand.push(Slot::empty());
        }

        let hand_cursor_disp = (!hand_cards.is_empty() && self.focus == Focus::Hand)
            .then(|| self.hand_cursor.saturating_sub(start))
            .filter(|&w| w < HAND_VISIBLE);
        let own_stage_cursor_disp = if self.focus == Focus::OwnStage { Some(self.own_stage_cursor) } else { None };
        let opp_stage_cursor_disp = if self.focus == Focus::OppStage { Some(self.opp_stage_cursor) } else { None };
        let focused_card = match self.focus {
            Focus::Hand => if hand_cards.is_empty() { None } else { hand_cards[self.hand_cursor].clone() },
            Focus::OwnStage => p1_stage[self.own_stage_cursor].card_no.clone(),
            Focus::OppStage => p2_stage[self.opp_stage_cursor].card_no.clone(),
        };

        // --- GBA texticon helpers (mirrors 3DS ui/text.rs) ---
        let heart_idx = |c: &HeartColor| match c {
            HeartColor::BAll | HeartColor::Draw | HeartColor::Score => None,
            _ => Some(c.index()),
        };
        let hearts_icon = |player: &rabuka_engine::player::Player| {
            let mut counts = [0u32; 8];
            for &cid in &player.stage.stage {
                if cid == EMPTY_SLOT { continue; }
                if let Some(card) = gs.card_database.get_card(cid) {
                    if let Some(ref bh) = card.base_heart {
                        let mult = gs.mods.heart_color_multiplier.get(&cid).copied();
                        for (col, cnt) in &bh.hearts {
                            if let Some(idx) = heart_idx(col) {
                                if let Some(hc) = mult { if hc != *col { continue; } }
                                counts[idx] += *cnt as u32;
                            }
                        }
                    }
                }
            }
            for (cid, mp) in &gs.mods.heart_modifiers {
                if !player.stage.stage.contains(cid) { continue; }
                for (col, val) in mp {
                    if let Some(idx) = heart_idx(col) {
                        counts[idx] = (counts[idx] as i32 + val.total()).max(0) as u32;
                    }
                }
            }
            let mut parts: Vec<String> = Vec::new();
            for (i, &cnt) in counts.iter().enumerate() {
                if cnt > 0 {
                    let name = match i {
                        0 => "heart_00", 1 => "heart_01", 2 => "heart_02",
                        3 => "heart_03", 4 => "heart_04", 5 => "heart_05",
                        6 => "heart_06", _ => "icon_all",
                    };
                    parts.push(format!("{{{{{}.png|{}}}}}{}", name, name, cnt));
                }
            }
            if parts.is_empty() { String::new() } else { parts.join(" ") }
        };
        let blade_total = |player: &rabuka_engine::player::Player| {
            let mut total: i32 = 0;
            for &cid in &player.stage.stage {
                if cid == EMPTY_SLOT { continue; }
                if let Some(card) = gs.card_database.get_card(cid) {
                    let is_wait = gs.mods.orientation_modifiers.get(&cid).map(|o| o.as_str()=="wait").unwrap_or(false);
                    if is_wait { continue; }
                    let bm = gs.mods.blade_modifiers.get(&cid).map(|m| m.total()).unwrap_or(0);
                    total += (card.blade as i32 + bm).max(0);
                }
            }
            total
        };
        let p2_hearts = hearts_icon(you);
        let p1_hearts = hearts_icon(me);
        let p2_blade = blade_total(you);
        let p1_blade = blade_total(me);
        let p2_hb = if p2_hearts.is_empty() && p2_blade==0 { String::new() } else if p2_hearts.is_empty() { format!("{{{{icon_blade.png|BLADE}}}}{}", p2_blade) } else if p2_blade==0 { p2_hearts.clone() } else { format!("{} {{{{icon_blade.png|BLADE}}}}{}", p2_hearts, p2_blade) };
        let p1_hb = if p1_hearts.is_empty() && p1_blade==0 { String::new() } else if p1_hearts.is_empty() { format!("{{{{icon_blade.png|BLADE}}}}{}", p1_blade) } else if p1_blade==0 { p1_hearts.clone() } else { format!("{} {{{{icon_blade.png|BLADE}}}}{}", p1_hearts, p1_blade) };

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
                    "H{} {{{{icon_energy.png|E}}}}{}/{}",
                    you.hand.cards.len(),
                    you.energy_zone.active_count(),
                    you.energy_zone.cards.len()
                ),
                if p2_hb.is_empty() {
                    format!("D{} W{} S{}", you.main_deck.cards.len(), you.waitroom.cards.len(), you.success_live_card_zone.cards.len())
                } else { p2_hb },
            ],
            p1_info: [
                format!(
                    "H{} {{{{icon_energy.png|E}}}}{}/{}",
                    me.hand.cards.len(),
                    me.energy_zone.active_count(),
                    me.energy_zone.cards.len()
                ),
                if p1_hb.is_empty() {
                    format!("D{} W{} S{}", me.main_deck.cards.len(), me.waitroom.cards.len(), me.success_live_card_zone.cards.len())
                } else { p1_hb },
            ],
            p2_stage: [p2_stage[0].clone(), p2_stage[1].clone(), p2_stage[2].clone()],
            p1_stage: [p1_stage[0].clone(), p1_stage[1].clone(), p1_stage[2].clone()],
            p2_live: [
                p2_live_vec[0].clone(),
                p2_live_vec[1].clone(),
                p2_live_vec[2].clone(),
            ],
            p1_live: [
                p1_live_vec[0].clone(),
                p1_live_vec[1].clone(),
                p1_live_vec[2].clone(),
            ],
            p2_live_set: [
                p2_live_set_vec[0].clone(),
                p2_live_set_vec[1].clone(),
                p2_live_set_vec[2].clone(),
            ],
            p1_live_set: [
                p1_live_set_vec[0].clone(),
                p1_live_set_vec[1].clone(),
                p1_live_set_vec[2].clone(),
            ],
            hand,
            hand_more: end < hand_cards.len(),
            hand_offset_col: start,
            hand_cursor: hand_cursor_disp,
            own_stage_cursor: own_stage_cursor_disp,
            opp_stage_cursor: opp_stage_cursor_disp,
            focus: self.focus,
            focused_card,
        }
    }
}
