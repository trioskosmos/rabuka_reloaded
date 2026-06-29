/// Edge case tests for continuous (常時) abilities.
///
/// Tests complex compound conditions, cross-player conditions,
/// distinct-name checks, and conditional score modifications.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Target: PL!S-bp2-001-R (Riko)
// 常時: If own success_live_card_zone has 0 cards AND opponent has 1+,
//       gain 3 blade.
// So: compound(AND) condition with own zone count and opponent zone count.
// ====================================================================

/// Edge case: Condition fully met → gain 3 blade.
#[test]
fn riko_compound_and_condition_both_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let riko = game.id("PL!S-bp2-001-R");
    let opponent_live = game.id("PL!-sd1-019-SD");

    // Put riko on center stage
    game.add_to_stage(MemberArea::Center, riko);

    // Opponent has 1+ success live card
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(opponent_live);

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(riko);
    assert_eq!(
        blade_mod, 3,
        "Riko: own=0, opponent>=1 → should gain 3 blade, got {}",
        blade_mod
    );
}

/// Edge case: Condition NOT met (own has cards) → gain 0.
#[test]
fn riko_compound_and_own_has_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let riko = game.id("PL!S-bp2-001-R");
    let live_card = game.id("PL!-sd1-019-SD");

    game.add_to_stage(MemberArea::Center, riko);

    // Own success zone has a card (violates condition)
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(live_card);

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(riko);
    assert_eq!(
        blade_mod, 0,
        "Riko: own has card → condition not met, blade should be 0, got {}",
        blade_mod
    );
}

/// Edge case: Condition PARTIALLY met (own=0, opponent=0) → gain 0.
#[test]
fn riko_compound_and_opponent_empty() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let riko = game.id("PL!S-bp2-001-R");
    game.add_to_stage(MemberArea::Center, riko);

    // Neither player has success cards
    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(riko);
    assert_eq!(
        blade_mod, 0,
        "Riko: both empty → condition not met, blade should be 0, got {}",
        blade_mod
    );
}

/// Edge case: Condition goes from met → unmet when opponent's cards are removed.
#[test]
fn riko_compound_and_condition_removal() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let riko = game.id("PL!S-bp2-001-R");
    let opponent_live = game.id("PL!-sd1-019-SD");

    game.add_to_stage(MemberArea::Center, riko);
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(opponent_live);

    // Condition met
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(riko), 3);

    // Remove opponent card → condition no longer met
    game.state.player2.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(riko);
    assert_eq!(
        blade_mod, 0,
        "Riko: opponent card removed → blade should be 0, got {}",
        blade_mod
    );
}

// ====================================================================
// Target: PL!-bp5-003-R+ (Honoka)
// 常時: If 3+ members with DISTINCT names on your stage → gain heart03.
// So: location_condition with distinct=true, count=3, operator=>=
// ====================================================================

/// Edge case: Exactly 3 members with distinct names → gain 1 heart03.
#[test]
fn honoka_distinct_names_three_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-bp5-003-R+");
    let member2 = game.id("PL!-sd1-001-SD");
    let member3 = game.id("PL!-sd1-002-SD");

    // Fill all three stage areas with different-named members
    game.add_to_stage(MemberArea::LeftSide, honoka);
    game.add_to_stage(MemberArea::Center, member2);
    game.add_to_stage(MemberArea::RightSide, member3);

    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(honoka, HeartColor::Heart03);
    assert_eq!(
        heart_mod, 1,
        "Honoka: 3 distinct-named members → +1 heart03, got {}",
        heart_mod
    );
}

/// Edge case: Only 2 distinct members → gain 0.
#[test]
fn honoka_distinct_names_two_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-bp5-003-R+");
    let member2 = game.id("PL!-sd1-002-SD");

    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::RightSide, member2);

    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(honoka, HeartColor::Heart03);
    assert_eq!(
        heart_mod, 0,
        "Honoka: only 2 distinct members → no heart, got {}",
        heart_mod
    );
}

/// Edge case: 3 members but 2 have the same name (not distinct) → gain 0.
#[test]
fn honoka_distinct_names_duplicate_name() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-bp5-003-R+");
    let same_as_honoka = game.new_id("PL!-bp5-003-R+");
    let member3 = game.id("PL!-sd1-002-SD");

    game.add_to_stage(MemberArea::LeftSide, honoka);
    game.add_to_stage(MemberArea::Center, same_as_honoka);
    game.add_to_stage(MemberArea::RightSide, member3);

    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(honoka, HeartColor::Heart03);
    assert_eq!(
        heart_mod, 0,
        "Honoka: 2 copies of same member → not 3 distinct names, got {}",
        heart_mod
    );
}

