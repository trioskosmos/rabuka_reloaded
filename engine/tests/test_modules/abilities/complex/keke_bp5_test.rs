use crate::helpers::*;
use rabuka_engine::core::game_modifiers::{CardOrientation, ModifierEntry};
use rabuka_engine::zones::MemberArea;

/// Test for PL!SP-bp5-002-R＋ 唐 可可
/// [起動][左サイド][ターン1回] このメンバーをウェイトにする：カードを3枚引き、手札を2枚控え室に置く。
/// これにより控え室に置いたカードの中にブレードハートを持たないメンバーカードが1枚以上ある場合、このメンバーをアクティブにする。
/// 2枚ある場合、さらにライブ終了時まで、ブレードブレードを得る。

#[test]
fn keke_discard_0_no_blade_heart_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let keke = game.id("PL!SP-bp5-002-R＋");
    let live1 = game.id("PL!-sd1-020-SD"); // Live card
    let live2 = game.new_id("PL!-sd1-020-SD"); // Live card
    let filler_deck = game.id("PL!-sd1-010-SD"); // Filler member

    // Setup stage (Keke on Left)
    game.add_to_stage(MemberArea::LeftSide, keke);

    // Setup hand with 2 live cards to discard
    game.add_to_hand(live1);
    game.add_to_hand(live2);

    // Setup deck to draw 3 cards
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler_deck);
    }

    // Activate Keke's ability
    game.activate_ability(keke);

    // Keke should be wait as a cost
    assert_eq!(
        game.state.mods.orientation_modifiers.get(&keke),
        Some(&CardOrientation::Wait),
        "Keke should be wait after paying cost"
    );

    // We should be prompted to discard 2 cards
    assert_eq!(game.pending_choice_type().as_deref(), Some("SelectCard"));

    // Select the 2 live cards to discard
    game.select_indices(&[0, 1]);

    // Check resolution
    assert_eq!(
        game.state.mods.orientation_modifiers.get(&keke),
        Some(&CardOrientation::Wait),
        "Keke should remain wait because 0 no-blade-heart members were discarded"
    );

    let blades = game
        .state
        .mods
        .blade_modifiers
        .get(&keke)
        .map_or(0, ModifierEntry::total);
    assert_eq!(blades, 0, "Should have 0 blades");
}

#[test]
fn keke_discard_1_no_blade_heart_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let keke = game.id("PL!SP-bp5-002-R＋");
    let live1 = game.id("PL!-sd1-020-SD"); // Live card
                                           // Use a member WITHOUT blade heart (PL!-bp6-001-R＋ has no blade_heart, no blade)
    let no_bh_member = game.id("PL!-bp6-001-R\u{ff0b}");
    let filler_deck = game.new_id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::LeftSide, keke);

    game.add_to_hand(live1);
    game.add_to_hand(no_bh_member);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler_deck);
    }

    game.activate_ability(keke);

    assert_eq!(
        game.state.mods.orientation_modifiers.get(&keke),
        Some(&CardOrientation::Wait),
        "Keke should be wait after paying cost"
    );

    game.select_indices(&[0, 1]);

    assert_ne!(
        game.state.mods.orientation_modifiers.get(&keke),
        Some(&CardOrientation::Wait),
        "Keke should become active after discarding 1 no-blade-heart member"
    );

    let blades = game
        .state
        .mods
        .blade_modifiers
        .get(&keke)
        .map_or(0, ModifierEntry::total);
    assert_eq!(blades, 0, "Should have 0 blades");
}

#[test]
fn keke_discard_2_no_blade_heart_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let keke = game.id("PL!SP-bp5-002-R\u{ff0b}");
    let no_bh_member1 = game.id("PL!-bp6-001-R\u{ff0b}");
    let no_bh_member2 = game.new_id("PL!-bp6-001-R\u{ff0b}");
    let filler_deck = game.new_id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::LeftSide, keke);

    game.add_to_hand(no_bh_member1);
    game.add_to_hand(no_bh_member2);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler_deck);
    }

    game.activate_ability(keke);

    assert_eq!(
        game.state.mods.orientation_modifiers.get(&keke),
        Some(&CardOrientation::Wait),
        "Keke should be wait after paying cost"
    );

    game.select_indices(&[0, 1]);

    assert_ne!(
        game.state.mods.orientation_modifiers.get(&keke),
        Some(&CardOrientation::Wait),
        "Keke should become active after discarding 2 no-blade-heart members"
    );

    let blades = game
        .state
        .mods
        .blade_modifiers
        .get(&keke)
        .map_or(0, ModifierEntry::total);
    assert_eq!(blades, 2, "Should gain 2 blades");
}

/// Discard 2 PL!SP-sd1-011-P (鬼塚冬毬, has blade:1 but no blade_heart).
/// According to the ability text, only cards that "do not have blade heart"
/// count toward the condition. PL!SP-sd1-011-P has blade_heart=None, so
/// it should be counted as a member without blade heart.
#[test]
fn keke_discard_2_tomari_no_blade_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let keke = game.id("PL!SP-bp5-002-R\u{ff0b}");
    let tomari1 = game.id("PL!SP-sd1-011-P");
    let tomari2 = game.new_id("PL!SP-sd1-011-P");
    let filler_deck = game.new_id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::LeftSide, keke);

    game.add_to_hand(tomari1);
    game.add_to_hand(tomari2);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler_deck);
    }

    game.activate_ability(keke);

    assert_eq!(
        game.state.mods.orientation_modifiers.get(&keke),
        Some(&CardOrientation::Wait),
        "Keke should be wait after paying cost"
    );

    game.select_indices(&[0, 1]);

    assert_ne!(
        game.state.mods.orientation_modifiers.get(&keke),
        Some(&CardOrientation::Wait),
        "Keke should become active after discarding 2 no-blade-heart members"
    );

    let blades = game
        .state
        .mods
        .blade_modifiers
        .get(&keke)
        .map_or(0, ModifierEntry::total);
    // PL!SP-sd1-011-P has blade=1 but blade_heart=None → has_blade_heart()=false
    // So both count as "member cards without blade heart" → blade bonus should trigger
    assert_eq!(
        blades, 2,
        "PL!SP-sd1-011-P has no blade_heart → both count toward the condition → blade+2"
    );
}
