#![cfg(feature = "3ds")]
// Per-SetupPhase handlers extracted from the bin's Step::Setup match arm.
// Inner `unsafe {}` blocks are inherited verbatim from the original arm
// (which carried the bin-level #![allow(unused_unsafe)]).
#![allow(unused_unsafe)]

// Setup state machine: one handler function per SetupPhase.
// Each handler returns the next Step. Bodies were moved verbatim from the
// Step::Setup match arm in the bin (see extract_setup.py).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::deck_builder::DeckBuilder;
use rabuka_engine::deck_parser::{DeckEntry, DeckList, DeckParser};
use rabuka_engine::game_setup;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;

use crate::dprintln;
use crate::ffi::*;
use crate::game::PlayState;
use crate::i18n::Lang;
use crate::lang::{current_lang, set_lang, tl, tl_fmt};
use crate::steps::{Overlay, SetupPhase, Step};
use crate::uds;
use crate::ui::card_atlas::CardAtlas;
use crate::ui::colors::*;
use crate::ui::grid::{card_grid_input, render_card_detail, render_card_grid, GridAction};
use crate::ui::hint::render_hint_bar;
use crate::util::{base64_decode, looks_like_b64, ticks_to_ms};

/// On-device test suite — runs QA checks in limited 3DS memory.
/// Accessed via "Run Tests" menu. Each test returns a result line.
fn run_on_device_tests(cards: Arc<Vec<Card>>, decks: Vec<DeckList>) -> Vec<String> {
    let mut r: Vec<String> = Vec::new();
    let t0 = unsafe { _3ds_system_tick() };
    r.push(format!("CARDS: {}", cards.len()));
    let mut cards_vec = (*cards).clone();
    CardLoader::attach_abilities(&mut cards_vec);
    let wa = cards_vec.iter().filter(|c| !c.abilities.is_empty()).count();
    r.push(if wa > 0 {
        format!("ABILITIES: {} (OK)", wa)
    } else {
        "ABILITIES: NONE (FAIL!)".into()
    });
    r.push(format!("DECKS: {}", decks.len()));
    if let Some(c) = cards.first() {
        let nl = c.name.len();
        r.push(format!("CARD[0]: {} ({}ch) OK", &c.name[..nl.min(20)], nl));
    } else {
        r.push("CARD[0]: NONE (FAIL!)".into());
    }
    let he = cards.iter().any(|c| {
        let cn: &str = &c.card_no;
        cn.contains("LL-E-005")
    });
    r.push(if he {
        "ENERGY: found (OK)".into()
    } else {
        "ENERGY: missing (FAIL!)".into()
    });
    if decks.len() >= 2 {
        match rabuka_engine::game_setup::test_ai_vs_ai(&cards_vec, &decks[0], &decks[1], 5) {
            Ok(n) => r.push(format!("AI PLAY: {} actions (OK)", n)),
            Err(e) => r.push(format!("AI PLAY: FAIL {}", e)),
        }
    } else {
        r.push("AI PLAY: skip (need 2 decks)".into());
    }
    let ms = ticks_to_ms(unsafe { _3ds_system_tick() } - t0);
    r.push(format!("TIME: {}ms", ms));
    r.push("=== DONE ===".into());
    r
}

fn pick_mode(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    n: usize,
    was_dirty: bool,
    cur: usize,
) -> Step {
    unsafe {
        let cur = cur.min(3);
        if was_dirty {
            if _3ds_is_cli_mode() {
                _3ds_clear_top();
                _3ds_text_add_top(format!("{}\n\0", tl("SELECT MODE")).as_ptr());
                for (i, m) in ["VS AI", "Sandbox (2 players)", "QR Scan", "Local MP"]
                    .iter()
                    .enumerate()
                {
                    let arrow = if i == cur { ">" } else { " " };
                    _3ds_text_add_top(format!("{} [{}] {}\n\0", arrow, i, tl(m)).as_ptr());
                }
                _3ds_text_add_top(
                    format!(
                        "{}\n\0",
                        match current_lang() {
                            Lang::English => "English / 英語",
                            Lang::Japanese => "日本語 / English",
                        }
                    )
                    .as_ptr(),
                );
                let tip = tl("L=help R=lang/言語 A=confirm B=back");
                _3ds_text_add_top(format!("\n{}\0", tip).as_ptr());
            } else {
                _3ds_top_clear();
                _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                _3ds_top_queue_text(
                    100.0,
                    8.0,
                    COL_GOLD,
                    0.65f32,
                    format!("{}\0", tl("SELECT MODE")).as_ptr(),
                );
                for (i, m) in ["VS AI", "Sandbox", "QR Scan", "Local MP"]
                    .iter()
                    .enumerate()
                {
                    let y = 40.0 + i as f32 * 38.0;
                    let bg = if i == cur { COL_SEL } else { COL_DIM };
                    unsafe {
                        _3ds_top_queue_rect(40.0, y, 320.0, 36.0, bg);
                    }
                    if i == cur {
                        unsafe {
                            _3ds_top_queue_rect(40.0, y, 320.0, 36.0, COL_HIGHLIGHT);
                        }
                    }
                    let color = if i == cur { COL_GOLD } else { COL_LIGHT };
                    let label = tl(m);
                    unsafe {
                        _3ds_top_queue_text(
                            50.0,
                            y + 6.0,
                            color,
                            0.65f32,
                            format!("{}\0", label).as_ptr(),
                        );
                    }
                }
                render_hint_bar(&tl("L=help  R=lang/言語  A=confirm  B=back"));
            }
        }
        if keys & 0x00000200 != 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::ControlGuide(0),
                true,
            )
        } else if keys & 0x00000040 != 0 && cur > 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickMode(cur - 1),
                true,
            )
        } else if keys & 0x00000080 != 0 && cur + 1 < 4 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickMode(cur + 1),
                true,
            )
        } else if keys & 0x00000100 != 0 {
            set_lang(current_lang().toggle());
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickMode(cur),
                true,
            )
        } else if keys & 0x00000002 != 0 {
            Step::Done(Ok(()))
        } else if keys & 0x00000008 != 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickMode(cur),
                false,
            )
        } else if keys & 0x00000001 != 0 {
            if cur == 2 {
                // "QR Scan"
                Step::Setup(cards.clone(), decks.clone(), SetupPhase::QrScan(0), true)
            } else if cur == 3 {
                // "Local Multiplayer" — pick deck then connect
                Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    SetupPhase::MultiplayerDeck(0),
                    true,
                )
            } else if n == 0 {
                Step::Done(Err("No decks!".into()))
            } else {
                Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    SetupPhase::PickDeck(0, cur == 0, false),
                    true,
                )
            }
        } else {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickMode(cur),
                false,
            )
        }
    }
}

