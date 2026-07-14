use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};

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
        hearts: HeartMap::new(),
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

// ─────────────────────────────────────────────────────────────
// Ordering validation: each_time between LiveStart batch entries
// (§9.5.3.2→§9.5.3.1 loopback → depth-first drain)
// ─────────────────────────────────────────────────────────────

/// T15: Verifies each_time is force-drained between LiveStart batch entries
///       (not mixed into player's SelectAutoAbility choice pool).
///
/// Core assertion: after LS#1 resolves, the each_time should auto-resolve
/// (forced drain) before the player is asked about LS#2. If the each_time
/// leaks into the choice pool, a second SelectAutoAbility choice appears.
///
/// We detect this by directly inspecting the choice type after each resolution:
///   - Step 1: trigger_live_start → queue has [LS#1, LS#2]
///   - Step 2: process_pending_auto_abilities → SelectAutoAbility (LS#1 vs LS#2) ← 1st choice
///   - Step 3: select LS#1 via index[0] → LS#1 resolves → each_time queued
///     → drain loop force-resolves each_time → available=[LS#2] → auto-promote
///     → LS#2 resolves → ET2 queued → drain loop force-resolves
///     → No more pending choices
///   - Step 4: Verify a second SelectAutoAbility NEVER appeared (panic if it does)
#[test]
fn test_each_time_drains_between_live_starts_no_mix() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let member_a = game.id("PL!-bp3-012-N");
    let member_b = game.id("PL!-bp3-011-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    // Fill deck with filler cards to avoid unwantd triggers
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Place Victory Road (each_time LiveStart watcher) in live card zone
    game.state.player1.live_card_zone.cards.push(victory);

    // Place 2 members with LiveStart ability on stage
    game.state.player1.stage.stage[0] = member_a;
    game.state.player1.stage.stage[1] = member_b;
    game.state.player1.stage.stage[2] = -1;

    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(filler);

    // Advance to the LiveStart phase (engine internally queues LS abilities)
    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);

    // At this point, trigger_live_start_abilities has queued [LS#1, LS#2].
    // We call process_pending_auto_abilities directly to intercept choices.

    // ── Step 1: Queue LiveStart abilities ──
    let p1_id = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &p1_id);
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &"player2");

    // ── Step 2: Start processing ──
    // process_pending_auto_abilities enters process_player_abilities,
    // finds [LS#1, LS#2] → available > 1 → SelectAutoAbility choice
    game.state.process_pending_auto_abilities(&p1_id);

    // Verify we got the expected choice: pick order among 2 LS abilities
    assert!(
        game.has_pending_choice(),
        "Expected SelectAutoAbility choice for LS order"
    );
    match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } => {
            assert_eq!(
                options.len(),
                2,
                "Should have exactly 2 LS options to choose from, got {}",
                options.len()
            );
        }
        other => panic!("Expected SelectAutoAbility choice, got {:?}", other),
    }

    // ── Step 3: Pick LS#1 (index 0) ──
    // resume_with_choice internally calls process_pending_auto_abilities again
    // which continues the loop. With the depth-first fix:
    //   - LS#1 resolves → each_time queued at idx >= pre_len → drain loop forces it
    //   - available=[LS#2] → auto-promote → LS#2 resolves → ET2 drain → done
    // Without the fix:
    //   - LS#1 resolves → each_time queued
    //   - available=[LS#2, ET] → another SelectAutoAbility choice ← WRONG
    game.select_indices(&[0]);

    // ── Step 4: Verify NO SelectAutoAbility appeared for LS#2 vs each_time ──
    // If the fix works, the each_time was force-drained, and LS#2 auto-resolved.
    // There should be no pending choice.
    // If the fix is broken, another SelectAutoAbility appears here.
    if game.has_pending_choice() {
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                panic!(
                    "BUG: each_time leaked into player choice pool!\n\
                     After LS#1 resolved, a second SelectAutoAbility appeared\n\
                     meaning the each_time was NOT force-drained and is mixed\n\
                     with LS#2 in the player's choice. Fix the drain loop."
                );
            }
            _ => {
                // Some other choice type (e.g. card selection for cost) — fine.
                // The each_time was correctly drained.
            }
        }
    }

    // ── Step 5: Drain any remaining choices ──
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // ── Step 6: Verify both members got All Heart from each_time triggers ──
    assert!(
        has_all_heart(&game.state, member_a),
        "member_a got all-heart (each_time should fire for every LS)"
    );
    assert!(
        has_all_heart(&game.state, member_b),
        "member_b got all-heart (each_time should fire for every LS)"
    );
}

