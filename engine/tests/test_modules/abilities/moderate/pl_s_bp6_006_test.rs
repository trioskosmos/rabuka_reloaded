use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn blade_count(v: &TestGame, cid: i16) -> i32 {
    v.state
        .mods
        .blade_modifiers
        .get(&cid)
        .map(|e| e.total())
        .unwrap_or(0)
}

fn drain_auto(v: &mut TestGame) {
    while v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => v.select_indices(&[0]),
            _ => v.select_indices(&[]),
        }
    }
}

/// 006 played from hand → draws 2, NO blade gain
#[test]
fn yoshiko_006_played_from_hand_no_blade() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let yoshiko = v.id("PL!S-bp6-006-R");
    let filler = v.id("PL!-sd1-010-SD");

    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(yoshiko);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(filler);
    }
    v.give_energy(20);

    v.play_to_stage(yoshiko, MemberArea::Center);
    drain_auto(&mut v);

    assert_eq!(v.state.player1.hand.cards.len(), 2, "Should have drawn 2");
    assert_eq!(blade_count(&v, yoshiko), 0, "No blade from hand play");
}

/// 006 revived from discard via 008 → draws 2, gains 3 blade
#[test]
fn yoshiko_006_revived_from_discard_gains_blade() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let yoshiko = v.id("PL!S-bp6-006-R");
    let mari = v.id("PL!S-bp6-008-R");
    let filler = v.id("PL!-sd1-010-SD");

    v.state.player1.hand.cards.clear();
    v.state.player1.waitroom.cards.clear();
    v.state.player1.stage.stage = [mari, -1, -1];
    v.state.player1.waitroom.cards.push(yoshiko);
    v.give_energy(20);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(filler);
    }

    // Starts activation, pays costs, but MoveCards needs a card selection choice
    v.activate_ability(mari);

    // Handle "select card from discard" choice
    assert!(
        v.has_pending_choice(),
        "MoveCards discard-selection prompt expected after 008 activation"
    );
    assert_eq!(
        v.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (waiting room, count=1, no skip)"
    );
    // Find yoshiko in waitroom after 008's cost moved 008 there too
    let idx = v
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == yoshiko)
        .unwrap();
    v.select_indices(&[idx]);
    // After selection, the MoveCards effect places 006 on stage
    // Then 006's debut triggers
    drain_auto(&mut v);

    assert!(
        v.state.player1.waitroom.cards.contains(&mari),
        "008 in discard"
    );
    assert!(
        v.state.player1.stage.stage.contains(&yoshiko),
        "006 should be on stage"
    );

    // Debut already processed: drew 2, gained 3 blade
    assert_eq!(v.state.player1.hand.cards.len(), 2, "Should have drawn 2");
    assert_eq!(
        blade_count(&v, yoshiko),
        3,
        "Should gain 3 blade from discard"
    );
}
