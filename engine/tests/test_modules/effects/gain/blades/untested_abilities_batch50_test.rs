/// Untested-abilities batch 50 — color-choice grants & score-tie retrieval.
///
/// - PL!N-sd2-005-SD2 宮下愛 (ライブ開始時): specify any heart color -> gain
///   2 of that color until live end.
/// - PL!SP-pb2-030-N 若菜四季 (ライブ開始時): choose among {02,03,06} ->
///   original hearts become the chosen color (batch-39 family twin).
/// - PL!HS-cl1-012-CL Edelied (ライブ成功時): if own and opponent's total
///   live scores are TIED, retrieve one cost>=9 member from the
///   yell-revealed cards.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

// ====================================================================
// PL!N-sd2-005-SD2 宮下愛 — specify color -> +2 of it
// ====================================================================

#[test]
fn sd2005_specify_color_grants_exactly_two_of_one_color() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!N-sd2-005-SD2");
    game.state.player1.stage.stage[1] = me;
    // Hand cards for the optional discard-2 cost.
    game.add_to_hand(game.new_id("PL!-sd1-010-SD"));
    game.add_to_hand(game.new_id("PL!S-sd1-001-SD"));

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    // First prompt: the optional discard-2-hand cost selection.
    assert!(game.has_pending_choice(), "discard-2 cost prompted");
    game.select_indices(&[0, 1]);
    // Second prompt: specify the heart color (heart04 = 4th of six).
    assert!(game.has_pending_choice(), "color specification offered");
    game.select_option(3); // heart04

    // Exactly one color ends at +2; all others untouched.
    let all = [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ];
    let boosted: Vec<HeartColor> = all
        .iter()
        .filter(|c| game.state.mods.get_heart_modifier(me, **c) > 0)
        .copied()
        .collect();
    assert_eq!(boosted.len(), 1, "exactly one color boosted");
    assert_eq!(
        game.state.mods.get_heart_modifier(me, boosted[0]),
        2,
        "the chosen color gained exactly +2"
    );
}

// ====================================================================
// PL!SP-pb2-030-N 若菜四季 — chosen-color transform twin
// ====================================================================

fn wakaba_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id("PL!SP-pb2-030-N");
    game.state.player1.stage.stage[1] = me;
    let bystander = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = bystander;
    me
}

#[test]
fn pb2030_choose_first_option_transforms() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = wakaba_setup(&mut game);

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.select_option(0); // heart02

    assert_eq!(
        game.state.mods.heart_color_multiplier.get(&me).copied(),
        Some(HeartColor::Heart02),
        "chose heart02 -> original hearts become heart02"
    );
}

#[test]
fn pb2030_choose_third_option_transforms() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = wakaba_setup(&mut game);

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.select_option(2); // heart06

    assert_eq!(
        game.state.mods.heart_color_multiplier.get(&me).copied(),
        Some(HeartColor::Heart06),
        "chose heart06 -> original hearts become heart06"
    );
    assert_eq!(
        game.state
            .mods
            .heart_color_multiplier
            .get(&game.id_ref("PL!-sd1-010-SD"))
            .copied(),
        None,
        "bystander untouched"
    );
}

// ====================================================================
// PL!HS-cl1-012-CL Edelied — tie-score gates cost>=9 yell-revealed fetch
//
// Live score totals only count lives whose need_heart is satisfied by
// stage members (calculate_live_score). We use PL!S-bp7-024-L
// (score=1, need={heart04:2}) on both sides with a heart04>=2 member
// staged per player so each side's total is genuinely 1.
// ====================================================================

const TIED_LIVE: &str = "PL!S-bp7-024-L"; // score 1, need {heart04:2}
const SATISFYING_MEMBER: &str = "PL!S-PR-014-PR"; // base_heart heart04:2

fn edelied_setup(game: &mut TestGame) {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let live = game.id("PL!HS-cl1-012-CL");
    game.state.player1.live_card_zone.cards.push(live);
}

/// Place the tied live + a satisfying member for one player.
fn place_tied_live(game: &mut TestGame, player: u8) {
    let l = game.new_id(TIED_LIVE);
    let member = game.id(SATISFYING_MEMBER);
    let (p, stage_idx) = if player == 1 {
        (&mut game.state.player1, 2usize)
    } else {
        (&mut game.state.player2, 2usize)
    };
    p.live_card_zone.cards.push(l);
    p.stage.stage[stage_idx] = member;
    // calculate_live_score reads player.stage_hearts (a precomputed
    // snapshot set during execute_live_victory_determination); tests that
    // bypass the live flow must populate it explicitly.
    p.stage_hearts = Some(p.calculate_stage_hearts(
        &game.db,
        &game.state.mods.heart_color_multiplier,
        &game.state.mods.heart_override,
        &game.state.mods.heart_modifiers,
        &game.state.mods.heart_copy,
    ));
}

#[test]
fn cl1012_tie_score_retrieves_cost_nine_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    edelied_setup(&mut game);

    // Both sides run the same satisfied live -> totals 1 == 1 (tie).
    place_tied_live(&mut game, 1);
    place_tied_live(&mut game, 2);

    // Yell-revealed pool: an EXPENSIVE member and a cheap one.
    let expensive = game.new_id("PL!HS-bp5-004-R"); // cost 15 >= 9
    let cheap = game.new_id("PL!SP-PR-003-PR"); // cost 2 < 9
    game.state.revealed_cards.push(expensive);
    game.state.revealed_cards.push(cheap);

    let live_id = game.id_ref("PL!HS-cl1-012-CL");
    fire_trigger(&mut game, live_id, AbilityTrigger::LiveSuccess, "ライブ成功時");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&expensive),
        "tie -> cost>=9 member retrieved to hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&cheap),
        "cost-2 member not eligible"
    );
}

#[test]
fn cl1012_unequal_scores_no_retrieval() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    edelied_setup(&mut game);

    // p1 runs the satisfied live (total=1); p2's identical live is NOT
    // satisfied (no matching member) -> total=0. 1 != 0 -> no tie.
    place_tied_live(&mut game, 1);

    let expensive = game.new_id("PL!HS-bp5-004-R");
    game.state.revealed_cards.push(expensive);

    let live_id = game.id_ref("PL!HS-cl1-012-CL");
    fire_trigger(&mut game, live_id, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert!(
        !game.state.player1.hand.cards.contains(&expensive),
        "scores not tied -> no retrieval"
    );
}