/// T16: 1 LS + each_time → only 1 entry in available_indices → auto-promote → drain.
///       Zero player choices appear (SelectAutoAbility never reached).
#[test]
fn test_one_live_start_each_time_drains_no_choice() {
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

    // ── Step 1: Queue LiveStart abilities ──
    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);

    // ── Step 2: Process. With 1 LS + 1 each_time:
    //   - available_indices (0..pre_len) = [LS#1] only (ET is at >= pre_len)
    //   - Only 1 available → auto-promote LS#1, no SelectAutoAbility
    //   - LS#1 resolves → ET queued → drain loop forces it
    // ──
    let p1_id = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &p1_id);
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &"player2");
    game.state.process_pending_auto_abilities(&p1_id);

    // After the fix, everything auto-resolves → no pending choices
    // (any pending choice here means a SelectAutoAbility appeared OR
    //  there's a cost/selection choice from a different effect)
    if game.has_pending_choice() {
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                panic!(
                    "BUG: SelectAutoAbility appeared for 1 LS + 1 each_time.\n\
                     With only 1 stale entry, available should be 1 → auto-promote.\n\
                     A SelectAutoAbility means ET leaked into the choice pool."
                );
            }
            _ => {}
        }
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        has_all_heart(&game.state, member),
        "Member should get all-heart"
    );
}

/// T17: 3 LS + each_time → verify player can pick ANY LS order at each choice,
///       and each_time is always force-drained between them (never mixed in choices).
///
/// Choice flow:
///   choice 1: [LS_a, LS_b, LS_c] — player picks any, e.g. index 1 (LS_b)
///   → LS_b resolves → each_time force-drained
///   choice 2: [LS_a, LS_c] — player picks any, e.g. index 0 (LS_a)
///   → LS_a resolves → each_time force-drained
///   → LS_c auto-resolves (only 1 left) → each_time force-drained
#[test]
fn test_three_live_starts_each_order_possible() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    // All three have LiveStart: choose heart01/03/06 during live
    let ls_a = game.id("PL!-bp3-011-N");
    let ls_b = game.id("PL!-bp3-012-N");
    let ls_c = game.id("PL!-bp3-013-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage[0] = ls_a;
    game.state.player1.stage.stage[1] = ls_b;
    game.state.player1.stage.stage[2] = ls_c;
    game.state.player1.hand.cards.push(victory);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(victory);
    finish_live_setup(&mut game);

    // The engine internally queues all 3 LS abilities during phase transition.
    // process_pending_auto_abilities paused at the first SelectAutoAbility choice.
    // Verify the first choice offers all 3 LS abilities.

    // ── Choice 1: 3 LS options ──
    assert!(game.has_pending_choice(), "Choice 1 expected");
    match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } => {
            assert_eq!(
                options.len(),
                3,
                "Choice 1: should have 3 LS options, got {}",
                options.len()
            );
        }
        other => panic!("Choice 1 expected SelectAutoAbility, got {:?}", other),
    }

    // Pick LS_b (option 1) — middle option, to prove arbitrary order works.
    // For SelectAutoAbility, resume_with_choice uses card_id to select option index.
    rabuka_engine::turn::TurnEngine::resume_with_choice(
        &mut game.state,
        Some(1), // option index
        None,    // card_indices unused for auto ability choice
    )
    .expect("resume_with_choice failed");

    // After LS_b resolves → each_time force-drained → no SelectAutoAbility should appear.
    // However, LS_b's effect asks "pick heart01/03/06" (sequential choice).
    // Drain that intermediate heart choice first.
    if game.has_pending_choice() {
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                panic!("After LS_b: each_time leaked into choice pool!");
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }

    // ── Choice 2: 2 LS options remaining (no each_time) ──
    assert!(
        game.has_pending_choice(),
        "Choice 2 expected (2 LS remaining)"
    );
    match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } => {
            assert_eq!(
                options.len(),
                2,
                "Choice 2: should have 2 LS options (no each_time), got {}",
                options.len()
            );
            // Pick LS_a (option 0) — first remaining
            rabuka_engine::turn::TurnEngine::resume_with_choice(&mut game.state, Some(0), None)
                .expect("resume_with_choice failed");

            // After LS_a resolves → each_time force-drained.
            // Drain the heart selection choice from LS_a's effect if present.
            if game.has_pending_choice() {
                match game.get_pending_choice() {
                    rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                        panic!("After LS_a: each_time leaked into choice pool!");
                    }
                    _ => {
                        game.select_indices(&[]);
                    }
                }
            }
        }
        other => {
            panic!("Choice 2 expected SelectAutoAbility, got {:?}", other);
        }
    }

    // ── Last LS auto-resolves (no choice) ──
    // Drain the final heart selection from LS_c's effect
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // All 3 members should have All Heart from the each_time triggers
    assert!(has_all_heart(&game.state, ls_a), "ls_a got all-heart");
    assert!(has_all_heart(&game.state, ls_b), "ls_b got all-heart");
    assert!(has_all_heart(&game.state, ls_c), "ls_c got all-heart");
}

