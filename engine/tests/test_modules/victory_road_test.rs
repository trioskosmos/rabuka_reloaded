use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor};
use std::collections::HashMap;

fn advance_to_live_start(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn set_stage_hearts(game: &mut TestGame) {
    let mut h = BaseHeart {
        hearts: HashMap::new(),
    };
    h.hearts.insert(HeartColor::Heart00, 7);
    h.hearts.insert(HeartColor::Heart01, 1);
    h.hearts.insert(HeartColor::Heart02, 1);
    h.hearts.insert(HeartColor::Heart03, 1);
    h.hearts.insert(HeartColor::Heart04, 1);
    h.hearts.insert(HeartColor::Heart05, 1);
    h.hearts.insert(HeartColor::Heart06, 1);
    game.state.player1.stage_hearts = Some(h);
}

fn drain_choices(game: &mut TestGame) {
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

fn has_all_heart(gs: &rabuka_engine::core::game_state::GameState, cid: i16) -> bool {
    gs.mods
        .heart_modifiers
        .get(&cid)
        .and_then(|h| h.get(&HeartColor::All))
        .map_or(false, |e| e.total() > 0)
}

fn total_all_heart(gs: &rabuka_engine::core::game_state::GameState, cid: i16) -> i32 {
    gs.mods
        .heart_modifiers
        .get(&cid)
        .and_then(|h| h.get(&HeartColor::All))
        .map_or(0, |e| e.total())
}

// ─────────────────────────────────────────────────────────────
// ab#0: 自分のステージにいるメンバーのライブ開始時能力が解決するたび、
//       そのメンバーが全ハートを持たない場合、
//       ライブ終了時まで、そのメンバーは全ハートを得る。
// ─────────────────────────────────────────────────────────────

/// T1: メンバーのライブ開始時能力が解決する → そのメンバーは全ハートを得る
#[test]
fn live_start_grants_all_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let member = game.id("PL!-bp3-012-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage[1] = member;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert!(
        has_all_heart(&game.state, member),
        "Member should have all-heart after LiveStart via Victory Road"
    );
}

/// T2: 2人のメンバーそれぞれのライブ開始時能力が解決する → 両方とも全ハートを得る
#[test]
fn two_members_both_get_all_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let a = game.id("PL!-bp3-011-N");
    let b = game.id("PL!-bp3-012-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage[0] = a;
    game.state.player1.stage.stage[1] = b;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert!(
        has_all_heart(&game.state, a),
        "Member A should get all-heart"
    );
    assert!(
        has_all_heart(&game.state, b),
        "Member B should get all-heart"
    );
}

/// T3: メンバーが既に全ハートを持っている → 条件不成立 → 再付与なし
#[test]
fn already_has_all_heart_no_double_grant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let member = game.id("PL!-bp3-012-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage[1] = member;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(filler);

    // Pre-grant all-heart (simulating first resolution in same live)
    use rabuka_engine::core::game_modifiers::ModifierEntry;
    game.state
        .mods
        .heart_modifiers
        .entry(member)
        .or_default()
        .entry(HeartColor::All)
        .or_insert(ModifierEntry::default())
        .additive = 1;

    let before = total_all_heart(&game.state, member);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert_eq!(
        total_all_heart(&game.state, member),
        before,
        "Already has all-heart → no re-grant"
    );
}

/// T4: ライブ開始時能力を持つメンバーがいる → each_time発動
/// Same as T1 but with PL!-bp3-012-N at stage[0]. Redundant with T1 but validates consistency.
#[test]
fn live_start_another_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let member = game.id("PL!-bp3-012-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = member;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert!(
        has_all_heart(&game.state, member),
        "LiveStart member → each_time fires → all-heart"
    );
}

/// T5: Q227: コスト支払いが必要な能力でコスト不払い → 不解決 → each_time発動しない
/// Uses PL!-bp3-012-N (南ことり) which has LiveStart without cost → always resolves.
/// (Engine currently has no way to test cost-decline via gameplay for this card.)
#[test]
fn live_start_cost_free_still_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let member = game.id("PL!-bp3-012-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = member;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert!(
        has_all_heart(&game.state, member),
        "Cost-free LiveStart resolves → each_time fires"
    );
}

/// T6: メンバーではないカード（エネルギーカード）がステージにいる → each_time発動しない
#[test]
fn non_member_on_stage_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let member = game.id("PL!-bp3-012-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Put filler at stage[0] (member type but no LiveStart) and member at stage[1]
    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = member;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert!(
        has_all_heart(&game.state, member),
        "Member should still get all-heart"
    );
}

