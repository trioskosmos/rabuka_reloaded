//! In-game Start-menu overlay for the GBA port.
//!
//! The 3DS keeps Game Log / zone inspection / revealed cards on its second
//! screen (see `platforms/3ds/src/game/overlays.rs` `Overlay::StartMenu`).
//! The GBA has one small screen, so the same features live behind a Start
//! menu with submenus instead: MENU -> Game Log / Cards -> zone lists.
//! Everything renders through the shared text path, so `{{icon}}` tokens in
//! log entries and ability text render as baked texticons.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use rabuka_engine::card::HeartColor;
use rabuka_engine::game::platform_ui::{one_line, wrap_text};
use rabuka_engine::game_state::GameState;

use crate::display::{Display, COLS};
use crate::gba_ui::InputSource;
use crate::input::Button;
use crate::menu::show_card_detail;

/// List rows visible under the title line (screen is 20 text rows).
const VISIBLE: usize = 8;

/// Scrollable `items` picker. A confirms, B cancels (None), Start cancels (None).
fn select<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    items: &[String],
    title: &str,
) -> Option<usize> {
    let mut sel = 0usize;
    let mut scroll = 0usize;
    loop {
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + VISIBLE {
            scroll = sel + 1 - VISIBLE;
        }
        display.clear();
        display.println(title);
        let end = (scroll + VISIBLE).min(items.len());
        for n in scroll..end {
            let prefix = if n == sel { " >" } else { "  " };
            display.println(&one_line(&format!("{prefix} {}", items[n]), COLS as usize));
        }
        if items.len() > end {
            display.println(&format!("  .. {} more", items.len() - end));
        }
        display.swap_buffers();
        input.poll();
        if input.just_pressed(Button::Up) {
            sel = if sel == 0 { items.len() - 1 } else { sel - 1 };
        } else if input.just_pressed(Button::Down) {
            sel = if sel + 1 == items.len() { 0 } else { sel + 1 };
        } else if input.just_pressed(Button::A) {
            return Some(sel);
        } else if input.just_pressed(Button::B) || input.just_pressed(Button::Start) {
            return None;
        }
        display.wait();
    }
}

/// Scrollable card list for one zone; A pops the art+ability detail of the
/// focused card (like the 3DS zone viewer's card screen), B returns.
fn show_card_list<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
    title: &str,
    cards: &[i16],
) {
    let items: Vec<String> = cards
        .iter()
        .map(|&cid| {
            gs.card_database
                .get_card(cid)
                .map(|c| format!("{} {}", c.card_no, c.name))
                .unwrap_or_else(|| format!("#{}", cid))
        })
        .collect();
    if items.is_empty() {
        // Info screen, B/A closes.
        display.clear();
        display.println(title);
        display.println("(empty)");
        display.swap_buffers();
        loop {
            input.poll();
            if input.just_pressed(Button::A) || input.just_pressed(Button::B) {
                return;
            }
            display.wait();
        }
    }
    let mut sel = 0usize;
    let mut scroll = 0usize;
    loop {
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + VISIBLE {
            scroll = sel + 1 - VISIBLE;
        }
        display.clear();
        display.println(title);
        let end = (scroll + VISIBLE).min(items.len());
        for n in scroll..end {
            let prefix = if n == sel { " >" } else { "  " };
            display.println(&one_line(&format!("{prefix} {}", items[n]), COLS as usize));
        }
        if items.len() > end {
            display.println(&format!("  .. {} more", items.len() - end));
        }
        display.swap_buffers();
        input.poll();
        if input.just_pressed(Button::Up) {
            sel = if sel == 0 { items.len() - 1 } else { sel - 1 };
        } else if input.just_pressed(Button::Down) {
            sel = if sel + 1 == items.len() { 0 } else { sel + 1 };
        } else if input.just_pressed(Button::A) {
            let cid = cards[sel];
            if let Some(c) = gs.card_database.get_card(cid) {
                show_card_detail(display, input, gs, c.card_no.to_string());
            }
        } else if input.just_pressed(Button::L) {
            let cid = cards[sel];
            if let Some(c) = gs.card_database.get_card(cid) {
                show_card_detail(display, input, gs, c.card_no.to_string());
            }
        } else if input.just_pressed(Button::B) {
            return;
        }
        display.wait();
    }
}

