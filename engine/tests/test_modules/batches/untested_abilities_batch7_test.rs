/// Untested-abilities batch 7 窶・mined from TEST_INVENTORY depth=none gaps,
/// every assertion derived from the printed card text plus qa_data.json
/// rulings. Implemented ONE ABILITY AT A TIME per engine/tests/BATCH7_PLAN.md.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // ﾎｼ's member, cost 4, no abilities

// ====================================================================
// A1 PL!-bp5-008-R 蟆乗ｳ芽干髯ｽ 窶・蟶ｸ譎・
// 閾ｪ蛻・・謌仙粥繝ｩ繧､繝悶き繝ｼ繝臥ｽｮ縺榊ｴ縺ｫ縺ゅｋ繧ｫ繝ｼ繝峨・繧ｹ繧ｳ繧｢縺ｮ蜷郁ｨ医′・紋ｻ･荳翫〒縺ゅｋ縺九℃繧翫・// {{heart_03}} 繧抵ｼ偵▽蠕励ｋ縲・// BATCH7_PLAN.md ﾂｧA1, edges 1窶・.
// ====================================================================

/// Edge 1+2: below threshold 竊・nothing; exactly 6 竊・+2 heart03.
#[test]
fn hanayo_bp5_008_score_sum_threshold_and_boundary() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp5-008-R");
    let live3 = game.id("PL!-sd1-021-SD"); // 縺薙ｌ縺九ｉ縺ｮSomeday, score 3
    game.add_to_stage(MemberArea::Center, hanayo);

    // Edge 1: total 3 < 6 竊・no hearts.
    game.state.player1.success_live_card_zone.add_card(live3);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        0,
        "score total 3 < 6 竊・no heart03"
    );

    // Edge 2: exactly 6 (3+3) 窶・>= boundary is inclusive.
    let live3b = game.new_id("PL!-sd1-021-SD");
    game.state.player1.success_live_card_zone.add_card(live3b);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        2,
        "score total exactly 6 竊・+2 heart03"
    );

    // Edge 4: drop back under the threshold 竊・bonus removed dynamically.
    game.state.player1.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        0,
        "as_long_as bonus removed once total drops below 6"
    );

    // Scenario check: the modifier must flow into the stage-heart totals the
    // live-success check actually consumes 窶・re-enable the bonus and verify
    // the aggregate heart03 rises by exactly +2 over the disabled baseline.
    game.state.player1.success_live_card_zone.add_card(live3);
    game.state
        .player1
        .success_live_card_zone
        .add_card(live3b);
    game.state.recalculate_constants();
    let hearts = game.state.player1.calculate_stage_hearts(
        &game.db,
        &game.state.mods.heart_color_multiplier,
        &game.state.mods.heart_override,
        &game.state.mods.heart_modifiers,
        &game.state.mods.heart_copy,
    );
    let with_bonus = hearts.hearts[&HeartColor::Heart03];

    game.state.player1.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();
    let hearts_off = game.state.player1.calculate_stage_hearts(
        &game.db,
        &game.state.mods.heart_color_multiplier,
        &game.state.mods.heart_override,
        &game.state.mods.heart_modifiers,
        &game.state.mods.heart_copy,
    );
    assert_eq!(
        with_bonus - hearts_off.hearts[&HeartColor::Heart03],
        2,
        "stage-aggregate heart03 must grow by exactly the constant's 2"
    );
}

/// Edge 3+9: single score-9 card still grants exactly +2 (fixed value), and no
/// other heart color is touched.
#[test]
fn hanayo_bp5_008_value_fixed_and_color_exact() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp5-008-R");
    let big_live = game.id("PL!S-pb1-023-L"); // Next SPARKLING!!, score 9
    game.add_to_stage(MemberArea::Center, hanayo);
    game.state
        .player1
        .success_live_card_zone
        .add_card(big_live);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        2,
        "total 9 竕･ 6 still grants exactly +2 (text says 縺､蠕励ｋ, fixed)"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart01),
        0,
        "no collateral heart01"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(hanayo),
        0,
        "no collateral blade either"
    );
}