/// Edge case: Stage becomes empty → recalculate from 3→0 members.
#[test]
fn honoka_distinct_names_member_removed() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-bp5-003-R+");
    let member2 = game.id("PL!-sd1-001-SD");
    let member3 = game.id("PL!-sd1-002-SD");

    game.add_to_stage(MemberArea::LeftSide, honoka);
    game.add_to_stage(MemberArea::Center, member2);
    game.add_to_stage(MemberArea::RightSide, member3);

    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(honoka, HeartColor::Heart03),
        1
    );

    // Remove one member → only 2 distinct
    game.state.player1.stage.stage[0] = -1;
    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(honoka, HeartColor::Heart03);
    assert_eq!(
        heart_mod, 0,
        "Honoka: one member removed → only 2 distinct, no heart, got {}",
        heart_mod
    );
}

// ====================================================================
// Target: PL!N-bp4-012-P (Umi)
// 常時: If opponent's success_live_card_zone total score >= 6 → +1 score.
// So: comparison_condition with aggregate=total, comparison_type=score, operator=>=, count=6
// ====================================================================

/// Edge case: Opponent score exactly 6 → +1 score mod.
#[test]
fn umi_opponent_score_exactly_six() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let umi = game.id("PL!N-bp4-012-P");
    let live_6 = game.id("PL!SP-bp1-027-L");

    game.add_to_stage(MemberArea::Center, umi);

    // Single live card with score=6
    game.state.player2.success_live_card_zone.cards.push(live_6);

    game.state.recalculate_constants();

    let score_mod = game.state.mods.get_score_modifier(umi);
    assert_eq!(
        score_mod, 1,
        "Umi: opponent score >=6 → +1 score mod, got {}",
        score_mod
    );
}

/// Edge case: Opponent score < 6 → no score mod.
#[test]
fn umi_opponent_score_below_six() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let umi = game.id("PL!N-bp4-012-P");
    game.add_to_stage(MemberArea::Center, umi);

    // No success live cards → score = 0
    game.state.recalculate_constants();

    let score_mod = game.state.mods.get_score_modifier(umi);
    assert_eq!(
        score_mod, 0,
        "Umi: opponent score 0 → no score mod, got {}",
        score_mod
    );
}

/// Edge case: Score goes from >=6 to <6 when cards are removed.
#[test]
fn umi_opponent_score_drops_below_threshold() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let umi = game.id("PL!N-bp4-012-P");
    let live_6 = game.id("PL!SP-bp1-027-L");

    game.add_to_stage(MemberArea::Center, umi);
    game.state.player2.success_live_card_zone.cards.push(live_6);

    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_score_modifier(umi), 1);

    // Remove opponent's cards
    game.state.player2.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();

    let score_mod = game.state.mods.get_score_modifier(umi);
    assert_eq!(
        score_mod, 0,
        "Umi: opponent cards removed → 0 score mod, got {}",
        score_mod
    );
}

// ====================================================================
// Target: PL!N-bp4-007-R+ (Hanayo)
// 常時: If combined total energy of both players >= 15 → gain heart02 x2.
// So: card_count_condition across BOTH players.
// ====================================================================

/// Edge case: Combined energy exactly 15 → gain 2x heart02.
#[test]
fn hanayo_combined_energy_exactly_fifteen() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hanayo = game.id("PL!N-bp4-007-R+");
    let energy = game.id("LL-E-001-SD");

    game.add_to_stage(MemberArea::Center, hanayo);

    // P1: 10 energy, P2: 5 energy = 15 total
    let e1 = game.state.player1.energy_zone.active_count();
    game.state.player1.energy_zone.set_active_count(e1.min(10));
    for _ in 0..10 {
        game.state.player1.energy_zone.cards.push(energy);
    }
    for _ in 0..5 {
        game.state.player2.energy_zone.cards.push(energy);
    }

    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(hanayo, HeartColor::Heart02);
    assert_eq!(
        heart_mod, 2,
        "Hanayo: combined energy=15 → +2 heart02, got {}",
        heart_mod
    );
}

/// Edge case: Combined energy 14 → no heart gain.
#[test]
fn hanayo_combined_energy_below_fifteen() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hanayo = game.id("PL!N-bp4-007-R+");
    let energy = game.id("LL-E-001-SD");

    game.add_to_stage(MemberArea::Center, hanayo);

    for _ in 0..10 {
        game.state.player1.energy_zone.cards.push(energy);
    }
    for _ in 0..4 {
        game.state.player2.energy_zone.cards.push(energy);
    }

    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(hanayo, HeartColor::Heart02);
    assert_eq!(
        heart_mod, 0,
        "Hanayo: combined energy=14 → no heart, got {}",
        heart_mod
    );
}

