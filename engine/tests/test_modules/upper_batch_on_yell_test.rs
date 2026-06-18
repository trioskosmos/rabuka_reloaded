use crate::helpers::*;

// ═══════════════════════════════════════════════════════════════
// Group D: PL!S-bp2-007-R+ (Kurosawa Dia) — conditional draw
// エールにより公開された自分のカードの中にライブカードが
// 1枚以上あるとき、自分の手札が7枚以下の場合、カードを1枚引く。
// ═══════════════════════════════════════════════════════════════

fn setup_dia_test(game: &mut TestGame, revealed_ids: &[i16]) {
    let dia = game.id("PL!S-bp2-007-R\u{ff0b}");
    game.state.player1.stage.stage = [-1, dia, -1];
    for &id in revealed_ids {
        game.state.revealed_cards.push(id);
        game.state.player1.waitroom.cards.push(id);
    }
    // Fill deck so draw_card has cards to draw
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
}

/// Live card in yell + hand ≤7 → draw 1.
#[test]
fn dia_bp2_yell_with_live_card_and_hand_under_7_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp3-026-L");
    setup_dia_test(&mut game, &[live]);

    let hand_before = game.state.player1.hand.cards.len();
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let hand_after = game.state.player1.hand.cards.len();

    assert_eq!(
        hand_after,
        hand_before + 1,
        "Dia draws 1 with live card in yell + hand ≤7"
    );
}

/// Live card in yell but hand > 7 → no draw.
#[test]
fn dia_bp2_hand_over_7_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp3-026-L");
    setup_dia_test(&mut game, &[live]);

    // Push 8 cards to hand so it exceeds the ≤7 limit
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..8 {
        game.state.player1.hand.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let hand_after = game.state.player1.hand.cards.len();

    assert_eq!(
        hand_after, hand_before,
        "Dia must NOT draw when hand is >7 even with live card in yell"
    );
}

/// No live card → no draw.
#[test]
fn dia_bp2_no_live_card_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    setup_dia_test(&mut game, &[filler]);

    let hand_before = game.state.player1.hand.cards.len();
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let hand_after = game.state.player1.hand.cards.len();

    assert_eq!(
        hand_after, hand_before,
        "Dia must NOT draw with no live card in yell"
    );
}

// ═══════════════════════════════════════════════════════════════
// Group E: PL!-bp5-004-R+ (Sonoda Umi) — gain all hearts
// 自分がエールしたとき、エールにより公開された自分のカードの中に
// ブレードハートを持たないメンバーカードが3枚以上ある場合、
// ライブ終了時まで、全ハートを得る。
// ═══════════════════════════════════════════════════════════════

fn setup_umi_test(game: &mut TestGame, member_ids: &[i16]) {
    let umi = game.id("PL!-bp5-004-R\u{ff0b}");
    game.state.player1.stage.stage = [-1, umi, -1];
    for &id in member_ids {
        game.state.revealed_cards.push(id);
        game.state.player1.waitroom.cards.push(id);
    }
}

fn has_any_heart_modifiers(game: &TestGame) -> bool {
    game.state
        .mods
        .heart_modifiers
        .iter()
        .flat_map(|(_, hm)| hm.values())
        .any(|e| e.total() > 0)
}

/// 3 qualifying members → gains hearts.
#[test]
fn umi_bp5_three_members_without_blade_heart_gains_all_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m1 = game.id("PL!S-bp2-002-R"); // Riko, no blade_heart
    let m2 = game.id("PL!S-PR-013-PR"); // Chika, no blade_heart
    let m3 = game.id("PL!S-sd1-006-SD"); // Yoshiko, no blade_heart
    setup_umi_test(&mut game, &[m1, m2, m3]);

    assert!(!has_any_heart_modifiers(&game), "No hearts before");
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert!(
        has_any_heart_modifiers(&game),
        "Umi must give hearts with 3+ qualifying members in yell"
    );
}

/// 3 members WITH blade heart → condition fails → no hearts.
#[test]
fn umi_bp5_three_members_with_blade_heart_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // PL!-pb1-014-R has blade_heart — member card with blade heart
    let m1 = game.id("PL!-pb1-014-R");
    let m2 = game.id("PL!-pb1-014-R");
    let m3 = game.id("PL!-pb1-014-R");
    setup_umi_test(&mut game, &[m1, m2, m3]);

    let before = has_any_heart_modifiers(&game);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let after = has_any_heart_modifiers(&game);

    assert_eq!(
        before, after,
        "Umi must NOT give hearts with 3 members that HAVE blade heart"
    );
}

/// Only 2 qualifying members → no hearts.
#[test]
fn umi_bp5_two_members_no_heart_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m1 = game.id("PL!S-bp2-002-R");
    let m2 = game.id("PL!S-PR-013-PR");
    setup_umi_test(&mut game, &[m1, m2]);

    let before = has_any_heart_modifiers(&game);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let after = has_any_heart_modifiers(&game);

    assert_eq!(
        before, after,
        "Umi must NOT give hearts with only 2 qualifying members"
    );
}