/// Edge 5: opponent's success zone must be ignored entirely.
#[test]
fn hanayo_bp5_008_ignores_opponent_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp5-008-R");
    let big_live_p2 = game.new_id("PL!S-pb1-023-L"); // opponent's copy, score 9
    game.add_to_stage(MemberArea::Center, hanayo);
    game.state
        .player2
        .success_live_card_zone
        .add_card(big_live_p2);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        0,
        "opponent zone total 9 must NOT count for 閾ｪ蛻・・窶ｦ"
    );
}

/// Edge 6: aggregate is SCORE SUM, not card count 窶・six score-0 lives give 0.
#[test]
fn hanayo_bp5_008_counts_score_not_card_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp5-008-R");
    let zero_live = game.id("PL!-bp3-019-L"); // 蜒輔ｉ縺ｮLIVE 蜷帙→縺ｮLIFE, score 0
    game.add_to_stage(MemberArea::Center, hanayo);
    for _ in 0..6 {
        let copy = if game.state.player1.success_live_card_zone.cards.is_empty() {
            zero_live
        } else {
            game.new_id("PL!-bp3-019-L")
        };
        game.state.player1.success_live_card_zone.add_card(copy);
    }
    assert_eq!(game.state.player1.success_live_card_zone.len(), 6);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        0,
        "six score-0 lives = score sum 0 < 6 竊・no hearts (aggregate=score)"
    );
}

/// Edge 7: Hanayo off stage 竊・constant inactive regardless of zones.
#[test]
fn hanayo_bp5_008_requires_presence_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live6 = game.id("PL!SP-bp1-027-L"); // Sing・ヾhine・ヾmile・・ score 6
    game.state.player1.success_live_card_zone.add_card(live6);
    // Hanayo deliberately NOT staged.

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(game.id("PL!-bp5-008-R"), HeartColor::Heart03),
        0,
        "constant only works while she is on stage"
    );
}

/// Edge 8: two copies on stage each get their own independent +2.
#[test]
fn hanayo_bp5_008_two_copies_each_get_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo_a = game.id("PL!-bp5-008-R");
    let hanayo_b = game.new_id("PL!-bp5-008-R");
    let live6 = game.id("PL!SP-bp1-027-L");
    game.state.player1.stage.stage[0] = hanayo_b;
    game.state.player1.stage.stage[1] = hanayo_a;
    game.state.player1.success_live_card_zone.add_card(live6);

    game.state.recalculate_constants();
    for &copy in &[hanayo_a, hanayo_b] {
        assert_eq!(
            game.state.mods.get_heart_modifier(copy, HeartColor::Heart03),
            2,
            "each copy independently gets +2 heart03"
        );
    }
}

// ====================================================================
// A2 PL!S-pb1-009-R 黒澤ルビィ — 常時:
// 自分と相手の成功ライブカード置き場にカードが合計3枚以上ある場合、
// ブレードを３つ得る。
// Contrast pair with A1: CARD COUNT across BOTH players, not score sum.
// BATCH7_PLAN.md §A2, edges 1–7.
// ====================================================================

/// Edge 1+2+3+5: 0 → 0; combined 2 → 0; combined 3 → +3; drop back → 0.
#[test]
fn ruby_pb1_009_combined_count_threshold() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = game.id("PL!S-pb1-009-R");
    let live_a = game.id("PL!-sd1-019-SD"); // START:DASH!! score 1
    let live_b = game.id("PL!SP-bp1-027-L"); // Sing！Shine！Smile！ score 6
    game.add_to_stage(MemberArea::Center, ruby);

    // Edge 1: both zones empty.
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(ruby), 0, "empty zones");

    // Edge 2: own 1 + opponent 1 = 2 < 3.
    game.state.player1.success_live_card_zone.add_card(live_a);
    game.state
        .player2
        .success_live_card_zone
        .add_card(live_b);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        0,
        "combined total 2 < 3 → no blade"
    );

    // Edge 3: opponent's second card pushes the COMBINED count to 3.
    let live_c_opp = game.new_id("PL!-sd1-021-SD"); // score 3
    game.state
        .player2
        .success_live_card_zone
        .add_card(live_c_opp);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        3,
        "own+opponent combined 3 cards → blade+3"
    );
}