fn pick_deck(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    n: usize,
    was_dirty: bool,
    cur: usize,
    vs_ai: bool,
    is_multiplayer: bool,
) -> Step {
    {
        if was_dirty {
            let label = if !vs_ai {
                tl("P1 DECK")
            } else {
                tl("YOUR DECK")
            };
            if unsafe { _3ds_is_cli_mode() } {
                unsafe {
                    _3ds_clear_top();
                }
                unsafe {
                    _3ds_text_add_top(format!("{}\n\0", label).as_ptr());
                }
                for i in
                    cur.saturating_sub(6).min(n.saturating_sub(12))..(0usize.min(n) + 12).min(n)
                {
                    let arrow = if i == cur { ">" } else { " " };
                    unsafe {
                        _3ds_text_add_top(format!("{} {}\n\0", arrow, decks[i].name).as_ptr());
                    }
                }
                unsafe {
                    _3ds_text_add_top("\nA=select B=back\0".as_ptr());
                }
            } else {
                unsafe {
                    _3ds_top_clear();
                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                    _3ds_top_queue_text(
                        80.0,
                        8.0,
                        COL_GOLD,
                        0.65f32,
                        format!("SELECT {}\0", label).as_ptr(),
                    );
                }
                // Show 6 decks max: 240px screen - 30px title - 20px help = 190px
                // 190 / 6 = ~32px per row at 0.70 scale (~21px glyph)
                let start = cur.saturating_sub(3).min(n.saturating_sub(6));
                let end = (start + 6).min(n);
                for i in start..end {
                    let y = 30.0 + (i - start) as f32 * 32.0;
                    let bg = if i == cur { COL_SEL } else { COL_DIM };
                    unsafe {
                        _3ds_top_queue_rect(20.0, y, 360.0, 30.0, bg);
                    }
                    if i == cur {
                        unsafe {
                            _3ds_top_queue_rect(20.0, y, 360.0, 30.0, COL_HIGHLIGHT);
                        }
                    }
                    let color = if i == cur { COL_GOLD } else { COL_LIGHT };
                    unsafe {
                        _3ds_top_queue_text(
                            24.0,
                            y + 3.0,
                            color,
                            0.65f32,
                            format!("{}\0", decks[i].name).as_ptr(),
                        );
                    }
                }
                render_hint_bar(&tl("UP/DOWN=select  A=confirm  X=preview  B=back"));
            }
        }
        // X = preview deck contents
        if keys & 0x00000400 != 0 && cur < n {
            let card_db = std::sync::Arc::new(CardDatabase::load_or_create(cards.as_ref().clone()));
            let card_ids: Vec<i16> = DeckParser::deck_list_to_card_numbers(&decks[cur])
                .iter()
                .filter_map(|cn| card_db.get_card_id(cn))
                .collect();
            let deck_atlas = CardAtlas::load();
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::DeckViewer(
                    card_ids,
                    0,
                    0,
                    vs_ai,
                    is_multiplayer,
                    None,
                    card_db,
                    deck_atlas,
                ),
                true,
            )
        } else if keys & 0x00000040 != 0 && cur > 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickDeck(cur - 1, vs_ai, is_multiplayer),
                true,
            )
        } else if keys & 0x00000080 != 0 && cur + 1 < n {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickDeck(cur + 1, vs_ai, is_multiplayer),
                true,
            )
        } else if keys & 0x00000002 != 0 {
            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(4), true)
        } else if keys & 0x00000001 != 0 {
            if is_multiplayer {
                // Local Multiplayer: go to role selection
                Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    SetupPhase::MultiplayerPickRole(cur, 0),
                    true,
                )
            } else if vs_ai {
                Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    SetupPhase::Loading(cur, cur, true),
                    true,
                )
            } else {
                Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    SetupPhase::PickDeck2(0, cur, false),
                    true,
                )
            }
        } else {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickDeck(cur, vs_ai, is_multiplayer),
                false,
            )
        }
    }
}

fn multiplayer_deck(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    n: usize,
    was_dirty: bool,
    cur: usize,
) -> Step {
    {
        if was_dirty {
            if unsafe { _3ds_is_cli_mode() } {
                unsafe {
                    _3ds_clear_top();
                }
                let deck_hdr = tl("SELECT YOUR DECK");
                unsafe {
                    _3ds_text_add_top(format!("{}\n\0", deck_hdr).as_ptr());
                }
                for i in
                    cur.saturating_sub(6).min(n.saturating_sub(12))..(0usize.min(n) + 12).min(n)
                {
                    let arrow = if i == cur { ">" } else { " " };
                    unsafe {
                        _3ds_text_add_top(format!("{} {}\n\0", arrow, decks[i].name).as_ptr());
                    }
                }
                unsafe {
                    _3ds_text_add_top("\nA=select B=back\0".as_ptr());
                }
            } else {
                unsafe {
                    _3ds_top_clear();
                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                    let deck_hdr = tl("SELECT YOUR DECK");
                    _3ds_top_queue_text(
                        80.0,
                        8.0,
                        COL_GOLD,
                        0.65f32,
                        format!("{}\0", deck_hdr).as_ptr(),
                    );
                }
                let start = cur.saturating_sub(3).min(n.saturating_sub(6));
                let end = (start + 6).min(n);
                for i in start..end {
                    let y = 30.0 + (i - start) as f32 * 32.0;
                    let bg = if i == cur { COL_SEL } else { COL_DIM };
                    unsafe {
                        _3ds_top_queue_rect(20.0, y, 360.0, 30.0, bg);
                    }
                    if i == cur {
                        unsafe {
                            _3ds_top_queue_rect(20.0, y, 360.0, 30.0, COL_HIGHLIGHT);
                        }
                    }
                    let color = if i == cur { COL_GOLD } else { COL_LIGHT };
                    unsafe {
                        _3ds_top_queue_text(
                            24.0,
                            y + 3.0,
                            color,
                            0.65f32,
                            format!("{}\0", decks[i].name).as_ptr(),
                        );
                    }
                }
                render_hint_bar(&tl("UP/DOWN=select  A=confirm  B=back"));
            }
        }
        if keys & 0x00000040 != 0 && cur > 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::MultiplayerDeck(cur - 1),
                true,
            )
        } else if keys & 0x00000080 != 0 && cur + 1 < n {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::MultiplayerDeck(cur + 1),
                true,
            )
        } else if keys & 0x00000002 != 0 {
            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(4), true)
        } else if keys & 0x00000001 != 0 {
            // A = select deck, go to role selection with deck index
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::MultiplayerPickRole(cur, 0), // deck_idx=cur, role_cursor=0
                true,
            )
        } else {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::MultiplayerDeck(cur),
                false,
            )
        }
    }
}

fn pick_deck2(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    n: usize,
    was_dirty: bool,
    cur: usize,
    p1_idx: usize,
    vs_ai: bool,
) -> Step {
    {
        if was_dirty {
            if unsafe { _3ds_is_cli_mode() } {
                unsafe {
                    _3ds_clear_top();
                }
                let deck_hdr = tl("SELECT P2 DECK");
                unsafe {
                    _3ds_text_add_top(format!("{}\n\0", deck_hdr).as_ptr());
                }
                for i in
                    cur.saturating_sub(6).min(n.saturating_sub(12))..(0usize.min(n) + 12).min(n)
                {
                    let arrow = if i == cur { ">" } else { " " };
                    unsafe {
                        _3ds_text_add_top(format!("{} {}\n\0", arrow, decks[i].name).as_ptr());
                    }
                }
                unsafe {
                    _3ds_text_add_top("\nA=select B=same\0".as_ptr());
                }
            } else {
                unsafe {
                    _3ds_top_clear();
                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                    let deck_hdr = tl("SELECT P2 DECK");
                    _3ds_top_queue_text(
                        80.0,
                        8.0,
                        COL_GOLD,
                        0.65f32,
                        format!("{}\0", deck_hdr).as_ptr(),
                    );
                }
                let start = cur.saturating_sub(3).min(n.saturating_sub(6));
                let end = (start + 6).min(n);
                for i in start..end {
                    let y = 30.0 + (i - start) as f32 * 32.0;
                    let bg = if i == cur { COL_SEL } else { COL_DIM };
                    unsafe {
                        _3ds_top_queue_rect(20.0, y, 360.0, 30.0, bg);
                    }
                    if i == cur {
                        unsafe {
                            _3ds_top_queue_rect(20.0, y, 360.0, 30.0, COL_HIGHLIGHT);
                        }
                    }
                    let color = if i == cur { COL_GOLD } else { COL_LIGHT };
                    unsafe {
                        _3ds_top_queue_text(
                            24.0,
                            y + 3.0,
                            color,
                            0.65f32,
                            format!("{}\0", decks[i].name).as_ptr(),
                        );
                    }
                }
                render_hint_bar(&tl("X=preview  A=select  B=use same"));
            }
        }
        // X = preview deck contents
        if keys & 0x00000400 != 0 && cur < n {
            let card_db = std::sync::Arc::new(CardDatabase::load_or_create(cards.as_ref().clone()));
            let card_ids: Vec<i16> = DeckParser::deck_list_to_card_numbers(&decks[cur])
                .iter()
                .filter_map(|cn| card_db.get_card_id(cn))
                .collect();
            let deck_atlas = CardAtlas::load();
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::DeckViewer(card_ids, 0, 0, false, false, None, card_db, deck_atlas),
                true,
            )
        } else if keys & 0x00000040 != 0 && cur > 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickDeck2(cur - 1, p1_idx, vs_ai),
                true,
            )
        } else if keys & 0x00000080 != 0 && cur + 1 < n {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickDeck2(cur + 1, p1_idx, vs_ai),
                true,
            )
        } else if keys & 0x00000001 != 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::Loading(p1_idx, cur, false),
                true,
            )
        } else if keys & 0x00000002 != 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::Loading(p1_idx, p1_idx, false),
                true,
            )
        } else {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickDeck2(cur, p1_idx, vs_ai),
                false,
            )
        }
    }
}

