#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use rabuka_engine::card::Card;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::core::card_binary;
use rabuka_engine::game::link::{decode_setup, encode_setup, MSG_SETUP};
use rabuka_engine::game::platform_ui::{self, LinkTransport, MatchMode, PlatformUi};
use rabuka_engine::game::sav::SavDeck;
use rabuka_engine::rng;

use rabuka_gba::decks_baked::DECKS;
use rabuka_gba::gba_ui::GbaUi;
use rabuka_gba::input::{Button, Input};
use rabuka_gba::link::LinkCable;
use rabuka_gba::screens::Screen;
use rabuka_gba::ui::Display;

/// One side's link deck: display name + main-deck card numbers (no energy —
/// the engine adds default energy at setup, same as baked decks).
fn baked_sav_deck(idx: usize) -> SavDeck {
    SavDeck {
        name: DECKS[idx].name.to_string(),
        cards: DECKS[idx].cards.iter().map(|c| c.to_string()).collect(),
    }
}

/// Resolve card numbers to `Card`s through the full ROM blob (any card,
/// not just baked ones), then attach abilities. Returns `None` when any
/// number fails to resolve — the deck is dropped whole, never partial.
fn resolve_link_cards(card_nos: &[String]) -> Option<Vec<Card>> {
    // One full decode pass per link match (not per card): the folded
    // index binary-searches each number.
    let index = card_binary::blob_index_by_folded_no();
    let mut cards: Vec<Card> = Vec::with_capacity(card_nos.len());
    for no in card_nos {
        let idx = card_binary::resolve_card_no(&index, no)?;
        cards.push(card_binary::decode_card_from_blob(idx)?);
    }
    CardLoader::attach_abilities(&mut cards);
    Some(cards)
}

/// Blocking text prompt: shows `lines`, waits for A. Used for link status
/// (syncing / bad deck) so failures explain instead of silently looping.
fn wait_screen(display: &mut Display, input: &mut Input, lines: &[&str]) {
    display.clear();
    for l in lines {
        display.println(l);
    }
    display.swap_buffers();
    loop {
        input.poll();
        if input.just_pressed(Button::A) {
            return;
        }
        display.wait();
    }
}