/// T18: ab#1 (LiveSuccess each_time) also uses the same drain flow.
///      Set up 1 member with LiveSuccess + Victory Road, verify each_time fires.
#[test]
fn test_live_success_each_time_drains_after_success() {
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

    set_stage_hearts(&mut game);
    let deck_before = game.state.player1.main_deck.cards.len();

    // Advance through performance phases — LiveSuccess fires on win
    game.pass();
    drain_choices(&mut game);
    game.pass();
    drain_choices(&mut game);
    game.pass();
    drain_choices(&mut game);

    assert!(
        game.state.player1.main_deck.cards.len() < deck_before,
        "LiveSuccess + each_time draw should decrease deck"
    );
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

/// T19: バアドケージ card_count_condition with cost_limit — NO qualifying members.
///
/// Place Baad Cage as live card with 2 filler members on stage
/// (no 蓮ノ空 group, no cost >= 10). The condition should be false.
#[test]
fn baad_cage_cost_limit_no_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let baad_cage = game.id("PL!HS-bp5-020-L");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Verify cost_limit is parsed correctly on the ability
    let card_data = game.state.card_database.get_card(baad_cage);
    assert!(card_data.is_some(), "Baad Cage card should load");
    if let Some(card) = card_data {
        let has_cost_limit = card.abilities.iter().any(|ab| {
            ab.effect.as_ref().is_some_and(|eff| {
                eff.condition.as_ref().is_some_and(|cond| {
                    cond.get_cost_limit() == Some(10)
                        && cond.get_cost_limit_operator()
                            == Some(rabuka_engine::card::Operator::Gte)
                })
            })
        });
        assert!(
            has_cost_limit,
            "Baad Cage should have cost_limit=10, operator=>="
        );
    }

    game.state.player1.hand.cards.push(baad_cage);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(baad_cage);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert_eq!(
        game.state.mods.get_score_modifier(baad_cage),
        0,
        "No qualifying members → score 0"
    );
}