fn loading(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    p1_idx: usize,
    p2_idx: usize,
    vs_ai: bool,
) -> Step {
    {
        let r = (|| -> Result<(GameState, CardAtlas), String> {
            let mut cards_vec = (**cards).clone();
            CardLoader::attach_abilities(&mut cards_vec);
            let mut db = Arc::new(CardDatabase::load_or_create(cards_vec));
            let nums1 = DeckParser::deck_list_to_card_numbers(&decks[p1_idx]);
            let nums2 = if p1_idx == p2_idx {
                nums1.clone()
            } else {
                DeckParser::deck_list_to_card_numbers(&decks[p2_idx])
            };
            let mut pd1 = DeckBuilder::build_deck_from_database(&mut db, nums1)
                .map_err(|e| format!("Deck: {}", e))?;
            let mut pd2 = DeckBuilder::build_deck_from_database(&mut db, nums2)
                .map_err(|e| format!("Deck: {}", e))?;
            pd1.shuffle_main_deck();
            pd1.shuffle_energy_deck();
            pd2.shuffle_main_deck();
            pd2.shuffle_energy_deck();
            let mut deck_nos: HashSet<String> = HashSet::new();
            for cid in pd1
                .main_deck
                .iter()
                .chain(pd1.energy_deck.iter())
                .chain(pd2.main_deck.iter())
                .chain(pd2.energy_deck.iter())
            {
                if let Some(card) = db.get_card(*cid) {
                    deck_nos.insert(card.card_no.to_string());
                }
            }
            DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
            DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
            let mut p1 = Player::new("p1".into(), "P1".into(), true);
            p1.set_main_deck(pd1.main_deck);
            p1.set_energy_deck(pd1.energy_deck);
            let mut p2 = Player::new("p2".into(), "P2".into(), false);
            p2.set_main_deck(pd2.main_deck);
            p2.set_energy_deck(pd2.energy_deck);
            let mut gs = GameState::new(p1, p2, db);
            game_setup::setup_game(&mut gs);
            Ok((gs, CardAtlas::load()))
        })();
        match r {
            Ok((gs, atlas)) => {
                unsafe {
                    _3ds_board_enable(true);
                }
                Step::Play(PlayState {
                    gs,
                    cur: 0,
                    acts_cache: Vec::new(),
                    dirty: true,
                    redraw: true,
                    atlas,
                    vs_ai,
                    ai_vs_ai: false,
                    cli_mode: false,
                    detail_mode: false,
                    choice_image_mode: true,
                    choice_subview: false,
                    text_page: 0,
                    choice_grid_offset: 0,
                    list_scroll: 0,
                    detail_scroll_y: 0.0f32,
                    hand_offset: 0,
                    hand_offset_p2: 0,
                    touch_tap_count: 0,
                    viewing_card: None,
                    zone_viewer: None,
                    zone_viewer_offset: 0,
                    was_touching: false,
                    is_multiplayer: false,
                    is_host: false,
                    waiting_for_opponent: false,
                    overlay: Overlay::None,
                    pending_client_action: None,
                    last_client_action_seq: 0,
                    next_action_seq: 1,
                    dbg_tx_bytes: 0,
                    dbg_rx_bytes: 0,
                })
            }
            Err(e) => Step::Done(Err(e)),
        }
    }
}

fn testing(cards: &Arc<Vec<Card>>, decks: &Vec<DeckList>, keys: u32) -> Step {
    {
        let results = run_on_device_tests(cards.clone(), decks.clone());
        unsafe {
            _3ds_clear_both();
            _3ds_text_add_top("=== ON-DEVICE TESTS ===\n\0".as_ptr());
            for line in &results {
                _3ds_text_add_top(format!("{}\n\0", line).as_ptr());
            }
            _3ds_text_add_top("\nSTART=exit\0".as_ptr());
        }
        if keys & 0x00000008 != 0 {
            Step::Done(Ok(()))
        } else {
            Step::Setup(cards.clone(), decks.clone(), SetupPhase::Testing, false)
        }
    }
}

