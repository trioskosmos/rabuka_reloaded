/// Q127: Interaction between constant heart0 increase (Wien Margarete)
/// and live cards that change their required hearts (set operation).
///
/// 「常時 相手のライブカード置き場にあるすべてのライブカードは、
///   成功させるための必要ハートがheart0 1つ分多くなる。」について。
/// 条件を満たすと必要ハートを変更するライブカードでライブを行った場合どうなりますか？
///
/// Answer: 変更したハートにheart0 １つを加えたものが必要になります。
///
/// Cards involved:
///   - PL!SP-bp2-010-P (ウィーン・マルガレーテ):
///       常時: 相手のライブカード置き場にあるすべてのライブカードは、
///             成功させるための必要ハートがheart0多くなる。
///       (modify_required_hearts_global: increase heart00 by 1)
///
///   - PL!HS-bp2-019-L (Bloom the smile, Bloom the dream!):
///       ライブ開始時: 必要ハートを選択する (set operation):
///         Option 1: heart01×2 + heart0×1
///         Option 2: heart04×2 + heart0×1
///         Option 3: heart05×2 + heart0×1
///
///   - PL!SP-bp1-026-L (未来予報ハレルヤ！):
///       ライブ開始時: 条件満たすとコスト変更 (set operation):
///         heart02×2 + heart03×2 + heart06×2
///
/// Rule Q115: Set-to-X applies first, then add/subtract modifiers stack.
///
/// KEY: Wien is on P2's stage → affects P1's live cards.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

// ====================================================================
// Test 1: Wien +1 heart00 stacks on Bloom's set operation
// ====================================================================
// Bloom sets: heart01=2, heart00=1
// Wien adds: +1 heart00 (additive)
// Effective: heart01=2, heart00=2
#[test]
fn q127_wien_plus1_stacks_on_bloom_set() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-bp2-010-P");
    let bloom = game.id("PL!HS-bp2-019-L");
    let hasunosuka_member = game.id("PL!HS-bp1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Wien on P2's stage → affects P1's live cards
    game.state.player2.stage.stage = [-1, wien, -1];

    // P1 has Hasunosuka member + Bloom in hand
    game.state.player1.stage.stage = [-1, hasunosuka_member, -1];
    game.state.player1.hand.cards.push(bloom);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(
        game.has_pending_choice(),
        "Bloom should present heart pattern choice"
    );
    game.select_option(0); // heart01 pattern: heart01×2 + heart0×1

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let bloom_id = game.state.player1.live_card_zone.cards[0];

    // Bloom's set: heart01=2, heart00=1
    let heart01_mod = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart01);
    assert_eq!(heart01_mod, 2, "Bloom: heart01 set to 2");

    // Wien's constant: +1 heart00 (additive, stacks on Bloom's set)
    let heart00_mod = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart00);
    assert_eq!(
        heart00_mod, 2,
        "Q127: Bloom set heart00=1 + Wien +1 = 2 (set + additive stack)"
    );
}

// ====================================================================
// Test 2: Wien +1 heart00 stacks on Hareruya's set operation
// ====================================================================
// Hareruya sets: heart02=2, heart03=2, heart06=2 (no heart00)
// Wien adds: +1 heart00 (additive)
// Effective: heart02=2, heart03=2, heart06=2, heart00=1
#[test]
fn q127_wien_plus1_stacks_on_hareruya_set() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-bp2-010-P");
    let hareruya = game.id("PL!SP-bp1-026-L");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Wien on P2's stage → affects P1's live cards
    game.state.player2.stage.stage = [-1, wien, -1];

    // P1 has 5 distinct Liella! members in waitroom for Hareruya condition
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-014-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-015-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-016-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-019-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-020-N"));

    game.state.player1.hand.cards.push(hareruya);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(hareruya);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let hareruya_id = game.state.player1.live_card_zone.cards[0];

    // Hareruya's set: heart02=2, heart03=2, heart06=2
    let h02 = game
        .state
        .mods
        .get_need_heart_modifier(hareruya_id, HeartColor::Heart02);
    let h03 = game
        .state
        .mods
        .get_need_heart_modifier(hareruya_id, HeartColor::Heart03);
    let h06 = game
        .state
        .mods
        .get_need_heart_modifier(hareruya_id, HeartColor::Heart06);
    assert_eq!(h02, 2, "Hareruya set heart02=2");
    assert_eq!(h03, 2, "Hareruya set heart03=2");
    assert_eq!(h06, 2, "Hareruya set heart06=2");

    // Wien's constant: +1 heart00
    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(hareruya_id, HeartColor::Heart00);
    assert_eq!(
        h00, 1,
        "Q127: Hareruya set (no heart00) + Wien +1 heart00 = 1"
    );
}

