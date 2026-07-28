/// Platform-agnostic UI trait for shared game loop functions.
/// Each platform implements this trait for its Display+Input pair.
use crate::game::game_setup;
use crate::game_state::GameState;

pub trait PlatformUi {
    fn clear_screen(&mut self);
    fn println(&mut self, text: &str);
    fn draw_menu(&mut self, items: &[&str], selected: usize, title: &str);
    fn poll_input(&mut self);
    fn just_pressed_a(&self) -> bool;
    fn just_pressed_b(&self) -> bool;
    fn just_pressed_up(&self) -> bool;
    fn just_pressed_down(&self) -> bool;
    fn just_pressed_start(&self) -> bool;
    fn wait_vblank(&mut self);
}

/// AI turn: pick a random action and execute it.
pub fn ai_turn(gs: &mut GameState, acts: &[game_setup::Action]) -> bool {
    let _ = game_setup::execute_action(gs, &acts[crate::rng::rand_range(acts.len())]);
    true
}

/// Show the game result screen and wait for a button press.
pub fn show_result(ui: &mut dyn PlatformUi, gs: &GameState) {
    use std::format;
    loop {
        ui.clear_screen();
        ui.println("=== GAME OVER ===");
        ui.println(&format!("{:?}", gs.game_result));
        ui.println(&format!(
            "P1 success:{} wait:{}",
            gs.player1.success_live_card_zone.cards.len(),
            gs.player1.waitroom.cards.len()
        ));
        ui.println(&format!(
            "P2 success:{} wait:{}",
            gs.player2.success_live_card_zone.cards.len(),
            gs.player2.waitroom.cards.len()
        ));
        ui.println("Press A to continue");
        ui.poll_input();
        if ui.just_pressed_a() || ui.just_pressed_start() {
            break;
        }
        ui.wait_vblank();
    }
}

/// Select from a list of items. Returns the selected index.
pub fn select(ui: &mut dyn PlatformUi, items: &[&str], title: &str) -> usize {
    let mut sel = 0;
    loop {
        ui.draw_menu(items, sel, title);
        ui.poll_input();
        if ui.just_pressed_up() {
            sel = sel.saturating_sub(1);
        } else if ui.just_pressed_down() {
            if sel + 1 < items.len() {
                sel += 1;
            }
        } else if ui.just_pressed_a() {
            return sel;
        }
        ui.wait_vblank();
    }
}

/// Select from a list of string items with optional skip. Returns None if skipped.
pub fn menu_select(
    ui: &mut dyn PlatformUi,
    items: &[std::string::String],
    title: &str,
    allow_skip: bool,
) -> Option<usize> {
    let mut all_items: std::vec::Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    let skip_idx = if allow_skip {
        all_items.push("[Skip]");
        Some(all_items.len() - 1)
    } else {
        None
    };
    let mut sel = 0;
    loop {
        ui.draw_menu(&all_items, sel, title);
        ui.poll_input();
        if ui.just_pressed_up() {
            sel = sel.saturating_sub(1);
        } else if ui.just_pressed_down() {
            if sel + 1 < all_items.len() {
                sel += 1;
            }
        } else if ui.just_pressed_a() {
            if Some(sel) == skip_idx {
                return None;
            }
            return Some(sel);
        } else if ui.just_pressed_b() && allow_skip {
            return None;
        }
        ui.wait_vblank();
    }
}
