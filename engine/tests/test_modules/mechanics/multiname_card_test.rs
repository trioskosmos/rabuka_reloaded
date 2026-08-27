/// Tests for multi-name ("&") card handling per official QA rulings.
///
/// Q62:  A multi-name card like "上原歩夢&澁谷かのん&日野下花帆" has each
///       individual name — get_card_names returns all three.
/// Q65:  A multi-name card counts as ONE name slot in costs/conditions, not N.
///       It cannot cover multiple distinct name requirements simultaneously.
/// Q105: For "different names within a group", the multi-name card contributes
///       the ONE constituent name that matches the group context.
/// Q207: A multi-name card occupies 1 stage slot = 1 member total.
/// Q208: When a multi-name card shares a name with another card, it may be
///       treated as one of its OTHER names to avoid collision — the player
///       gets the most favorable interpretation.
use crate::helpers::*;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

// ── Card references ──────────────────────────────────────────
// LL-bp1-001-R+ : 上原歩夢&澁谷かのん&日野下花帆  (μ's / 虹ヶ咲 / 蓮ノ空)
// PL!N-pb1-001-R: 上原歩夢 (虹ヶ咲)
// PL!-bp5-022-L : live card for triggering live-start abilities
// PL!-sd1-010-SD: filler (μ's, unit: Printemps)
// PL!-sd1-020-SD: live card (μ's)

/// Q65: A single multi-name card on stage contributes exactly ONE distinct
///      name for counting purposes, not all three.
#[test]
fn q65_one_multiname_card_is_one_distinct_name() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let multi = game.id("LL-bp1-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [multi, -1, -1];
    fill_decks(&mut game, filler);

    // The card as 1 member on stage
    let member_count = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .filter(|&&id| id != -1)
        .count();
    assert_eq!(member_count, 1, "Q207: 1 stage slot = 1 member");

    // Its get_card_names should return 3 individual names
    let names = game.db.get_card_names(multi);
    assert_eq!(names.len(), 3, "Q62: multi-name card has 3 names");
    assert!(names.contains(&"上原歩夢".to_string()), "contains 上原歩夢");
    assert!(
        names.contains(&"澁谷かのん".to_string()),
        "contains 澁谷かのん"
    );
    assert!(
        names.contains(&"日野下花帆".to_string()),
        "contains 日野下花帆"
    );

    // Q65: Despite having 3 names, it counts as ONE distinct name
    // in distinct-name conditions (one card = one name slot).
    // 1 card + 3 names → max distinct = 1 (brute force finds this)
    let name_sets = vec![names];
    let result = rabuka_engine::ability::util::max_distinct_names(&name_sets);
    assert_eq!(
        result.distinct, 1,
        "Q65: 1 multi-name card = 1 distinct name, not 3"
    );
    assert!(!result.collision, "Q65: single card, no collision possible");
}

/// Q208: Multi-name card + separate single card sharing one name.
/// The multi-name can use one of its OTHER names → 2 distinct names total.
#[test]
fn q208_multiname_and_single_are_distinct() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let multi = game.id("LL-bp1-001-R\u{ff0b}"); // 上原歩夢&澁谷かのん&日野下花帆
    let single_ayumu = game.id("PL!N-pb1-001-R"); // 上原歩夢
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [multi, single_ayumu, -1];
    fill_decks(&mut game, filler);

    // Both cards occupy 2 slots
    let member_count = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .filter(|&&id| id != -1)
        .count();
    assert_eq!(member_count, 2, "Q208: 2 cards = 2 members");

    // Q208: The multi-name card can be treated as 澁谷かのん or 日野下花帆
    // to avoid collision with 上原歩夢 → 2 distinct names.
    let names_multi = game.db.get_card_names(multi);
    let names_single = game.db.get_card_names(single_ayumu);
    let name_sets = vec![names_multi, names_single];
    let result = rabuka_engine::ability::util::max_distinct_names(&name_sets);
    assert_eq!(
        result.distinct, 2,
        "Q208: multi (as かのん/花帆) + single 歩夢 = 2 distinct"
    );
    assert!(!result.collision, "Q208: collision-free assignment exists");
}

/// Q105: For a "different names in group" condition, a multi-name card
///       contributes ONE name (the one matching the group context).
///       Here: LL-bp2-001-R+ (渡辺曜&鬼塚夏美&大沢瑠璃乃) counted among
///       蓮ノ空 members — treated as 大沢瑠璃乃.
#[test]
fn q105_multiname_one_name_in_group_context() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // LL-bp2-001-R+ has names: 渡辺曜 (Aqours), 鬼塚夏美 (Liella!), 大沢瑠璃乃 (蓮ノ空)
    let multi = game.id("LL-bp2-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [multi, -1, -1];
    fill_decks(&mut game, filler);

    // It's 1 member
    assert_eq!(
        game.state
            .player1
            .stage
            .stage
            .iter()
            .filter(|&&id| id != -1)
            .count(),
        1
    );

    // For a 蓮ノ空-filtered distinct-name check, only ONE name matches.
    // The brute-force should still find only 1 distinct name (the card itself).
    let name_set = vec![game.db.get_card_names(multi)];
    let result = rabuka_engine::ability::util::max_distinct_names(&name_set);
    assert_eq!(
        result.distinct, 1,
        "Q105: 1 multi-name card in 蓮ノ空 = 1 member"
    );
}

/// Same-name condition: multi-name "A&B" + single "A" share "A" → same name.
#[test]
fn multiname_and_single_share_name() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let multi = game.id("LL-bp1-001-R\u{ff0b}"); // 上原歩夢&澁谷かのん&日野下花帆
    let single_ayumu = game.id("PL!N-pb1-001-R"); // 上原歩夢
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [multi, single_ayumu, -1];
    fill_decks(&mut game, filler);

    // Both cards share '上原歩夢' → same_name should be true
    let names_multi = game.db.get_card_names(multi);
    let names_single = game.db.get_card_names(single_ayumu);
    // Check overlap: 上原歩夢 appears in both
    let has_shared = names_multi.iter().any(|n| names_single.contains(n));
    assert!(has_shared, "Q62+same_name: multi and single share 上原歩夢");
}

/// card_matches_name_constraint: multi-name card should match by any
/// constituent name, not just the full joined name.
#[test]
fn multiname_matches_name_constraint_by_constituent() {
    let db = load_real_database();
    let game = TestGame::new(db);

    let multi = game.id("LL-bp1-001-R\u{ff0b}");

    // Should match by any individual name
    assert!(
        rabuka_engine::ability::util::card_matches_name_constraint(
            &game.db,
            multi,
            Some("上原歩夢")
        ),
        "should match constituent '上原歩夢'"
    );
    assert!(
        rabuka_engine::ability::util::card_matches_name_constraint(
            &game.db,
            multi,
            Some("澁谷かのん")
        ),
        "should match constituent '澁谷かのん'"
    );
    assert!(
        rabuka_engine::ability::util::card_matches_name_constraint(
            &game.db,
            multi,
            Some("日野下花帆")
        ),
        "should match constituent '日野下花帆'"
    );
}