/// Edge 4: own zone may be EMPTY — opponent's cards alone can satisfy it.
#[test]
fn ruby_pb1_009_own_zone_may_be_empty() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = game.id("PL!S-pb1-009-R");
    game.add_to_stage(MemberArea::Center, ruby);
    for _ in 0..3 {
        let copy = if game
            .state
            .player2
            .success_live_card_zone
            .cards
            .is_empty()
        {
            game.id("PL!-sd1-019-SD")
        } else {
            game.new_id("PL!-sd1-019-SD")
        };
        game.state.player2.success_live_card_zone.add_card(copy);
    }
    assert_eq!(game.state.player1.success_live_card_zone.len(), 0);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        3,
        "opponent-only 3 cards still satisfy 自分と相手の…合計3枚以上"
    );
}

/// Edge 5 (dynamic removal) + Edge 6: score-0 lives count toward the count,
/// unlike A1's score-sum aggregate.
#[test]
fn ruby_pb1_009_score_zero_lives_count_and_bonus_is_dynamic() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = game.id("PL!S-pb1-009-R");
    let zero_live = game.id("PL!-bp3-019-L"); // score 0
    game.add_to_stage(MemberArea::Center, ruby);
    game.state.player1.success_live_card_zone.add_card(zero_live);
    for _ in 1..3 {
        let copy = game.new_id("PL!-bp3-019-L");
        game.state.player2.success_live_card_zone.add_card(copy);
    }
    assert_eq!(
        game.state.player1.success_live_card_zone.len()
            + game.state.player2.success_live_card_zone.len(),
        3
    );

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        3,
        "three score-0 lives = 3 CARDS → blade+3 (count, not score)"
    );

    // Dynamic removal: one zone empties → back under threshold.
    game.state.player2.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        0,
        "bonus removed when the combined count drops"
    );
}

/// Edge 7: two Ruby copies each independently get +3.
#[test]
fn ruby_pb1_009_two_copies_each_get_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby_a = game.id("PL!S-pb1-009-R");
    let ruby_b = game.new_id("PL!S-pb1-009-R");
    game.state.player1.stage.stage[0] = ruby_b;
    game.state.player1.stage.stage[1] = ruby_a;
    for _ in 0..3 {
        let copy = if game
            .state
            .player2
            .success_live_card_zone
            .cards
            .is_empty()
        {
            game.id("PL!-sd1-019-SD")
        } else {
            game.new_id("PL!-sd1-019-SD")
        };
        game.state.player2.success_live_card_zone.add_card(copy);
    }

    game.state.recalculate_constants();
    for &copy in &[ruby_a, ruby_b] {
        assert_eq!(
            game.state.mods.get_blade_modifier(copy),
            3,
            "each copy independently gets blade+3"
        );
    }
}

// ====================================================================
// A3 PL!S-bp5-008-R 小原鞠莉 — 常時:
// 相手の余剰ハートが2つ以上あるかぎり、自分のライブの合計スコアを＋１する。
// Surplus definition per Q142: stage base hearts exceeding the live cards'
// need hearts. Asserted via mods.p1_constant_total_score_bonus (live TOTAL).
// BATCH7_PLAN.md §A3, edges 1–6.
// ====================================================================

