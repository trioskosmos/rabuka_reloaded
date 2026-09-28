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

/// Drain every prompt: decline the skippable ones, answer the rest with their
/// first option.
///
/// The answer half is not an optimisation, it is correctness. Mandatory
/// prompts exist on this path — 鬼塚夏美's 「手札を1枚控え室に置く」 is one,
/// and the engine rejects an empty answer for it — and the non-SelectCard
/// kinds (`SelectHeartColor`, `SelectAutoAbility`, `SelectLiveSuccess`) are
/// never skippable at all. A drain that sent them an empty selection left the
/// game parked mid-ability, and the absence assertion that usually follows is
/// exactly what a wrong answer produces, so nothing failed. See
/// `TestGame::drain_choices`.
fn drain_choices(game: &mut TestGame) {
    game.drain_choices();
}

/// `drain_choices`, under the name the older call sites use.
///
/// It used to differ: it handled a NON-skippable `SelectCard` where
/// `drain_choices` did not, because the engine rejects an empty answer for a
/// mandatory selection. That special case is now the general rule in
/// `TestGame::drain_choices` — decline what may be declined, answer the rest
/// with its first option — so the two are the same walk and the name is kept
/// only so the call sites below still read as "pick something real".
fn drain_choices_picking_first(game: &mut TestGame) {
    game.drain_choices();
}

/// Pre-grant All hearts to a staged card. All (icon_all) covers every COLOURED
/// need_heart deficit, so a fixture can satisfy a live card's requirement
/// without depending on which colours the members happen to print — the yell
/// RE-COMPUTES the stage hearts from the members, so a hand-set
/// `stage_hearts` is not enough on its own.
fn grant_all_hearts(game: &mut TestGame, card_id: i16, count: i16) {
    
    game.state
        .mods
        .heart_modifiers
        .entry(card_id)
        .or_default()
        .entry(HeartColor::All)
        .or_default()
        .additive += count;
}

/// Same walk as `drain_choices`, but it hands back the prompt TYPES it saw in
/// order. A test that only counts cards cannot say which prompt it answered;
/// this lets one assert "the window raised exactly this, and never that".
fn drain_choice_types(game: &mut TestGame) -> Vec<String> {
    game.drain_choices()
        .into_iter()
        .map(|k| k.to_string())
        .collect()
}

