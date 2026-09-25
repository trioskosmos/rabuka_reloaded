use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
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

/// The suppression is owner-scoped: the activating player's LiveStart is
/// suppressed, and an identically-named member on the opponent's stage is not.
#[test]
fn butterfly_suppresses_only_the_owners_live_start_members() {
    use rabuka_engine::turn::TurnEngine as Scanner;

    let db = load_real_database();
    let mut game = TestGame::new(db);

    let butterfly = game.id("PL!SP-pb2-046-L");
    let mei_p1 = game.id("PL!SP-pb1-007-R");
    let mei_p2 = game.new_id("PL!SP-pb1-007-R");
    assert_ne!(mei_p1, mei_p2);

    game.state.player1.stage.stage = [butterfly, mei_p1, -1];
    game.state.player2.stage.stage = [mei_p2, -1, -1];

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
}