/// Link match setup: exchange [`SavDeck`] entries with the peer (each side
/// sends its own deck) and agree on the host seed. The seed is fixed
/// (`0x5EED`, same as VsAi boot): both sides shuffle identically, which is
/// all lockstep needs. Returns the cable plus owned card lists and union
/// cards on success, `None` on abort (B), dead cable, or an unresolvable
/// peer deck. Never checks link liveness while waiting — a peer choosing
/// a deck looks exactly like silence, so only B aborts the wait.
fn link_setup(
    display: &mut Display,
    input: &mut Input,
    own: SavDeck,
    is_host: bool,
) -> Option<(LinkCable, Vec<String>, Vec<String>, Vec<Card>, u32)> {
    let mut cable = LinkCable::init();
    let seed: u64 = 0x5EED;
    let my_packet = encode_setup(seed, &own);

    // Sync loop: both sides pump continuously, so either side can send at
    // any time. Resend ours every 60 frames; B aborts to the menu.
    let mut frames = 0u32;
    let peer: (u64, SavDeck) = loop {
        cable.pump(32);
        if frames % 60 == 0 {
            cable.send_packet(&my_packet);
        }
        frames += 1;
        input.poll();
        if input.just_pressed(Button::B) {
            return None;
        }
        // Drain for a peer setup (validated by the sav codec on decode).
        let mut found = None;
        while let Some(pkt) = cable.recv_packet() {
            if pkt.first() == Some(&MSG_SETUP) {
                if let Some(parsed) = decode_setup(&pkt) {
                    found = Some(parsed);
                    break;
                }
            }
        }
        if let Some(p) = found {
            break p;
        }
        display.clear();
        display.println(if is_host {
            "HOST: wait guest B:Abort"
        } else {
            "JOIN: wait host B:Abort"
        });
        display.swap_buffers();
        display.wait();
    };
    // Traffic proved the peer alive; clear the menu-dwell misses so the
    // match starts with a clean liveness slate.
    cable.mark_alive();

    let seed = if is_host { seed } else { peer.0 };
    let (host_deck, guest_deck) = if is_host {
        (own, peer.1)
    } else {
        (peer.1, own)
    };
    let mut nos: Vec<String> =
        Vec::with_capacity(host_deck.cards.len() + guest_deck.cards.len());
    for n in host_deck.cards.iter().chain(guest_deck.cards.iter()) {
        if !nos.contains(n) {
            nos.push(n.clone());
        }
    }
    let all_cards = match resolve_link_cards(&nos) {
        Some(c) => c,
        None => {
            wait_screen(display, input, &["Bad peer deck", "A:Menu"]);
            return None;
        }
    };
    Some((cable, host_deck.cards, guest_deck.cards, all_cards, seed as u32))
}

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    let mut display = rabuka_gba::ui::Display::new(gba.graphics.get());
    let mut input = Input::new();
    rng::seed(0x5EED);

    let decks = DECKS;
    let names: Vec<&str> = decks.iter().map(|d| d.name).collect();
    let modes = ["VS AI", "2 Player", "Link Host", "Link Join", "AI vs AI"];

    // Explicit boot flow — see `screens::Screen` for the full button map:
    // ModeSelect -> DeckSelectP1 -> (DeckSelectP2) -> Match -> Result -> ...
    // A finished match restarts cleanly at ModeSelect instead of freezing.
    // Mode/deck picks keep GBA-tuned titles (button hints); the match loop
    // itself is the engine's shared `run_match` (AI heuristic included), so
    // the port no longer carries its own copy of the game loop. Link games
    // run the shared `run_link_match` lockstep loop instead (same engine,
    // peer's picks over the cable); abort any link wait with B.
    loop {
        let _ = Screen::ModeSelect;
        let mut ui = GbaUi::new(&mut display, &mut input);
        let as_ui = &mut ui as &mut dyn PlatformUi;
        let mode_idx = platform_ui::select(as_ui, &modes, "MODE Up/Dn:A/Start");

        // Link games leave the shared menu flow: pick a deck, sync over
        // the cable, play the lockstep loop, return here after.
        if mode_idx == 2 || mode_idx == 3 {
            let is_host = mode_idx == 2;
            let _ = Screen::DeckSelectP1;
            let d = platform_ui::select(as_ui, &names, "LINK DECK A:Pick");
            let own = baked_sav_deck(d);
            match link_setup(&mut display, &mut input, own, is_host) {
                Some((mut cable, p1_nos, p2_nos, all_cards, seed)) => {
                    let p1: Vec<&str> = p1_nos.iter().map(|s| s.as_str()).collect();
                    let p2: Vec<&str> = p2_nos.iter().map(|s| s.as_str()).collect();
                    rng::seed(seed);
                    // Fresh UI: `ui`'s borrow ended at the deck pick above.
                    let mut ui3 = GbaUi::new(&mut display, &mut input);
                    let _ = platform_ui::run_link_match(
                        &mut ui3,
                        &mut cable,
                        u8::from(!is_host),
                        &p1,
                        &p2,
                        all_cards,
                    );
                    let _ = Screen::Result;
                }
                None => {
                    let _ = Screen::ModeSelect;
                }
            }
            continue;
        }

        let mode = match mode_idx {
            1 => MatchMode::TwoPlayer,
            4 => MatchMode::AiVsAi,
            _ => MatchMode::VsAi,
        };

        let _ = Screen::DeckSelectP1;
        let d1 = platform_ui::select(as_ui, &names, "P1 DECK Up/Dn:A/Start");
        let _ = Screen::DeckSelectP2;
        let d2 = if matches!(mode, MatchMode::TwoPlayer) {
            platform_ui::select(as_ui, &names, "P2 DECK Up/Dn:A/Start")
        } else {
            rng::rand_range(names.len())
        };

        // Match (Screen::Board/Actions/StartMenu/CardDetail/ChoiceGrid are
        // driven by the engine from here; Screen::Result shows at the end).
        let _ = Screen::Board;
        let p1_cards = decks[d1].cards;
        let p2_cards = decks[d2].cards;
        let all_cards =
            rabuka_engine::game::deck_parser::load_two_decks_with_abilities(d1, d2);

        // Shared engine match loop (no platform copy).
        platform_ui::run_match(&mut ui, p1_cards, p2_cards, all_cards, mode);

        let _ = Screen::Result;
    }
}