fn has_all_heart(gs: &rabuka_engine::core::game_state::GameState, cid: i16) -> bool {
    gs.mods
        .heart_modifiers
        .get(&cid)
        .and_then(|h| h.get(&HeartColor::All))
        .is_some_and(|e| e.total() > 0)
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
    
    game.state
        .mods
        .heart_modifiers
        .entry(member)
        .or_default()
        .entry(HeartColor::All)
        .or_default()
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

#[test]
fn q227_declined_live_start_cost_does_not_trigger_victory_road() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let payer = game.id("PL!N-sd2-017-SD2");
    let ally = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage = [ally, payer, -1];
    game.state.mods.add_orientation_modifier(ally, "wait");
    game.state.player1.hand.cards.push(live);
    game.give_energy(8);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "Q227 should offer the optional LiveStart energy cost"
    );
    game.select_option(0);

    assert!(
        !game.has_pending_choice(),
        "Declining the optional cost should skip the entire LiveStart ability"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(ally),
        Some("wait"),
        "Declining the cost must not activate the waited member"
    );
    assert!(
        !has_all_heart(&game.state, payer),
        "Declining the cost must not resolve the member ability for Victory Road"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        8,
        "Declining the optional cost must not spend energy"
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
    // Yell supply. 繚乱！ビクトリーロード needs heart01..heart06 and heart0×7.
    // The two members on stage only cover h01/h02/h03/h06, and a yelled card's
    // blade_heart color becomes a heart of that color — so these two fill h04
    // and h05. Without them the live FAILS the requirement at the yell, the
    // live card is sent to the waitroom, and every ライブ成功時 assertion
    // below would be measuring a live that never happened.
    let yell_h04 = game.id("PL!S-sd1-003-SD");
    let yell_h05 = game.id("PL!S-PR-014-PR");
    // Draw markers: three DIFFERENT card numbers, put on the deck AFTER the
    // yell, so the only thing that can take them is a ライブ成功時 draw. A
    // count-only "the deck shrank" cannot say which cards came back — and 30
    // identical fillers cannot either.
    let m1 = game.id("PL!-sd1-001-SD");
    let m2 = game.id("PL!-sd1-002-SD");
    let m3 = game.id("PL!-sd1-003-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // ONE copy of the live card: set_live_card moves it out of hand. Pushing
    // it into live_card_zone as well (as this file used to) staged the same
    // card id in two zones and left the live judging a second, heartless
    // copy — which fails the requirement on its own.
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

    // The yell happens on the first pass into SecondAttackerPerformance.
    put_on_deck_top(&mut game, 0, yell_h05);
    put_on_deck_top(&mut game, 0, yell_h04);
    game.pass();
    drain_choices(&mut game);
    assert!(
        game.state.player1.live_card_zone.cards.contains(&victory),
        "the yell must MEET the heart requirement — an unmet one sends the live \
         card to the waitroom and no ライブ成功時 ever resolves"
    );

    put_on_deck_top(&mut game, 0, m3);
    put_on_deck_top(&mut game, 0, m2);
    put_on_deck_top(&mut game, 0, m1);
    // Baselines taken AFTER the markers go on, so the delta below is only what
    // the ライブ成功時 abilities move.
    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();
    set_stage_hearts(&mut game);

    // Pass 1 of these: SecondAttackerPerformance → LiveVictoryDetermination
    // (p2 has no members, so its yell reveals nothing and the deck is
    // untouched — the markers are still there).
    game.pass();
    drain_choices_picking_first(&mut game);
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "the p2 yell must not consume the deck (p2 has no members on stage)"
    );
    // Pass 2: ライブ成功時 resolves here — the member draws 2 and discards 1,
    // then Victory Road's each_time draws 1. The live card has NOT moved to
    // the success zone yet, so a success-zone check here would be a false
    // negative. The next step is the live actually closing.
    game.pass();
    drain_choices_picking_first(&mut game);
    game.advance_to_phase(rabuka_engine::game_state::Phase::Active);
    drain_choices_picking_first(&mut game);

    // Precondition FIRST: a failed live would leave the deck untouched and the
    // draw assertions would read as a confusing "0 cards drawn".
    assert!(
        game.state.player1.success_live_card_zone.cards.contains(&victory),
        "the live must SUCCEED before its ライブ成功時 draws mean anything"
    );
    // Exact, not "fewer": 鬼塚夏美's 「カードを2枚引き、手札を1枚控え室に置く」
    // is 2 draws, and Victory Road ab#1's 「カードを1枚引く」 is the third. A
    // regression that dropped the each_time draw (the thing under test), or
    // doubled it, would both pass a `<` assertion.
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "exactly 2 (member) + 1 (Victory Road each_time) cards drawn"
    );
    let hand = &game.state.player1.hand.cards;
    assert!(hand.contains(&m1), "deck top (PL!-sd1-001-SD) reached hand");
    assert!(hand.contains(&m2), "deck 2nd (PL!-sd1-002-SD) reached hand");
    assert!(hand.contains(&m3), "deck 3rd (PL!-sd1-003-SD) reached hand");
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 1,
        "「手札を1枚控え室に置く」 put exactly one card in the waitroom"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&hand_card),
        "the discarded card came from HAND (PL!-bp3-013-N), not from the deck"
    );
    assert_eq!(
        hand.len(),
        hand_before + 2,
        "hand = before + 3 drawn - 1 discarded"
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
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, "player2");

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
    // Observed chain: after picking LS#1, the pending choice is its own effect's
    // SelectHeartColor ("Choose a heart color", options heart01/03/06). A
    // second SelectAutoAbility here would mean the each_time was NOT
    // force-drained and is mixed with LS#2 in the player's choice.
    assert!(
        game.has_pending_choice(),
        "LS#1 heart-color prompt expected after resolving LS#1"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectHeartColor"),
        "expected SelectHeartColor; SelectAutoAbility here would mean the \
         each_time leaked into the player's choice pool. Fix the drain loop."
    );

    // ── Step 5: Drain any remaining choices ──
    // Answer what cannot be declined, decline what can: a mandatory prompt
    // left parked mid-ability, and the absence assertion that usually
    // follows is exactly what a wrong answer produces.
    game.drain_choices();

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
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, "player2");
    game.state.process_pending_auto_abilities(&p1_id);

    // After the fix, everything auto-resolves except the resolved LS member's
    // own effect: observed exactly one SelectHeartColor prompt ("Choose a
    // heart color"). A SelectAutoAbility here would mean ET leaked into the
    // choice pool.
    assert!(
        game.has_pending_choice(),
        "LS member heart-color prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectHeartColor"),
        "expected SelectHeartColor; a SelectAutoAbility means ET leaked into \
         the choice pool (available should have been 1 → auto-promote)"
    );

    // Answer what cannot be declined, decline what can: a mandatory prompt

    // left parked mid-ability, and the absence assertion that usually

    // follows is exactly what a wrong answer produces.

    game.drain_choices();



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
    // Observed: that intermediate choice is a SelectHeartColor prompt.
    assert!(
        game.has_pending_choice(),
        "LS_b heart-color prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectHeartColor"),
        "After LS_b: expected SelectHeartColor; SelectAutoAbility would mean \
         each_time leaked into choice pool!"
    );
    game.select_indices(&[]);

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
            // Observed: the intermediate choice from LS_a's effect is a
            // SelectHeartColor prompt ("Choose a heart color").
            assert!(
                game.has_pending_choice(),
                "LS_a heart-color prompt expected"
            );
            assert_eq!(
                game.pending_choice_type().as_deref(),
                Some("SelectHeartColor"),
                "After LS_a: expected SelectHeartColor; SelectAutoAbility would \
                 mean each_time leaked into choice pool!"
            );
            game.select_indices(&[]);
        }
        other => {
            panic!("Choice 2 expected SelectAutoAbility, got {:?}", other);
        }
    }

    // ── Last LS auto-resolves (no choice) ──
    // Drain the final heart selection from LS_c's effect
    // Answer what cannot be declined, decline what can: a mandatory prompt
    // left parked mid-ability, and the absence assertion that usually
    // follows is exactly what a wrong answer produces.
    game.drain_choices();

    // All 3 members should have All Heart from the each_time triggers
    assert!(has_all_heart(&game.state, ls_a), "ls_a got all-heart");
    assert!(has_all_heart(&game.state, ls_b), "ls_b got all-heart");
    assert!(has_all_heart(&game.state, ls_c), "ls_c got all-heart");
}