fn qr_scan(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    was_dirty: bool,
    ctx: usize,
) -> Step {
    {
        let mut qr_ctx = ctx;
        let mut qr_start_failed = false;
        if was_dirty && qr_ctx == 0 {
            let ptr = unsafe { _3ds_qr_start() };
            if ptr.is_null() {
                unsafe {
                    _3ds_clear_both();
                    _3ds_text_add_top(format!("{}\n\0", tl("Camera init failed")).as_ptr());
                    _3ds_text_add_top(format!("{}\0", tl("B=back")).as_ptr());
                }
                qr_start_failed = true;
            } else {
                qr_ctx = ptr as usize;
                if unsafe { _3ds_is_cli_mode() } {
                    unsafe {
                        _3ds_clear_top();
                        _3ds_text_add_top(format!("{}\n\0", tl("QR SCAN")).as_ptr());
                        _3ds_text_add_top(
                            format!("{}\n{}\0", tl("Point camera at QR code"), tl("B=cancel"))
                                .as_ptr(),
                        );
                    }
                } else {
                    unsafe {
                        _3ds_top_clear();
                        let qr_hdr = tl("QR SCAN");
                        _3ds_top_queue_text(
                            120.0,
                            8.0,
                            COL_GOLD,
                            0.65f32,
                            format!("{}\0", qr_hdr).as_ptr(),
                        );
                        let qr_msg = tl("Point camera at deck QR code");
                        _3ds_top_queue_text(
                            40.0,
                            60.0,
                            COL_LIGHT,
                            0.65f32,
                            format!("{}\0", qr_msg).as_ptr(),
                        );
                        let qr_auto = tl("Auto-detects when QR is visible");
                        _3ds_top_queue_text(
                            40.0,
                            85.0,
                            COL_MED,
                            0.65f32,
                            format!("{}\0", qr_auto).as_ptr(),
                        );
                        let qr_cancel = tl("B=cancel");
                        _3ds_top_queue_text(
                            40.0,
                            220.0,
                            COL_MED,
                            0.60f32,
                            format!("{}\0", qr_cancel).as_ptr(),
                        );
                    }
                }
            }
        }
        if qr_start_failed {
            unsafe {
                _3ds_clear_both();
            }
            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(2), true)
        } else if keys & 0x00000002 != 0 {
            if qr_ctx != 0 {
                unsafe {
                    _3ds_qr_free(qr_ctx as *mut u8);
                }
            }
            unsafe {
                _3ds_clear_both();
            }
            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(2), true)
        } else {
            let mut buf = [0u8; 2048];
            let r = unsafe { _3ds_qr_poll(qr_ctx as *mut u8, buf.as_mut_ptr(), buf.len() as u32) };
            if r > 0 {
                dprintln!("[QR] poll r={}", r);
                if qr_ctx != 0 {
                    unsafe {
                        _3ds_qr_free(qr_ctx as *mut u8);
                    }
                }
                dprintln!("[QR] freed context");
                let text = String::from_utf8_lossy(&buf[..r as usize]).to_string();
                dprintln!("[QR] text len={} b64={}", text.len(), looks_like_b64(&text));
                // Try binary QR: base64-encoded binary index format
                let cards_read = if looks_like_b64(&text) {
                    dprintln!("[QR] b64 decode...");
                    if let Some(decoded) = base64_decode(&text) {
                        dprintln!("[QR] b64 ok len={} building sorted...", decoded.len());
                        if let Some(sorted) = CardAtlas::build_qr_sorted(&cards) {
                            dprintln!("[QR] sorted={} decode...", sorted.len());
                            let result = CardAtlas::decode_qr_binary(&sorted, &cards, &decoded);
                            dprintln!("[QR] decode={:?})", result.is_some());
                            result.unwrap_or_default()
                        } else {
                            dprintln!("[QR] sorted alloc FAILED");
                            Vec::new()
                        }
                    } else {
                        dprintln!("[QR] b64 decode FAILED");
                        Vec::new()
                    }
                } else {
                    dprintln!("[QR] not b64, text={}", &text[..text.len().min(40)]);
                    Vec::new()
                };
                dprintln!("[QR] cards_read={}", cards_read.len());
                let cards_read = if cards_read.is_empty() {
                    DeckParser::parse_deck_content(&text)
                } else {
                    cards_read
                };
                dprintln!("[QR] final={} entering QrResult/NotDeck", cards_read.len());
                if cards_read.is_empty() {
                    Step::Setup(
                        cards.clone(),
                        decks.clone(),
                        SetupPhase::QrNotDeck(text, 90),
                        true,
                    )
                } else {
                    Step::Setup(
                        cards.clone(),
                        decks.clone(),
                        SetupPhase::QrResult(cards_read),
                        true,
                    )
                }
            } else if r < 0 {
                if qr_ctx != 0 {
                    unsafe {
                        _3ds_qr_free(qr_ctx as *mut u8);
                    }
                }
                unsafe {
                    _3ds_clear_both();
                    _3ds_text_add_top(
                        format!("{}\n\0", tl_fmt("Camera error", &[("e", &r.to_string())]))
                            .as_ptr(),
                    );
                    _3ds_text_add_top(format!("{}\0", tl("B=back")).as_ptr());
                }
                Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(2), true)
            } else {
                Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    SetupPhase::QrScan(qr_ctx),
                    false,
                )
            }
        }
    }
}

fn qr_result(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    was_dirty: bool,
    cards_read: Vec<String>,
) -> Step {
    {
        dprintln!(
            "[QR] QrResult entered, {} cards, dirty={}",
            cards_read.len(),
            was_dirty
        );
        if was_dirty {
            if unsafe { _3ds_is_cli_mode() } {
                unsafe {
                    _3ds_clear_top();
                    _3ds_text_add_top(format!("{}\n\0", tl("QR DECK")).as_ptr());
                    for c in cards_read.iter().take(20) {
                        _3ds_text_add_top(format!("  {}\n\0", c).as_ptr());
                    }
                    _3ds_text_add_top(
                        format!("\n{} cards\nA=use  B=discard\0", cards_read.len()).as_ptr(),
                    );
                }
            } else {
                unsafe {
                    _3ds_top_clear();
                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                    _3ds_top_queue_text(
                        120.0,
                        8.0,
                        COL_GOLD,
                        0.65f32,
                        format!("{}\0", tl("QR DECK")).as_ptr(),
                    );
                    _3ds_top_queue_text(
                        200.0,
                        32.0,
                        COL_LIGHT,
                        0.65f32,
                        format!("{} cards imported\0", cards_read.len()).as_ptr(),
                    );
                    // Count unique cards
                    let mut counts: HashMap<String, u32> = HashMap::new();
                    for c in cards_read.iter() {
                        *counts.entry(c.clone()).or_insert(0) += 1;
                    }
                    let mut sorted: Vec<_> = counts.into_iter().collect();
                    sorted.sort_by(|a, b| a.0.cmp(&b.0));
                    let mut y = 55.0f32;
                    for (card_no, qty) in sorted.iter().take(15) {
                        _3ds_top_queue_text(
                            40.0,
                            y,
                            COL_LIGHT,
                            0.60f32,
                            format!("{} x {}\0", card_no, qty).as_ptr(),
                        );
                        y += 11.0;
                    }
                    _3ds_top_queue_text(
                        40.0,
                        230.0,
                        COL_MED,
                        0.60f32,
                        format!("{}\0", tl("A=use deck  B=discard")).as_ptr(),
                    );
                }
            }
        }
        if keys & 0x00000002 != 0 {
            unsafe {
                _3ds_clear_both();
            }
            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(2), true)
        } else if keys & 0x00000001 != 0 {
            // Build a DeckList from the scanned cards and add to decks list
            let entry_map = cards_read
                .iter()
                .fold(HashMap::<&str, u32>::new(), |mut m, c| {
                    *m.entry(c.as_str()).or_insert(0) += 1;
                    m
                });
            let entries: Vec<_> = entry_map
                .into_iter()
                .map(|(card_no, qty)| DeckEntry {
                    card_no: card_no.to_string(),
                    quantity: qty as u8,
                })
                .collect();
            let qr_deck = DeckList {
                name: "QR Scanned".to_string(),
                entries,
            };
            let mut new_decks = decks.clone();
            new_decks.push(qr_deck);
            unsafe {
                _3ds_clear_both();
            }
            Step::Setup(cards.clone(), new_decks, SetupPhase::PickMode(0), true)
        } else {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::QrResult(cards_read.clone()),
                false,
            )
        }
    }
}

fn qr_not_deck(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    was_dirty: bool,
    scanned_text: String,
    frames_left: u32,
) -> Step {
    {
        if was_dirty {
            if unsafe { _3ds_is_cli_mode() } {
                unsafe {
                    _3ds_clear_top();
                    _3ds_text_add_top(format!("{}\n\0", tl("NOT A DECK QR")).as_ptr());
                    let preview = if scanned_text.len() > 40 {
                        &scanned_text[..40]
                    } else {
                        &scanned_text
                    };
                    _3ds_text_add_top(format!("  {}\n\0", preview).as_ptr());
                    _3ds_text_add_top(format!("\n{}\n\0", tl("B=back")).as_ptr());
                }
            } else {
                unsafe {
                    _3ds_top_clear();
                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                    _3ds_top_queue_text(
                        100.0,
                        8.0,
                        COL_GOLD,
                        0.65f32,
                        format!("{}\0", tl("NOT A DECK QR")).as_ptr(),
                    );
                    let preview = if scanned_text.len() > 40 {
                        &scanned_text[..40]
                    } else {
                        &scanned_text
                    };
                    _3ds_top_queue_text(
                        20.0,
                        60.0,
                        COL_LIGHT,
                        0.60f32,
                        format!("{}\0", preview).as_ptr(),
                    );
                    _3ds_top_queue_text(
                        20.0,
                        220.0,
                        COL_MED,
                        0.60f32,
                        format!("{}\0", tl("B=back")).as_ptr(),
                    );
                }
            }
        }
        if keys & 0x00000002 != 0 {
            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(2), true)
        } else if frames_left > 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::QrNotDeck(scanned_text, frames_left - 1),
                false,
            )
        } else {
            Step::Setup(cards.clone(), decks.clone(), SetupPhase::QrScan(0), true)
        }
    }
}

