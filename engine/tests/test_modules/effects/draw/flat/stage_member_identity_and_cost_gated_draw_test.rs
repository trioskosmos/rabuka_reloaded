use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";
const CLEAN_KOTORI: &str = "PL!-pb1-021-PR";

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .expect("card should have the requested trigger ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

fn wakashi_sp1008_setup(game: &mut TestGame) -> i16 {
    let wakashi = game.id("PL!SP-bp1-008-R"); // cost 13 — also nico bp3-009's condition card
    game.state.player1.stage.stage[1] = wakashi;
    fill_decks(game, game.id(FILLER));
    wakashi
}

#[test]
fn shiki_sp_bp1_008_debut_without_mei_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakashi = wakashi_sp1008_setup(&mut game);
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, wakashi, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "no 米女メイ on stage → single draw"
    );
}

#[test]
fn shiki_sp_bp1_008_debut_with_mei_draws_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakashi = wakashi_sp1008_setup(&mut game);
    let mei = game.id("PL!SP-pb1-007-R"); // 米女メイ
    game.state.player1.stage.stage[0] = mei;
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, wakashi, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 2,
        "米女メイ on stage → additional draw"
    );
}

#[test]
fn nico_bp3_009_debut_cost_thirteen_member_present_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let niko = game.id("PL!-bp3-009-R＋");
    let wakashi = game.id("PL!SP-bp1-008-R"); // cost 13
    game.state.player1.stage.stage[0] = wakashi;
    game.state.player1.stage.stage[1] = niko;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, niko, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "cost-13 member present → draw 1"
    );
}

#[test]
fn nico_bp3_009_debut_no_cost_thirteen_member_skips_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let niko = game.id("PL!-bp3-009-R＋");
    game.state.player1.stage.stage[1] = niko;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, niko, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "only herself (cost 2) on stage → no draw"
    );
}

#[test]
fn honoka_bp4_001_live_start_cheaper_stage_total_draws_one_otherwise_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let honoka = game.id("PL!-bp4-001-R"); // cost 9
    let big = game.id("PL!S-bp5-009-R"); // 黒澤ルビィ cost 15
    let small = game.id(CLEAN_KOTORI); // cost 5

    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    // Cheaper: my total 9 < opponent 15 → draw.
    game.state.player1.stage.stage[1] = honoka;
    game.state.player2.stage.stage[1] = big;
    let before = game.state.player1.hand.cards.len();
    trigger_auto(&mut game, honoka, AbilityTrigger::LiveStart, "ライブ開始時");
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before + 1,
        "stage total 9 < opponent 15 → draw 1"
    );

    // Now more expensive: my 9 > opponent 5 → no draw.
    game.state.player2.stage.stage[1] = small;
    let before2 = game.state.player1.hand.cards.len();
    trigger_auto(&mut game, honoka, AbilityTrigger::LiveStart, "ライブ開始時");
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before2,
        "stage total 9 > opponent 5 → no draw"
    );
}