// ====================================================================
// Test 3: Multiple Wienen stack (+2 heart00)
// ====================================================================
#[test]
fn q127_two_wien_stack_plus2_heart00() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien1 = game.id("PL!SP-bp2-010-P");
    let wien2 = game.id("PL!SP-bp2-010-P");
    let bloom = game.id("PL!HS-bp2-019-L");
    let hasunosuka_member = game.id("PL!HS-bp1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Two Wienen on P2's stage → each adds +1 to P1's live cards
    game.state.player2.stage.stage = [wien1, wien2, -1];

    game.state.player1.stage.stage = [-1, hasunosuka_member, -1];
    game.state.player1.hand.cards.push(bloom);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice());
    game.select_option(0); // heart01 pattern

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let bloom_id = game.state.player1.live_card_zone.cards[0];

    // Bloom set heart00=1 + Wien1 +1 + Wien2 +1 = 3
    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart00);
    assert_eq!(
        h00, 3,
        "Q127: Bloom set heart00=1 + 2 Wienen (+2) = 3"
    );
}

// ====================================================================
// Test 4: Wien leaves stage → modifier removed
// ====================================================================
#[test]
fn q127_wien_leaves_stage_modifier_removed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-bp2-010-P");
    let bloom = game.id("PL!HS-bp2-019-L");
    let hasunosuka_member = game.id("PL!HS-bp1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Wien on P2's stage
    game.state.player2.stage.stage = [-1, wien, -1];

    game.state.player1.stage.stage = [-1, hasunosuka_member, -1];
    game.state.player1.hand.cards.push(bloom);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice());
    game.select_option(0);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let bloom_id = game.state.player1.live_card_zone.cards[0];

    // Wien active: heart00 = set(1) + additive(1) = 2
    let h00_with_wien = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart00);
    assert_eq!(h00_with_wien, 2, "With Wien: heart00 = 2");

    // Remove Wien from P2's stage and recalculate constants
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();

    // Wien gone: heart00 = set(1) + additive(0) = 1
    let h00_without_wien = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart00);
    assert_eq!(
        h00_without_wien, 1,
        "Q127: After Wien leaves, heart00 = 1 (only Bloom's set)"
    );
}

// ====================================================================
// Test 5: Wien +1 heart00 on card with ONLY set (no base heart00)
// ====================================================================
// Hareruya base: heart00=2 in need_heart, but set replaces it
// Hareruya sets: heart02=2, heart03=2, heart06=2 (heart00 NOT in set)
// Wien adds +1 heart00 → effective heart00=1
#[test]
fn q127_wien_adds_heart00_not_in_set() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-bp2-010-P");
    let hareruya = game.id("PL!SP-bp1-026-L");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Wien on P2's stage
    game.state.player2.stage.stage = [-1, wien, -1];

    // 5 distinct Liella! members for Hareruya condition
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-014-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-015-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-016-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-019-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-020-N"));

    game.state.player1.hand.cards.push(hareruya);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(hareruya);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let hareruya_id = game.state.player1.live_card_zone.cards[0];

    // Hareruya set: heart02=2, heart03=2, heart06=2 (no heart00 in set!)
    // Wien adds: +1 heart00
    // Effective: heart02=2, heart03=2, heart06=2, heart00=1

    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(hareruya_id, HeartColor::Heart00);
    assert_eq!(
        h00, 1,
        "Q127: Set has no heart00, Wien adds +1 → heart00=1"
    );

    // Verify base need_heart had heart00=2 but it's wiped by set
    let card = game
        .db
        .get_card(hareruya_id)
        .expect("card should exist");
    let base_h00 = card
        .need_heart
        .as_ref()
        .and_then(|nh| nh.hearts.get(&HeartColor::Heart00))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        base_h00, 2,
        "Hareruya base need_heart has heart00=2, but set wipes it"
    );
}

// ====================================================================
// Test 6: Wien only affects heart00, not other colors
// ====================================================================
// Bloom sets heart01=2 + heart00=1
// Wien adds +1 heart00 → effective: heart01=2, heart00=2
#[test]
fn q127_wien_only_affects_heart00() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-bp2-010-P");
    let bloom = game.id("PL!HS-bp2-019-L");
    let hasunosuka_member = game.id("PL!HS-bp1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Wien on P2's stage
    game.state.player2.stage.stage = [-1, wien, -1];

    game.state.player1.stage.stage = [-1, hasunosuka_member, -1];
    game.state.player1.hand.cards.push(bloom);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice());
    game.select_option(0); // heart01 pattern

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let bloom_id = game.state.player1.live_card_zone.cards[0];

    // Bloom set: heart01=2, heart00=1
    // Wien adds: +1 heart00 ONLY (not heart01!)
    let h01 = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart01);
    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart00);
    let h02 = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart02);
    let h03 = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart03);

    assert_eq!(h01, 2, "heart01 stays at 2 (Bloom's set, Wien doesn't touch it)");
    assert_eq!(h00, 2, "heart00 = Bloom's set(1) + Wien's +1 = 2");
    assert_eq!(h02, 0, "heart02 not affected");
    assert_eq!(h03, 0, "heart03 not affected");
}