fn deck_viewer(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    was_dirty: bool,
    card_ids: &Vec<i16>,
    mut offset: usize,
    vs_ai: bool,
    is_multiplayer: bool,
    viewing_card: &mut Option<i16>,
    card_db: &Arc<CardDatabase>,
    atlas: &CardAtlas,
) -> Step {
    {
        if was_dirty {
            unsafe {
                _3ds_top_clear();
                _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                _3ds_top_queue_text(
                    4.0,
                    4.0,
                    COL_GOLD,
                    0.65f32,
                    format!("{}  (B=close, X=detail)\0", tl("DECK PREVIEW")).as_ptr(),
                );
            }
            if viewing_card.is_none() {
                render_card_grid(card_ids, offset, 5, 2, 28.0, card_db, atlas);
            } else {
                render_card_detail(viewing_card.unwrap(), card_db, 0.0);
            }
        }
        let action = card_grid_input(keys, &mut offset, viewing_card, card_ids, 5);
        match action {
            GridAction::CloseGrid => Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::PickDeck(offset / 10, vs_ai, is_multiplayer),
                true,
            ),
            GridAction::CloseDetail => Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::DeckViewer(
                    card_ids.clone(),
                    offset,
                    0,
                    vs_ai,
                    is_multiplayer,
                    *viewing_card,
                    card_db.clone(),
                    atlas.clone(),
                ),
                true,
            ),
            GridAction::None => Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::DeckViewer(
                    card_ids.clone(),
                    offset,
                    0,
                    vs_ai,
                    is_multiplayer,
                    *viewing_card,
                    card_db.clone(),
                    atlas.clone(),
                ),
                false,
            ),
            _ => Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::DeckViewer(
                    card_ids.clone(),
                    offset,
                    0,
                    vs_ai,
                    is_multiplayer,
                    *viewing_card,
                    card_db.clone(),
                    atlas.clone(),
                ),
                true,
            ),
        }
    }
}

fn control_guide(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    was_dirty: bool,
    page: usize,
) -> Step {
    {
        const GUIDE_PAGES: &[&str] = &[
            "=== MENU CONTROLS ===\n\n\
                                 UP/DOWN = Navigate items\n\
                                 A = Confirm / Select\n\
                                 B = Back / Cancel\n\
                                 X = Preview deck contents\n\
                                 L = Open this help guide\n\
                                 R = Toggle language",
            "=== IN-GAME CONTROLS ===\n\n\
                                 Touch = Select cards on board\n\
                                 L = View card detail overlay\n\
                                 X = Toggle detail mode\n\
                                 R = Toggle action view (debug)\n\
                                 Y = Switch text/graphic mode (debug)\n\
                                 START = In-game pause menu",
            "=== GAME MODES ===\n\n\
                                 VS AI: Play against the computer\n\
                                 Sandbox: Local 2-player hotseat\n\
                                 QR Scan: Import deck via QR code\n\
                                 Local MP: Play on local network",
            "=== CARD ZONES ===\n\n\
                                 Hand: Cards you can play\n\
                                 Energy: Powers member abilities\n\
                                 Stage: Active battle area\n\
                                 Success: Scored cards (win here)\n\
                                 Wait: Drawn-from-deck pile\n\
                                 Deck: Face-down draw pile",
        ];
        let total = GUIDE_PAGES.len();
        let page = page.min(total - 1);
        let guide_text = GUIDE_PAGES[page];
        if was_dirty {
            if unsafe { _3ds_is_cli_mode() } {
                unsafe {
                    _3ds_clear_top();
                    _3ds_text_add_top(format!("{}\n\0", tl("HELP")).as_ptr());
                    for line in guide_text.split('\n') {
                        _3ds_text_add_top(format!("{}\n\0", line).as_ptr());
                    }
                    _3ds_text_add_top(
                        format!("\nPage {}/{}  L/R=pages  B=back\0", page + 1, total).as_ptr(),
                    );
                }
            } else {
                unsafe {
                    _3ds_top_clear();
                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                    _3ds_top_queue_rect(20.0, 15.0, 360.0, 195.0, 0xE60A0E1Au32);
                    _3ds_top_queue_rect(20.0, 15.0, 360.0, 2.0, COL_DIM);
                    _3ds_top_queue_rect(20.0, 208.0, 360.0, 2.0, COL_DIM);
                    _3ds_top_queue_rect(20.0, 15.0, 2.0, 195.0, COL_DIM);
                    _3ds_top_queue_rect(378.0, 15.0, 2.0, 195.0, COL_DIM);
                    let mut y = 25.0f32;
                    for line in guide_text.split('\n') {
                        if line.starts_with("===") {
                            _3ds_top_queue_text(
                                40.0,
                                y,
                                COL_GOLD,
                                0.65f32,
                                format!("{}\0", line).as_ptr(),
                            );
                        } else if !line.is_empty() {
                            _3ds_top_queue_text(
                                30.0,
                                y,
                                COL_LIGHT,
                                0.60f32,
                                format!("{}\0", line).as_ptr(),
                            );
                        }
                        y += 16.0;
                    }
                    _3ds_top_queue_text(
                        4.0,
                        215.0,
                        COL_MED,
                        0.55f32,
                        format!("Page {}/{}   L/R=pages  B=back\0", page + 1, total).as_ptr(),
                    );
                }
            }
        }
        if keys & 0x00000002 != 0 {
            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(0), true)
        } else if keys & 0x00000100 != 0 && page + 1 < total {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::ControlGuide(page + 1),
                true,
            )
        } else if keys & 0x00000200 != 0 && page > 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::ControlGuide(page - 1),
                true,
            )
        } else {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::ControlGuide(page),
                false,
            )
        }
    }
}

fn multiplayer_pick_role(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    was_dirty: bool,
    deck_idx: usize,
    cur: usize,
) -> Step {
    {
        if was_dirty {
            if unsafe { _3ds_is_cli_mode() } {
                unsafe {
                    _3ds_clear_top();
                    let mp_hdr = tl("MULTIPLAYER");
                    _3ds_text_add_top(format!("{}\n\0", mp_hdr).as_ptr());
                    let host_net = tl("Host = create network");
                    _3ds_text_add_top(format!("{}\n\0", host_net).as_ptr());
                    let client_net = tl("Client = join network");
                    _3ds_text_add_top(format!("{}\n\n\0", client_net).as_ptr());
                    let arrow_h = if cur == 0 { ">" } else { " " };
                    let arrow_c = if cur == 1 { ">" } else { " " };
                    let host_label = tl("Host");
                    let client_label = tl("Client");
                    _3ds_text_add_top(format!("{} {}\n\0", arrow_h, host_label).as_ptr());
                    _3ds_text_add_top(format!("{} {}\n\0", arrow_c, client_label).as_ptr());
                    _3ds_text_add_top("\nUP/DOWN=select A=confirm B=back\0".as_ptr());
                }
            } else {
                unsafe {
                    _3ds_top_clear();
                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                    let mp_hdr = tl("MULTIPLAYER");
                    _3ds_top_queue_text(
                        100.0,
                        8.0,
                        COL_GOLD,
                        0.65f32,
                        format!("{}\0", mp_hdr).as_ptr(),
                    );
                }
                let host_label = tl("Host");
                let client_label = tl("Client");
                let labels = [
                    format!("{} ({})", host_label, tl("Host = create network")),
                    format!("{} ({})", client_label, tl("Client = join network")),
                ];
                for (i, m) in labels.iter().enumerate() {
                    let y = 60.0 + i as f32 * 64.0;
                    let bg = if i == cur { COL_SEL } else { COL_DIM };
                    unsafe {
                        _3ds_top_queue_rect(40.0, y, 320.0, 50.0, bg);
                    }
                    if i == cur {
                        unsafe {
                            _3ds_top_queue_rect(40.0, y, 320.0, 50.0, COL_HIGHLIGHT);
                        }
                    }
                    let color = if i == cur { COL_GOLD } else { COL_LIGHT };
                    unsafe {
                        _3ds_top_queue_text(
                            50.0,
                            y + 12.0,
                            color,
                            0.65f32,
                            format!("{}\0", m).as_ptr(),
                        );
                    }
                }
                render_hint_bar(&tl("UP/DOWN=select  A=confirm  B=back"));
            }
        }
        if keys & 0x00000040 != 0 && cur > 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::MultiplayerPickRole(deck_idx, cur - 1),
                true,
            )
        } else if keys & 0x00000080 != 0 && cur + 1 < 2 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::MultiplayerPickRole(deck_idx, cur + 1),
                true,
            )
        } else if keys & 0x00000002 != 0 {
            // B = back to PickMode
            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(4), true)
        } else if keys & 0x00000001 != 0 {
            // A = select role
            // deck_idx is the deck index from MultiplayerDeck
            // cur is the role cursor (0=Host, 1=Client)
            if cur == 0 {
                // Host
                Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    SetupPhase::MultiplayerHostWait(deck_idx),
                    true,
                )
            } else {
                // Client
                Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    SetupPhase::MultiplayerClientScan(deck_idx, 0),
                    true,
                )
            }
        } else {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::MultiplayerPickRole(deck_idx, cur),
                false,
            )
        }
    }
}