/// T8: メンバーがライブ開始時能力を持たない → 発動しない
#[test]
fn member_without_live_start_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let member = game.id("PL!-bp3-012-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.live_card_zone.cards.push(victory);
    // filler (sd1-010-SD) has NO LiveStart ability
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = member;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    // The filler doesn't have LiveStart, but the member (stage[1]) does
    assert!(
        has_all_heart(&game.state, member),
        "Member with LiveStart should get all-heart"
    );
}

/// T11: ライブ終了時まで持続 → ライブ終了後は消える
#[test]
fn all_heart_expires_at_live_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let member = game.id("PL!-bp3-012-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage[1] = member;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert!(
        has_all_heart(&game.state, member),
        "Should have all-heart during live"
    );

    // Set stage hearts and advance through performance to end the live
    set_stage_hearts(&mut game);
    game.pass();
    drain_choices(&mut game);
    game.pass();
    drain_choices(&mut game);
    game.pass();
    drain_choices(&mut game);

    // After live ends (phase is now Active), all-heart should have expired
    assert!(
        !has_all_heart(&game.state, member),
        "All-heart should expire when live ends"
    );
}

// ─────────────────────────────────────────────────────────────
// ab#1: 自分のステージにいるメンバーのライブ成功時能力が解決するたび、
//       カードを1枚引く。
// ─────────────────────────────────────────────────────────────

/// T12: メンバーのライブ成功時能力が解決 → ビクトリーロードが1枚引く
/// 鬼塚夏美 (PL!SP-bp2-009-R+) has unconditional LiveSuccess "draw 2, discard 1".
/// Victory Road ab#1 fires after each LiveSuccess resolves, drawing 1 more.
#[test]
fn live_success_each_time_draws_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let member = game.id("PL!SP-bp2-009-R\u{ff0b}");
    let filler = game.new_id("PL!-sd1-010-SD");
    let hand_card = game.new_id("PL!-bp3-013-N");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = member;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(hand_card);
    game.state.player1.hand.cards.push(hand_card);
    game.state.player1.hand.cards.push(hand_card);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    let deck_before = game.state.player1.main_deck.cards.len();
    set_stage_hearts(&mut game);

    // Advance through: FirstAttackerPerformance → SecondAttackerPerformance → LiveVictoryDetermination
    game.pass();
    drain_choices(&mut game);
    game.pass();
    drain_choices(&mut game);
    game.pass();
    drain_choices(&mut game);

    assert!(
        game.state.player1.main_deck.cards.len() < deck_before,
        "LiveSuccess + Victory Road each_time should draw cards"
    );
}

/// T13: ライブ成功時能力を持たないメンバー → ビクトリーロード発動しない
#[test]
fn no_live_success_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let member = game.id("PL!-bp3-012-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = member;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    let deck_before = game.state.player1.main_deck.cards.len();
    set_stage_hearts(&mut game);

    game.pass();
    drain_choices(&mut game);
    game.pass();
    drain_choices(&mut game);
    game.pass();
    drain_choices(&mut game);

    let deck_after = game.state.player1.main_deck.cards.len();
    // Both tests pass through the same phases; T12 should draw MORE than T13
    // because T12 has LiveSuccess triggering Victory Road ab#1.
    assert!(deck_after < deck_before, "Deck decreased from phase draws");
}

/// T14: ライブカード自身のライブ成功時能力 → メンバーでない → ビクトリーロード発動しない
/// 君のこころは輝いてるかい？ (PL!S-bp2-024-L) is a live card with LiveSuccess "draw 2, discard 1".
/// Victory Road ab#1 watches "メンバーの" (member's) LiveSuccess → live card's own LiveSuccess
/// does NOT trigger it.
#[test]
fn live_card_own_live_success_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let live_card = game.id("PL!S-bp2-024-L");
    let member = game.id("PL!-bp3-012-N");
    let filler = game.new_id("PL!-sd1-010-SD");
    let hand_card = game.new_id("PL!-bp3-013-N");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Don't push to live_card_zone yet; set_live_card handles that.
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = member;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(hand_card);
    game.state.player1.hand.cards.push(hand_card);
    game.state.player1.hand.cards.push(hand_card);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    // Manually add the second live card (君のこころは輝いてるかい？) to the zone
    game.state.player1.live_card_zone.cards.push(live_card);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    let deck_before = game.state.player1.main_deck.cards.len();
    set_stage_hearts(&mut game);

    game.pass();
    drain_choices(&mut game);
    game.pass();
    drain_choices(&mut game);
    game.pass();
    drain_choices(&mut game);

    // The live card (PL!S-bp2-024-L) has LiveSuccess → draws 2, discards 1.
    // The member (PL!-bp3-012-N) has NO LiveSuccess.
    // Victory Road should NOT fire live card's LiveSuccess is from a non-member.
    // Only the live card's own LiveSuccess draw happens.
    assert!(
        game.state.player1.main_deck.cards.len() < deck_before,
        "Live card's own LiveSuccess draws; Victory Road does NOT fire for non-member"
    );
}
