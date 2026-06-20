/// Analysis tests for Chisato's jidou auto ability (area move → energy).
///
/// Chisato (PL!SP-bp2-003-R):
///   {{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、
///   自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
///   condition: movement=moved, movement_state=has_moved
///
/// Key question: When Chisato's auto is queued alongside another ability,
/// does the resolution order affect whether energy is gained?
use crate::helpers::*;

/// Helper: activate Kinako's kidou and select a position destination.
/// Returns the option index for the given area name.
fn select_position_option(game: &mut TestGame, area: &str) {
    let actions = game.generated_actions();
    let idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .is_some_and(|a2| a2 == area)
        })
        .unwrap_or_else(|| panic!("No '{}' position option found", area));
    game.select_generated(idx);
}

/// Helper: resolve a SelectAutoAbility by choosing the option at `option_index`.
/// `option_index` is the index within the SelectAutoAbility options list.
fn select_auto_ability_option(game: &mut TestGame, option_index: i16) {
    // For SelectAutoAbility, the result is built from card_id = option_index.
    TurnEngine::resume_with_choice(&mut game.state, Some(option_index), None)
        .expect("select_auto_ability_option failed");
}

use rabuka_engine::turn::TurnEngine;

/// SCENARIO: Playing Shiki to stage while Chisato is on stage.
/// Both Chisato's auto and Shiki's debut abilities get queued simultaneously.
///
/// Test: Choose Chisato's auto FIRST (before Shiki's debuts).
/// Expected: Chisato hasn't moved → movement condition FAIL → no energy gain.
#[test]
fn chisato_auto_first_no_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chisato = game.id("PL!SP-bp2-003-R");
    let shiki = game.id("PL!SP-bp4-008-R＋");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(shiki);
    game.state.player1.stage.stage[0] = chisato;
    game.give_energy(20);
    for _ in 0..10 {
        game.state
            .player1
            .energy_deck
            .cards
            .push(game.id("LL-E-001-SD"));
    }

    // Play Shiki to Right side → debut + TAS enqueues Chisato's auto
    game.play_to_stage(shiki, rabuka_engine::zones::MemberArea::RightSide);

    // Should have a SelectAutoAbility with 3 options:
    //   0: Shiki left debut (draw 2 discard 1)
    //   1: Shiki right debut (active 2 energy)
    //   2: Chisato auto (area move → energy)
    assert!(
        game.has_pending_choice(),
        "Expected SelectAutoAbility after playing Shiki"
    );

    // ===== ORDER: Choose Chisato's auto FIRST (option index 2) =====
    select_auto_ability_option(&mut game, 2);

    // Chisato's auto resolves: movement condition checks if Chisato moved.
    // She hasn't → FAIL → no energy.
    // After resolve, more auto abilities may be queued (Shiki's two debuts).
    // Drain remaining auto choices (they don't involve Chisato moving).
    game.drain_auto_ability_choices();

    // Assert: Chisato did NOT gain energy (she never moved)
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        20,
        "Chisato should NOT gain energy when her auto resolves before any position change"
    );
}

/// SCENARIO: Playing Shiki to stage while Chisato is on stage.
///
/// Test: Choose Shiki's debut FIRST (before Chisato's auto).
/// Expected: Shiki's debut resolves (draw 2 discard 1 or active energy).
/// Then Chisato's auto resolves. Chisato hasn't moved → movement condition
/// still FAILS. (No position change happens from debut abilities.)
#[test]
fn debut_first_chisato_auto_second_no_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chisato = game.id("PL!SP-bp2-003-R");
    let shiki = game.id("PL!SP-bp4-008-R＋");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(shiki);
    game.state.player1.stage.stage[0] = chisato;
    game.give_energy(20);
    for _ in 0..10 {
        game.state
            .player1
            .energy_deck
            .cards
            .push(game.id("LL-E-001-SD"));
    }

    game.play_to_stage(shiki, rabuka_engine::zones::MemberArea::RightSide);

    assert!(game.has_pending_choice(), "Expected SelectAutoAbility");

    // ===== ORDER: Choose Shiki's RIGHT debut first (option index 1) =====
    // Shiki's right debut: "active 2 energy" — no position change.
    select_auto_ability_option(&mut game, 1);

    // Then Shiki's left debut (draw 2 discard 1) still pending + Chisato's auto.
    // Drain remaining auto choices.
    game.drain_auto_ability_choices();

    // Assert: still no energy gain — no position change occurred
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        20,
        "No energy gain — debut abilities don't cause position change"
    );
}