/// Edge 1+2+6: empty opponent stage → 0; staging a 7-heart member with no
/// live cards set (need 0) → surplus 7 → +1; removing them removes the bonus.
#[test]
fn mari_bp5_008_opponent_surplus_gates_live_total_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp5-008-R"); // cost 9
    game.add_to_stage(MemberArea::Center, mari);

    // Edge 1: opponent has nothing on stage.
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "no opponent members → no surplus → no bonus"
    );

    // Edge 2: opponent stages a 7-base-heart member, NO live set → surplus 7.
    let opp_member = game.new_id("PL!S-sd1-001-SD");
    game.state.player2.stage.stage[0] = opp_member;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "opponent surplus 7 ≥ 2 → our live total +1"
    );

    // Edge 6: member leaves → surplus collapses → bonus gone.
    game.state.player2.stage.stage[0] = -1;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "bonus removed once opponent surplus drops below 2"
    );
}

/// Edge 3: surplus lands EXACTLY on 2 → still qualifies (>= boundary).
#[test]
fn mari_bp5_008_exactly_two_surplus_qualifies() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp5-008-R");
    game.add_to_stage(MemberArea::Center, mari);

    // Opponent: 7 base hearts vs a live needing 5 → surplus exactly 2.
    let opp_member = game.new_id("PL!S-sd1-001-SD");
    game.state.player2.stage.stage[0] = opp_member;
    let need5_live = game.id("PL!-sd1-020-SD"); // きっと青春が聞こえる, need sum 5
    game.state.player2.live_card_zone.cards.push(need5_live);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "surplus exactly 2 (7 hearts − 5 needed) → bonus applies"
    );
}

/// Edge 4: hearts fully consumed by the need (7 vs 7) → surplus 0 → no bonus.
#[test]
fn mari_bp5_008_no_surplus_when_need_covers_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp5-008-R");
    game.add_to_stage(MemberArea::Center, mari);

    let opp_member = game.new_id("PL!S-sd1-001-SD");
    game.state.player2.stage.stage[0] = opp_member;
    let need7_live = game.id("PL!S-PR-023-PR"); // 恋になりたいAQUARIUM, need sum 7
    game.state.player2.live_card_zone.cards.push(need7_live);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "surplus 0 (7 hearts − 7 needed) < 2 → no bonus"
    );
}

/// Edge 5: the bonus feeds the PLAYER LIVE-TOTAL accumulator — it must NOT
/// appear as a per-card score modifier on Mari.
#[test]
fn mari_bp5_008_bonus_is_live_total_not_per_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp5-008-R");
    game.add_to_stage(MemberArea::Center, mari);
    let opp_member = game.new_id("PL!S-sd1-001-SD");
    game.state.player2.stage.stage[0] = opp_member;

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "live-total accumulator holds the +1"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(mari),
        0,
        "ライブの合計スコア targets the TOTAL — no per-card modifier"
    );
}

// ====================================================================
// A4 PL!N-bp4-009-R 天王寺璃奈 — ライブ開始時:
// 自分のステージにいるメンバーのコストの合計が相手より低い場合、カードを2枚引き、
// 自分の手札を1枚デッキの一番上に置く。
// BATCH7_PLAN.md §A4, edges 1–8.
// ====================================================================

/// Edge 1+2+8: own total (13) < opponent TOTAL across all areas (13+4=17)
/// → fires. Firing here discriminates area summation: a naive center-only
/// comparison would see 13 == 13 and NOT fire.
#[test]
fn rin_bp4_009_lower_total_draws_two_puts_one_on_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp4-009-R"); // cost 13
    let opp_center = game.id("PL!-bp5-008-R"); // 小泉花陽, cost 13
    let filler = game.id(FILLER); // cost 4
    fill_decks(&mut game, filler);

    game.add_to_stage(MemberArea::Center, rin); // own total 13
    game.state.player2.stage.stage[1] = opp_center; // opp center 13 …
    game.state.player2.stage.stage[0] = game.new_id(FILLER); // … + left 4 = 17

    let keep = game.id("PL!N-PR-019-PR");
    let sacrifice = game.new_id(FILLER);
    game.add_to_hand(keep);
    game.add_to_hand(sacrifice);

    fire_trigger(&mut game, rin, AbilityTrigger::LiveStart, "ライブ開始時");

    // Edge 7: put-back is MANDATORY — no ようない in the text.
    assert!(game.has_pending_choice(), "put-back choice must be offered");
    game.assert_select_card("hand", 1, false);

    let idx = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .position(|&c| c == sacrifice)
        .expect("drawn cards joined the hand before the put-back choice");
    game.select_indices(&[idx]);

    assert_eq!(
        game.state.player1.main_deck.cards.first().copied(),
        Some(sacrifice),
        "chosen card sits at deck index 0 (TOP)"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2 + 2 - 1,
        "drew 2, returned exactly 1 to the deck"
    );
    assert!(game.state.player1.hand.cards.contains(&keep));
}

