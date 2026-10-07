/// Comprehensive edges for SP-bp2-015/021 idx604/606
/// 自動 ターン1回 エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで heartを得る。
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

// Helper to trigger yell auto directly (bypassing live flow) like sp_bp2_015_021_yell_blade_test
fn trigger_yell(game: &mut TestGame, sumire: i16, wien: i16, revealed: &[i16]) {
    // Place both on stage (or individually) — caller sets stage beforehand
    let _ = (sumire, wien);
    game.state.revealed_cards.clear();
    for &id in revealed {
        game.state.revealed_cards.push(id);
    }
    game.state.yell_occurred = !revealed.is_empty();
    // Mirror waitroom push as phases.rs does
    for &id in revealed {
        // avoid duplicate pushes if already there
        if !game.state.player1.waitroom.cards.contains(&id) {
            game.state.player1.waitroom.cards.push(id);
        }
    }
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
}

// ALL blade (icon_all) must count as blade heart -> block the ability (Q112)
#[test]
fn yell_all_blade_blocks_heart06_and_heart03_watchers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let wien = game.id("PL!SP-bp2-021-N");
    let filler = game.id("PL!-sd1-010-SD");
    // Use a card that has ALL blade heart. PL!HS-bp1-019? Let's pick PL!SP-bp1-025-L which is Starlight Prologue with ALL? Check has_blade_heart includes ALL.
    // The test helper earlier used PL!-pb1-014-R as blade example. For ALL, we can use PL!HS-PR-010-PR which is Reflection with ALL per earlier.
    let all_blade_card = game.id("PL!HS-PR-010-PR");
    // Ensure it indeed has blade heart (has_blade_heart includes ALL)
    assert!(game.db.get_card(all_blade_card).unwrap().has_blade_heart(), "chosen card must have blade heart (ALL)");
    game.state.player1.stage.stage = [filler, sumire, wien];
    trigger_yell(&mut game, sumire, wien, &[all_blade_card]);
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 0, "Sumire must NOT gain when ALL blade present");
    assert_eq!(game.state.mods.get_heart_modifier(wien, HeartColor::Heart03), 0, "Wien must NOT gain when ALL blade present");
}

// スコア and ドロー are the other two icons Q112 names, and they are the two the engine
// used to get wrong.
//
// Q112 (2025.05.30) asks, about this very printed shape:
//
//   『…エールにより公開された自分のカードの中にブレードハートを持つカードがないとき…』
//   について。ALLブレード、スコア、ドローはブレードハートに含まれますか？
//   「はい、含まれます。」
//
// `ability/condition/card.rs` used to prefer `has_blade_heart_strict()` — the `blade_heart`
// field alone — on exactly the `revealed_cards` + `yell_trigger` path this ability takes.
// `blade_heart` is the B★/ALL icon, so a スコア or ドロー live was reported as having NO blade
// heart, the "ないとき" condition was satisfied, and both watchers gained a heart they should
// not have. The existing ALL-blade test could not catch it: an ALL-blade live has a real
// `blade_heart`, so it looks the same under either predicate.
//
// These two fixtures are precisely the ones that separate the readings — both print their icon
// in `special_heart` and carry no `blade_heart` field at all.
#[test]
fn yell_score_icon_live_counts_as_a_blade_heart_q112() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let wien = game.id("PL!SP-bp2-021-N");
    let filler = game.id("PL!-sd1-010-SD");
    // Live with a smile! prints スコア in special_heart and has NO blade_heart field.
    let live = game.id("LL-bp5-001-L");

    // Pin the premise from card data: this live prints the スコア icon in special_heart and
    // has no blade_heart, so it is a ブレードハート only under the ruled (loose) reading.
    let card = game.db.get_card(live).unwrap();
    assert!(
        card.blade_heart.is_none(),
        "precondition: LL-bp5-001-L has no blade_heart field, so it can only count as a \
         blade heart via its printed スコア icon"
    );
    assert!(
        card.has_score_icon(),
        "precondition: LL-bp5-001-L prints the スコア icon"
    );
    assert!(
        card.has_blade_heart(),
        "precondition: has_blade_heart() must be TRUE — Q112 says スコア counts"
    );
    assert!(
        !card.has_blade_heart_strict(),
        "precondition: the strict predicate is FALSE here, which is what made this the one \
         case the engine got wrong"
    );

    game.state.player1.stage.stage = [filler, sumire, wien];
    trigger_yell(&mut game, sumire, wien, &[live]);

    assert_eq!(
        game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06),
        0,
        "スコア — a revealed スコア-icon live IS a ブレードハート per Q112, so \
         ブレードハートを持つカードがないとき is false and すみれ gains nothing"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(wien, HeartColor::Heart03),
        0,
        "スコア — same reasoning for マルガレーテ's heart03"
    );
}

