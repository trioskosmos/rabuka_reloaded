//! In-game Start-menu overlay for the GBA port.
//!
//! The 3DS keeps Game Log / zone inspection / revealed cards on its second
//! screen (see `platforms/3ds/src/game/overlays.rs` `Overlay::StartMenu`).
//! The GBA has one small screen, so the same features live behind a Start
//! menu: ONE screen with pinned stats on top and a scrolling list below
//! (Game Log, card zones, Close) — stats and zones visible at the same
//! time. Zone contents render as a 3DS-style card grid (not a text list)
//! with choice-menu buttons.
//!
//! Zone order is fixed: own waitroom, opponent waitroom, the two success
//! zones, then everything else. Opp hands/decks stay hidden (like the web
//! hidden-card filter). Everything renders through the shared text path,
//! so `{{icon}}` tokens in log entries and ability text render as baked
//! texticons.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use rabuka_engine::card::HeartColor;
use rabuka_engine::game::platform_ui::{card_ability_text, one_line, wrap_text};
use rabuka_engine::game_state::GameState;

use crate::board::live_set_hidden;
use crate::display::{Display, COLS};
use crate::gba_ui::InputSource;
use crate::input::Button;
use crate::menu::show_card_detail;

/// Grid pages show 5 stage-size cards (mirrors the engine choice grid).
const GRID_COLS: usize = 5;
const GRID_PP: usize = 5;
const GRID_X0: i32 = 2;
const GRID_Y: i32 = 13;

/// Scrollable game-log viewer: `gs.rule_log` newest-last, Up/Down scroll
/// (mirrors the 3DS GameLog overlay's offset-from-end window). A/B/Start
/// closes. Single hint line in the title.
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
    const VISIBLE: usize = 8;
    let mut off = lines.len().saturating_sub(VISIBLE); // start at the newest
    loop {
        display.clear();
        display.println("GAME LOG  A/B/Sta close");
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
        } else if input.just_pressed(Button::A)
            || input.just_pressed(Button::B)
            || input.just_pressed(Button::Start)
        {
            return;
        }
        display.wait();
    }
}