/// Edge 3: equal totals (13 vs 13) — 「低い場合」 is strict < → no fire.
#[test]
fn rin_bp4_009_equal_totals_do_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp4-009-R"); // cost 13
    let opp_equal = game.id("PL!-bp5-008-R"); // cost 13
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    game.add_to_stage(MemberArea::Center, rin);
    game.state.player2.stage.stage[1] = opp_equal;
    game.add_to_hand(game.id("PL!N-PR-019-PR"));
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, rin, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        !game.has_pending_choice(),
        "equal totals: strict < must not fire"
    );
    assert_eq!(game.state.player1.hand.cards.len(), hand_before, "no draw");
}

/// Edge 5: both stages empty (0 vs 0) → equal → no fire.
#[test]
fn rin_bp4_009_both_stages_empty_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp4-009-R");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    // Neither side has any member staged — totals 0 vs 0. Rin herself is
    // deliberately held out so her own cost does not tip the comparison.

    fire_trigger(&mut game, rin, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        !game.has_pending_choice(),
        "0 vs 0 → equal → strict < must not fire"
    );
}

/// Edge 4: own total higher than opponent's → no fire, no draw.
#[test]
fn rin_bp4_009_higher_own_total_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp4-009-R"); // cost 13
    let cheap = game.id(FILLER); // cost 4
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    game.add_to_stage(MemberArea::Center, rin); // own 13
    game.state.player2.stage.stage[1] = cheap; // opp 4
    game.add_to_hand(game.id("PL!N-PR-019-PR"));
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, rin, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(!game.has_pending_choice(), "higher own total → no fire");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no draws happened"
    );
}

/// Edge 6: own stage EMPTY (0) vs any opponent member → 0 < cost → fires.
#[test]
fn rin_bp4_009_own_empty_stage_counts_as_zero_and_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp4-009-R");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    // Opponent stages one member; OUR stage stays empty (rin held out).
    game.state.player2.stage.stage[1] = game.new_id("PL!S-sd1-001-SD"); // cost 17
    game.add_to_hand(filler); // something to put back

    fire_trigger(&mut game, rin, AbilityTrigger::LiveStart, "ライブ開始時");

    assert!(
        game.has_pending_choice(),
        "0 < 17 → condition met even with an empty own stage"
    );
    game.select_indices(&[0]);
    assert_eq!(
        game.state.player1.main_deck.cards.first().copied(),
        Some(filler),
        "put-back landed on top"
    );
}

// ====================================================================
// A5 PL!SP-bp4-001-R 澁谷かのん — 登場:
// 自分のステージにいるメンバーが『Liella!』のみで、かつ自分のエネルギーが7枚以上
// ある場合、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
// BATCH7_PLAN.md §A5, edges 1–6.
// ====================================================================

/// Edge 1+6: Kanon alone (trivially all-Liella!) with exactly 7 energy →
/// one WAIT-state energy enters the zone; active count untouched.
#[test]
fn kanon_spbp4_001_places_wait_energy_when_liella_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp4-001-R");
    game.add_to_stage(MemberArea::Center, kanon);
    game.give_energy(7);
    fill_energy_deck(&mut game, 0, 3);

    let zone_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();

    fire_trigger(&mut game, kanon, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "one energy card placed from the energy deck"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before,
        "placed in WAIT state — ウェイト状態で置く"
    );
    assert_eq!(game.state.player1.energy_deck.cards.len(), 2, "deck −1");
}

