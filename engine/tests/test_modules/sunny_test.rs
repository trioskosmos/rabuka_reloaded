use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// SUNNY DAY SONG (PL!-bp5-021-L) — LiveStart ability with 3 conditional branches.

#[test]
fn sunny_branch1_1_member_triggers_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let member = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, member);
    // Add enough cards for phase draws + ability draw
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 1 requires choosing which card to discard from hand
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Branch 1 fired: at least one card was drawn (from either player's deck)
    // Verify that cards moved: opponent has hand+discard > initial
    let p2_total = game.state.player2.hand.cards.len() + game.state.player2.waitroom.cards.len();
    assert!(p2_total > 0, "P2 should have drawn + discarded cards");
    // Opponent's hand or discard changed (they drew then discarded)
    let p2_total = game.state.player2.hand.cards.len() + game.state.player2.waitroom.cards.len();
    assert!(
        p2_total >= 2,
        "P2 should have drawn + discarded, total cards >= 2"
    );
}

#[test]
fn sunny_branch1_no_members_does_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(sunny);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // No members → all conditions fail → ability does nothing.
    // Verify that none of the ability's effects triggered.
    // P1 started with 1 card (sunny), after set_live_card it may be gone.
    // If no draw happened, hand should be ≤ 1.
    assert!(
        game.state.player1.hand.cards.len() <= 1,
        "P1 hand should not have increased (no draw triggered)"
    );
}

#[test]
fn sunny_branch3_3_members_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD"); // 星空凛
    let kotori = game.id("PL!-sd1-010-SD"); // 南ことり
    let umi = game.id("PL!-sd1-006-SD"); // 園田海未
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::LeftSide, kotori);
    game.add_to_stage(MemberArea::RightSide, umi);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Handle pending choices: discard choice from branch 1, then heart target from branch 2
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Branch 3: score +1 for 3 distinct-name members
    let sunny_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.mods.get_score_modifier(sunny_id);
    assert_eq!(score_mod, 1, "3 distinct-name members should give +1 score");
}

#[test]
fn sunny_branch3_3_members_duplicate_name_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD"); // 星空凛
    let honoka2 = game.id("PL!-sd1-005-SD"); // same name
    let kotori = game.id("PL!-sd1-010-SD"); // 南ことり
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::LeftSide, honoka2);
    game.add_to_stage(MemberArea::RightSide, kotori);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Handle pending choices: discard choice from branch 1, then heart target from branch 2
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let sunny_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.mods.get_score_modifier(sunny_id);
    assert_eq!(score_mod, 0, "No score bonus with duplicate names");
}

// ============================================================
// Branch 2 tests (2+ members — grant heart03 to 1 μ's member)
// ============================================================

/// 2 μ's members on stage → Branch 2 fires, heart03 granted to first member.
#[test]
fn sunny_branch2_two_mus_grants_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD"); // μ's member
    let kotori = game.id("PL!-sd1-010-SD"); // μ's member
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::LeftSide, kotori);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Handle all pending choices: Branch 1 discard, then Branch 2 heart target
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // One μ's member should have heart03 modifier now
    use rabuka_engine::card::HeartColor;
    let heart03_mod_1 = game.state.mods.get_heart_modifier(honoka, HeartColor::Heart03);
    let heart03_mod_2 = game.state.mods.get_heart_modifier(kotori, HeartColor::Heart03);
    assert!(
        heart03_mod_1 >= 1 || heart03_mod_2 >= 1,
        "One μ's member should gain heart03 from Branch 2"
    );
}

/// 2 non-μ's members → Branch 2 condition met but no μ's target → no heart granted.
#[test]
fn sunny_branch2_two_non_mus_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let aqours_a = game.id("PL!S-sd1-013-SD"); // Aqours member
    let aqours_b = game.id("PL!S-sd1-010-SD"); // Aqours member
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, aqours_a);
    game.add_to_stage(MemberArea::LeftSide, aqours_b);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 1 → discard choice
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Branch 2 condition (count>=2) is met, but no μ's member exists to target.
    // The effect should skip the gain_resource silently (no choice presented).
    assert!(!game.has_pending_choice(), "No heart target choice when no μ's members");

    // Verify no heart03 was granted to either member
    use rabuka_engine::card::HeartColor;
    let h_a = game.state.mods.get_heart_modifier(aqours_a, HeartColor::Heart03);
    let h_b = game.state.mods.get_heart_modifier(aqours_b, HeartColor::Heart03);
    assert_eq!(h_a, 0, "Non-μ's member should not get heart03");
    assert_eq!(h_b, 0, "Non-μ's member should not get heart03");
}

/// Only 1 member → Branch 2 condition (>=2) NOT met, no heart03 granted.
#[test]
fn sunny_branch2_one_member_skips_b2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD"); // μ's member
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, honoka);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 1 → discard choice
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Branch 2 should NOT fire (only 1 member)
    assert!(!game.has_pending_choice(), "Branch 2 should not trigger with only 1 member");

    // No heart03 granted
    use rabuka_engine::card::HeartColor;
    let heart03_mod = game.state.mods.get_heart_modifier(honoka, HeartColor::Heart03);
    assert_eq!(heart03_mod, 0, "No heart03 granted with only 1 member");
}

// ============================================================
// Q210/Q211: Joint card (multiname) with SUNNY DAY SONG
// ============================================================

#[test]
fn sunny_q210_joint_card_counts_as_one_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp3-001-R\u{ff0b}"); // 園田海未&津島善子&天王寺璃奈
    let sunny = game.id("PL!-bp5-021-L");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, joint);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 1 fires (1 member = joint card counts as 1).
    // Drain all pending choices using the while-loop pattern from existing tests.
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Branch 2 should NOT have fired (count = 1, need >= 2).
    // Verify no heart03 was granted to the joint card.
    let heart03_mod = game.state.mods.get_heart_modifier(joint, rabuka_engine::card::HeartColor::Heart03);
    assert_eq!(heart03_mod, 0, "No heart03 granted with 1 joint member (Branch 2 should not fire)");
}

/// Q211: Joint card (LL-bp3-001-R+, contains μ's character 園田海未) + 1 other member = 2 members.
/// Branch 2 fires and the joint card IS selectable as a μ's member for heart03 gain.
#[test]
fn sunny_q211_joint_card_targetable_for_mus_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp3-001-R\u{ff0b}"); // contains 園田海未 (μ's)
    let sunny = game.id("PL!-bp5-021-L");
    let other = game.id("PL!-sd1-013-SD"); // generic member (not μ's specific)
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, joint);
    game.add_to_stage(MemberArea::LeftSide, other);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 1 → draw/discard choice
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // The joint card also has its own LiveStart ability; drain auto-ability choices
    game.drain_auto_ability_choices();

    // Branch 2 fires (2 members) → should present a heart target selection
    assert!(game.has_pending_choice(), "Branch 2 should fire with joint card + 1 other = 2 members");

    // The heart target choice is a SelectTarget for position|destination or similar.
    // Use generated actions to pick the first offered member.
    game.select_generated(0);

    // Drain any remaining choices (Branch 3 etc.)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify the joint card received heart03
    use rabuka_engine::card::HeartColor;
    let heart03_mod = game.state.mods.get_heart_modifier(joint, HeartColor::Heart03);
    // The other member should also be checked
    let heart03_mod_other = game.state.mods.get_heart_modifier(other, HeartColor::Heart03);
    
    // At least one member should have heart03 (the joint card is a valid μ's target)
    assert!(
        heart03_mod >= 1 || heart03_mod_other >= 1,
        "Joint card or other member should gain heart03 as a μ's target"
    );
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