/// Game Log viewer: `gs.rule_log` newest-last, Up/Down scroll (mirrors the
/// 3DS GameLog overlay's offset-from-end window).
fn show_game_log<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
) {
    // Wrap each entry to screen width so the window counts real rows.
    let mut lines: Vec<String> = Vec::new();
    for entry in &gs.rule_log {
        lines.extend(wrap_text(entry, COLS as usize));
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    let mut off = lines.len().saturating_sub(VISIBLE); // start at the newest
    loop {
        display.clear();
        display.println("GAME LOG  A/B close");
        let end = (off + VISIBLE).min(lines.len());
        for l in off..end {
            display.println(&lines[l]);
        }
        display.swap_buffers();
        input.poll();
        if input.just_pressed(Button::Up) {
            off = off.saturating_sub(1);
        } else if input.just_pressed(Button::Down) && off + VISIBLE < lines.len() {
            off += 1;
        } else if input.just_pressed(Button::A) || input.just_pressed(Button::B) {
            return;
        }
        display.wait();
    }
}

/// The Cards submenu: one entry per zone, active player first. Mirrors the
/// 3DS zone-viewer entry points (hand/deck/waitroom/success/revealed).
fn run_cards_menu<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
) {
    loop {
        let me = gs.active_player();
        let you = if me.id == gs.player1.id {
            &gs.player2
        } else {
            &gs.player1
        };
        use rabuka_engine::core::constants::EMPTY_SLOT;
        let stage_cards = |p: &rabuka_engine::player::Player| {
            p.stage.stage.iter().filter(|&&c| c != EMPTY_SLOT).copied().collect::<Vec<i16>>()
        };
        let zones: Vec<(String, Vec<i16>)> = alloc::vec![
            (
                format!("Hand ({})", me.hand.cards.len()),
                me.hand.cards.to_vec()
            ),
            (
                format!("Main Deck ({})", me.main_deck.cards.len()),
                me.main_deck.cards.to_vec()
            ),
            (
                format!("Energy ({})", me.energy_zone.cards.len()),
                me.energy_zone.cards.to_vec()
            ),
            (
                format!("Energy Deck ({})", me.energy_deck.cards.len()),
                me.energy_deck.cards.to_vec()
            ),
            (
                format!("Waitroom ({})", me.waitroom.cards.len()),
                me.waitroom.cards.to_vec()
            ),
            (
                format!("Success Live ({})", me.success_live_card_zone.cards.len()),
                me.success_live_card_zone.cards.to_vec()
            ),
            (
                format!("Exclusion ({})", me.exclusion_zone.cards.len()),
                me.exclusion_zone.cards.to_vec()
            ),
            (
                format!("Stage ({})", stage_cards(me).len()),
                stage_cards(me)
            ),
            (
                format!("Opp Stage ({})", stage_cards(you).len()),
                stage_cards(you)
            ),
            (
                format!("Live ({})", me.live_card_zone.cards.len()),
                me.live_card_zone.cards.to_vec()
            ),
            (
                format!("Opp Live ({})", you.live_card_zone.cards.len()),
                you.live_card_zone.cards.to_vec()
            ),
            (
                format!("Opp Waitroom ({})", you.waitroom.cards.len()),
                you.waitroom.cards.to_vec()
            ),
            (
                format!("Opp Success ({})", you.success_live_card_zone.cards.len()),
                you.success_live_card_zone.cards.to_vec()
            ),
            (
                format!(
                    "Revealed ({})",
                    gs.revealed_cards.len() + gs.revealed_cost_cards.len()
                ),
                {
                    let mut v: Vec<i16> = gs.revealed_cards.to_vec();
                    v.extend_from_slice(&gs.revealed_cost_cards);
                    v
                }
            ),
        ];
        let items: Vec<String> = zones.iter().map(|(label, _)| label.clone()).collect();
        match select(display, input, &items, "CARDS  B back") {
            Some(i) => {
                let (label, cards) = &zones[i];
                show_card_list(display, input, gs, label, cards);
            }
            None => return,
        }
    }
}