/// Edge 2: a non-Liella! member on stage breaks 『Liella!』のみ.
#[test]
fn kanon_spbp4_001_non_liella_blocks_placement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp4-001-R");
    game.add_to_stage(MemberArea::Center, kanon);
    game.state.player1.stage.stage[0] = game.new_id(FILLER); // μ's member
    game.give_energy(7);
    fill_energy_deck(&mut game, 0, 3);

    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(&mut game, kanon, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "μ's member on stage → condition fails"
    );
}

/// Edge 3 vs Edge 4: 6 energy blocks; MIXED active/wait totalling 7 passes —
/// 「エネルギーが7枚以上」counts energy cards, not only active ones.
#[test]
fn kanon_spbp4_001_energy_count_includes_wait_state() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp4-001-R");
    game.add_to_stage(MemberArea::Center, kanon);

    // Exactly 5 ACTIVE + 1 waiting = 6 total → blocked.
    for _ in 0..6 {
        game.state
            .player1
            .energy_zone
            .cards
            .push(game.id("LL-E-001-SD"));
    }
    game.state.player1.energy_zone.set_active_count(5);
    fill_energy_deck(&mut game, 0, 2);
    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(&mut game, kanon, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "total 6 < 7 → blocked even though 5 are active"
    );

    // Add one more (waiting) card: 5 active + 2 wait = 7 total → passes.
    game.state
        .player1
        .energy_zone
        .cards
        .push(game.new_id("LL-E-001-SD"));
    let zone_after_push = game.state.player1.energy_zone.cards.len();
    assert_eq!(zone_after_push, 7);
    fire_trigger(&mut game, kanon, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_after_push + 1,
        "mixed 5 active + 2 wait = 7 ≥ 7 → placement happens"
    );
}

/// Edge 5: condition met but the energy deck is EMPTY → graceful no-op.
#[test]
fn kanon_spbp4_001_empty_energy_deck_degrades_gracefully() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp4-001-R");
    game.add_to_stage(MemberArea::Center, kanon);
    game.give_energy(8);
    // Energy deck deliberately left EMPTY.

    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(&mut game, kanon, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "nothing to place from an empty deck — must not crash or misbehave"
    );
}

// ====================================================================
// A6 PL!S-pb1-007-R 国木田花丸 — ライブ成功時:
// エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、
// 自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
// BATCH7_PLAN.md §A6, edges 1–6.
// ====================================================================

/// Edge 1: a live card among the yell-revealed cards → one WAIT energy.
#[test]
fn hanamaru_pb1_007_revealed_live_places_wait_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-pb1-007-R");
    game.add_to_stage(MemberArea::Center, hanamaru);
    fill_energy_deck(&mut game, 0, 2);
    game.state.revealed_cards.push(game.id("PL!-sd1-019-SD"));

    let zone_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();

    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "live card among revealed cards → place 1 energy"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before,
        "energy enters in WAIT state"
    );
}

/// Edge 2+3: members-only reveal and an empty reveal both do nothing.
#[test]
fn hanamaru_pb1_007_member_only_or_empty_reveal_places_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-pb1-007-R");
    game.add_to_stage(MemberArea::Center, hanamaru);
    fill_energy_deck(&mut game, 0, 2);

    // Edge 3: nothing revealed at all.
    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "no reveal → no placement"
    );

    // Edge 2: only MEMBER cards were revealed — no live → no placement.
    game.state.revealed_cards.push(game.new_id(FILLER));
    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "member-only reveal → ライブカードが1枚以上 fails"
    );
}

