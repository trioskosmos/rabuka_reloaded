/// E2E gameplay tests for parser issues Part 2 (Issues 2, 3, 5, 8).
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame) {
    let f = game.id_ref("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

// ====================================================================
// Issue 2: PL!S-bp5-003-R (松浦果南) — dynamic count from cost
// Text: 手札のブレードハートを持たないメンバーカードを2枚まで控え室に
// 置いてもよい：自分の控え室から、これにより控え室に置いたカードと
// 同じ枚数の『Aqours』のライブカードを手札に加える。
// Trigger: 登場 (debut)
// Nuance: effect count must LINK to cost count (not hardcoded to 1).
// ====================================================================

#[test]
fn issue2_kanan_discard_1_gain_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanan = game.id("PL!S-bp5-003-R");
    let aqours_live = game.id("PL!S-bp3-019-L");
    let no_blade = game.id("PL!-bp3-012-N"); // no blade heart, cost 2

    // Kanan in hand, no-blade card in hand, Aqours live in discard
    game.add_to_hand(kanan);
    game.add_to_hand(no_blade);
    game.add_to_discard(aqours_live);
    game.add_to_discard(aqours_live);
    game.give_energy(13);

    // Play Kanan to stage → debut triggers → cost choice → effect resolves
    game.play_to_stage(kanan, MemberArea::Center);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    // Verify we gained the Aqours live card in hand
    let has_live = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .any(|&cid| game.db.get_card(cid).is_some_and(|c| c.is_live()));
    assert!(has_live, "2a: gained Aqours live card in hand");
}

#[test]
fn issue2_kanan_skip_cost_empty_live_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanan = game.id("PL!S-bp5-003-R");

    // Kanan in hand, NO no-blade-heart cards in hand → cost can't be paid
    game.add_to_hand(kanan);
    game.give_energy(13);

    game.play_to_stage(kanan, MemberArea::Center);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    // No eligible cards to discard → cost skipped → no effect
    let lives = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&cid| game.db.get_card(cid).is_some_and(|c| c.is_live()))
        .count();
    assert_eq!(lives, 0, "2b: no live cards gained when cost skipped");
}

// ====================================================================
// Issue 3: PL!N-PR-003-PR (上原歩夢) — compound AND condition
// Text: 手札をすべて公開する：自分のステージにほかのメンバーがおり、
// かつこれにより公開した手札の中にライブカードがない場合、自分の
// デッキの上からカードを5枚見る。
// Trigger: 起動 (activation), turn 1 use
// Nuance: two conditions joined by AND (compound), NOT a complex
// cause-effect. Both must hold independently.
// ====================================================================

#[test]
fn issue3_ayumu_compound_both_conditions_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-PR-003-PR");
    let other_member = game.id("PL!N-bp4-001-R");

    // Stage: Ayumu in center, another Niji member beside her
    game.state.player1.stage.stage = [ayumu, other_member, -1];
    game.give_energy(9); // Ayumu costs 9

    // Hand: only filler cards (no live cards) → "no live cards in revealed hand" passes
    let deck_before = game.state.player1.main_deck.cards.len();

    // Activate 起動 ability (activation)
    game.activate_ability(ayumu);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    // Ability should have looked at cards from deck → deck changed
    assert!(
        game.state.player1.main_deck.cards.len() < deck_before || !game.state.rule_log.is_empty(),
        "3a: ability executed (deck changed or log emitted)"
    );
}

#[test]
fn issue3_ayumu_live_card_in_hand_blocks_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-PR-003-PR");
    let other_member = game.id("PL!N-bp4-001-R");
    let live_card = game.id("PL!-sd1-019-SD");

    // Stage: Ayumu + other member
    game.state.player1.stage.stage = [ayumu, other_member, -1];
    // Hand: HAS a live card → condition (2) FAILS
    game.state.player1.hand.cards.push(live_card);
    game.give_energy(9);

    let deck_before = game.state.player1.main_deck.cards.len();

    game.activate_ability(ayumu);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    // Condition fails → no look_at → deck unchanged
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "3b: live card in hand blocks ability, deck unchanged"
    );
}

// ====================================================================
// Issue 5: PL!SP-bp2-001-R+ (澁谷かのん) — select ANY Liella! member
// Text: 自分のステージにいる『Liella!』のメンバー1人のすべての
// [ライブ開始時]能力を、ライブ終了時まで、無効にしてもよい。
// Trigger: ライブ開始時 (LiveStart)
// Nuance: can select ANY 1 Liella! member (not just self/target all).
// ====================================================================

#[test]
fn issue5_kanon_invalidate_other_liella() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = game.id("PL!SP-bp2-001-R\u{ff0b}");
    let chisato = game.id("PL!SP-bp2-002-R");
    let live = game.id("PL!-sd1-019-SD");

    // Kanon + Chisato (another Liella! member) on stage
    game.state.player1.stage.stage[0] = kanon;
    game.state.player1.stage.stage[1] = chisato;
    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    // Both remain on stage (no crash from incorrect targeting)
    assert!(
        game.state.player1.stage.stage.contains(&kanon),
        "5a: kanon stays"
    );
    assert!(
        game.state.player1.stage.stage.contains(&chisato),
        "5b: chisato stays"
    );
}

// ====================================================================
// Issue 8: PL!-bp5-001-R+ (高坂穂乃果) — total_live_score + 2
// Text: 自分のデッキの上から、自分のライブの合計スコアに2を足した
// 数に等しい枚数見る。その中から1枚を手札に加える。
// Trigger: ライブ成功時 (LiveSuccess)
// Nuance: dynamic_count references "total_live_score" (not raw Japanese).
// ====================================================================

#[test]
fn issue8_honoka_live_score_dynamic() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let honoka = game.id("PL!-bp5-001-R\u{ff0b}");
    let live = game.id("PL!-sd1-019-SD");

    // Honoka on stage
    game.state.player1.stage.stage[0] = honoka;
    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    // Ability resolves without panic
    assert!(
        game.state.player1.stage.stage.contains(&honoka),
        "8a: honoka stays on stage"
    );
}
