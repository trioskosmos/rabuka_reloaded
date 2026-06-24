/// Q228: 園田海未 (PL!-bp5-004-R＋) — Activation costs EEEE (4) but reduced
/// by 1 per unique unit/group among your stage members.
use crate::helpers::*;

/// 園田海未 (lilywhite) + multi-name card (3 unique series) → 4 groups → cost=0.
#[test]
fn umi_q228_four_unique_groups_cost_zero() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp5-004-R\u{ff0b}");
    let multi = game.id("LL-bp1-001-R\u{ff0b}");
    let _filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [umi, multi, -1]; // exactly 2 cards, no filler

    game.activate_ability(umi);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let active = game.state.player1.energy_zone.active_count();
    eprintln!("[UMI] active energy consumed: {} (expected 0)", active);
    assert_eq!(active, 0, "Cost=0, no active energy consumed");
}

/// umi + 2 fillers (Printemps) → 2 unique units → cost=4-2=2.
/// 2 energy → all consumed.
#[test]
fn umi_q228_two_groups_cost_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp5-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [umi, filler, filler];
    game.state.player1.hand.cards.push(filler);
    game.give_energy(2);

    game.activate_ability(umi);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let active = game.state.player1.energy_zone.active_count();
    eprintln!(
        "[UMI] active energy remaining: {} (expected 0 — spent 2)",
        active
    );
    assert_eq!(active, 0, "Active energy consumed: 2 of 2");
}

/// 1 energy given → insufficient for cost=2.
#[test]
fn umi_q228_insufficient_energy_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp5-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [umi, filler, filler];
    game.state.player1.hand.cards.push(filler);
    game.give_energy(1);

    game.activate_ability(umi);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let active = game.state.player1.energy_zone.active_count();
    eprintln!("[UMI] active energy remaining: {} (expected 1)", active);
    assert_eq!(active, 1, "No active energy spent — cost=2 > 1 available");
}