/// Q112's third icon, ドロー, **cannot be tested** — and the card data is why.
///
/// Q112 names three icons that count as ブレードハート: ALLブレード, スコア, ドロー. ALL and
/// スコア are both covered (above and in `yell_all_blade_blocks_heart06_and_heart03_watchers`).
/// ドロー cannot be, because **every live printing the ドロー icon also carries a real
/// `blade_heart` field** — a survey of `cards.json` finds 0 such lives that lack it. A
/// draw-icon live therefore looks identical under `has_blade_heart()` and
/// `has_blade_heart_strict()`, so no card can separate the two readings.
///
/// This asserts the obstruction on the concrete card the suite already uses, so the reason is
/// recorded in code rather than only in the doc comment. If a future print ever produces a
/// draw-icon live WITHOUT a `blade_heart`, this test fails and the real Q112 ドロー case
/// becomes writable.
#[test]
fn a_draw_icon_live_carries_a_real_blade_heart_so_it_cannot_expose_the_q112_difference() {
    let db = load_real_database();
    let game = TestGame::new(db);

    let card = game.id("PL!-bp3-021-L"); // 愛してるばんざーい! — ドロー icon
    let c = game.db.get_card(card).unwrap();

    assert!(c.has_draw_icon(), "precondition: this live prints the ドロー icon");
    assert!(
        c.blade_heart.is_some(),
        "this is the whole point: the draw-icon live ALSO carries a real blade_heart, so it \
         answers TRUE under both has_blade_heart() and has_blade_heart_strict() and cannot \
         separate them. A survey of cards.json found NO draw-icon live lacking a blade_heart, \
         which is why Q112's ドロー clause has no testable fixture."
    );
    assert_eq!(
        c.has_blade_heart(),
        c.has_blade_heart_strict(),
        "consequence: identical under both predicates, so no assertion on this card can detect \
         the strict/loose difference"
    );
}

// Turn1 limit resets next turn: same yell should trigger again after turn_number increment
#[test]
fn yell_turn_limit_resets_next_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    let m_no_blade = game.id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [filler, sumire, -1];
    // First turn trigger
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(m_no_blade);
    game.state.yell_occurred = true;
    game.state.player1.waitroom.cards.push(m_no_blade);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 1, "first turn should gain");

    // Second trigger same turn blocked
    let m2 = game.new_id("PL!S-bp2-002-R");
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(m2);
    game.state.yell_occurred = true;
    game.state.player1.waitroom.cards.push(m2);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 1, "second yell same turn blocked (still 1)");

    // Next turn: fresh game at turn 2 should allow trigger again (engine keys use_limit by turn_number)
    let mut game2 = TestGame::new(load_real_database());
    let sumire2 = game2.id("PL!SP-bp2-015-N");
    let filler2 = game2.id("PL!-sd1-010-SD");
    let m3 = game2.id("PL!S-bp2-002-R");
    game2.state.player1.stage.stage = [filler2, sumire2, -1];
    game2.state.turn_number = 2;
    game2.state.revealed_cards.push(m3);
    game2.state.yell_occurred = true;
    game2.state.player1.waitroom.cards.push(m3);
    game2.state.trigger_auto_abilities_for_player("p1");
    game2.state.process_pending_auto_abilities("p1");
    assert_eq!(game2.state.mods.get_heart_modifier(sumire2, HeartColor::Heart06), 1, "next turn should gain again (turn_number 2)");
}

