use crate::helpers::*;
use crate::test_modules::support::baton_swap_auto_helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// PL!N-PR-025-PR 優木せつ菜 (was one direction only: other-member arrival)
// {{jidou.png|自動}}{{turn2.png|ターン2回}}自分のステージに、このメンバーか、
// ほかのメンバーがバトンタッチして登場したとき、カードを1枚引く。
//
// Turn-2: when THIS member or ANOTHER member baton-touches onto your stage,
// draw 1. Covers: self arrival, the twice-per-turn budget (2 draw, 3rd
// refused), normal debut (no fire), opponent-stage arrival (no fire).
// ====================================================================

fn setsuna_setup(game: &mut TestGame) -> i16 {
    let setsuna = game.id("PL!N-PR-025-PR");
    assert_eq!(
        game.db.get_card(setsuna).unwrap().card_no,
        "PL!N-PR-025-PR",
        "staged the PR print of Setsuna"
    );
    // Deck via ONE template id duplicated (fill_decks pattern): bulk deck
    // filler must not burn pool slots — the pool holds 11 instances per
    // card_no, and new_id past exhaustion mints data-less ids that detonate
    // on first DB lookup (flaky "Only member cards can be placed on stage").
    let deck_filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(deck_filler);
    }
    game.give_energy(30);
    setsuna
}

fn use_count(game: &TestGame, setsuna: i16) -> u8 {
    game.state
        .turn_limited_abilities_used
        .get(&(setsuna, 0, game.state.turn_number))
        .copied()
        .unwrap_or(0)
}

#[test]
fn setsuna_self_baton_arrival_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = setsuna_setup(&mut game);
    let filler = game.id(FILLER);

    // Setsuna herself arrives via baton touch onto occupied center.
    replace_member_by_baton_touch(&mut game, filler, setsuna, MemberArea::Center);
    resolve_auto_choices_accepting_optionals(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "self baton arrival should draw 1"
    );
    assert_eq!(use_count(&game, setsuna), 1, "one use recorded");
}

#[test]
fn setsuna_turn2_budget_two_draws_third_refused() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = setsuna_setup(&mut game);
    // Fresh instances throughout: game.id() interns by card_no, and reusing
    // one id across waitroom/stage aliases the zones and confuses movement
    // recording. Every physical card below gets its own id.
    let filler_l = game.new_id(FILLER);
    let filler_c = game.new_id(FILLER);
    let filler_r = game.new_id(FILLER);
    let arr2 = game.new_id("PL!-sd1-002-SD");
    // Ability-less arriver for fire 3 (Kotori's debut would draw on its own
    // and confound the count).
    let arr3 = game.new_id("PL!-sd1-010-SD");

    // Fire 1 (self): Setsuna baton-touches onto center.
    replace_member_by_baton_touch(&mut game, filler_c, setsuna, MemberArea::Center);    resolve_auto_choices_accepting_optionals(&mut game);
    assert_eq!(game.state.player1.hand.cards.len(), 1, "first firing draws");
    assert_eq!(use_count(&game, setsuna), 1);

    // Fire 2 (other): arriver takes occupied left while Setsuna watches.
    game.state.player1.stage.set_area(MemberArea::LeftSide, filler_l);
    game.state.player1.hand.cards.push(arr2);
    game.play_to_stage(arr2, MemberArea::LeftSide);
    resolve_auto_choices_accepting_optionals(&mut game);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "second firing draws (hand 1+1-1+1)"
    );
    assert_eq!(use_count(&game, setsuna), 2);

    // Fire 3: budget exhausted — arrival happens, no draw.
    game.state.player1.stage.set_area(MemberArea::RightSide, filler_r);
    game.state.player1.hand.cards.push(arr3);
    game.play_to_stage(arr3, MemberArea::RightSide);
    resolve_auto_choices_accepting_optionals(&mut game);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "third arrival same turn: budget exhausted, no draw (hand 2+1-1+0)"
    );
    assert_eq!(use_count(&game, setsuna), 2, "no third use recorded");
}

#[test]
fn setsuna_normal_debut_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = setsuna_setup(&mut game);
    let filler = game.id(FILLER);

    // Left occupied, right EMPTY: arrival to right is a plain debut.
    game.state.player1.stage.set_area(MemberArea::LeftSide, filler);
    game.state.player1.hand.cards.push(setsuna);
    game.play_to_stage(setsuna, MemberArea::RightSide);
    resolve_auto_choices_accepting_optionals(&mut game);

    assert!(
        game.state.player1.hand.cards.is_empty(),
        "plain debut (no baton touch) must not fire"
    );
    assert_eq!(use_count(&game, setsuna), 0, "no use recorded");
}

#[test]
fn setsuna_opponent_stage_arrival_does_not_fire() {
    use rabuka_engine::game_state::Phase;
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = setsuna_setup(&mut game);
    game.state.player1.stage.set_area(MemberArea::LeftSide, setsuna);
    let p2filler = game.new_id("PL!-sd1-010-SD");
    let arriver = game.new_id("PL!-sd1-002-SD");

    // Opponent's own board and hand.
    game.give_energy_for(Side::P2, 30);
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(game.new_id("PL!-sd1-010-SD"));
    }
    game.state.player2.stage.set_area(MemberArea::Center, p2filler);
    game.state.player2.hand.cards.push(arriver);

    // Advance P1 Main -> Active -> Energy -> Draw -> P2 Main.
    for _ in 0..4 {
        game.pass();
    }
    assert_eq!(
        game.state.current_phase,
        Phase::Main,
        "test setup must reach P2 Main, got {:?}",
        game.state.current_phase
    );

    // Opponent baton-touches onto the OPPONENT's stage.
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(arriver),
        None,
        Some(MemberArea::Center),
        None,
    )
    .expect("p2 play failed");
    resolve_auto_choices_accepting_optionals(&mut game);

    assert!(
        game.state.player2.stage.stage.contains(&arriver),
        "opponent arrival happened on opponent stage"
    );
    assert!(
        game.state.player1.hand.cards.is_empty(),
        "opponent-stage arrival must not fire own Setsuna"
    );
    assert_eq!(use_count(&game, setsuna), 0, "no use recorded");
}