fn multiplayer_host_wait(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    was_dirty: bool,
    p1_idx: usize,
) -> Step {
    {
        // Initialize UDS as host on first entry
        if was_dirty {
            let init_result = uds::uds_init(true);
            match init_result {
                Ok(()) => {
                    if unsafe { _3ds_is_cli_mode() } {
                        unsafe {
                            _3ds_clear_top();
                            _3ds_text_add_top(
                                format!("{}\n\0", tl("HOST: Network created!")).as_ptr(),
                            );
                            let wait_msg = tl("Waiting for client...");
                            _3ds_text_add_top(format!("{}\n\0", wait_msg).as_ptr());
                            let b_cancel = tl("B = cancel");
                            _3ds_text_add_top(format!("{}\n\0", b_cancel).as_ptr());
                        }
                    } else {
                        unsafe {
                            _3ds_top_clear();
                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                            _3ds_top_queue_text(
                                80.0,
                                8.0,
                                COL_GOLD,
                                0.65f32,
                                format!("{}\0", tl("HOST: Network created!")).as_ptr(),
                            );
                            let wait_msg = tl("Waiting for client...");
                            _3ds_top_queue_text(
                                50.0,
                                100.0,
                                COL_LIGHT,
                                0.65f32,
                                format!("{}\0", wait_msg).as_ptr(),
                            );
                            _3ds_top_queue_text(
                                50.0,
                                230.0,
                                COL_MED,
                                0.60f32,
                                format!("{}\0", tl("B=cancel")).as_ptr(),
                            );
                        }
                    }
                }
                Err(e) => {
                    if unsafe { _3ds_is_cli_mode() } {
                        unsafe {
                            _3ds_clear_top();
                            _3ds_text_add_top(
                                format!(
                                    "{}\n\0",
                                    tl_fmt("UDS INIT FAILED", &[("e", &e.to_string())])
                                )
                                .as_ptr(),
                            );
                            _3ds_text_add_top(format!("{}\n\0", tl("B = back")).as_ptr());
                        }
                    } else {
                        unsafe {
                            _3ds_top_clear();
                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                            _3ds_top_queue_text(
                                80.0,
                                8.0,
                                0xFF0000FF,
                                0.65f32,
                                format!(
                                    "{}\0",
                                    tl_fmt("UDS INIT FAILED", &[("e", &e.to_string())])
                                )
                                .as_ptr(),
                            );
                        }
                    }
                }
            }
        }
        // Poll for client connection: try to receive a hello packet
        let mut hello = [0u8; 4];
        match uds::uds_recv(&mut hello) {
            Ok(n) if n > 0 => {
                // Client connected! Read client's deck index from hello packet
                let p2_idx = if n >= 2 { hello[1] as usize } else { 0 };
                Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    SetupPhase::MultiplayerSyncDeck(p1_idx, p2_idx, true),
                    true,
                )
            }
            _ => {
                if keys & 0x00000002 != 0 {
                    // B = cancel
                    uds::uds_exit();
                    Step::Setup(
                        cards.clone(),
                        decks.clone(),
                        SetupPhase::MultiplayerPickRole(p1_idx, 0),
                        true,
                    )
                } else {
                    Step::Setup(
                        cards.clone(),
                        decks.clone(),
                        SetupPhase::MultiplayerHostWait(p1_idx),
                        false,
                    )
                }
            }
        }
    }
}

fn multiplayer_client_scan(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    was_dirty: bool,
    p1_idx: usize,
    frames: u32,
) -> Step {
    {
        // B = back to role selection
        if keys & 0x00000002 != 0 {
            uds::uds_exit();
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::MultiplayerPickRole(p1_idx, 0),
                true,
            )
        } else {
            // A = force rescan now, or auto-rescan every ~3s (180 frames)
            let do_scan = was_dirty || keys & 0x00000001 != 0 || frames == 0;
            if do_scan {
                if was_dirty || keys & 0x00000001 != 0 {
                    let _ = uds::uds_init(false);
                }
                let hosts = uds::uds_scan_networks();
                if hosts.is_empty() {
                    // No hosts found — rescan after delay
                    if unsafe { _3ds_is_cli_mode() } {
                        unsafe {
                            _3ds_clear_top();
                            _3ds_text_add_top(format!("{}\n\0", tl("Scanning...")).as_ptr());
                            _3ds_text_add_top(format!("{}\n\0", tl("A=refresh B=back")).as_ptr());
                        }
                    } else {
                        unsafe {
                            _3ds_top_clear();
                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                            _3ds_top_queue_text(
                                80.0,
                                100.0,
                                COL_MED,
                                0.75f32,
                                format!("{}\0", tl("Scanning...")).as_ptr(),
                            );
                            _3ds_top_queue_text(
                                80.0,
                                230.0,
                                COL_MED,
                                0.60f32,
                                format!("{}\0", tl("A=refresh B=back")).as_ptr(),
                            );
                        }
                    }
                    Step::Setup(
                        cards.clone(),
                        decks.clone(),
                        SetupPhase::MultiplayerClientScan(p1_idx, 180),
                        false,
                    )
                } else {
                    // Hosts found — go to selection
                    Step::Setup(
                        cards.clone(),
                        decks.clone(),
                        SetupPhase::MultiplayerClientHostSelect(p1_idx, hosts, 0),
                        true,
                    )
                }
            } else {
                // Waiting for rescan timer — decrement and keep scanning text
                Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    SetupPhase::MultiplayerClientScan(p1_idx, frames - 1),
                    false,
                )
            }
        }
    }
}