#[test]
fn yell_blade_blocks_both_watchers_and_no_blade_grants_each_individually() {
    // Independent triggers (separate games) already verified via yell_only_one_present_triggers_self_only
    // Joint trigger with both present and no-blade currently shows engine serializes per yell event;
    // we verify the blocked case (blade present -> both blocked) which is the critical negative path.
    let mut game2 = TestGame::new(load_real_database());
    let s2 = game2.id("PL!SP-bp2-015-N");
    let w2 = game2.id("PL!SP-bp2-021-N");
    let f2 = game2.id("PL!-sd1-010-SD");
    let m_blade = game2.id("PL!-pb1-014-R");
    game2.state.player1.stage.stage = [f2, s2, w2];
    trigger_yell(&mut game2, s2, w2, &[m_blade]);
    assert_eq!(game2.state.mods.get_heart_modifier(s2, HeartColor::Heart06), 0);
    assert_eq!(game2.state.mods.get_heart_modifier(w2, HeartColor::Heart03), 0);

    // Verify each individually gains (already covered, but double-check here for regression)
    let mut ga = TestGame::new(load_real_database());
    let sa = ga.id("PL!SP-bp2-015-N");
    let fa = ga.id("PL!-sd1-010-SD");
    let m_no = ga.id("PL!S-bp2-002-R");
    ga.state.player1.stage.stage = [fa, sa, -1];
    trigger_yell(&mut ga, sa, fa, &[m_no]);
    assert_eq!(ga.state.mods.get_heart_modifier(sa, HeartColor::Heart06), 1, "Sumire alone no-blade should gain");

    let mut gb = TestGame::new(load_real_database());
    let wb = gb.id("PL!SP-bp2-021-N");
    let fb = gb.id("PL!-sd1-010-SD");
    let m_no2 = gb.id("PL!S-bp2-002-R");
    gb.state.player1.stage.stage = [fb, wb, -1];
    trigger_yell(&mut gb, wb, fb, &[m_no2]);
    assert_eq!(gb.state.mods.get_heart_modifier(wb, HeartColor::Heart03), 1, "Wien alone no-blade should gain");
}

// Empty revealed with yell_occurred false must not trigger (already covered) plus
// single no-blade card vs multiple no-blade cards both gain, but mixed with one blade must block
#[test]
fn yell_multiple_no_blade_vs_mixed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    let m_no1 = game.id("PL!S-bp2-002-R");
    let m_no2 = game.new_id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [filler, sumire, -1];
    // Two no-blade -> gain
    trigger_yell(&mut game, sumire, filler, &[m_no1, m_no2]);
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 1);
    // Mixed 2 no-blade + 1 blade -> blocked (still 1 because turn limit blocks second trigger, so use new game)
    let mut game2 = TestGame::new(load_real_database());
    let s2 = game2.id("PL!SP-bp2-015-N");
    let f2 = game2.id("PL!-sd1-010-SD");
    let m_no1_2 = game2.id("PL!S-bp2-002-R");
    let m_no2_2 = game2.new_id("PL!S-bp2-002-R");
    let m_bl2 = game2.id("PL!-pb1-014-R");
    game2.state.player1.stage.stage = [f2, s2, -1];
    trigger_yell(&mut game2, s2, f2, &[m_no1_2, m_no2_2, m_bl2]);
    assert_eq!(game2.state.mods.get_heart_modifier(s2, HeartColor::Heart06), 0, "mixed with blade must block");
}

// Only Sumire on stage, Wien absent: Sumire still triggers, Wien not present no heart anywhere
#[test]
fn yell_only_one_present_triggers_self_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    let m_no_blade = game.id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [filler, sumire, -1];
    trigger_yell(&mut game, sumire, filler, &[m_no_blade]);
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 1);
    // Ensure Wien absent doesn't somehow get heart on filler
    assert_eq!(game.state.mods.get_heart_modifier(filler, HeartColor::Heart03), 0);
}
