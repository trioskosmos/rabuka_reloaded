/// Untested-abilities batch 10 — depth=none gaps from TEST_INVENTORY:
/// - PL!HS-pb1-013-R ab#1 (ライブ成功時): draw when a higher-cost member is on stage
/// - PL!SP-bp2-023-L ab#0 (ライブ開始時): score +1 when own success zone < opponent's
/// - PL!SP-pb1-024-L ab#0 (ライブ開始時): score +1 with 2+ distinct KALEIDOSCORE members
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member, cost 4

// ====================================================================
// PL!HS-pb1-013-R ab#1 (ライブ成功時):
// 「自分のステージに、このメンバーよりコストが高いメンバーがいる場合、カードを1枚引く。」
// This card costs 9.
// ====================================================================

const HIGHER_COST_MEMBER: &str = "PL!HS-bp5-004-R"; // cost 15 (> 9)
const LOWER_COST_MEMBER: &str = "PL!SP-PR-007-PR"; // cost 2 (< 9)

#[test]
fn pb1013_higher_cost_member_on_stage_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-pb1-013-R"); // cost 9
    let big = game.id(HIGHER_COST_MEMBER); // cost 15
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = big;
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "a member costing more than 9 is on stage -> draw 1"
    );
    assert!(
        game.state.player1.hand.cards.contains(&drawn),
        "the drawn card is the one stocked on the deck"
    );
}

#[test]
fn pb1013_only_lower_cost_members_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-pb1-013-R"); // cost 9
    let small = game.id(LOWER_COST_MEMBER); // cost 2
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = small;
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no member above cost 9 -> no draw"
    );
}

// ====================================================================
// PL!SP-bp2-023-L ab#0 (ライブ開始時):
// 「自分の成功ライブカード置き場のカード枚数が相手より少ない場合、このカードのスコアを＋１する。」
// ====================================================================

#[test]
fn go_master_start_fewer_success_cards_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp2-023-L");
    game.state.player1.live_card_zone.cards.push(live);

    let s1 = game.new_id(FILLER);
    game.state.player1.success_live_card_zone.cards.push(s1);
    let o1 = game.new_id(FILLER);
    let o2 = game.new_id(FILLER);
    game.state.player2.success_live_card_zone.cards.push(o1);
    game.state.player2.success_live_card_zone.cards.push(o2);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "1 < 2 success cards -> score +1"
    );
}

#[test]
fn go_master_start_equal_success_cards_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp2-023-L");
    game.state.player1.live_card_zone.cards.push(live);

    let s1 = game.new_id(FILLER);
    game.state.player1.success_live_card_zone.cards.push(s1);
    let o1 = game.new_id(FILLER);
    game.state.player2.success_live_card_zone.cards.push(o1);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "1 vs 1 is not fewer -> no score"
    );
}

// ====================================================================
// PL!SP-pb1-024-L ab#0 (ライブ開始時):
// 「自分のステージに名前の異なる『KALEIDOSCORE』のメンバーが2人以上いる場合、このカードのスコアを＋１する。」
// ====================================================================

#[test]
fn note_mermaid_two_distinct_kaleidoscope_members_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-pb1-024-L");
    game.state.player1.live_card_zone.cards.push(live);

    // Two different KALEIDOSCORE characters.
    let ren = game.id("PL!SP-bp1-013-PR");
    let wien = game.id("PL!SP-PR-017-PR");
    game.state.player1.stage.stage[0] = ren;
    game.state.player1.stage.stage[1] = wien;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "two distinct KALEIDOSCORE members -> score +1"
    );
}

#[test]
fn note_mermaid_duplicate_kaleidoscope_names_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-pb1-024-L");
    game.state.player1.live_card_zone.cards.push(live);

    // Two copies of the SAME character -> not "名前の異なる".
    let ren1 = game.id("PL!SP-bp1-013-PR");
    let ren2 = game.new_id("PL!SP-pb1-013-PR");
    game.state.player1.stage.stage[0] = ren1;
    game.state.player1.stage.stage[1] = ren2;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "duplicate names do not count as distinct"
    );
}

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
fn bp5026_total_eleven_scores_plus_one() {
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
