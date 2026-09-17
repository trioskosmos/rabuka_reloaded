use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// PL!SP-bp5-026-L — Liella heart-total constant gates live score +1
//
// aggregate=total on group_condition + Stage dispatches to
// sum_group_hearts_in_stage (base heart sums of matching members).
// The group filter 「Liella!」 matches via series containing
// スーパースター (card_series_matches_group in util.rs).
// ====================================================================

fn bp5026_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let live = game.id("PL!SP-bp5-026-L");
    game.state.player1.live_card_zone.cards.push(live);
    live
}

#[test]
fn bp5026_total_above_threshold_scores_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = bp5026_setup(&mut game);
    // Three Superstar-series members: 9 + 8 + any third member >= 0.
    // The two big ones alone give 17 >= 11.
    let big1 = game.id("PL!SP-pb2-005-R"); // hearts {02:3, 03:3, 06:3} = 9
    let big2 = game.id("PL!SP-bp4-004-P"); // hearts {02:3, 03:3, 06:2} = 8
    let small = game.new_id("PL!-sd1-010-SD"); // μ's — not Liella!, excluded
    game.state.player1.stage.stage[0] = big1;
    game.state.player1.stage.stage[1] = big2;
    game.state.player1.stage.stage[2] = small;

    fire_live_start(&mut game, live);

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "two Liella! members' base hearts total 17 >= 11 -> score +1"
    );
}

#[test]
fn bp5026_total_below_threshold_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = bp5026_setup(&mut game);
    // Two low-heart Liella members + one non-Liella outsider.
    // Liella total: 2 + 2 = 4 < 11.
    let l1 = game.id("PL!SP-pb2-036-N");
    let l2 = game.id("PL!SP-pb2-037-N");
    let outsider = game.new_id("PL!-sd1-010-SD"); // μ's — not counted
    game.state.player1.stage.stage[0] = l1;
    game.state.player1.stage.stage[1] = l2;
    game.state.player1.stage.stage[2] = outsider;

    fire_live_start(&mut game, live);

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "total 4 < 11 -> no bonus"
    );
}
