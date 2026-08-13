#[cfg(feature = "no_std")]
use alloc::format;
#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};
#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no_std"))]
use std::format;

use crate::card::Card;
use crate::card::CardDatabase;
use crate::game::deck_builder::DeckBuilder;
use crate::game::game_setup;
use crate::game_state::GameResult;
use crate::game_state::GameState;
use crate::player::Player;
use crate::rng;
use crate::turn::TurnEngine;
use crate::Arc;

pub trait PlatformUi {
    fn clear_screen(&mut self);
    fn println(&mut self, text: &str);
    fn swap_buffers(&mut self);
    fn poll_input(&mut self);
    fn just_pressed_a(&self) -> bool;
    fn just_pressed_b(&self) -> bool;
    fn just_pressed_up(&self) -> bool;
    fn just_pressed_down(&self) -> bool;
    fn just_pressed_start(&self) -> bool;
    fn wait_vblank(&mut self);

    /// Render a graphical board for `gs` (consoles that support one). Called
    /// from the human-turn loop in place of `swap_buffers` so a board can be
    /// drawn. Return `true` if the renderer consumed this frame's input (e.g.
    /// it is in a board-navigation mode that handles Up/Down itself) so the
    /// engine skips its own action navigation/execution for this frame. The
    /// default just swaps the text buffer and does not consume input.
    fn render_board(&mut self, _gs: &GameState) -> bool {
        self.swap_buffers();
        false
    }

    /// Shoulder buttons. Default off; consoles without them keep the menu
    /// behaviour unchanged. L/R open the full-text detail viewer on the
    /// currently highlighted option.
    fn just_pressed_l(&self) -> bool {
        false
    }
    fn just_pressed_r(&self) -> bool {
        false
    }

    /// Max characters that fit on one menu line, in half-width columns
    /// (a CJK glyph is 2 columns). Used to keep each option on a single
    /// line and to wrap the detail viewer. 30 fits a GBA screen row
    /// (240px / 8px tiles).
    fn option_cols(&self) -> usize {
        30
    }
}

/// Width of a single character in half-width columns (CJK glyphs = 2).
fn char_cols(c: char) -> usize {
    if (c as u32) < 0x1100 {
        1
    } else {
        2
    }
}

/// Wrap `text` into lines of at most `cols` half-width columns, honouring
/// existing newlines as hard breaks.
fn wrap_text(text: &str, cols: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut w = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            if !cur.is_empty() {
                out.push(core::mem::take(&mut cur));
            }
            w = 0;
            continue;
        }
        let cw = char_cols(ch);
        if w + cw > cols && !cur.is_empty() {
            out.push(core::mem::take(&mut cur));
            w = 0;
        }
        cur.push(ch);
        w += cw;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Truncate `text` to a single line of at most `cols` columns, marking the
/// cut so the player knows more is available (see the L/R detail viewer).
fn one_line(text: &str, cols: usize) -> String {
    let mut s = String::new();
    let mut w = 0usize;
    for ch in text.chars() {
        let cw = char_cols(ch);
        if w + cw > cols {
            s.push_str("..");
            break;
        }
        s.push(ch);
        w += cw;
    }
    s
}

/// Scrollable full-text viewer. Shows `lines` in a window; Up/Down scroll,
/// and A/B/L/R close it back to the menu without disturbing the option list's
/// own scroll position.
fn show_lines(ui: &mut dyn PlatformUi, lines: &[String]) {
    let mut off = 0usize;
    const H: usize = 8; // 9 screen rows, one held for a hint bar
    loop {
        ui.clear_screen();
        ui.println("A/B close, Up/Down scroll");
        let end = (off + H).min(lines.len());
        for l in off..end {
            ui.println(&lines[l]);
        }
        if lines.len() > end {
            ui.println(&format!("  .. {} more", lines.len() - end));
        }
        ui.swap_buffers();
        ui.poll_input();
        if ui.just_pressed_up() {
            off = off.saturating_sub(1);
        } else if ui.just_pressed_down() && off + H < lines.len() {
            off += 1;
        } else if ui.just_pressed_a()
            || ui.just_pressed_b()
            || ui.just_pressed_l()
            || ui.just_pressed_r()
        {
            return;
        }
        ui.wait_vblank();
    }
}

