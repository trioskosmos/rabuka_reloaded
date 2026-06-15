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

    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }

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
        Some(&"wait".to_string()),
        "Hanayo should be wait after activation cost"
    );

    // Effect had no valid targets (no other members) → no pending choice
    assert!(!game.has_pending_choice(), "No choice needed — no other members to activate");
}

/// Other members present but all active → effect still has no wait targets.
#[test]
fn q248_hanayo_activate_others_all_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp6-008-R");
    let friend = game.id("PL!-sd1-010-SD"); // abilityless filler
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }

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
        Some(&"wait".to_string()),
        "Hanayo should be wait after activation"
    );

    // Friend should remain active (was already active, no wait target found)
    assert_eq!(
        game.state.mods.get_orientation_modifier(friend),
        None,
        "Friend should still be active (no orientation modifier)"
    );

    assert!(!game.has_pending_choice(), "No choice — no wait members to activate");
}

/// Another member in wait state → normal activation: the wait member becomes active.
#[test]
fn q248_hanayo_activate_with_wait_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp6-008-R");
    let friend = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }

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
        Some(&"wait".to_string()),
        "Hanayo should be wait after activation"
    );

    // Effect: friend should now be active
    assert_eq!(
        game.state.mods.get_orientation_modifier(friend),
        Some(&"active".to_string()),
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

    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }

    game.add_to_hand(hanayo);
    game.give_energy(8);

    game.play_to_stage(hanayo, MemberArea::Center);
    game.activate_ability(hanayo); // first activation succeeds

    // Second activation should fail (use_limit=1)
    let err = game.try_activate_ability(hanayo).unwrap_err();
    assert!(
        err.contains("use_limit") || err.contains("already used") || err.contains("限界")
            || err.contains("No activatable ability"),
        "Second activation should be blocked by use_limit, got: {:?}",
        err
    );
}