fn multiplayer_client_host_select(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    keys: u32,
    p1_idx: usize,
    hosts: &Vec<u16>,
    cursor: usize,
) -> Step {
    {
        let n = hosts.len();
        if n == 0 {
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::MultiplayerClientScan(p1_idx, 0),
                true,
            )
        } else if keys & 0x00000002 != 0 {
            // B = back to scan
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::MultiplayerClientScan(p1_idx, 0),
                true,
            )
        } else if keys & 0x00000001 != 0 {
            // A = connect to selected host
            let selected = hosts[cursor];
            match uds::uds_connect_network(selected) {
                Ok(()) => {
                    let hello = [0xAAu8, (p1_idx & 0xFF) as u8];
                    let _ = uds::uds_send(&hello);
                    Step::Setup(
                        cards.clone(),
                        decks.clone(),
                        SetupPhase::MultiplayerSyncDeck(p1_idx, 0, false),
                        true,
                    )
                }
                Err(_) => Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    SetupPhase::MultiplayerClientHostSelect(p1_idx, hosts.clone(), cursor),
                    true,
                ),
            }
        } else {
            // UP/DOWN to navigate
            let mut new_cursor = cursor;
            if keys & 0x00000040 != 0 && cursor > 0 {
                new_cursor = cursor - 1;
            }
            if keys & 0x00000080 != 0 && cursor + 1 < n {
                new_cursor = cursor + 1;
            }
            // Draw host list
            if unsafe { _3ds_is_cli_mode() } {
                unsafe {
                    _3ds_clear_top();
                }
                let hdr = tl("SELECT HOST");
                unsafe {
                    _3ds_text_add_top(format!("{}\n\0", hdr).as_ptr());
                }
                for (i, _) in hosts.iter().enumerate() {
                    let prefix = "";
                    let label = format!("{}{}\0", prefix, format!("Host {}", i + 1));
                    unsafe {
                        _3ds_text_add_top(format!("{}\n\0", label).as_ptr());
                    }
                }
                unsafe {
                    _3ds_text_add_top(format!("{}\n\0", tl("A=connect B=back")).as_ptr());
                }
            } else {
                unsafe {
                    _3ds_top_clear();
                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                    _3ds_top_queue_text(
                        80.0,
                        8.0,
                        COL_GOLD,
                        0.65f32,
                        format!("{}\0", tl("SELECT HOST")).as_ptr(),
                    );
                }
                for i in 0..n {
                    let y = 50.0 + i as f32 * 32.0;
                    let col = if i == new_cursor { COL_SEL } else { COL_LIGHT };
                    let prefix = "";
                    unsafe {
                        _3ds_top_queue_text(
                            40.0,
                            y,
                            col,
                            0.65f32,
                            format!("{}{}\0", prefix, format!("Host {}", i + 1)).as_ptr(),
                        );
                    }
                }
                unsafe {
                    _3ds_top_queue_text(
                        40.0,
                        220.0,
                        COL_MED,
                        0.60f32,
                        format!("{}\0", tl("A=connect B=back")).as_ptr(),
                    );
                }
            }
            Step::Setup(
                cards.clone(),
                decks.clone(),
                SetupPhase::MultiplayerClientHostSelect(p1_idx, hosts.clone(), new_cursor),
                false,
            )
        }
    }
}

fn multiplayer_sync_deck(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    was_dirty: bool,
    p1_idx: usize,
    p2_idx: usize,
    is_host: bool,
) -> Step {
    {
        if is_host {
            // Host: send template IDs so client calls create_copy in the same order → matching instance IDs.
            let seed = unsafe { _3ds_system_tick() } as u64;
            let r = (|| -> Result<Vec<u8>, String> {
                use rabuka_engine::card::CardDatabase;
                let mut cards_vec = (**cards).clone();
                CardLoader::attach_abilities(&mut cards_vec);
                let db = CardDatabase::load_or_create(cards_vec);
                let nums1 = DeckParser::deck_list_to_card_numbers(&decks[p1_idx]);
                let nums2 = if p1_idx == p2_idx {
                    nums1.clone()
                } else {
                    DeckParser::deck_list_to_card_numbers(&decks[p2_idx])
                };
                // Convert card_no strings to template IDs (same on both machines)
                let to_ids = |nos: &Vec<String>| -> Vec<u16> {
                    nos.iter()
                        .filter_map(|no| db.get_card_id(no).map(|id| id as u16))
                        .collect()
                };
                let sync = uds::DeckSync {
                    seed,
                    p1_main_templates: to_ids(&nums1),
                    p1_energy_templates: Vec::new(),
                    p2_main_templates: to_ids(&nums2),
                    p2_energy_templates: Vec::new(),
                };
                let data = sync.to_bytes();
                uds::uds_send(&data).map_err(|e| format!("Send: {}", e))?;
                Ok(data)
            })();
            match r {
                Ok(data) => Step::Setup(
                    cards.clone(),
                    decks.clone(),
                    // Pass the sync bytes to the host too so BOTH consoles build
                    // their GameState from the same templates + seed — identical
                    // start state is what lets action-only sync work (no full
                    // state transfers during gameplay).
                    SetupPhase::MultiplayerLoading(p1_idx, p2_idx, true, Some(data), seed),
                    true,
                ),
                Err(e) => {
                    uds::uds_exit();
                    Step::Done(Err(format!("Deck sync failed: {}", e)))
                }
            }
        } else {
            // Client: Receive deck order from host
            if was_dirty {
                if unsafe { _3ds_is_cli_mode() } {
                    unsafe {
                        _3ds_clear_top();
                        _3ds_text_add_top(format!("{}\n\0", tl("Receiving deck data...")).as_ptr());
                    }
                } else {
                    unsafe {
                        _3ds_top_clear();
                        _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                        _3ds_top_queue_text(
                            80.0,
                            8.0,
                            COL_GOLD,
                            0.65f32,
                            format!("{}\0", tl("Receiving deck data...")).as_ptr(),
                        );
                    }
                }
            }
            // Try to receive deck sync
            let mut recv_buf = [0u8; 4096];
            match uds::uds_recv(&mut recv_buf) {
                Ok(n) if n > 0 => {
                    if let Some(sync) = uds::DeckSync::from_bytes(&recv_buf[..n]) {
                        let sync_bytes = recv_buf[..n].to_vec();
                        Step::Setup(
                            cards.clone(),
                            decks.clone(),
                            SetupPhase::MultiplayerLoading(
                                p1_idx,
                                p2_idx,
                                false,
                                Some(sync_bytes),
                                sync.seed,
                            ),
                            true,
                        )
                    } else {
                        Step::Setup(
                            cards.clone(),
                            decks.clone(),
                            SetupPhase::MultiplayerSyncDeck(p1_idx, p2_idx, false),
                            false,
                        )
                    }
                }
                _ => {
                    // No data yet, keep waiting
                    Step::Setup(
                        cards.clone(),
                        decks.clone(),
                        SetupPhase::MultiplayerSyncDeck(p1_idx, p2_idx, false),
                        false,
                    )
                }
            }
        }
    }
}