/// Wrap `text` and show it in a scrollable viewer (L: full action/ability text).
fn show_detail(ui: &mut dyn PlatformUi, text: &str) {
    let cols = ui.option_cols();
    let lines = wrap_text(text, cols);
    show_lines(ui, &lines);
}

/// Format a heart map as "R2 B3"-style codes (like the 3DS stat line).
fn heart_str(hm: &crate::card::HeartMap) -> String {
    hm.iter()
        .map(|(c, v)| format!("{}{}", c.short_label(), v))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Single-line stat summary, mirroring the 3DS card-detail stat line.
fn card_stat_text(card: &Card) -> String {
    use crate::card::CardType;
    match card.card_type {
        CardType::Member => {
            let mut s = String::new();
            if let Some(cost) = card.cost {
                if cost > 0 {
                    s.push_str(&format!("E{}  ", cost));
                }
            }
            let hs = card
                .base_heart
                .as_ref()
                .map(|bh| heart_str(&bh.hearts))
                .unwrap_or_default();
            if !hs.is_empty() {
                s.push_str(&hs);
                s.push_str("  ");
            }
            if card.blade > 0 {
                s.push_str(&format!("BL{}", card.blade));
            }
            s
        }
        CardType::Live => {
            let mut s = String::new();
            if let Some(score) = card.score {
                if score > 0 {
                    s.push_str(&format!("SC{}  ", score));
                }
            }
            let ns = card
                .need_heart
                .as_ref()
                .map(|nh| heart_str(&nh.hearts))
                .unwrap_or_default();
            if !ns.is_empty() {
                s.push_str(&ns);
            }
            s
        }
        CardType::Energy => String::new(),
    }
}

/// A card's ability text. In compact builds `ability_text()` is empty, so the
/// abilities are decoded from bytecode (which reconstructs `full_text`).
/// `{{...}}` icon markers are kept so the renderer can show them.
fn card_ability_text(card: &Card) -> String {
    let mut parts: Vec<String> = Vec::new();
    for ab in card.resolved_abilities() {
        let t = ab.full_text.trim();
        if !t.is_empty() {
            parts.push(t.to_string());
        }
    }
    if parts.is_empty() {
        card.ability_text().trim().to_string()
    } else {
        parts.join("\n")
    }
}

/// Scrollable card detail (R): header, stat line, then the ability text.
fn show_card_stats(ui: &mut dyn PlatformUi, card: &Card) {
    let cols = ui.option_cols();
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("[{}] {}", card.card_no, card.name));
    lines.push(card_stat_text(card));
    let ab = card_ability_text(card);
    if !ab.is_empty() {
        lines.extend(wrap_text(&ab, cols));
    }
    show_lines(ui, &lines);
}

/// AI turn: pick a random action and execute it.
pub fn ai_turn(gs: &mut GameState, acts: &[game_setup::Action]) -> bool {
    use crate::game_setup::ActionType;
    // Mulligan phases MUST be concluded, otherwise the AI can keep toggling
    // card selections forever and the game can never reach the main phase.
    // Prefer a Confirm/Skip over the per-card Select actions.
    for a in acts {
        if matches!(
            a.action_type,
            ActionType::ConfirmMulligan | ActionType::SkipMulligan
        ) {
            let _ = game_setup::execute_action(gs, a);
            return true;
        }
    }
    let _ = game_setup::execute_action(gs, &acts[crate::rng::rand_range(acts.len())]);
    true
}