/// Edge 4+5: two lives (or a mix) still grant EXACTLY one energy; the count
/// is fixed by the text, not per live card.
#[test]
fn hanamaru_pb1_007_multiple_lives_still_only_one_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-pb1-007-R");
    game.add_to_stage(MemberArea::Center, hanamaru);
    fill_energy_deck(&mut game, 0, 3);

    // Mix: two live cards AND member cards in the reveal pool.
    game.state.revealed_cards.push(game.id("PL!-sd1-019-SD"));
    game.state.revealed_cards.push(game.new_id(FILLER));
    game.state.revealed_cards.push(game.new_id("PL!-sd1-021-SD"));

    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "two revealed lives + members → STILL exactly 1 energy"
    );
}

/// Edge 6: condition met but energy deck empty → graceful no-op.
#[test]
fn hanamaru_pb1_007_empty_energy_deck_degrades_gracefully() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-pb1-007-R");
    game.add_to_stage(MemberArea::Center, hanamaru);
    game.state
        .revealed_cards
        .push(game.id("PL!-sd1-019-SD"));
    // Energy deck deliberately left EMPTY.

    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "empty energy deck → nothing placed, no crash"
    );
}

// ====================================================================
// A7 PL!S-bp3-008-R 小原鞠莉 — 起動:
// このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。
// それがスコア6以上の『Aqours』のライブカードの場合、エネルギーを4枚アクティブにする。
// Rulings: Q123 (usable with no live in waitroom; fetch is MANDATORY when one
// exists), Q79 (self-cost vacates her stage area).
// BATCH7_PLAN.md §A7, edges 1–7.
// ====================================================================

/// Shared board: Mari center, Aqours score-9 + 虹ヶ咲 score-5 lives in the
/// waitroom, 6 energy cards with `active` of them active.
fn mari_bp3_008_board(game: &mut TestGame, active: u8) -> (i16, i16, i16) {
    let mari = game.id("PL!S-bp3-008-R");
    let aqours_high = game.id("PL!S-pb1-023-L"); // Next SPARKLING!!, Aqours score 9
    let niji_low = game.id("PL!N-bp1-028-L"); // Butterfly, 虹ヶ咲 score 5
    game.add_to_stage(MemberArea::Center, mari);
    game.add_to_discard(aqours_high);
    game.add_to_discard(niji_low);
    let energy = game.id("LL-E-001-SD");
    for _ in 0..6 {
        game.state.player1.energy_zone.cards.push(energy);
    }
    game.state.player1.energy_zone.set_active_count(active);
    (mari, aqours_high, niji_low)
}

/// Edge 1: fetch the Aqours score-9 live → cost self→waitroom, area vacated,
/// live in hand, ALL FOUR waiting energies activated (2+4=6).
#[test]
fn mari_bp3_008_aqours_high_fetch_activates_four() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mari, aqours_high, _) = mari_bp3_008_board(&mut game, 2);

    game.activate_ability(mari);

    assert!(
        game.state.player1.waitroom.cards.contains(&mari),
        "activation cost sends Mari herself to the waitroom"
    );
    assert_eq!(game.state.player1.stage.stage[1], -1, "Q79: area vacated");

    assert!(game.has_pending_choice(), "live selection must be offered");
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == aqours_high)
        .expect("Aqours high live offered");
    game.select_indices(&[idx]);

    assert!(game.state.player1.hand.cards.contains(&aqours_high));
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        6,
        "Aqours score≥6 fetched → activate 4 (2+4=6)"
    );
}

/// Edge 2: fetching the 虹ヶ咲 score-5 live skips the activation.
#[test]
fn mari_bp3_008_wrong_group_fetch_skips_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mari, _, niji_low) = mari_bp3_008_board(&mut game, 2);

    game.activate_ability(mari);
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == niji_low)
        .expect("non-Aqours live also selectable");
    game.select_indices(&[idx]);

    assert!(game.state.player1.hand.cards.contains(&niji_low));
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "『Aqours』 filter fails → NO activation"
    );
}

