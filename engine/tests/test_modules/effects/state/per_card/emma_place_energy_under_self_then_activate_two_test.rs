/// Q215: Emma Verde (PL!N-bp5-008-R) — Activation: place 1 energy under member,
/// then activate 2 energy. Wait-state energy CAN be placed.
///
/// Hard edge cases:
/// 1. Wait-position energy placed → activation still works
/// 2. Only wait energy in zone → still works
///
/// The ability is two effects: a cost that moves 1 energy UNDER the member, and
/// an effect that activates 2. The tests assert both halves, because a
/// card-count assertion alone cannot tell "the cost paid and 2 activated" from
/// "the cost paid and the activation did nothing".
use crate::helpers::*;

/// How many energy cards sit under `member_id`, and how many of the zone's
/// cards are active.
fn under_and_active(game: &TestGame, member_id: i16) -> (usize, usize) {
    let p = &game.state.player1;
    let slot = p.stage.stage.iter().position(|&id| id == member_id);
    let under = slot.map(|i| p.stage.under_cards[i].len()).unwrap_or(0);
    (under, p.energy_zone.active_count() as usize)
}

/// The activation half only does anything when there IS wait energy, so this
/// stages 2 active + 3 wait and asserts both effects: 1 under Emma, 2 flipped.
///
/// This used to be `give_energy(5)`, which pushes all 5 as ACTIVE — so the
/// "activate 2" effect had nothing to do and the test asserted only a card
/// count. It passed while exercising half the ability.
#[test]
fn emma_bp5_q215_wait_energy_placed_then_activate() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);
    // Active prefix, wait suffix: leave 3 in the wait state.
    game.state.player1.energy_zone.set_active_count(2);
    let active_before = game.state.player1.energy_zone.active_count() as usize;
    assert_eq!(active_before, 2, "precondition: 2 active, 3 wait");

    game.activate_ability(emma);
    // select_energy_from_zone(n) answers indices 0..n, so the cost always takes
    // from the front — and zone order is active-prefix, so that is an ACTIVE
    // card. 2 active - 1 paid + 2 flipped = 3 active at the end.
    game.select_energy_from_zone(1);

    // The "activate 2" effect resolves AUTOMATICALLY (ChangeState on active
    // energies) — no further selection prompt appears.
    assert!(
        !game.has_pending_choice(),
        "activate-2-energy effect must auto-resolve without prompting"
    );

    let (under, active_after) = under_and_active(&game, emma);
    assert_eq!(under, 1, "the cost placed exactly 1 energy under Emma");
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        4,
        "the cost removed 1 energy from the zone"
    );
    assert_eq!(
        active_after, 3,
        "2 active, minus the 1 the cost took, plus the 2 the effect flipped. \
         This was previously asserted only as a card count, against an \
         all-active zone where the effect had nothing to do — so it passed \
         while exercising half the ability."
    );
}

/// Every energy already active: the card reads 「エネルギーを2枚アクティブに
/// する」 with no 「まで」 on it, so whether it may do nothing is genuinely
/// ambiguous. Pinned as-is: 0 are flipped. If that is ever decided the other
/// way, this is the test that says so.
#[test]
fn emma_bp5_q215_all_energy_already_active_activates_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5); // all active

    game.activate_ability(emma);
    game.select_energy_from_zone(1);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let (under, active_after) = under_and_active(&game, emma);
    assert_eq!(under, 1, "the cost still placed 1 energy under Emma");
    assert_eq!(
        active_after, 4,
        "no wait energy existed, so none became active. The zone is one \
         shorter because of the cost. Ambiguous against the card text — see \
         the doc comment."
    );
}

/// Only 1 energy in the zone, and it is ACTIVE (there is no wait energy to
/// activate). The cost pays, then the activation has nothing to do.
#[test]
fn emma_bp5_q215_only_wait_energy_available() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.hand.cards.push(filler);
    game.give_energy(1); // 1 active energy, 0 wait

    // The cost takes the only energy; the activation then has nothing to
    // activate, which must not panic and must not re-prompt forever.
    game.activate_ability(emma);
    let mut guard = 0;
    while game.has_pending_choice() {
        guard += 1;
        assert!(guard <= 5, "runaway energy-selection prompts");
        assert_eq!(
            game.pending_choice_type().as_deref(),
            Some("SelectCard"),
            "expected SelectCard energy-zone prompt"
        );
        game.select_indices(&[0]);
    }

    let (under, _) = under_and_active(&game, emma);
    assert_eq!(under, 1, "the single energy went under Emma as the cost");
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        0,
        "the zone is empty: the cost consumed its only energy and the \
         activation had none left to flip"
    );
}

/// Zero energy in zone → cost cannot be paid → ability fails silently.
#[test]
fn emma_bp5_q215_no_energy_cost_fails_gracefully() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.hand.cards.push(filler);
    // No energy given

    let result = game.try_activate_ability(emma);
    // Cost succeeds (place 0 energy under member), effect fails silently
    assert!(
        result.is_ok(),
        "Activation with 0 energy should not panic/crash"
    );
    // State unchanged — no energy to remove, no cards moved
    assert_eq!(
        game.state.player1.stage.stage[1], emma,
        "Emma should remain on stage after activation with 0 energy"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        0,
        "Energy zone should still be empty after activation with 0 energy"
    );
}
