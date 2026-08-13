#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

extern crate alloc;

use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

use rabuka_engine::card::Card;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::platform_ui;
use rabuka_engine::game_state::GameState;
use rabuka_engine::rng;

use rabuka_gba::board::Board;
use rabuka_gba::decks_baked::DECKS;
use rabuka_gba::display::Display;
use rabuka_gba::input::{Button, Input};

struct GbaUi<'u, 'd> {
    display: &'u mut Display<'d>,
    input: &'u mut Input,
    board: Board,
    board_view: bool,
}

impl<'u, 'd> platform_ui::PlatformUi for GbaUi<'u, 'd> {
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
    fn wait_vblank(&mut self) {
        self.display.wait();
    }
    fn render_board(&mut self, gs: &GameState) -> bool {
        // Select toggles between the Board view and the Action view.
        if self.input.just_pressed(Button::Select) {
            self.board_view = !self.board_view;
        }
        if self.board_view {
            let left = self.input.just_pressed(Button::Left);
            let right = self.input.just_pressed(Button::Right);
            let up = self.input.just_pressed(Button::Up);
            let down = self.input.just_pressed(Button::Down);
            let action_lines: alloc::vec::Vec<alloc::string::String> = self
                .display
                .text()
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.to_string())
                .collect();
            let frame = self.board.build(gs, up, down, left, right, action_lines);
            if self.input.just_pressed(Button::R) {
                if let Some(cn) = &frame.focused_card {
                    self.card_detail(gs, cn.clone());
                    return true;
                }
            }
            self.display.render_board_frame(&frame);
            true
        } else {
            self.display.render_action_text();
            false
        }
    }
}

impl<'u, 'd> GbaUi<'u, 'd> {
    /// Show the focused card's art + stats until A/B/L/R is pressed.
    fn card_detail(&mut self, gs: &GameState, card_no: alloc::string::String) {
        use rabuka_gba::card_art_gen::CARD_ART;
        let art = CARD_ART.iter().find(|a| a.card_no == card_no);
        let mut lines: alloc::vec::Vec<alloc::string::String> = alloc::vec::Vec::new();
        if let Some(card) = gs.card_database.get_card_by_no(&card_no) {
            lines.push(format!("[{}] {}", card.card_no, card.name));
            lines.push(stat_line(card));
        } else {
            lines.push(card_no);
        }
        self.display.render_card_detail(art, &lines);
        loop {
            self.input.poll();
            if self.input.just_pressed(Button::A)
                || self.input.just_pressed(Button::B)
                || self.input.just_pressed(Button::L)
                || self.input.just_pressed(Button::R)
            {
                return;
            }
            self.display.wait();
        }
    }
}

fn load_deck_cards(
    _decks: &[rabuka_gba::decks_baked::DeckInfo],
    idx1: usize,
    idx2: usize,
) -> Vec<Card> {
    let mut cards = rabuka_engine::game::deck_parser::load_two_decks(idx1, idx2);
    CardLoader::attach_abilities(&mut cards);
    cards
}

/// Compact single-line card stats, mirroring the 3DS stat line.
fn stat_line(card: &Card) -> alloc::string::String {
    use rabuka_engine::card::CardType;
    let mut s = alloc::string::String::new();
    match card.card_type {
        CardType::Member => {
            if let Some(cost) = card.cost {
                if cost > 0 {
                    s.push_str(&format!("E{} ", cost));
                }
            }
            if let Some(bh) = &card.base_heart {
                for (c, v) in bh.hearts.iter() {
                    s.push_str(&format!("{}{} ", c.short_label(), v));
                }
            }
            if card.blade > 0 {
                s.push_str(&format!("BL{}", card.blade));
            }
        }
        CardType::Live => {
            if let Some(score) = card.score {
                if score > 0 {
                    s.push_str(&format!("SC{} ", score));
                }
            }
            if let Some(nh) = &card.need_heart {
                for (c, v) in nh.hearts.iter() {
                    s.push_str(&format!("{}{} ", c.short_label(), v));
                }
            }
        }
        CardType::Energy => {}
    }
    s
}

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    let mut display = Display::new(gba.graphics.get());
    let mut input = Input::new();
    rng::seed(0x5EED);

    let decks = DECKS;
    let names: Vec<&str> = decks.iter().map(|d| d.name).collect();

    // Run the whole flow (mode select -> deck select -> match) forever. If a
    // match ends early (e.g. the player presses B to pass at the RPS screen, or
    // a game result is reached), restart cleanly at the mode select instead of
    // dropping into a frozen black screen.
    loop {
        let ui = GbaUi {
            display: &mut display,
            input: &mut input,
            board: Board::new(),
            board_view: true,
        };
        platform_ui::run_embedded_game(ui, &names, |i| decks[i].cards, |a, b| {
            load_deck_cards(decks, a, b)
        });
    }
}