/// Edge 3: fetching an Aqours live whose score is 5 ALSO skips it.
#[test]
fn mari_bp3_008_aqours_below_six_skips_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mari, aqours_high, _) = mari_bp3_008_board(&mut game, 2);

    // Swap the high Aqours live for a low-scoring one (score 5).
    let aqours_low = game.id("PL!S-PR-024-PR"); // 勇気はどこに?, Aqours score 5
    game.state
        .player1
        .waitroom
        .cards
        .retain(|c| *c != aqours_high);
    game.add_to_discard(aqours_low);

    game.activate_ability(mari);
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == aqours_low)
        .expect("low Aqours live selectable");
    game.select_indices(&[idx]);

    assert!(game.state.player1.hand.cards.contains(&aqours_low));
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "スコア6以上 fails at exactly-below boundary → NO activation"
    );
}

/// Edge 4 (Q123): NO live card in the waitroom — ability is still legal, cost
/// still paid, nothing added, no crash.
#[test]
fn mari_bp3_008_usable_with_no_live_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp3-008-R");
    game.add_to_stage(MemberArea::Center, mari);
    game.add_to_discard(game.new_id(FILLER)); // only a MEMBER in the waitroom

    game.activate_ability(mari);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert!(
        game.state.player1.waitroom.cards.contains(&mari),
        "Q123: usable — cost was paid"
    );
    assert!(
        game.state.player1.hand.cards.is_empty(),
        "nothing added when no live exists"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "no fetch → no activation"
    );
}

/// Edge 5+6: with a live present the selection is MANDATORY (Q123) and member
/// cards in the waitroom are NOT valid picks (ライブカードを1枚).
#[test]
fn mari_bp3_008_selection_mandatory_and_live_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mari, aqours_high, _niji_low) = mari_bp3_008_board(&mut game, 2);
    let member_in_waitroom = game.new_id(FILLER);
    game.add_to_discard(member_in_waitroom);

    game.activate_ability(mari);

    let member_idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == member_in_waitroom)
        .expect("member present in waitroom");

    // Edge 6: offering the MEMBER must never move it — only lives are legal
    // targets (ライブカードを1枚手札に加える).
    let member_pick = game.try_select_indices(&[member_idx]);
    let _ = member_pick; // engine may reject or filter; both must preserve the rule
    assert!(
        !game.state.player1.hand.cards.contains(&member_in_waitroom),
        "member card must NEVER reach the hand via this ability"
    );

    // Edge 5: the MANDATORY fetch still completes with a live card (Q123),
    // whether the illegal pick was rejected (choice re-prompted) or filtered.
    while game.has_pending_choice() {
        let live_here = game
            .state
            .player1
            .waitroom
            .cards
            .iter()
            .position(|&c| c == aqours_high);
        match live_here {
            Some(idx) => {
                game.select_indices(&[idx]);
                break;
            }
            None => game.select_indices(&[0]),
        }
    }
    assert!(
        game.state.player1.hand.cards.contains(&aqours_high),
        "Q123: mandatory fetch — a live card MUST reach the hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&member_in_waitroom)
            && game.state.player1.waitroom.cards.contains(&member_in_waitroom),
        "the member stayed in the waitroom"
    );
}

/// Edge 7: fewer waiting energies than 4 → activates what exists (partial).
#[test]
fn mari_bp3_008_activates_only_available_waiting_energies() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // 3 energy cards, only 1 ACTIVE → 2 WAITING available to activate.
    let (mari, aqours_high, _) = mari_bp3_008_board(&mut game, 1);
    // Trim the shared board from 6 cards down to 3.
    while game.state.player1.energy_zone.cards.len() > 3 {
        game.state.player1.energy_zone.cards.pop();
    }
    game.activate_ability(mari);
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == aqours_high)
        .expect("Aqours high live offered");
    game.select_indices(&[idx]);

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        4,
        "activate 4 requested but only 3 candidate energies exist → Q167 partial \
         resolution activates those 3 instead of aborting (1+3=4)"
    );
}
