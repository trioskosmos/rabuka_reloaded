//! Whole-corpus smoke test: EVERY card's abilities are decoded from the
//! bytecode store, activated where possible, and executed against a real
//! GameState — asserting nothing panics anywhere.
//!
//! This is the gameplay-as-written safety net at maximum radius: any decoder
//! regression, missing field handling, or executor bug that blows up on ANY
//! of the 900+ printed abilities fails here immediately, even for cards with
//! no dedicated test file.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// After execution, the game must remain structurally sane: a card instance
/// lives in exactly one zone, no zone lists an id twice, and stage/energy
/// areas hold no garbage.
fn assert_state_invariants(game: &TestGame) {
    let p1 = &game.state.player1;
    let p2 = &game.state.player2;

    fn zone_vecs<'a>(p: &'a rabuka_engine::player::Player) -> Vec<(&'static str, &'a [i16])> {
        vec![
            ("stage", &p.stage.stage[..]),
            ("hand", &p.hand.cards),
            ("waitroom", &p.waitroom.cards),
            ("main_deck", &p.main_deck.cards),
            ("live_card_zone", &p.live_card_zone.cards),
            ("success_live_zone", &p.success_live_card_zone.cards),
            ("energy_zone", &p.energy_zone.cards),
        ]
    }

    for (pname, p) in [("p1", p1), ("p2", p2)] {
        let mut seen: std::collections::HashMap<i16, &'static str> =
            std::collections::HashMap::new();
        for (zone, cards) in zone_vecs(p) {
            let mut in_zone: std::collections::HashSet<i16> = std::collections::HashSet::new();
            for &id in cards {
                if id < 0 {
                    continue; // empty stage slots / placeholders
                }
                if !in_zone.insert(id) {
                    panic!("{pname}: card {id} listed twice in {zone}");
                }
                if let Some(prev) = seen.insert(id, zone) {
                    panic!("{pname}: card {id} simultaneously in '{prev}' and '{zone}'");
                }
            }
        }
        // Stage is exactly 3 area slots; anything else corrupts formation.
        assert_eq!(p.stage.stage.len(), 3, "{pname}: stage slot count drifted");
    }
}

fn smoke_one_card(db: &std::sync::Arc<rabuka_engine::card::CardDatabase>, card_no: &str) {
    let mut game = TestGame::new(db.clone());
    let cid = game.id(card_no);

    // Put the card on stage as the activating card so self-targeting,
    // position gates and constant recalculation have something to chew on.
    game.add_to_stage(MemberArea::Center, cid);
    game.state.activating_card = Some(cid);

    // Attempt activation (kidou abilities, costed effects). Many legitimately
    // refuse (unpaid costs, wrong phase) — Err is fine, panic is not.
    let _ = game.try_activate_ability(cid);

    // Resolve any pushed choices (bounded so pathological loops can't hang).
    for _ in 0..8 {
        if !game.has_pending_choice() {
            break;
        }
        let _ = game.try_select_indices(&[0]);
    }

    // Auto-ability pipeline: condition evaluation + effect execution.
    let pid = game.state.player1.id.clone();
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();

    assert_state_invariants(&game);
}

#[test]
fn every_card_executes_without_panicking() {
    let db = load_real_database();

    let card_nos: Vec<String> = {
        let mut v: Vec<String> = db
            .cards
            .values()
            .map(|c| c.card_no.to_string())
            .collect();
        v.sort();
        v
    };
    assert!(
        card_nos.len() > 2000,
        "expected the full card database, got {} cards",
        card_nos.len()
    );

    // Silence per-panic hook spam; we aggregate failures ourselves.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    // Cards are independent (each gets its own GameState over the shared
    // read-only Arc<CardDatabase>), so smoke them across a thread pool.
    // Failures are collected per-worker and merged in deterministic order
    // (chunks preserve card_no sort order).
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .max(1);
    let chunk_size = card_nos.len().div_ceil(workers);

    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = card_nos
            .chunks(chunk_size)
            .map(|chunk| {
                let db = db.clone();
                scope.spawn(move || {
                    let mut fails: Vec<String> = Vec::new();
                    for no in chunk {
                        let ok = catch_unwind(AssertUnwindSafe(|| smoke_one_card(&db, no)));
                        if let Err(e) = ok {
                            let msg = e
                                .downcast_ref::<String>()
                                .cloned()
                                .or_else(|| {
                                    e.downcast_ref::<&str>().map(|s| s.to_string())
                                })
                                .unwrap_or_else(|| "non-string panic".into());
                            fails.push(format!("{no}: {msg}"));
                        }
                    }
                    fails
                })
            })
            .collect();
        let mut all: Vec<String> = Vec::new();
        for h in handles {
            match h.join() {
                Ok(fails) => all.extend(fails),
                Err(_) => all.push("smoke worker thread panicked (not caught)".into()),
            }
        }
        all
    });

    std::panic::set_hook(prev_hook);

    assert!(
        failures.is_empty(),
        "{} card(s) panicked while executing their abilities:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
