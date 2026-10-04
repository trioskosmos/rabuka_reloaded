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
fn distinct_group_activation_discount_q228_four_unique_groups_cost_zero() {
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

    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    let active = game.state.player1.energy_zone.active_count();
    assert_eq!(active, 0, "Cost=0, no active energy consumed");
}

#[test]
fn distinct_group_activation_discount_q228_zero_cost_all_rarities() {
    for suffix in ["AR", "P", "R＋", "SEC"] {
        let db = load_real_database();
        let mut game = TestGame::new(db);
        let card_no = format!("PL!-bp5-004-{suffix}");
        let umi = game.id(&card_no);
        let multi = game.id("LL-bp1-001-R＋");

        game.state.player1.stage.stage = [umi, multi, -1];
        assert_eq!(game.state.distinct_stage_groups("p1"), 4);
        game.activate_ability(umi);
        // Answer what cannot be declined, decline what can: a mandatory
        // prompt left parked mid-ability produces exactly the absence
        // the next assertion checks, so nothing fails.
        game.drain_choices();

        assert_eq!(
            game.state.player1.energy_zone.active_count(),
            0,
            "{card_no}: four groups should reduce the cost to zero"
        );
    }
}

/// umi + 2 fillers — all 3 are μ's (1 group) → printed 4E − 1 = 3.
/// With only 2 active energy the activation is NOT legal (rules 9.6.2.3):
/// rejected outright, no energy spent.
#[test]
fn distinct_group_activation_discount_q228_one_group_cost_three_not_offered_at_two_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp5-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    let filler_copy2 = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [umi, filler, filler_copy2];
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
fn distinct_group_activation_discount_q228_reduction_charges_effective_cost() {
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
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        3,
        "resolution charges the reduced cost 2 (5 − 2 = 3), not printed 4"
    );
}

/// The discount counts MY stage only: 「自分のステージにいるメンバーの中の
/// グループ名1種類につき」.
///
/// Every other test in this file varies player 1's stage and leaves player 2's
/// alone, so nothing here can tell 「自分のステージ」 from 「両者のステージ」 — the
/// opponent's members never appear in a discount assertion at all. Since the
/// opponent's board is exactly where this ability does its waiting (「相手の
/// ステージにいるコスト10以下のメンバー」), it is the natural place for a stage read
/// to reach by mistake.
///
/// The shape is chosen so the two readings disagree in the strongest possible way.
/// Player 1 holds ONE member, so the correct cost is 4E − 1 = 3 against 2 energy:
/// not affordable. If the opponent's groups were counted too, three distinct
/// groups on player 2 would take the cost to 0 and the activation would be offered
/// — which is what this asserts it is not.
#[test]
fn distinct_group_activation_discount_q228_ignores_the_opponents_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp5-004-R＋"); // μ's — the only card on MY stage
    let liella = game.id("PL!SP-bp1-007-R＋");
    let niji = game.id("PL!N-bp1-012-R＋");
    let joint = game.id("LL-bp1-001-R＋"); // 虹ヶ咲 + Liella! + 蓮ノ空

    game.state.player1.stage.stage = [umi, -1, -1];
    game.state.player2.stage.stage = [liella, niji, joint];

    assert_eq!(
        game.state.distinct_stage_groups("p1"),
        1,
        "precondition: my stage holds a single group"
    );
    assert!(
        game.state.distinct_stage_groups("p2") >= 3,
        "precondition: the opponent's stage is the one with group variety in it — \
         got {}",
        game.state.distinct_stage_groups("p2")
    );

    game.give_energy(2); // one short of the correct cost of 3

    let offered = game_setup::generate_possible_actions(&game.state)
        .into_iter()
        .any(|a| {
            a.action_type == game_setup::ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(umi)
        });
    assert!(
        !offered,
        "cost is 4E − 1 (my stage only) = 3, which 2 energy cannot pay. The \
         opponent's {} groups must not discount it",
        game.state.distinct_stage_groups("p2")
    );

    // And the press is refused without spending anything, so the check above is
    // not merely about which offers get generated.
    let err = game.try_activate_ability(umi).unwrap_err();
    assert!(
        err.contains("cost") || err.contains("energy"),
        "expected affordability rejection, got: {err}"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "a rejected activation leaves the energy untouched"
    );
}

/// The same board with the opponent's stage emptied — the discount is identical.
///
/// Without this, the test above could pass for the wrong reason: an implementation
/// that ignored BOTH stages would also decline to offer the activation. Together
/// the two say the number comes from my stage and from nothing else.
#[test]
fn distinct_group_activation_discount_q228_same_cost_with_an_empty_opponent_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp5-004-R＋");
    let joint = game.id("LL-bp1-001-R＋");

    game.state.player1.stage.stage = [umi, joint, -1]; // μ's + 3 groups = 4 → cost 0
    game.state.player2.stage.stage = [-1, -1, -1];

    assert_eq!(game.state.distinct_stage_groups("p1"), 4);
    assert_eq!(game.state.distinct_stage_groups("p2"), 0);

    let offer = game_setup::generate_possible_actions(&game.state)
        .into_iter()
        .find(|a| {
            a.action_type == game_setup::ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(umi)
        })
        .expect("four of my own groups reduce the cost to zero");
    assert_eq!(
        offer.parameters.as_ref().and_then(|p| p.final_cost),
        Some(0),
        "the discount comes from my stage alone"
    );
}

#[test]
fn distinct_group_activation_selects_one_eligible_opponent_physical_instance() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-bp5-004-R\u{ff0b}");
    let ally = game.id("PL!-sd1-010-SD");
    let target_a = game.id("PL!HS-pb1-002-R");
    let target_b = game.new_id("PL!HS-pb1-002-R");
    let too_expensive = game.id("PL!S-bp5-009-R");
    assert_ne!(target_a, target_b);
    assert_eq!(
        game.db.get_card(target_a).unwrap().card_no,
        game.db.get_card(target_b).unwrap().card_no
    );

    game.state.player1.stage.stage = [ally, umi, -1];
    game.state.player2.stage.stage = [target_a, target_b, too_expensive];
    game.give_energy(3);

    game.activate_ability(umi);
    assert!(game.has_pending_choice());
    game.select_indices(&[1]);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    assert_eq!(
        game.state.mods.get_orientation_modifier(target_b),
        Some("wait")
    );
    assert_ne!(
        game.state.mods.get_orientation_modifier(target_a),
        Some("wait")
    );
    assert_ne!(
        game.state.mods.get_orientation_modifier(too_expensive),
        Some("wait")
    );
    assert!(game.state.player2.stage.stage.contains(&target_b));
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
}