fn multiplayer_loading(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    p1_idx: usize,
    p2_idx: usize,
    is_host: bool,
    deck_sync_bytes: Option<Vec<u8>>,
    seed: u64,
) -> Step {
    {
        let r = (|| -> Result<(GameState, CardAtlas), String> {
            let mut cards_vec = (**cards).clone();
            CardLoader::attach_abilities(&mut cards_vec);
            let mut db = Arc::new(CardDatabase::load_or_create(cards_vec));
            // If we have deck sync data from host, use it directly
            if let Some(ref sync_bytes) = deck_sync_bytes {
                let sync = uds::DeckSync::from_bytes(sync_bytes).ok_or("Invalid deck sync data")?;
                // Build decks from template IDs using create_copy in same order → matching instance IDs
                let build_from_templates =
                    |db: &mut Arc<CardDatabase>,
                     templates: &Vec<u16>|
                     -> Result<rabuka_engine::deck_builder::Deck, String> {
                        let mut deck = rabuka_engine::deck_builder::Deck {
                            main_deck: std::collections::VecDeque::new(),
                            energy_deck: std::collections::VecDeque::new(),
                        };
                        for &tid in templates {
                            let cid = Arc::make_mut(db).create_copy(tid as i16);
                            if let Some(card) = db.get_card(cid) {
                                match card.card_type {
                                    rabuka_engine::card::CardType::Energy => {
                                        deck.energy_deck.push_back(cid)
                                    }
                                    _ => deck.main_deck.push_back(cid),
                                }
                            }
                        }
                        Ok(deck)
                    };
                let mut pd1 = build_from_templates(&mut db, &sync.p1_main_templates)
                    .map_err(|e| format!("Deck1: {}", e))?;
                let mut pd2 = build_from_templates(&mut db, &sync.p2_main_templates)
                    .map_err(|e| format!("Deck2: {}", e))?;
                DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
                DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
                // Shuffle with same seed as host for identical deck order
                rabuka_engine::rng::seed(sync.seed as u32);
                pd1.shuffle_main_deck();
                pd1.shuffle_energy_deck();
                pd2.shuffle_main_deck();
                pd2.shuffle_energy_deck();
                let mut p1 = Player::new("p1".into(), "P1".into(), true);
                p1.set_main_deck(pd1.main_deck);
                p1.set_energy_deck(pd1.energy_deck);
                let mut p2 = Player::new("p2".into(), "P2".into(), false);
                p2.set_main_deck(pd2.main_deck);
                p2.set_energy_deck(pd2.energy_deck);
                let mut gs = GameState::new(p1, p2, db);
                game_setup::setup_game(&mut gs);
                return Ok((gs, CardAtlas::load()));
            }
            // No deck sync: build from local files (host or non-multiplayer)
            let nums1 = DeckParser::deck_list_to_card_numbers(&decks[p1_idx]);
            let nums2 = if p1_idx == p2_idx {
                nums1.clone()
            } else {
                DeckParser::deck_list_to_card_numbers(&decks[p2_idx])
            };
            let mut pd1 = DeckBuilder::build_deck_from_database(&mut db, nums1)
                .map_err(|e| format!("Deck: {}", e))?;
            let mut pd2 = DeckBuilder::build_deck_from_database(&mut db, nums2)
                .map_err(|e| format!("Deck: {}", e))?;
            rabuka_engine::rng::seed(seed as u32);
            pd1.shuffle_main_deck();
            pd1.shuffle_energy_deck();
            pd2.shuffle_main_deck();
            pd2.shuffle_energy_deck();
            let mut deck_nos: HashSet<String> = HashSet::new();
            for cid in pd1
                .main_deck
                .iter()
                .chain(pd1.energy_deck.iter())
                .chain(pd2.main_deck.iter())
                .chain(pd2.energy_deck.iter())
            {
                if let Some(card) = db.get_card(*cid) {
                    deck_nos.insert(card.card_no.to_string());
                }
            }
            DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
            DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
            let mut p1 = Player::new("p1".into(), "P1".into(), true);
            p1.set_main_deck(pd1.main_deck);
            p1.set_energy_deck(pd1.energy_deck);
            let mut p2 = Player::new("p2".into(), "P2".into(), false);
            p2.set_main_deck(pd2.main_deck);
            p2.set_energy_deck(pd2.energy_deck);
            let mut gs = GameState::new(p1, p2, db);
            game_setup::setup_game(&mut gs);
            Ok((gs, CardAtlas::load()))
        })();
        match r {
            Ok((gs, atlas)) => {
                unsafe {
                    _3ds_board_enable(true);
                }
                Step::Play(PlayState {
                    gs,
                    cur: 0,
                    acts_cache: Vec::new(),
                    dirty: true,
                    redraw: true,
                    atlas,
                    vs_ai: false,
                    ai_vs_ai: false,
                    cli_mode: false,
                    detail_mode: false,
                    choice_image_mode: true,
                    choice_subview: false,
                    text_page: 0,
                    choice_grid_offset: 0,
                    list_scroll: 0,
                    detail_scroll_y: 0.0f32,
                    hand_offset: 0,
                    hand_offset_p2: 0,
                    touch_tap_count: 0,
                    viewing_card: None,
                    zone_viewer: None,
                    zone_viewer_offset: 0,
                    was_touching: false,
                    is_multiplayer: true,
                    is_host,
                    waiting_for_opponent: !is_host,
                    overlay: Overlay::None,
                    pending_client_action: None,
                    last_client_action_seq: 0,
                    next_action_seq: 1,
                    dbg_tx_bytes: 0,
                    dbg_rx_bytes: 0,
                })
            }
            Err(e) => Step::Done(Err(e)),
        }
    }
}

/// Route one setup frame to the handler for the active SetupPhase.
pub fn setup_step(
    cards: &Arc<Vec<Card>>,
    decks: &Vec<DeckList>,
    phase: &SetupPhase,
    keys: u32,
    dirty: bool,
) -> Step {
    let n = decks.len();
    let was_dirty = dirty;
    let new_step = match phase.clone() {
        SetupPhase::PickMode(cur) => pick_mode(cards, decks, keys, n, was_dirty, cur),
        SetupPhase::PickDeck(cur, vs_ai, is_multiplayer) => {
            pick_deck(cards, decks, keys, n, was_dirty, cur, vs_ai, is_multiplayer)
        }
        SetupPhase::MultiplayerDeck(cur) => multiplayer_deck(cards, decks, keys, n, was_dirty, cur),
        SetupPhase::PickDeck2(cur, p1_idx, vs_ai) => {
            pick_deck2(cards, decks, keys, n, was_dirty, cur, p1_idx, vs_ai)
        }
        SetupPhase::Loading(p1_idx, p2_idx, vs_ai) => loading(cards, decks, p1_idx, p2_idx, vs_ai),
        SetupPhase::Testing => testing(cards, decks, keys),
        SetupPhase::QrScan(ctx) => qr_scan(cards, decks, keys, was_dirty, ctx),
        SetupPhase::QrResult(cards_read) => qr_result(cards, decks, keys, was_dirty, cards_read),
        SetupPhase::QrNotDeck(scanned_text, frames_left) => {
            qr_not_deck(cards, decks, keys, was_dirty, scanned_text, frames_left)
        }
        SetupPhase::DeckViewer(
            ref card_ids,
            offset,
            _,
            vs_ai,
            is_multiplayer,
            ref mut viewing_card,
            ref card_db,
            ref atlas,
        ) => deck_viewer(
            cards,
            decks,
            keys,
            was_dirty,
            card_ids,
            offset,
            vs_ai,
            is_multiplayer,
            viewing_card,
            card_db,
            atlas,
        ),
        SetupPhase::ControlGuide(page) => control_guide(cards, decks, keys, was_dirty, page),
        SetupPhase::MultiplayerPickRole(deck_idx, cur) => {
            multiplayer_pick_role(cards, decks, keys, was_dirty, deck_idx, cur)
        }
        SetupPhase::MultiplayerHostWait(p1_idx) => {
            multiplayer_host_wait(cards, decks, keys, was_dirty, p1_idx)
        }
        SetupPhase::MultiplayerClientScan(p1_idx, frames) => {
            multiplayer_client_scan(cards, decks, keys, was_dirty, p1_idx, frames)
        }
        SetupPhase::MultiplayerClientHostSelect(p1_idx, ref hosts, cursor) => {
            multiplayer_client_host_select(cards, decks, keys, p1_idx, hosts, cursor)
        }
        SetupPhase::MultiplayerSyncDeck(p1_idx, p2_idx, is_host) => {
            multiplayer_sync_deck(cards, decks, was_dirty, p1_idx, p2_idx, is_host)
        }
        SetupPhase::MultiplayerLoading(p1_idx, p2_idx, is_host, deck_sync_bytes, seed) => {
            multiplayer_loading(cards, decks, p1_idx, p2_idx, is_host, deck_sync_bytes, seed)
        }
    };
    new_step
}