// ====================================================================
// Target: PL!SP-bp1-001-P (Kanon)
// 常時: If no other members on your stage → cannot live.
// So: restriction with condition: location_condition(negation, exclude_self)
// ====================================================================

/// Edge case: Kanon is alone on stage → cannot_live restriction.
#[test]
fn kanon_alone_cannot_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id("PL!SP-bp1-001-P");

    // Kanon alone on stage (no other members)
    game.add_to_stage(MemberArea::Center, kanon);

    game.state.recalculate_constants();

    assert!(
        game.state.is_action_prohibited("cannot_live"),
        "Kanon alone: cannot_live restriction should be active"
    );
}

/// Edge case: Kanon with another member → no restriction.
#[test]
fn kanon_with_other_member_no_restriction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id("PL!SP-bp1-001-P");
    let other = game.id("PL!-sd1-002-SD");

    game.add_to_stage(MemberArea::Center, kanon);
    game.add_to_stage(MemberArea::LeftSide, other);

    game.state.recalculate_constants();

    assert!(
        !game.state.is_action_prohibited("cannot_live"),
        "Kanon with other: no restriction expected"
    );
}

// ====================================================================
// Target: PL!S-pb1-005-R (You)
// 常時: 相手のエネルギーが自分より多い場合、ブレードを3得る。
// Condition: comparison_condition, resource_type=energy, operator=>
//            target=opponent, comparison_target=self
// Previously broken: comparison_target="self" with resource_type="energy"
// returned 0 instead of self's energy count.
// ====================================================================

#[allow(dead_code)]
fn you_pb1_has_blade(gs: &rabuka_engine::game_state::GameState, cid: i16) -> bool {
    gs.mods.get_blade_modifier(cid) > 0
}

fn setup_you_pb1(game: &mut TestGame, p1_energy: usize, p2_energy: usize) -> i16 {
    let you = game.id("PL!S-pb1-005-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_stage(MemberArea::Center, you);
    for _ in 0..p1_energy {
        game.state.player1.energy_zone.cards.push(filler);
    }
    game.state.player1.energy_zone.set_active_count(p1_energy);
    for _ in 0..p2_energy {
        game.state.player2.energy_zone.cards.push(filler);
    }
    game.state.player2.energy_zone.set_active_count(p2_energy);
    you
}

#[test]
fn you_pb1_opponent_more_energy_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let you = setup_you_pb1(&mut game, 3, 7);
    game.state.recalculate_constants();
    assert!(
        you_pb1_has_blade(&game.state, you),
        "You: opponent 7 > self 3 → should gain blade"
    );
    assert_eq!(game.state.mods.get_blade_modifier(you), 3);
}

#[test]
fn you_pb1_opponent_equal_energy_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let you = setup_you_pb1(&mut game, 5, 5);
    game.state.recalculate_constants();
    assert!(
        !you_pb1_has_blade(&game.state, you),
        "You: opponent 5 == self 5 → no blade"
    );
}

#[test]
fn you_pb1_opponent_less_energy_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let you = setup_you_pb1(&mut game, 8, 3);
    game.state.recalculate_constants();
    assert!(
        !you_pb1_has_blade(&game.state, you),
        "You: opponent 3 < self 8 → no blade"
    );
}

#[test]
fn you_pb1_energy_changes_dynamically() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let filler = game.id("PL!-sd1-010-SD");
    let you = setup_you_pb1(&mut game, 3, 7);

    // Opponent has more → blade
    game.state.recalculate_constants();
    assert!(you_pb1_has_blade(&game.state, you));

    // Now P1 gains more energy to surpass opponent → blade removed
    for _ in 0..10 {
        game.state.player1.energy_zone.cards.push(filler);
    }
    game.state.player1.energy_zone.set_active_count(13);
    game.state.recalculate_constants();
    assert!(
        !you_pb1_has_blade(&game.state, you),
        "You: self 13 > opponent 7 → blade should be removed"
    );

    // Later opponent gains even more energy → blade returns
    for _ in 0..10 {
        game.state.player2.energy_zone.cards.push(filler);
    }
    game.state.player2.energy_zone.set_active_count(17);
    game.state.recalculate_constants();
    assert!(
        you_pb1_has_blade(&game.state, you),
        "You: opponent 17 > self 13 → blade should return"
    );
    assert_eq!(game.state.mods.get_blade_modifier(you), 3);
}
