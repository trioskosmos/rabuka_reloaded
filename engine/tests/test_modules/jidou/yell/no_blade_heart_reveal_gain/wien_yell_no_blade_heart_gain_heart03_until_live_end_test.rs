/// Tests for ウィーン・マルガレーテ (PL!SP-bp2-021-N) — Auto ability:
///
/// 自動 ターン1回 エールにより公開された自分のカードの中に
/// ブレードハートを持つカードがないとき、ライブ終了時まで、heart03を得る。
///
/// Q112: ALL blade, score, draw count as blade heart. Test via existing engine.
/// Q113: No cheer → no trigger.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_success(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

/// Q113: No cheer → no revealed cards with blade heart check → no trigger.
#[test]
fn wien_q113_no_cheer_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-bp2-021-N");
    let filler = game.id("PL!-sd1-010-SD");
    let zero_blade = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [zero_blade, wien, -1];
    game.state.player1.hand.cards.push(filler);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(filler);
    game.set_live_card(filler);
    advance_to_live_success(&mut game);

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(wien, HeartColor::Heart03);
    assert_eq!(heart_mod, 0, "No cheer → ability should not trigger (Q113)");
}

/// Cheer with blade heart cards → condition fails → no heart03.
#[test]
fn wien_q112_cheer_with_blade_heart_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-bp2-021-N");
    let filler = game.id("PL!-sd1-010-SD");
    let bladed_member = game.id("PL!S-sd1-003-SD");

    game.state.player1.stage.stage = [bladed_member, wien, -1];
    game.state.player1.hand.cards.push(filler);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(filler);
    game.set_live_card(filler);
    advance_to_live_success(&mut game);

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(wien, HeartColor::Heart03);
    assert_eq!(
        heart_mod, 0,
        "Blade heart cards exist → condition fails (Q112)"
    );
}

/// Q112 positive: a REAL yell revealing no blade heart grants heart03.
///
/// This test previously used a 0-SCORE filler live card, which produces no yell
/// at all — so `yell_occurred` never became true and the ability correctly never
/// triggered. It asserted `heart == 0` and blamed an "auto-trigger bug"; there was
/// no bug, only a fixture that never yelled, and the ability was therefore never
/// exercised through the real pipeline.
///
/// The corrected version, using a live card with a real score requirement and
/// reading the modifier DURING the live (it is ライブ終了時まで), is
/// `jidou/comb combination/yell_real_yell_no_blade_heart_grants_heart_test.rs`:
/// `wien_real_yell_with_no_blade_heart_grants_heart03_during_the_live`. What
/// remains here is the honest negative: **no yell → no heart**, which is the
/// behaviour the broken fixture was accidentally asserting.
#[test]
fn wien_q112_positive_no_blade_heart_triggers_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-bp2-021-N");
    let filler = game.id("PL!-sd1-010-SD");
    let energy_card = game.id("LL-E-001-SD"); // no blade_heart
    let bladed_member = game.id("PL!S-sd1-003-SD"); // has blades to trigger cheer

    game.state.player1.stage.stage = [bladed_member, wien, -1];
    game.state.player1.hand.cards.push(filler);

    // Fill deck with energy cards (they have no blade_heart)
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(energy_card);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(filler);
    game.set_live_card(filler); // 0-SCORE live: no yell happens at all
    advance_to_live_success(&mut game);

    assert!(
        !game.state.yell_occurred,
        "precondition: a 0-score live card produces NO yell, which is why this \
         ability was never exercised here"
    );
    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(wien, HeartColor::Heart03);
    assert_eq!(
        heart_mod, 0,
        "no yell → the yell-triggered 自動 must not fire. (The positive case is \
         jidou/combination/yell_real_yell_no_blade_heart_grants_heart_test.rs; \
         this is NOT an engine bug — the old message was wrong.)"
    );
}