/// Show the game result screen and wait for a button press.
pub fn show_result(ui: &mut dyn PlatformUi, gs: &GameState) {
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
    let mut sel: usize = 0;
    let mut scroll: usize = 0;
    const VIS: usize = 7;
    let cols = ui.option_cols();
    loop {
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + VIS {
            scroll = sel + 1 - VIS;
        }
        ui.clear_screen();
        ui.println(title);
        let end = (scroll + VIS).min(items.len());
        for n in scroll..end {
            let prefix = if n == sel { " >" } else { "  " };
            ui.println(&one_line(&format!("{prefix} {}", items[n]), cols));
        }
        if items.len() > end {
            ui.println(&format!("  .. {} more", items.len() - end));
        }
        ui.swap_buffers();
        ui.poll_input();
        if ui.just_pressed_up() {
            sel = if sel == 0 { items.len() - 1 } else { sel - 1 };
        } else if ui.just_pressed_down() {
            sel = if sel + 1 == items.len() { 0 } else { sel + 1 };
        } else if ui.just_pressed_l() || ui.just_pressed_r() {
            show_detail(ui, items[sel]);
        } else if ui.just_pressed_a() {
            return sel;
        }
        ui.wait_vblank();
    }
}

/// Select from a list of string items with optional skip. Returns None if skipped.
pub fn menu_select(
    ui: &mut dyn PlatformUi,
    items: &[String],
    title: &str,
    allow_skip: bool,
) -> Option<usize> {
    let mut all_items: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    let skip_idx = if allow_skip {
        all_items.push("[Skip]");
        Some(all_items.len() - 1)
    } else {
        None
    };
    let mut sel: usize = 0;
    let mut scroll: usize = 0;
    const VIS: usize = 7;
    let cols = ui.option_cols();
    loop {
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + VIS {
            scroll = sel + 1 - VIS;
        }
        ui.clear_screen();
        ui.println(title);
        let end = (scroll + VIS).min(all_items.len());
        for n in scroll..end {
            let prefix = if n == sel { " >" } else { "  " };
            ui.println(&one_line(&format!("{prefix} {}", all_items[n]), cols));
        }
        if all_items.len() > end {
            ui.println(&format!("  .. {} more", all_items.len() - end));
        }
        ui.swap_buffers();
        ui.poll_input();
        if ui.just_pressed_up() {
            sel = if sel == 0 { all_items.len() - 1 } else { sel - 1 };
        } else if ui.just_pressed_down() {
            sel = if sel + 1 == all_items.len() { 0 } else { sel + 1 };
        } else if ui.just_pressed_l() || ui.just_pressed_r() {
            show_detail(ui, &all_items[sel]);
        } else if ui.just_pressed_a() {
            if Some(sel) == skip_idx {
                return None;
            }
            return Some(sel);
        }
        ui.wait_vblank();
    }
}

