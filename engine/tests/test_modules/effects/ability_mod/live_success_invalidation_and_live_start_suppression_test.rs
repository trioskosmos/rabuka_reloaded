use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::game_state::Phase;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn fill_both_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn give_opponent_energy_cards(game: &mut TestGame, count: usize) {
    let energy = game.id("LL-E-001-SD");
    game.state.player2.energy_deck.cards.clear();
    for _ in 0..count.max(1) {
        game.state.player2.energy_deck.cards.push(energy);
    }
}

/// Pass into `target` and return both players' active energy counts as they
/// were on the last step BEFORE entering it. Snapshotting early (e.g. right
/// after the live card is set) charges any intervening Energy-phase top-up to
/// whatever fired inside `target`.
fn pass_into_phase_capturing_energy(game: &mut TestGame, target: Phase) -> (i32, i32) {
    let mut last = (0i32, 0i32);
    for _ in 0..16 {
        if game.state.current_phase == target {
            return last;
        }
        last = (
            game.state.player1.energy_zone.active_count() as i32,
            game.state.player2.energy_zone.active_count() as i32,
        );
        game.pass();
        while game.has_pending_choice() {
            game.select_indices(&[]);
        }
    }
    panic!(
        "the turn never reached {:?} within 16 passes (stuck at {:?})",
        target, game.state.current_phase
    );
}

/// Boundary: heart02 total of exactly 6 across TWO Aqours members invalidates
/// Genki Zenkai's own LiveSuccess. Uses heart modifiers rather than three
/// members so the threshold is proven to count the heart total, not the number
/// of qualifying members.
#[test]
fn genki_exactly_6_heart02_across_two_members_invalidates_live_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let genki = game.id("PL!S-pb1-019-L");
    let chika = game.id("PL!S-sd1-010-SD"); // Aqours, heart02: 2
    let ruby = game.id("PL!S-pb1-018-N"); // Aqours, heart02: 2
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_stage(MemberArea::Center, ruby);
    game.add_to_stage(MemberArea::RightSide, filler);
    game.state.mods.add_heart_modifier(chika, HeartColor::Heart02, 2);

    let total_heart02: i32 = [chika, ruby]
        .iter()
        .map(|&id| {
            let printed = game
                .db
                .get_card(id)
                .and_then(|c| c.base_heart.as_ref())
                .and_then(|h| h.hearts.get(&HeartColor::Heart02))
                .copied()
                .unwrap_or(0);
            printed as i32 + game.state.mods.get_heart_modifier(id, HeartColor::Heart02)
        })
        .sum();
    assert_eq!(total_heart02, 6, "setup must reach exactly 6 heart02");

    fill_both_decks(&mut game, filler);
    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(genki);
    game.set_live_card(genki);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state
            .is_ability_invalidated(genki, &AbilityTrigger::LiveSuccess),
        "heart02 total of exactly 6 must invalidate Genki's LiveSuccess"
    );
    assert!(
        !game.state
            .is_ability_invalidated(genki, &AbilityTrigger::LiveStart),
        "only LiveSuccess is invalidated; LiveStart stays valid"
    );
}

/// Boundary: heart02 total of 5 (one short of the threshold) leaves LiveSuccess
/// valid, and it therefore actually resolves.
#[test]
fn genki_5_heart02_keeps_live_success_and_it_resolves() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let genki = game.id("PL!S-pb1-019-L");
    let chika = game.id("PL!S-sd1-010-SD"); // Aqours, heart02: 2
    let ruby = game.id("PL!S-pb1-018-N"); // Aqours, heart02: 2
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_stage(MemberArea::Center, ruby);
    game.add_to_stage(MemberArea::RightSide, filler);
    game.state.mods.add_heart_modifier(chika, HeartColor::Heart02, 1);

    let total_heart02: i32 = [chika, ruby]
        .iter()
        .map(|&id| {
            let printed = game
                .db
                .get_card(id)
                .and_then(|c| c.base_heart.as_ref())
                .and_then(|h| h.hearts.get(&HeartColor::Heart02))
                .copied()
                .unwrap_or(0);
            printed as i32 + game.state.mods.get_heart_modifier(id, HeartColor::Heart02)
        })
        .sum();
    assert_eq!(total_heart02, 5, "setup must reach exactly 5 heart02");

    give_opponent_energy_cards(&mut game, 10);
    fill_both_decks(&mut game, filler);
    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(genki);
    game.set_live_card(genki);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        !game.state
            .is_ability_invalidated(genki, &AbilityTrigger::LiveSuccess),
        "heart02 total of 5 is below the threshold; LiveSuccess must stay valid"
    );

    for _ in 0..3 {
        game.pass();
        while game.has_pending_choice() {
            game.select_indices(&[]);
        }
    }

    assert!(
        !game.state.player2.energy_zone.cards.is_empty(),
        "an un-invalidated LiveSuccess must resolve and give the opponent energy"
    );
}