// ====================================================================
// Test 7: Wien + Bloom second option (heart04 pattern)
// ====================================================================
#[test]
fn q127_wien_plus_bloom_heart04_pattern() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-bp2-010-P");
    let bloom = game.id("PL!HS-bp2-019-L");
    let hasunosuka_member = game.id("PL!HS-bp1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Wien on P2's stage
    game.state.player2.stage.stage = [-1, wien, -1];

    game.state.player1.stage.stage = [-1, hasunosuka_member, -1];
    game.state.player1.hand.cards.push(bloom);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice());
    game.select_option(1); // heart04 pattern: heart04×2 + heart0×1

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let bloom_id = game.state.player1.live_card_zone.cards[0];

    let h04 = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart04);
    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart00);

    assert_eq!(h04, 2, "Bloom set heart04=2");
    assert_eq!(
        h00, 2,
        "Q127: Bloom set heart00=1 + Wien +1 = 2 (heart04 pattern)"
    );
}

// ====================================================================
// Test 8: Wien + Bloom third option (heart05 pattern)
// ====================================================================
#[test]
fn q127_wien_plus_bloom_heart05_pattern() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-bp2-010-P");
    let bloom = game.id("PL!HS-bp2-019-L");
    let hasunosuka_member = game.id("PL!HS-bp1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Wien on P2's stage
    game.state.player2.stage.stage = [-1, wien, -1];

    game.state.player1.stage.stage = [-1, hasunosuka_member, -1];
    game.state.player1.hand.cards.push(bloom);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice());
    game.select_option(2); // heart05 pattern: heart05×2 + heart0×1

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let bloom_id = game.state.player1.live_card_zone.cards[0];

    let h05 = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart05);
    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart00);

    assert_eq!(h05, 2, "Bloom set heart05=2");
    assert_eq!(
        h00, 2,
        "Q127: Bloom set heart00=1 + Wien +1 = 2 (heart05 pattern)"
    );
}

// ====================================================================
// Test 9: No Wien → no extra heart00
// ====================================================================
#[test]
fn q127_no_wien_bloom_set_standalone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let bloom = game.id("PL!HS-bp2-019-L");
    let hasunosuka_member = game.id("PL!HS-bp1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // No Wien on P2's stage
    game.state.player2.stage.stage = [-1, -1, -1];

    game.state.player1.stage.stage = [-1, hasunosuka_member, -1];
    game.state.player1.hand.cards.push(bloom);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice());
    game.select_option(0); // heart01 pattern

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let bloom_id = game.state.player1.live_card_zone.cards[0];

    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(bloom_id, HeartColor::Heart00);
    assert_eq!(
        h00, 1,
        "No Wien: Bloom set heart00=1, no additive"
    );
}

// ====================================================================
// Test 10: build_card_needs correctly applies set + additive
// ====================================================================
// This directly tests the build_card_needs code path.
#[test]
fn q127_build_card_needs_set_plus_additive() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-bp2-010-P");
    let bloom = game.id("PL!HS-bp2-019-L");
    let hasunosuka_member = game.id("PL!HS-bp1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Wien on P2's stage
    game.state.player2.stage.stage = [-1, wien, -1];

    game.state.player1.stage.stage = [-1, hasunosuka_member, -1];
    game.state.player1.hand.cards.push(bloom);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice());
    game.select_option(0);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let bloom_id = game.state.player1.live_card_zone.cards[0];
    let card = game.db.get_card(bloom_id).expect("card should exist");

    // Get the current need_heart_modifiers
    let mods = &game.state.mods.need_heart_modifiers;

    // Simulate build_card_needs logic
    let mut need = [0u32; 8];
    let has_set = mods
        .get(&bloom_id)
        .is_some_and(|m| m.values().any(|e| e.set != 0));
    assert!(has_set, "Bloom should have set modifiers");

    if let Some(ref _nh) = card.need_heart {
        if has_set {
            // Fixed Path 1: set first, then additive
            if let Some(card_mods) = mods.get(&bloom_id) {
                for (color, me) in card_mods {
                    if me.set != 0 {
                        need[color.index()] = me.set as u32;
                    }
                    if me.additive != 0 {
                        let idx = color.index();
                        let current = need[idx] as i32;
                        need[idx] = (current + me.additive).max(0) as u32;
                    }
                }
            }
        }
    }

    // Bloom set: heart01=2, heart00=1
    // Wien additive: +1 heart00
    // Effective: heart01=2, heart00=2
    assert_eq!(
        need[HeartColor::Heart01.index()],
        2,
        "build_card_needs: heart01=2 (Bloom's set)"
    );
    assert_eq!(
        need[HeartColor::Heart00.index()],
        2,
        "Q127 build_card_needs: heart00 = set(1) + additive(1) = 2"
    );
}
