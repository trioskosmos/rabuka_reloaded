//! The integrated board UI shared by the main binary and the auto-play smoke
//! test, mirroring the 3DS's two-screen flow on one GBA screen:
//!
//! - **Board view** (default): graphical board with card fronts, actionable
//!   badges and a bottom bar showing the selected action. Up/Down/A drive the
//!   engine's action list; Left/Right scroll the hand cursor; R pops the art
//!   detail of the cursored hand card.
//! - **Actions view**: full-screen action list (Select or B returns). Input
//!   stays with the engine so Up/Down/A/L/R work exactly like the text ports.

use alloc::vec::Vec;

use rabuka_engine::game::platform_ui;
use rabuka_engine::game_state::GameState;

use crate::board::Board;
use crate::card_art_gen::CARD_FRONTS;
use crate::display::Display;
use crate::input::{Button, Input};

/// Anything that can feed button presses to [`GbaUi`] (real hardware input or
/// a scripted driver in tests).
pub trait InputSource {
    fn poll(&mut self);
    fn just_pressed(&self, btn: Button) -> bool;
}

impl InputSource for Input {
    fn poll(&mut self) {
        Input::poll(self);
    }
    fn just_pressed(&self, btn: Button) -> bool {
        Input::just_pressed(self, btn)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum View {
    Board,
    Actions,
}

pub struct GbaUi<'u, 'd, I: InputSource> {
    pub display: &'u mut Display<'d>,
    pub input: &'u mut I,
    pub board: Board,
    view: View,
    actionable: Vec<alloc::string::String>,
    action_line: alloc::string::String,
    action_index: usize,
    action_total: usize,
}

impl<'u, 'd, I: InputSource> GbaUi<'u, 'd, I> {
    pub fn new(display: &'u mut Display<'d>, input: &'u mut I) -> Self {
        GbaUi {
            display,
            input,
            board: Board::new(),
            view: View::Board,
            actionable: Vec::new(),
            action_line: alloc::string::String::new(),
            action_index: 0,
            action_total: 0,
        }
    }

    fn render_board_view(&mut self, gs: &GameState) -> bool {
        // Start opens the in-game menu overlay (Game Log / Cards). The
        // blocking menu consumes input until closed; report the frame as
        // consumed so the engine skips its own navigation for it.
        if self.input.just_pressed(Button::Start) {
            crate::overlay::run_start_menu(self.display, self.input, gs);
            return true;
        }

        if self.input.just_pressed(Button::Select) {
            self.view = View::Actions;
            self.display.render_action_text();
            return false;
        }

        // L cycles board focus: Hand -> Own Stage -> Opp Stage
        if self.input.just_pressed(Button::L) {
            self.board.cycle_focus();
            let frame = self.board.build(
                gs,
                &self.actionable,
                &self.action_line,
                self.action_index,
                self.action_total,
            );
            self.display.render_board_frame(&frame);
            return true;
        }

        let left = self.input.just_pressed(Button::Left);
        let right = self.input.just_pressed(Button::Right);
        let scrolled = if left || right {
            let delta = if left { -1 } else { 1 };
            let hand_len = gs.active_player().hand.cards.len();
            self.board.move_focused(delta, hand_len)
        } else {
            false
        };

        // R pops detail of focused card (hand or stage); L already handled.
        if self.input.just_pressed(Button::R) && !scrolled {
            let frame = self.board.build(
                gs,
                &self.actionable,
                &self.action_line,
                self.action_index,
                self.action_total,
            );
            if let Some(cn) = &frame.focused_card {
                crate::menu::show_card_detail(self.display, self.input, gs, cn.clone());
                return true;
            }
        }

        let frame = self.board.build(
            gs,
            &self.actionable,
            &self.action_line,
            self.action_index,
            self.action_total,
        );
        self.display.render_board_frame(&frame);
        scrolled
    }
}

impl<'u, 'd, I: InputSource> platform_ui::PlatformUi for GbaUi<'u, 'd, I> {
    fn clear_screen(&mut self) {
        self.display.clear();
    }
    fn println(&mut self, text: &str) {
        self.display.println(text);
    }
    fn swap_buffers(&mut self) {
        self.display.swap_buffers();
    }
    fn poll_input(&mut self) {
        self.input.poll();
    }
    fn just_pressed_a(&self) -> bool {
        self.input.just_pressed(Button::A)
    }
    fn just_pressed_b(&self) -> bool {
        self.input.just_pressed(Button::B)
    }
    fn just_pressed_up(&self) -> bool {
        self.input.just_pressed(Button::Up)
    }
    fn just_pressed_down(&self) -> bool {
        self.input.just_pressed(Button::Down)
    }
    fn just_pressed_start(&self) -> bool {
        self.input.just_pressed(Button::Start)
    }
    fn just_pressed_l(&self) -> bool {
        self.input.just_pressed(Button::L)
    }
    fn just_pressed_r(&self) -> bool {
        self.input.just_pressed(Button::R)
    }
    fn just_pressed_left(&self) -> bool {
        self.input.just_pressed(Button::Left)
    }
    fn just_pressed_right(&self) -> bool {
        self.input.just_pressed(Button::Right)
    }
    fn wait_vblank(&mut self) {
        self.display.wait();
    }
    fn draw_card_image(
        &mut self,
        card_no: &str,
        x: i32,
        y: i32,
        cols: i32,
        rows: i32,
        _palette_index: usize,
    ) {
        // Find the card front in CARD_FRONTS (8bpp shared MASTER_PAL)
        if let Some(front) = CARD_FRONTS.iter().find(|f| f.card_no == card_no) {
            let ts = unsafe { agb::display::tiled::TileSet::new(front.tiles, agb::display::tiled::TileFormat::EightBpp) };
            let mut art_bg = agb::display::tiled::RegularBackground::new(
                agb::display::Priority::P0,
                agb::display::tiled::RegularBackgroundSize::Background32x32,
                agb::display::tiled::TileFormat::EightBpp,
            );
            for ty in 0..rows {
                for tx in 0..cols {
                    let sidx = (ty * cols + tx) as u16;
                    art_bg.set_tile(
                        (x + tx, y + ty),
                        &ts,
                        agb::display::tiled::TileSetting::new(sidx, agb::display::tiled::TileEffect::new(false, false, 0)),
                    );
                }
            }
        }
    }

    fn show_board_overlay(&mut self, gs: &GameState) {
        let frame = self.board.build(
            gs,
            &self.actionable,
            &self.action_line,
            self.action_index,
            self.action_total,
        );
        self.display.render_board_frame(&frame);
        self.display.swap_buffers();
    }
    fn set_actionable_cards(&mut self, card_nos: &[alloc::string::String]) {
        self.actionable.clear();
        self.actionable.extend_from_slice(card_nos);
    }
    fn set_selected_action(&mut self, desc: &str, index: usize, total: usize) {
        use alloc::string::ToString;
        self.action_line = desc.to_string();
        self.action_index = index;
        self.action_total = total;
    }
    fn render_board(&mut self, gs: &GameState) -> bool {
        match self.view {
            View::Board => self.render_board_view(gs),
            View::Actions => {
                if self.input.just_pressed(Button::Start) {
                    crate::overlay::run_start_menu(self.display, self.input, gs);
                    return true;
                }
                if self.input.just_pressed(Button::Select)
                    || self.input.just_pressed(Button::B)
                {
                    self.view = View::Board;
                }
                // Re-render the list every frame so engine-driven scrolling
                // shows live; do not consume input.
                self.display.render_action_text();
                false
            }
        }
    }
}
