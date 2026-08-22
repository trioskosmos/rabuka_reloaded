/// Q228: 園田海未 (PL!-bp5-004-R＋) — Activation costs EEEE (4) but reduced
/// by 1 per unique unit/group among your stage members.
use crate::helpers::*;
use rabuka_engine::game_setup;

/// 園田海未 (lilywhite) + multi-name card (3 unique series) → 4 groups → cost=0.
///
/// Group counting resolves multi-name joint cards through all their series
/// lines: LL-bp1-001-R＋ carries 虹ヶ咲 + Liella! + 蓮ノ空; umi adds μ's
/// → 4 distinct groups → 4E − 4×1 = 0. Zero energy given, activation
/// succeeds for free.
#[test]
fn umi_q228_four_unique_groups_cost_zero() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp5-004-R\u{ff0b}");
    let multi = game.id("LL-bp1-001-R\u{ff0b}");
    let _filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [umi, multi, -1]; // exactly 2 cards, no filler

    // Pre-check the evaluator directly: exactly 4 distinct groups on stage.
    assert_eq!(
        game.state.distinct_stage_groups("p1"),
        4,
        "umi(μ's) + joint card(虹ヶ咲/Liella!/蓮ノ空) = 4 groups"
    );

    game.activate_ability(umi);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let active = game.state.player1.energy_zone.active_count();
    eprintln!("[UMI] active energy consumed: {} (expected 0)", active);
    assert_eq!(active, 0, "Cost=0, no active energy consumed");
}

/// umi + 2 fillers — all 3 are μ's (1 group) → printed 4E − 1 = 3.
/// With only 2 active energy the activation is NOT legal (rules 9.6.2.3):
/// rejected outright, no energy spent.
#[test]
fn umi_q228_two_groups_cost_3_not_offered_at_2_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp5-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [umi, filler, filler];
    game.state.player1.hand.cards.push(filler);
    game.give_energy(2);

    assert_eq!(
        game.state.distinct_stage_groups("p1"),
        1,
        "all three members are μ's"
    );

    // Generation must withhold the unpayable activation entirely.
    let offers: Vec<_> = game_setup::generate_possible_actions(&game.state)
        .into_iter()
        .filter(|a| {
            a.action_type == game_setup::ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(umi)
        })
        .collect();
    assert!(
        offers.is_empty(),
        "cost 3 with 2 active energy must not be offered"
    );

    // A direct press is rejected as well — and spends nothing.
    let err = game.try_activate_ability(umi).unwrap_err();
    assert!(
        err.contains("cost") || err.contains("energy"),
        "expected affordability rejection, got: {}",
        err
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "rejected activation leaves energy untouched"
    );
}

/// 1 group on stage → effective cost 3; offered when affordable, and the
/// REDUCED amount (3, not printed 4) is what resolution actually charges.
#[test]
fn umi_q228_reduction_charges_effective_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp5-004-R\u{ff0b}");
    let other_group = game.id("PL!SP-bp1-007-R\u{ff0b}"); // Liella!

    // umi (μ's) + one Liella! member → 2 groups → 4E − 2 = 2.
    game.state.player1.stage.stage = [umi, other_group, -1];
    game.give_energy(5);

    assert_eq!(
        game.state.distinct_stage_groups("p1"),
        2,
        "μ's + Liella! = 2 groups"
    );

    // The generated offer shows the reduced final_cost.
    let offer = game_setup::generate_possible_actions(&game.state)
        .into_iter()
        .find(|a| {
            a.action_type == game_setup::ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(umi)
        })
        .expect("activation should be offered (effective 2 ≤ active 5)");
    assert_eq!(
        offer.parameters.as_ref().and_then(|p| p.final_cost),
        Some(2),
        "offer must display the reduced effective cost"
    );
    assert_eq!(
        offer.parameters.as_ref().and_then(|p| p.base_cost),
        Some(4),
        "printed base cost stays 4 for display"
    );

    game.activate_ability(umi);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        3,
        "resolution charges the reduced cost 2 (5 − 2 = 3), not printed 4"
    );
}