/// Butterfly Wing suppresses the LiveStart abilities of its OWN stage members
/// while its own LiveSuccess still scores. Both halves of the printed card are
/// asserted in one flow: no energy from the suppressed LiveStart, score +1 from
/// the LiveSuccess that reads the same board.
#[test]
fn butterfly_suppresses_own_live_start_but_still_scores_on_live_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let butterfly = game.id("PL!SP-pb2-046-L");
    let mei_r = game.id("PL!SP-pb1-007-R"); // LiveStart member, heart02: 2
    let mei_p = game.id("PL!SP-pb1-007-P\u{ff0b}"); // LiveStart member, heart02: 2
    let keke = game.id("PL!SP-pb1-013-N"); // no ability, heart06 for the requirement
    let energy = game.id("LL-E-001-SD");

    game.add_to_stage(MemberArea::LeftSide, mei_r);
    game.add_to_stage(MemberArea::Center, mei_p);
    game.add_to_stage(MemberArea::RightSide, keke);
    for _ in 0..3 {
        game.state.player1.energy_zone.cards.push(energy);
    }
    game.state.player1.energy_zone.set_active_count(0);

    fill_both_decks(&mut game, keke);
    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(butterfly);
    game.set_live_card(butterfly);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "the suppressed LiveStart must not activate energy"
    );

    for _ in 0..3 {
        game.pass();
        while game.has_pending_choice() {
            game.select_indices(&[]);
        }
    }

    let live = game
        .state
        .performance_snapshots
        .first()
        .expect("a performance snapshot must exist")
        .lives
        .iter()
        .find(|l| l.card_id == butterfly)
        .expect("Butterfly Wing must appear in the performance");
    assert_eq!(
        live.score - live.base_score,
        1,
        "LiveSuccess must still add +1 for the LiveStart member on stage"
    );
}

/// The suppression is owner-scoped: 自分のステージにいるメンバー — the
/// activating player's LiveStart is suppressed, and an identically-named member
/// on the opponent's stage still resolves through a real live performance.
#[test]
fn butterfly_suppresses_only_the_owners_live_start_members() {
    use rabuka_engine::turn::TurnEngine as Scanner;

    let db = load_real_database();
    let mut game = TestGame::new(db);

    let butterfly = game.id("PL!SP-pb2-046-L");
    let mei_p1 = game.id("PL!SP-pb1-007-R");
    let mei_p2 = game.new_id("PL!SP-pb1-007-R");
    assert_ne!(mei_p1, mei_p2);
    let energy = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [butterfly, mei_p1, -1];
    game.state.player2.stage.stage = [mei_p2, -1, -1];
    // Both players need activatable energy so "did the LiveStart fire?" is
    // readable from the energy zone instead of a boolean. 米女メイ activates 2
    // cards it finds WAITED, so each side needs inactive cards in the zone.
    for _ in 0..6 {
        game.state.player1.energy_zone.cards.push(energy);
        game.state.player2.energy_zone.cards.push(energy);
    }
    game.state.player1.energy_zone.set_active_count(0);
    game.state.player2.energy_zone.set_active_count(0);

    // Precondition: the scanner sees the suppressor on p1's side only.
    assert!(
        Scanner::is_trigger_suppressed(&game.state, "p1", "live_start"),
        "own stage holds the suppressor, so the owner's LiveStart is suppressed"
    );
    assert!(
        !Scanner::is_trigger_suppressed(&game.state, "p2", "live_start"),
        "the opponent's stage holds no suppressor, so its LiveStart is not suppressed"
    );
    assert!(
        !Scanner::is_trigger_suppressed(&game.state, "p1", "live_success"),
        "only the LiveStart trigger is suppressed; LiveSuccess is untouched"
    );

    fill_both_decks(&mut game, mei_p1);
    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(butterfly);
    game.set_live_card(butterfly);

    // The second attacker's ライブ開始時 is only scanned when that player holds
    // a live card (triggers.rs guards a live-less second attacker), and the
    // harness only drives p1's actions, so p2's live is placed directly —
    // the established idiom for a second attacker's reveal.
    let opp_live = game.id("PL!-sd1-019-SD");
    game.state.player2.live_card_zone.cards.push(opp_live);

    // The Active phases already spent part of each energy zone. Re-seat both
    // counters so most of each zone is active and at least 2 cards are waited:
    // エネルギーを2枚アクティブにする can then only move the counter if it really
    // fires, and a suppressed one has to leave it alone. The energy decks stay
    // empty so no Energy phase can refill the zone inside the live window.
    for zone in [
        &mut game.state.player1.energy_zone,
        &mut game.state.player2.energy_zone,
    ] {
        zone.set_active_count(4);
    }
    assert!(
        game.state.player1.energy_zone.cards.len()
            - game.state.player1.energy_zone.active_count() as usize
            >= 2,
        "p1 must keep waited energy so 2枚アクティブ has something to activate"
    );
    assert!(
        game.state.player2.energy_zone.cards.len()
            - game.state.player2.energy_zone.active_count() as usize
            >= 2,
        "p2 must keep waited energy so 2枚アクティブ has something to activate"
    );
    assert!(game.state.player1.energy_deck.cards.is_empty());
    assert!(game.state.player2.energy_deck.cards.is_empty());

    // Both performers' ライブ開始時 are scanned in the same phase
    // (phases.rs: trigger_live_start_abilities for first and second attacker),
    // so the whole comparison happens on entry to FirstAttackerPerformance.
    // The baseline is taken on the last step INTO that phase, because a phase
    // earlier in the window may still top up an energy zone and would otherwise
    // be charged to the LiveStart.
    let (p1_before, p2_before) =
        pass_into_phase_capturing_energy(&mut game, Phase::FirstAttackerPerformance);
    let p1_after = game.state.player1.energy_zone.active_count() as i32;
    let p2_after = game.state.player2.energy_zone.active_count() as i32;

    assert_eq!(
        p1_after - p1_before,
        0,
        "米女メイ on the owner's stage must have its LiveStart suppressed: \
         エネルギーを2枚アクティブにする must NOT apply to the suppressor owner"
    );
    assert_eq!(
        p2_after - p2_before,
        2,
        "the same printed LiveStart on the OPPONENT's stage must still resolve: \
         エネルギーを2枚アクティブにする (p2 active {} before, {} after)",
        p2_before,
        p2_after
    );
}
