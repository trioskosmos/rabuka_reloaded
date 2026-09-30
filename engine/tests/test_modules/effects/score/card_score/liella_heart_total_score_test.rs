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

/// The bound itself: 「総数が11以上」, staged at exactly 11.
///
/// The two tests above use 17 (qualifies) and 4 (does not), so `>= 12`, `> 11`, or summing
/// only the first member all pass them. Only a total that lands ON 11 separates those.
///
/// `PL!SP-pb2-005-R` sums to 9 and `PL!SP-pb2-036-N` to 2, giving exactly 11. Both totals
/// are recomputed from `cards.json` below and asserted, rather than trusted from the
/// comments above — a rebalanced print would otherwise move the fixture across the bound
/// and quietly invert the test.
#[test]
fn bp5026_total_of_exactly_eleven_is_the_inclusive_bound() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = bp5026_setup(&mut game);
    let big = game.id("PL!SP-pb2-005-R");
    let small = game.id("PL!SP-pb2-036-N");
    let outsider = game.new_id("PL!-sd1-010-SD"); // μ's — must stay uncounted

    // Pin the premise from card data: the two Liella! members must sum to exactly the bound,
    // and the μ's member must contribute nothing.
    let total_of = |g: &TestGame, id: i16| -> i32 {
        g.db
            .get_card(id)
            .and_then(|c| c.base_heart.as_ref())
            .map(|h| h.hearts.values().map(|&v| v as i32).sum())
            .unwrap_or(0)
    };
    let big_total = total_of(&game, big);
    let small_total = total_of(&game, small);
    assert_eq!(
        big_total + small_total,
        11,
        "precondition: the staged Liella! members must total EXACTLY the printed 11以上 bound \
         (got {big_total} + {small_total}); otherwise this is not the boundary case"
    );

    game.state.player1.stage.stage[0] = big;
    game.state.player1.stage.stage[1] = small;
    game.state.player1.stage.stage[2] = outsider;

    fire_live_start(&mut game, live);

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "総数が11以上 — a total of exactly 11 satisfies the bound, so the score must be +1. \
         A value of 0 means the comparison is exclusive (`> 11`) or reads 12+"
    );
}

/// The sharpest negative available: the single big member alone, one Liella! short of the
/// bound — 9 where 11 is required.
///
/// Paired with the test above (9 + 2 = 11 → +1) this brackets the comparison at 9 and 11
/// with only 2 hearts between them, so an off-by-one of any size up to 2 is caught.
#[test]
fn bp5026_nine_liella_hearts_is_below_the_bound() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = bp5026_setup(&mut game);
    let big = game.id("PL!SP-pb2-005-R");
    let outsiders = [
        game.new_id("PL!-sd1-010-SD"),
        game.new_id("PL!-sd1-010-SD"),
    ];
    game.state.player1.stage.stage = [big, outsiders[0], outsiders[1]];

    fire_live_start(&mut game, live);

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "PL!SP-pb2-005-R alone sums to 9, which is 11以上 NOT met — no bonus. This is the \
         nearest below-boundary total reachable with real cards."
    );
}
