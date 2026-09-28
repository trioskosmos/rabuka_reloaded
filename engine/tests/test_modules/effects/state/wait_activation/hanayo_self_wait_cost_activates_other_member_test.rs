/// Tests for PL!-bp6-008-R (小泉花陽) — Activation ability with wait/no-wait edge cases
///
/// Ability: 起動 ターン1回 このメンバーをウェイトにする：
///          自分のステージにいるほかのメンバー1人をアクティブにする。
///
/// Cost: self_cost change_state to wait
/// Effect: change_state to active on another member (count=1, exclude_self)
///
/// Q248: "ステージにウェイト状態のメンバーがいない状態でも、
///        起動を使うことはできますか？"
/// A: はい。できます。
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Q248 main case: No wait members on stage (in fact, no other members at all).
/// Activation should succeed, cost is paid (self → wait), effect finds no targets → no-op.
#[test]
fn q248_hanayo_activate_no_other_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp6-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.add_to_hand(hanayo);
    game.give_energy(8); // cost 7 + 1

    // Play to Center (no other members on stage)
    game.play_to_stage(hanayo, MemberArea::Center);
    assert!(!game.has_pending_choice(), "Hanayo has no debut ability");

    // Activate — should succeed even with no other members
    game.activate_ability(hanayo);

    // Cost was paid: Hanayo is now wait
    assert_eq!(
        game.state.mods.get_orientation_modifier(hanayo),
        Some("wait"),
        "Hanayo should be wait after activation cost"
    );

    // Effect had no valid targets (no other members) → no pending choice
    assert!(
        !game.has_pending_choice(),
        "No choice needed — no other members to activate"
    );
}

/// Other members present but all active → effect still has no wait targets.
#[test]
fn q248_hanayo_activate_others_all_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp6-008-R");
    let friend = game.id("PL!-sd1-010-SD"); // abilityless filler
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.add_to_hand(hanayo);
    game.give_energy(8);

    // Place friend on stage (active by default)
    game.state.player1.stage.stage = [friend, -1, -1];
    game.add_to_stage(MemberArea::Center, hanayo);

    // Activate
    game.activate_ability(hanayo);

    // Cost paid
    assert_eq!(
        game.state.mods.get_orientation_modifier(hanayo),
        Some("wait"),
        "Hanayo should be wait after activation"
    );

    // Friend should remain active (was already active, no wait target found)
    assert_eq!(
        game.state.mods.get_orientation_modifier(friend),
        None,
        "Friend should still be active (no orientation modifier)"
    );

    assert!(
        !game.has_pending_choice(),
        "No choice — no wait members to activate"
    );
}

/// Another member in wait state → normal activation: the wait member becomes active.
#[test]
fn q248_hanayo_activate_with_wait_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp6-008-R");
    let friend = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.add_to_hand(hanayo);
    game.give_energy(8);

    // Place friend on stage in wait state
    game.state.player1.stage.stage = [friend, -1, -1];
    game.state.mods.add_orientation_modifier(friend, "wait");
    game.add_to_stage(MemberArea::Center, hanayo);

    // Activate — should find friend as valid target
    game.activate_ability(hanayo);

    // Cost paid: Hanayo becomes wait
    assert_eq!(
        game.state.mods.get_orientation_modifier(hanayo),
        Some("wait"),
        "Hanayo should be wait after activation"
    );

    // Effect: friend should now be active
    assert_eq!(
        game.state.mods.get_orientation_modifier(friend),
        Some("active"),
        "Friend should be activated by the effect"
    );

    assert!(!game.has_pending_choice(), "No remaining choices");
}

/// Use limit: cannot activate twice in one turn.
#[test]
fn q248_hanayo_use_limit_blocks_second_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp6-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // 葉乃's 起動 makes ANOTHER member active, so it needs a second member to
    // target. Without one, the second activation was refused for want of a
    // target — not because of the use_limit — and the old four-way disjunction
    // accepted "No activatable ability" as a pass. With a real target present,
    // the only thing that can refuse the second activation is the ターン1回.
    game.state.player1.stage.stage = [
        hanayo,
        game.new_id("PL!-sd1-002-SD"),
        -1,
    ];

    game.add_to_hand(hanayo);
    game.give_energy(8);

    game.play_to_stage(hanayo, MemberArea::Center);
    game.activate_ability(hanayo); // first activation succeeds

    // The first activation asks WHICH member to make active. That choice must be
    // answered before a second activation can be attempted at all — while it is
    // pending, every activation is refused with "Cannot activate ability while
    // another choice is pending", which is yet a third reason this test could
    // pass without touching the use_limit.
    let mut guard = 0;
    while game.has_pending_choice() {
        guard += 1;
        assert!(guard < 10, "runaway choice prompts after the first activation");
        game.select_indices(&[0]);
    }

    // The first activation must have COMPLETED before the refusal below means
    // anything. Its cost is 「このメンバーをウェイトにする」, so 葉乃 herself
    // sitting in wait is this codebase's idiom for "the activation resolved and
    // paid" — the same check `q248_hanayo_activate_no_other_members` uses.
    assert_eq!(
        game.state.mods.get_orientation_modifier(hanayo),
        Some("wait"),
        "the first 起動 must have completed (its self→wait cost paid) — otherwise \
         the second activation's refusal is not evidence of the use_limit"
    );

    // Second activation is refused: the 起動 is ターン1回 and is already spent.
    //
    // Three things were wrong with this test before, and all three let it pass
    // without ever reaching the use_limit:
    //   1. the stage held no second member, so the first 起動 (which makes
    //      ANOTHER member active) had no target;
    //   2. the first activation's "which member?" choice was never answered, so
    //      every later activation was refused with "another choice is pending";
    //   3. the assertion accepted any of four error strings, including
    //      "No activatable ability" — which is not a use_limit message at all.
    //
    // The engine reports an exhausted 起動 as the GENERIC "No activatable ability
    // found for this card at its current location": `find_gained_activation`
    // `continue`s past an ability that is `ability_under_use_limit`-exhausted
    // (turn/actions/mod.rs:521-527), and when nothing qualifies the `.ok_or(..)`
    // at :546 surfaces that message. So the refusal string cannot distinguish
    // "used up" from "no target" — what makes the refusal meaningful is the
    // precondition above: a target existed, the first activation demonstrably
    // took effect, and so the only thing left to refuse the second is the limit.
    let err = game
        .try_activate_ability(hanayo)
        .expect_err("the second activation must be refused once the 起動 is ターン1回");
    assert!(
        !err.is_empty(),
        "the refusal should carry the engine's reason; got an empty error. \
         (For a used-up 起動 that reason is the generic 'No activatable ability \
         found ...' — see the note above.)"
    );
}

#[test]
fn q248_hanayo_p_activate_no_other_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-bp6-008-P");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.add_to_hand(hanayo);
    game.give_energy(8);
    game.play_to_stage(hanayo, MemberArea::Center);
    game.activate_ability(hanayo);

    assert_eq!(game.state.mods.get_orientation_modifier(hanayo), Some("wait"));
    assert!(!game.has_pending_choice());
}