/// Scrollable action list for a human player's turn.
/// Returns true if an action was executed, false if the turn was passed.
pub fn human_turn(
    ui: &mut dyn PlatformUi,
    gs: &mut GameState,
    acts: &[game_setup::Action],
) -> bool {
    let mut sel = 0;
    let mut scroll = 0;
    const VIS: usize = 5;
    let cols = ui.option_cols();
    loop {
        ui.clear_screen();
        ui.println(&format!("Turn {} | {:?}", gs.turn_number, gs.current_phase));
        let p1 = &gs.player1;
        let p2 = &gs.player2;
        let is_p1 = gs.active_player().id == "p1";
        let tag = |a: bool| if a { ">>" } else { "  " };
        ui.println(&format!(
            "{} P1 h:{} e:{} dk:{}",
            tag(is_p1),
            p1.hand.cards.len(),
            p1.energy_zone.active_count(),
            p1.main_deck.cards.len()
        ));
        ui.println(&format!(
            "{} P2 h:{} e:{} dk:{}",
            tag(!is_p1),
            p2.hand.cards.len(),
            p2.energy_zone.active_count(),
            p2.main_deck.cards.len()
        ));
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + VIS {
            scroll = sel + 1 - VIS;
        }
        let end = (scroll + VIS).min(acts.len());
        for a in scroll..end {
            let prefix = if a == sel { " >" } else { "  " };
            let line = acts[a].description.lines().next().unwrap_or("");
            let tag_str = match &acts[a].parameters {
                Some(p) => p
                    .card_no
                    .as_ref()
                    .and_then(|n| gs.card_database.get_card_by_no(n))
                    .map(|c| format!(" [{}]", c.name))
                    .unwrap_or_default(),
                None => String::new(),
            };
            ui.println(&one_line(&format!("{prefix}{line}{tag_str}"), cols));
        }
        if acts.len() > end {
            ui.println(&format!("  .. {} more", acts.len() - end));
        }
        let consumed = ui.render_board(gs);
        ui.poll_input();
        if !consumed {
            if ui.just_pressed_down() {
                sel = if sel + 1 == acts.len() { 0 } else { sel + 1 };
            } else if ui.just_pressed_up() {
                sel = if sel == 0 { acts.len() - 1 } else { sel - 1 };
            } else if ui.just_pressed_l() {
                show_detail(ui, &acts[sel].description);
            } else if ui.just_pressed_r() {
                let card = acts[sel]
                    .parameters
                    .as_ref()
                    .and_then(|p| p.card_no.as_ref())
                    .and_then(|n| gs.card_database.get_card_by_no(n));
                if let Some(c) = card {
                    show_card_stats(ui, c);
                }
            } else if ui.just_pressed_a() {
                let _ = game_setup::execute_action(gs, &acts[sel]);
                return true;
            }
        }
        ui.wait_vblank();
    }
}

