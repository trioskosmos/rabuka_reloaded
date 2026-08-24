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

    let mut failures: Vec<String> = Vec::new();
    for no in &card_nos {
        let ok = catch_unwind(AssertUnwindSafe(|| smoke_one_card(&db, no)));
        if let Err(e) = ok {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "non-string panic".into());
            failures.push(format!("{no}: {msg}"));
        }
    }

    std::panic::set_hook(prev_hook);

    assert!(
        failures.is_empty(),
        "{} card(s) panicked while executing their abilities:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