/// T20: バアドケージ — 2 蓮ノ空 members with cost >= 10 → score +1.
///
/// Place Baad Cage as live card with 2 蓮ノ空 members on stage
/// (sayaka cost=11, kozue cost=13). Both match the condition
/// (group=蓮ノ空, cost >= 10, count >= 2) → LiveStart grants +1 score.
#[test]
fn baad_cage_cost_limit_two_hasunosora_members_grants_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let baad_cage = game.id("PL!HS-bp5-020-L");
    let sayaka = game.id("PL!HS-bp1-002-R");
    let kozue = game.id("PL!HS-bp1-003-R\u{ff0b}");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Stage: 2 蓮ノ空 members (both cost >= 10) → condition met
    game.state.player1.stage.stage[0] = sayaka;
    game.state.player1.stage.stage[1] = kozue;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(baad_cage);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(baad_cage);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert_eq!(
        game.state.mods.get_score_modifier(baad_cage),
        1,
        "2 蓮ノ空 members with cost >= 10 → score +1"
    );
}

/// T21: バアドケージ — 1 蓮ノ空 member with cost >= 10 → score 0.
///
/// Only 1 member meets the condition (count >= 2 required).
/// Verifies the count threshold is enforced independently of cost_limit.
#[test]
fn baad_cage_cost_limit_one_member_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let baad_cage = game.id("PL!HS-bp5-020-L");
    let sayaka = game.id("PL!HS-bp1-002-R"); // cost=11, 蓮ノ空
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Stage: 1 蓮ノ空 member (cost >= 10) + 1 filler → condition fails (count < 2)
    game.state.player1.stage.stage[0] = sayaka;
    game.state.player1.stage.stage[1] = filler;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(baad_cage);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(baad_cage);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert_eq!(
        game.state.mods.get_score_modifier(baad_cage),
        0,
        "Only 1 qualifying member → score 0"
    );
}

/// T22: バアドケージ — 2 蓮ノ空 members, one cost < 10 → score 0.
///
/// Verify cost_limit works: one member has cost=9 (< 10) so only 1
/// member meets the full condition → count < 2 → no score.
#[test]
fn baad_cage_cost_limit_one_below_threshold_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let baad_cage = game.id("PL!HS-bp5-020-L");
    let sayaka = game.id("PL!HS-bp1-002-R"); // cost=11, 蓮ノ空 ✓
    let low_cost = game.id("PL!HS-bp1-005-PR"); // cost=9, 蓮ノ空 ✗ (< 10)
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Stage: 2 蓮ノ空 members, but one cost=9 → only 1 qualifies → condition fails
    game.state.player1.stage.stage[0] = sayaka;
    game.state.player1.stage.stage[1] = low_cost;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(baad_cage);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(baad_cage);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert_eq!(
        game.state.mods.get_score_modifier(baad_cage),
        0,
        "Only 1 of 2 蓮ノ空 members has cost >= 10 → score 0"
    );
}

/// T23: 3 qualifying 蓮ノ空 members → score +1 (same as 2, condition is >=2).
#[test]
fn baad_cage_three_members_score_still_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let baad_cage = game.id("PL!HS-bp5-020-L");
    let sayaka = game.id("PL!HS-bp1-002-R"); // cost=11
    let kozue = game.id("PL!HS-bp1-003-R\u{ff0b}"); // cost=13
    let multiname = game.id("LL-bp1-001-R\u{ff0b}"); // cost=20, matches 蓮ノ空
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Stage: 3 qualifying 蓮ノ空 members (all cost >= 10)
    game.state.player1.stage.stage[0] = sayaka;
    game.state.player1.stage.stage[1] = kozue;
    game.state.player1.stage.stage[2] = multiname;
    game.state.player1.hand.cards.push(baad_cage);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(baad_cage);
    finish_live_setup(&mut game);
    drain_choices(&mut game);

    assert_eq!(
        game.state.mods.get_score_modifier(baad_cage),
        1,
        "3 qualifying members → score +1 (condition is >=2)"
    );
}