/// Handle a pending player choice (SelectCard, SelectTarget, etc).
/// Returns true if the choice was handled.
pub fn handle_choice(ui: &mut dyn PlatformUi, gs: &mut GameState) -> bool {
    use crate::ability::types::Choice;
    use crate::ability::util::zone_cards;
    let choice = match gs.get_pending_choice() {
        Some(c) => c.clone(),
        None => return true,
    };

    match choice {
        Choice::SelectAutoAbility {
            options,
            description,
            ..
        } => {
            let items: Vec<String> = options
                .iter()
                .map(|o| format!("{}: {}", o.card_name, o.ability_text))
                .collect();
            if items.is_empty() {
                TurnEngine::resume_with_choice(gs, Some(0), None).ok();
                return true;
            }
            let sel = menu_select(ui, &items, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectCard {
            zone,
            count,
            allow_skip,
            target_player_id,
            description,
            filtered_indices,
            ..
        } => {
            let player = target_player_id
                .as_ref()
                .and_then(|pid| {
                    if pid == &gs.player1.id {
                        Some(&gs.player1)
                    } else if pid == &gs.player2.id {
                        Some(&gs.player2)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| gs.active_player());
            let card_ids = zone_cards(player, &zone);
            let items: Vec<String> = match filtered_indices {
                Some(ref indices) => indices
                    .iter()
                    .map(|&i| {
                        if i < card_ids.len() {
                            gs.card_database
                                .get_card(card_ids[i])
                                .map(|c| format!("{} {}", c.card_no, c.name))
                                .unwrap_or_else(|| format!("#{}", card_ids[i]))
                        } else {
                            format!("#{}", i)
                        }
                    })
                    .collect(),
                None => card_ids
                    .iter()
                    .map(|cid| {
                        gs.card_database
                            .get_card(*cid)
                            .map(|c| format!("{} {}", c.card_no, c.name))
                            .unwrap_or_else(|| format!("#{}", cid))
                    })
                    .collect(),
            };

            if count <= 1 {
                let sel = menu_select(ui, &items, &description, allow_skip);
                match sel {
                    None => {
                        TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                    }
                    Some(idx) => {
                        let actual_idx = filtered_indices.as_ref().map(|fi| fi[idx]).unwrap_or(idx);
                        TurnEngine::resume_with_choice(gs, None, Some(vec![actual_idx])).ok();
                    }
                }
            } else {
                let mut selected: Vec<usize> = Vec::new();
                while selected.len() < count.min(items.len()) {
                    let display_items: Vec<String> = items
                        .iter()
                        .enumerate()
                        .map(|(i, name)| {
                            if selected.contains(&i) {
                                format!("[X] {}", name)
                            } else {
                                format!("[ ] {}", name)
                            }
                        })
                        .collect();
                    let sel = menu_select(ui, &display_items, &description, allow_skip);
                    match sel {
                        None => break,
                        Some(idx) => {
                            if !selected.contains(&idx) {
                                selected.push(idx);
                            }
                        }
                    }
                }
                let actual_indices: Vec<usize> = match filtered_indices {
                    Some(ref fi) => selected.iter().map(|&i| fi[i]).collect(),
                    None => selected,
                };
                TurnEngine::resume_with_choice(gs, None, Some(actual_indices)).ok();
            }
            true
        }
        Choice::SelectTarget {
            target,
            options,
            description,
            allow_skip,
            ..
        } => {
            let items: Vec<String> = match options {
                Some(ref opts) if !opts.is_empty() => opts.clone(),
                _ => (0..2).map(|i| format!("Option {}", i + 1)).collect(),
            };
            let sel = menu_select(ui, &items, &description, allow_skip);
            match sel {
                None => match target.as_str() {
                    "choice" | "choice_string" | "conditional_optional" => {
                        TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                    }
                    _ => {
                        TurnEngine::resume_with_choice(gs, Some(-1), None).ok();
                    }
                },
                Some(idx) => {
                    TurnEngine::resume_with_choice(gs, Some(idx as i16), None).ok();
                }
            };
            true
        }
        Choice::SelectPosition {
            description,
            allow_skip,
            ..
        } => {
            let items: Vec<String> = ["Left", "Center", "Right"]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let sel = menu_select(ui, &items, &description, allow_skip);
            match sel {
                None => TurnEngine::resume_with_choice(gs, Some(-1), None).ok(),
                Some(idx) => TurnEngine::resume_with_choice(gs, Some(idx as i16), None).ok(),
            };
            true
        }
        Choice::SelectHeartColor {
            options,
            description,
            ..
        } => {
            let sel = menu_select(ui, &options, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectHeartType {
            options,
            description,
            ..
        } => {
            let sel = menu_select(ui, &options, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectLiveSuccess {
            options,
            description,
            ..
        } => {
            let items: Vec<String> = options.iter().map(|o| o.card_name.clone()).collect();
            if items.is_empty() {
                TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                return true;
            }
            let sel = menu_select(ui, &items, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, None, Some(vec![sel])).ok();
            true
        }
    }
}

// ====================================================================
// Shared embedded match runner.
//
// This is the single implementation of the match front-end that the
// per-console targets used to copy ~150 lines of: mode/table select, deck
// build, player setup, and the "keep-the-match-honest" loop
// (check_victory -> settle_auto -> loop-detect -> act). Every platform's
// `main()` should call `run_embedded_game` and supply only its backend Ui and
// a way to load the card set + its encoded decks. This mirrors the sequence
// the web server uses (`game_setup::generate_possible_actions`,
// `game_setup::settle_auto`, `TurnEngine::check_victory_condition`, ...) so
// host and consoles converge on one loop contract.
// ====================================================================

/// How a match is driven.
#[derive(Clone, Copy, PartialEq)]
pub enum MatchMode {
    VsAi,
    TwoPlayer,
    AiVsAi,
}

/// Run a full embedded match, converging on the web server's loop sequence.
/// `deck_names` are shown in the selection menus; `cards_of(i)` returns the
/// encoded card numbers of pick `i`; `load_all(i, j)` returns every `Card`
/// needed to build the database for the two selected decks (the union of the
/// two decks' cards — RAM-memory platforms bake this per-deck).
///
/// Returns once a terminal `GameResult` is reached.
pub fn run_embedded_game<U, C, A>(
    ui: U,
    deck_names: &[&str],
    cards_of: C,
    load_all: A,
) -> GameResult
where
    U: PlatformUi,
    C: Fn(usize) -> &'static [&'static str],
    A: FnOnce(usize, usize) -> Vec<Card>,
{
    let mut ui = ui;

    let modes = ["VS AI", "2 Player", "AI vs AI"];
    let mode_idx = select(&mut ui, &modes, "Mode");
    let mode = match mode_idx {
        1 => MatchMode::TwoPlayer,
        2 => MatchMode::AiVsAi,
        _ => MatchMode::VsAi,
    };

    let d1 = select(&mut ui, deck_names, "Your Deck");
    let d2 = if matches!(mode, MatchMode::TwoPlayer) {
        select(&mut ui, deck_names, "P2 Deck")
    } else {
        rng::rand_range(deck_names.len())
    };

    let p1_cards = cards_of(d1);
    let p2_cards = cards_of(d2);
    let all_cards = load_all(d1, d2);
    run_match(&mut ui, p1_cards, p2_cards, all_cards, mode)
}

/// Assemble a match from two deck card-number lists plus the complete `Card`
/// union, then run the shared game loop to a terminal `GameResult`. Exposed
/// separately so host tests can drive a full match without console menus.
pub fn run_match<U: PlatformUi>(
    ui: &mut U,
    p1_cards: &[&str],
    p2_cards: &[&str],
    all_cards: Vec<Card>,
    mode: MatchMode,
) -> GameResult {
    let mut db = Arc::new(CardDatabase::load_or_create(all_cards));

    let nums1: Vec<String> = p1_cards.iter().map(|c| c.to_string()).collect();
    let nums2: Vec<String> = p2_cards.iter().map(|c| c.to_string()).collect();

    let mut pd1 = DeckBuilder::build_deck_from_database(&mut db, nums1).expect("build P1 deck");
    let mut pd2 = DeckBuilder::build_deck_from_database(&mut db, nums2).expect("build P2 deck");
    DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
    DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
    pd1.shuffle_main_deck();
    pd1.shuffle_energy_deck();
    pd2.shuffle_main_deck();
    pd2.shuffle_energy_deck();

    let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    let mut p2 = Player::new("p2".into(), "Player 2".into(), false);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);

    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);

    // ---- the shared, honest loop (mirrors the web server's per-request walk) ----
    loop {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(ui, &gs);
            break;
        }
        game_setup::settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(ui, &gs);
            break;
        }
        if gs.is_loop_detected() {
            show_result(ui, &gs);
            break;
        }

        let acts = game_setup::generate_possible_actions(&gs);
        if acts.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            gs.reset_loop_detection();
            continue;
        }

        if gs.has_pending_choice() {
            if !handle_choice(ui, &mut gs) {
                break;
            }
            gs.reset_loop_detection();
            continue;
        }

        // In VsAi the human only plays P1. During RPS, `active_player()` stays the
        // first attacker (P1) for the whole hand, so the AI never gets a turn and
        // the match soft-locks waiting for a P2 RPS choice. The engine routes RPS
        // picks positionally (1st act -> P1, 2nd act -> P2), so once P1 has chosen,
        // the next RPS pick is P2's and must be taken by the AI.
        let rps_ai_turn = matches!(mode, MatchMode::VsAi)
            && gs.current_phase == crate::game_state::Phase::RockPaperScissors
            && gs.player1_rps_choice.is_some();
        let is_ai = matches!(mode, MatchMode::AiVsAi)
            || rps_ai_turn
            || (matches!(mode, MatchMode::VsAi) && gs.active_player().id != gs.player1.id);
        let ok = if is_ai {
            ai_turn(&mut gs, &acts)
        } else {
            human_turn(ui, &mut gs, &acts)
        };
        if !ok {
            break;
        }
        gs.reset_loop_detection();
        game_setup::settle_auto(&mut gs);
    }

    gs.game_result
}