/// Graphical zone viewer: a 3DS-style card grid (stage-size art, cursor
/// badge, selected name + ability preview) with choice-menu buttons —
/// D-pad wraps around pages, A pops the art+ability detail of the cursor
/// card, B/Start goes back. `hidden` renders faceless "[Hidden]" rows
/// (unrevealed live-set cards) instead of leaking identities.
fn show_zone_grid<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
    title: &str,
    cards: &[i16],
    hidden: bool,
) {
    if cards.is_empty() {
        display.clear();
        display.println(title);
        display.println("(empty)");
        display.swap_buffers();
        loop {
            input.poll();
            if input.just_pressed(Button::A)
                || input.just_pressed(Button::B)
                || input.just_pressed(Button::Start)
            {
                return;
            }
            display.wait();
        }
    }
    // Resolve display rows: (db id, card_no for art, name).
    let rows: Vec<(i16, String, String)> = cards
        .iter()
        .map(|&cid| {
            if hidden {
                (cid, String::new(), String::from("[Hidden]"))
            } else {
                match gs.card_database.get_card(cid) {
                    Some(c) => (cid, c.card_no.to_string(), format!("{} {}", c.card_no, c.name)),
                    None => (cid, String::new(), format!("#{}", cid)),
                }
            }
        })
        .collect();
    let n = rows.len();
    let total_pages = (n + GRID_PP - 1) / GRID_PP;
    let mut sel = 0usize;
    display.reset_vram();
    loop {
        let page = sel / GRID_PP;
        let page_start = page * GRID_PP;
        let page_end = (page_start + GRID_PP).min(n);

        display.clear();
        if total_pages > 1 {
            display.println(&one_line(
                &format!("{} [{}/{}]", title, page + 1, total_pages),
                COLS as usize,
            ));
        } else {
            display.println(&one_line(title, COLS as usize));
        }
        // Queue art below the text block (tile rows 13+, like the choice grid).
        for i in 0..(page_end - page_start) {
            let idx = page_start + i;
            let col = (i % GRID_COLS) as i32;
            let x = GRID_X0 + col * 5;
            display.queue_card_image(&rows[idx].1, x, GRID_Y, 5, 6, idx == sel, false);
        }
        display.println(&one_line(&format!("> {}", rows[sel].2), COLS as usize));
        if !hidden {
            if let Some(c) = gs.card_database.get_card(rows[sel].0) {
                let abil = card_ability_text(c).replace('\n', " ");
                if !abil.trim().is_empty() {
                    if let Some(line) = wrap_text(&abil, COLS as usize).into_iter().next() {
                        display.println(&line);
                    }
                }
            }
        }
        display.println("A:Detail B:Back");
        display.swap_buffers();
        input.poll();
        if input.just_pressed(Button::Left) {
            sel = (sel + n - 1) % n;
        } else if input.just_pressed(Button::Right) {
            sel = (sel + 1) % n;
        } else if input.just_pressed(Button::Up) || input.just_pressed(Button::Down) {
            let up = input.just_pressed(Button::Up);
            let col = sel % GRID_COLS;
            let rows_n = (n + GRID_COLS - 1) / GRID_COLS;
            let row = sel / GRID_COLS;
            let new_row = if up {
                (row + rows_n - 1) % rows_n
            } else {
                (row + 1) % rows_n
            };
            sel = (new_row * GRID_COLS + col).min(n - 1);
        } else if input.just_pressed(Button::A) {
            if hidden {
                display.clear();
                display.println(title);
                display.println("(hidden until performance)");
                display.swap_buffers();
                loop {
                    input.poll();
                    if input.just_pressed(Button::A)
                        || input.just_pressed(Button::B)
                        || input.just_pressed(Button::Start)
                    {
                        break;
                    }
                    display.wait();
                }
            } else if !rows[sel].1.is_empty() {
                let no = rows[sel].1.clone();
                show_card_detail(display, input, gs, no);
            }
        } else if input.just_pressed(Button::B) || input.just_pressed(Button::Start) {
            display.reset_vram();
            return;
        } else {
            display.wait();
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

/// Compact per-player stats lines (no trailing hint — the menu screen owns
/// the single hint line). Live-need hearts follow the 3DS rule: always for
/// the active player, for the other player only once they performed.
fn build_stats_lines(gs: &GameState) -> Vec<String> {
    let me = gs.active_player();
    let active_is_p1 = me.id == gs.player1.id;
    let active_idx = if active_is_p1 { 0 } else { 1 };
    let performed = gs.opponent_has_performed(active_idx);
    let mut out: Vec<String> = Vec::new();
    out.push(format!("T{} {:?} {}>", gs.turn_number, gs.current_phase, if active_is_p1 {"P1"} else {"P2"}));
    let p1 = &gs.player1; let p2 = &gs.player2;
    for (label, p, is_active) in [("P1", p1, active_is_p1), ("P2", p2, !active_is_p1)] {
        let hearts = hearts_icon_for(p, gs);
        let blade = blade_total_for(p, gs);
        let hb = if hearts.is_empty() && blade==0 { String::new() } else if hearts.is_empty() { format!("{{{{icon_blade.png|BLADE}}}}{}", blade) } else if blade==0 { hearts.clone() } else { format!("{} {{{{icon_blade.png|BLADE}}}}{}", hearts, blade) };
        out.push(format!("{} H{} {{{{icon_energy.png|E}}}}{} D{} W{} S{}", label, p.hand.cards.len(), p.energy_zone.active_count(), p.main_deck.cards.len(), p.waitroom.cards.len(), p.success_live_card_zone.cards.len()));
        if !hb.is_empty() {
            for l in wrap_text(&hb, COLS as usize) { out.push(l); }
        } else {
            out.push(String::from("  Hearts/Blade --"));
        }
        // Live need, gated like the 3DS need display (render.rs:316).
        if matches!(gs.current_phase, rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker | rabuka_engine::game_state::Phase::LiveCardSetSecondAttacker | rabuka_engine::game_state::Phase::FirstAttackerPerformance | rabuka_engine::game_state::Phase::SecondAttackerPerformance) {
            if is_active || performed {
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
    }
    out
}

/// Zone entries in fixed menu order: own waitroom, opponent waitroom, the
/// two success zones, then everything else. Each entry is
/// (label, card ids, hidden) — hidden renders faceless rows.
fn menu_zones(gs: &GameState) -> Vec<(String, Vec<i16>, bool)> {
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
    let hide_live = live_set_hidden(&gs.current_phase);
    let mut revealed: Vec<i16> = gs.revealed_cards.to_vec();
    revealed.extend_from_slice(&gs.revealed_cost_cards);
    alloc::vec![
        (format!("Waitroom ({})", me.waitroom.cards.len()), me.waitroom.cards.to_vec(), false),
        (format!("Opp Waitroom ({})", you.waitroom.cards.len()), you.waitroom.cards.to_vec(), false),
        (format!("Success ({})", me.success_live_card_zone.cards.len()), me.success_live_card_zone.cards.to_vec(), false),
        (format!("Opp Success ({})", you.success_live_card_zone.cards.len()), you.success_live_card_zone.cards.to_vec(), false),
        (format!("Hand ({})", me.hand.cards.len()), me.hand.cards.to_vec(), false),
        (format!("Main Deck ({})", me.main_deck.cards.len()), me.main_deck.cards.to_vec(), false),
        (format!("Energy ({})", me.energy_zone.cards.len()), me.energy_zone.cards.to_vec(), false),
        (format!("Energy Deck ({})", me.energy_deck.cards.len()), me.energy_deck.cards.to_vec(), false),
        (format!("Exclusion ({})", me.exclusion_zone.cards.len()), me.exclusion_zone.cards.to_vec(), false),
        (format!("Stage ({})", stage_cards(me).len()), stage_cards(me), false),
        (format!("Opp Stage ({})", stage_cards(you).len()), stage_cards(you), false),
        (format!("Live Set ({})", me.live_card_zone.cards.len()), me.live_card_zone.cards.to_vec(), hide_live),
        (format!("Opp Live Set ({})", you.live_card_zone.cards.len()), you.live_card_zone.cards.to_vec(), hide_live),
        (format!("Revealed ({})", revealed.len()), revealed, false),
    ]
}

/// Blocking Start-menu overlay: ONE screen — pinned stats on top, scrolling
/// list below (Game Log, card zones, Close). A confirms, B/Start closes.
/// Openable from the board, the action list, and (via the `open_start_menu`
/// hook) choice menus.
pub fn run_start_menu<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
) {
    // Pinned header: single hint line + compact stats.
    let mut header: Vec<String> = Vec::new();
    header.push(String::from("MENU  B/Sta:Close"));
    header.extend(build_stats_lines(gs).into_iter().take(5));
    let vis = 10usize.saturating_sub(header.len() + 1).max(2);

    enum Item {
        Log,
        Zone(usize),
        Close,
    }
    let zones = menu_zones(gs);
    let mut items: Vec<(String, Item)> = Vec::new();
    items.push((String::from("Game Log"), Item::Log));
    for (i, (label, _, _)) in zones.iter().enumerate() {
        items.push((label.clone(), Item::Zone(i)));
    }
    items.push((String::from("Close"), Item::Close));

    let mut sel = 0usize;
    let mut scroll = 0usize;
    loop {
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + vis {
            scroll = sel + 1 - vis;
        }
        display.clear();
        for h in &header {
            display.println(h);
        }
        let end = (scroll + vis).min(items.len());
        for n in scroll..end {
            let prefix = if n == sel { " >" } else { "  " };
            display.println(&one_line(&format!("{prefix} {}", items[n].0), COLS as usize));
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
            match items[sel].1 {
                Item::Log => show_game_log(display, input, gs),
                Item::Zone(i) => {
                    let (label, cards, hide) = &zones[i];
                    show_zone_grid(display, input, gs, label, cards, *hide);
                }
                Item::Close => return,
            }
        } else if input.just_pressed(Button::B) || input.just_pressed(Button::Start) {
            return;
        }
        display.wait();
    }
}