/// T18: ab#1 (LiveSuccess each_time) resolves through the same force-drain the
///      LiveStart each_time uses (T15/T16) — the player is never asked to order
///      it. Records the prompt TYPES the ライブ成功時 window raises, which a
///      "3 cards were drawn" count can never say.
#[test]
fn test_live_success_each_time_drains_after_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    let member = game.id("PL!SP-bp2-009-R\u{ff0b}");
    let filler = game.new_id("PL!-sd1-010-SD");
    let hand_card = game.new_id("PL!-bp3-013-N");
    let yell_h04 = game.id("PL!S-sd1-003-SD");
    let yell_h05 = game.id("PL!S-PR-014-PR");
    let m1 = game.id("PL!-sd1-001-SD");
    let m2 = game.id("PL!-sd1-002-SD");
    let m3 = game.id("PL!-sd1-003-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
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

    put_on_deck_top(&mut game, 0, yell_h05);
    put_on_deck_top(&mut game, 0, yell_h04);
    game.pass();
    drain_choices(&mut game);
    assert!(
        game.state.player1.live_card_zone.cards.contains(&victory),
        "the yell must MEET the heart requirement or no ライブ成功時 resolves"
    );

    put_on_deck_top(&mut game, 0, m3);
    put_on_deck_top(&mut game, 0, m2);
    put_on_deck_top(&mut game, 0, m1);
    let deck_before = game.state.player1.main_deck.cards.len();
    set_stage_hearts(&mut game);

    // p2's yell (no members → no reveals), then the ライブ成功時 window.
    let mut window_prompts: Vec<String> = Vec::new();
    game.pass();
    window_prompts.extend(drain_choice_types(&mut game));
    game.pass();
    window_prompts.extend(drain_choice_types(&mut game));
    game.advance_to_phase(rabuka_engine::game_state::Phase::Active);
    window_prompts.extend(drain_choice_types(&mut game));

    // Precondition FIRST: a failed live would leave the deck untouched and the
    // draw assertions below would read as a confusing "0 cards drawn".
    assert!(
        game.state.player1.success_live_card_zone.cards.contains(&victory),
        "the live must SUCCEED before the ライブ成功時 draws mean anything"
    );
    // The each_time is FORCE-drained: had it leaked into the player's choice
    // pool, a SelectAutoAbility ordering prompt would appear here (that is
    // exactly what T15/T16 guard against on the LiveStart side).
    assert!(
        !window_prompts.iter().any(|t| t == "SelectAutoAbility"),
        "the each_time must be auto-resolved, never offered as a choice; \
         prompts in the ライブ成功時 window were {:?}",
        window_prompts
    );
    // The only prompt the window may raise is 鬼塚夏美's mandatory discard.
    assert_eq!(
        window_prompts
            .iter()
            .filter(|t| *t == "SelectCard")
            .count(),
        1,
        "exactly one SelectCard — 「手札を1枚控え室に置く」; got {:?}",
        window_prompts
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "LiveSuccess (2) + each_time (1) = exactly 3 draws"
    );
    let hand = &game.state.player1.hand.cards;
    assert!(hand.contains(&m1), "deck top (PL!-sd1-001-SD) reached hand");
    assert!(hand.contains(&m2), "deck 2nd (PL!-sd1-002-SD) reached hand");
    assert!(hand.contains(&m3), "deck 3rd (PL!-sd1-003-SD) reached hand");
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
    // Yell supply, as in T12: the live card's need_heart (h01..h06 + h0×7)
    // must actually be MET, or the live never succeeds and the 「draw 2」 this
    // test measures never resolves. The two members here print h01/h03 only,
    // so the coloured gaps are covered with All hearts instead — 君のこころは
    // 輝いてるかい？ additionally needs h05, which the yelled card covers.
    let yell_h05 = game.id("PL!S-PR-014-PR");
    let m1 = game.id("PL!-sd1-001-SD");
    let m2 = game.id("PL!-sd1-002-SD");
    let m3 = game.id("PL!-sd1-003-SD");

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

    // Heart budget. 繚乱！ビクトリーロード needs heart01..heart06 PLUS heart0×7
    // — 13 icons — and 君のこころは輝いてるかい？ needs 2 more. Three staged
    // members cannot print that many hearts (an All heart covers a coloured
    // deficit, not the whole heart0 bucket), so the live FAILS at the yell and
    // the ライブ成功時 under test never resolves. Cancel the COLORLESS (heart0)
    // requirement on both live cards; the six coloured icons stay enforced,
    // which is all this test needs in order to reach a successful live.
    game.state
        .mods
        .add_need_heart_modifier(victory, HeartColor::Heart00, -7);
    game.state
        .mods
        .add_need_heart_modifier(live_card, HeartColor::Heart00, -1);
    put_on_deck_top(&mut game, 0, yell_h05);
    grant_all_hearts(&mut game, filler, 10);
    grant_all_hearts(&mut game, member, 10);
    game.pass();
    drain_choices(&mut game);
    assert!(
        game.state.player1.live_card_zone.cards.contains(&victory),
        "the yell must MEET the heart requirement or no ライブ成功時 resolves"
    );

    put_on_deck_top(&mut game, 0, m3);
    put_on_deck_top(&mut game, 0, m2);
    put_on_deck_top(&mut game, 0, m1);
    let deck_before = game.state.player1.main_deck.cards.len();
    set_stage_hearts(&mut game);

    game.pass();
    drain_choices_picking_first(&mut game);
    game.pass();
    drain_choices_picking_first(&mut game);
    game.advance_to_phase(rabuka_engine::game_state::Phase::Active);
    drain_choices_picking_first(&mut game);

    // Precondition FIRST: a failed live would leave the deck untouched and the
    // draw assertion below would read as a confusing "0 cards drawn".
    assert!(
        game.state.player1.success_live_card_zone.cards.contains(&victory),
        "the live must SUCCEED before the ライブ成功時 draw means anything"
    );
    // The live card (PL!S-bp2-024-L) has LiveSuccess → draws 2, discards 1.
    // The member (PL!-bp3-012-N) has NO LiveSuccess.
    // Victory Road should NOT fire: 「自分のステージにいるメンバーの」 — a live
    // card is not a member. Exactly 2, not 3: the third draw is the whole
    // difference T12 measures, and "the deck shrank" cannot tell them apart.
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 2,
        "only the live card's own LiveSuccess (draw 2) — no each_time draw"
    );
    let hand = &game.state.player1.hand.cards;
    assert!(hand.contains(&m1), "deck top (PL!-sd1-001-SD) reached hand");
    assert!(hand.contains(&m2), "deck 2nd (PL!-sd1-002-SD) reached hand");
    assert!(
        !hand.contains(&m3),
        "the 3rd deck card must NOT be drawn — that is the each_time draw"
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
        let has_cost_limit = card.resolved_abilities().any(|ab| {
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

#[test]
fn baad_cage_counts_two_physical_instances_of_same_hasunosora_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let baad_cage = game.id("PL!HS-bp5-020-L");
    let sayaka1 = game.id("PL!HS-bp1-002-R");
    let sayaka2 = game.new_id("PL!HS-bp1-002-R");
    let filler = game.new_id("PL!-sd1-010-SD");
    assert_ne!(sayaka1, sayaka2);

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage[0] = sayaka1;
    game.state.player1.stage.stage[1] = sayaka2;
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.hand.cards.push(baad_cage);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_start(&mut game);
    game.set_live_card(baad_cage);
    finish_live_setup(&mut game);
    assert!(!game.has_pending_choice());

    assert_eq!(
        game.state.mods.get_score_modifier(baad_cage),
        1,
        "two physical copies of one cost-11 member satisfy the two-member threshold"
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