/// SCENARIO: Third member (Kinako) position changes, swapping with a filler.
/// Chisato is on stage but does NOT move.
///
/// Test: Chisato's auto is queued by TAS (because Kinako's play triggers TAS).
/// When it resolves, the movement condition should FAIL because Chisato
/// didn't move.  No energy should be gained.
///
/// This test checks whether the movement condition correctly identifies
/// that the activating card (Chisato) didn't move, even though a position
/// change occurred involving other cards.
#[test]
fn third_member_position_change_no_energy_for_chisato() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chisato = game.id("PL!SP-bp2-003-R");
    let kinako = game.id("PL!SP-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(kinako);
    game.give_energy(20);
    for _ in 0..10 {
        game.state
            .player1
            .energy_deck
            .cards
            .push(game.id("LL-E-001-SD"));
    }
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Place Chisato on Center, filler on Right
    game.state.player1.stage.stage = [-1, chisato, filler];

    // Play Kinako to Left
    game.play_to_stage(kinako, rabuka_engine::zones::MemberArea::LeftSide);

    // Playing Kinako triggers TAS → Chisato's auto queued.
    // Drain it before activating Kinako's kidou.
    // But DON'T drain automatically — check what happens first.
    if game.has_pending_choice() {
        let choice = game.get_pending_choice().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                // Select the first available (whatever it is) to clear queue
                select_auto_ability_option(&mut game, 0);
                // Continue draining
                game.drain_auto_ability_choices();
            }
            _ => {
                game.select_option(0);
            }
        }
    }

    // Now activate Kinako's kidou (position change)
    game.activate_ability(kinako);

    // Should have a position destination choice
    assert!(
        game.has_pending_choice(),
        "Expected position destination choice"
    );

    // Select Right as destination (swap Kinako with filler)
    select_position_option(&mut game, "right");

    // After the swap, TAS scans and may queue Chisato's auto again.
    game.drain_auto_ability_choices();

    // Verify: Kinako is now on Right, filler is on Left
    assert_eq!(game.state.player1.stage.stage[2], kinako);
    assert_eq!(game.state.player1.stage.stage[0], filler);
    // Chisato still on Center
    assert_eq!(game.state.player1.stage.stage[1], chisato);

    // KEY: Chisato did NOT move → no energy gain
    let energy_before = 20;
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before,
        "Chisato should NOT gain energy — she didn't move"
    );
}

/// SCENARIO: Happy path — Kinako position changes WITH Chisato.
/// Chisato moves → auto triggers → energy gained.
#[test]
fn position_change_involving_chisato_grants_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako = game.id("PL!SP-bp5-006-R");
    let chisato = game.id("PL!SP-bp2-003-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(kinako);
    game.add_to_hand(chisato);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state
            .player1
            .energy_deck
            .cards
            .push(game.id("LL-E-001-SD"));
    }
    game.give_energy(20);

    game.play_to_stage(chisato, rabuka_engine::zones::MemberArea::LeftSide);
    game.play_to_stage(kinako, rabuka_engine::zones::MemberArea::RightSide);

    assert_eq!(game.state.player1.stage.stage[0], chisato);
    assert_eq!(game.state.player1.stage.stage[2], kinako);

    // Drain any auto abilities queued from play_to_stage TAS
    game.drain_auto_ability_choices();

    game.activate_ability(kinako);
    assert!(
        game.has_pending_choice(),
        "Expected position destination choice"
    );

    select_position_option(&mut game, "left");
    game.drain_auto_ability_choices();

    // Verify swap
    assert_eq!(game.state.player1.stage.stage[0], kinako);
    assert_eq!(game.state.player1.stage.stage[2], chisato);

    // Chisato moved → energy gained
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        21,
        "Chisato should gain energy when she moves"
    );
}