fn hearts_icon_for(player: &rabuka_engine::player::Player, gs: &GameState) -> String {
    use rabuka_engine::card::HeartColor;
    let heart_idx = |c: &HeartColor| match c {
        HeartColor::BAll | HeartColor::Draw | HeartColor::Score => None,
        _ => Some(c.index()),
    };
    let mut counts = [0u32; 8];
    for &cid in &player.stage.stage {
        if cid == rabuka_engine::core::constants::EMPTY_SLOT { continue; }
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
}

fn blade_total_for(player: &rabuka_engine::player::Player, gs: &GameState) -> i32 {
    let mut total: i32 = 0;
    for &cid in &player.stage.stage {
        if cid == rabuka_engine::core::constants::EMPTY_SLOT { continue; }
        if let Some(card) = gs.card_database.get_card(cid) {
            let is_wait = gs.mods.orientation_modifiers.get(&cid).map(|o| o.as_str()=="wait").unwrap_or(false);
            if is_wait { continue; }
            let bm = gs.mods.blade_modifiers.get(&cid).map(|m| m.total()).unwrap_or(0);
            total += (card.blade as i32 + bm).max(0);
        }
    }
    total
}

fn build_stats_lines(gs: &GameState) -> Vec<String> {
    let me = gs.active_player();
    let _you = if me.id == gs.player1.id { &gs.player2 } else { &gs.player1 };
    let mut out: Vec<String> = Vec::new();
    out.push(format!("T{} {:?} {}>", gs.turn_number, gs.current_phase, if me.id==gs.player1.id {"P1"} else {"P2"}));
    // P1 stats
    let p1 = &gs.player1; let p2 = &gs.player2;
    for (label, p) in [("P1", p1), ("P2", p2)] {
        let hearts = hearts_icon_for(p, gs);
        let blade = blade_total_for(p, gs);
        let hb = if hearts.is_empty() && blade==0 { String::new() } else if hearts.is_empty() { format!("{{{{icon_blade.png|BLADE}}}}{}", blade) } else if blade==0 { hearts.clone() } else { format!("{} {{{{icon_blade.png|BLADE}}}}{}", hearts, blade) };
        out.push(format!("{} H{} {{{{icon_energy.png|E}}}}{} D{} W{} S{}", label, p.hand.cards.len(), p.energy_zone.active_count(), p.main_deck.cards.len(), p.waitroom.cards.len(), p.success_live_card_zone.cards.len()));
        if !hb.is_empty() {
            // wrap hearts/blade to fit 30 cols
            for l in wrap_text(&hb, COLS as usize) { out.push(l); }
        } else {
            out.push(String::from("  Hearts/Blade --"));
        }
        // live need if in live phase
        if matches!(gs.current_phase, rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker | rabuka_engine::game_state::Phase::LiveCardSetSecondAttacker | rabuka_engine::game_state::Phase::FirstAttackerPerformance | rabuka_engine::game_state::Phase::SecondAttackerPerformance) {
            let mut need_counts = [0u32; 8];
            for &cid in &p.live_card_zone.cards {
                if cid == rabuka_engine::core::constants::EMPTY_SLOT { continue; }
                if let Some(card) = gs.card_database.get_card(cid) {
                    if let Some(ref need) = card.need_heart {
                        for (col, cnt) in &need.hearts {
                            let idx = match *col { HeartColor::Heart00=>0, HeartColor::Heart01=>1, HeartColor::Heart02=>2, HeartColor::Heart03=>3, HeartColor::Heart04=>4, HeartColor::Heart05=>5, HeartColor::Heart06=>6, HeartColor::All=>7, _=> 99 };
                            if idx < 8 { need_counts[idx] += *cnt as u32; }
                        }
                    }
                }
            }
            let mut need_parts: Vec<String> = Vec::new();
            for (i, &c) in need_counts.iter().enumerate() { if c>0 { let n = match i {0=>"heart_00",1=>"heart_01",2=>"heart_02",3=>"heart_03",4=>"heart_04",5=>"heart_05",6=>"heart_06",_=>"icon_all"}; need_parts.push(format!("{{{{{}.png|{}}}}}{}", n,n,c)); } }
            if !need_parts.is_empty() {
                out.push(format!("Need {}", need_parts.join(" ")));
            }
        }
    }
    out.push(String::from("-- A:Menu B:Close --"));
    out
}

fn show_stats<I: InputSource>(display: &mut Display, input: &mut I, gs: &GameState) -> Option<bool> {
    let lines = build_stats_lines(gs);
    let mut off = 0usize;
    loop {
        display.clear();
        display.println("STATS  A:Menu B:Close  Up/Down:Scroll");
        let end = (off + VISIBLE).min(lines.len());
        for l in off..end { display.println(&lines[l]); }
        if lines.len() > end { display.println(&format!("  .. {} more", lines.len()-end)); }
        display.swap_buffers();
        input.poll();
        if input.just_pressed(Button::Up) { off = off.saturating_sub(1); }
        else if input.just_pressed(Button::Down) && off + VISIBLE < lines.len() { off += 1; }
        else if input.just_pressed(Button::A) { return Some(true); }
        else if input.just_pressed(Button::B) { return Some(false); }
        display.wait();
    }
}

/// Blocking Start-menu overlay. 3DS shows stats on the top screen; GBA has
/// one screen so Start first shows the stats (hearts/blade) then offers the
/// menu. A/B on stats goes to menu/close.
pub fn run_start_menu<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
) {
    loop {
        match show_stats(display, input, gs) {
            Some(true) => {}, // continue to menu below
            _ => return,
        }
        let items = alloc::vec![
            String::from("Game Log"),
            String::from("Cards"),
            String::from("Back to Stats"),
            String::from("Close"),
        ];
        match select(display, input, &items, "MENU  Start:Back") {
            Some(0) => show_game_log(display, input, gs),
            Some(1) => run_cards_menu(display, input, gs),
            Some(2) => continue, // back to stats
            _ => return,
        }
        // Check Start to exit menu entirely
        input.poll();
        if input.just_pressed(Button::Start) {
            return;
        }
        display.wait();
    }
}
