/// Tests for PL!S-bp2-005-R+ (渡辺 曜) ab#0 — Q124
///
/// Ability (登場): 手札1枚を捨ててもよい：
///   デッキの上から7枚見る。その中からheart02とheart04とheart05を
///   持つメンバーカードを3枚まで公開し手札に加えてもよい。残りは捨てる。
///
/// Q124: ブレードハート(b_heart02/b_heart04/b_heart05)は
///        条件を満たすハートとしてカウントされるか？
/// Answer: いいえ。基本ハートのみ。ブレードハートは無効。

mod helpers;
use helpers::*;

/// Top 7 of deck contains:
///   [0] PL!S-sd1-001-SD (千歌): base heart02=3, heart04=2, heart05=2 → ✓ qualifies
///   [1] PL!SP-sd1-001-SD (かのん): no base heart02/04/05, blade heart b_heart02=1 → ✗ fails
///   [2-6] filler cards with no heart02/04/05
///
/// After look_and_select: only 千歌 should be added to hand.
/// かのん should NOT (blade heart doesn't count).
#[test]
fn you_q124_blade_heart_excluded_base_heart_included() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    // 千歌: base heart02=3, heart04=2, heart05=2 → SHOULD qualify
    let qualifying = game.id("PL!S-sd1-001-SD");
    // かのん: base heart03=2, heart06=1, blade b_heart02=1 → NO base heart02/04/05
    let blade_only = game.id("PL!SP-sd1-001-SD");

    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    // Deck top 7: [qualifying, blade_only, filler ×5]
    game.state.player1.main_deck.cards.push(qualifying);
    game.state.player1.main_deck.cards.push(blade_only);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, rabuka_engine::zones::MemberArea::LeftSide);

    // Accept optional discard cost
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // look_and_select: player sees top 7, picks from matching cards
    // The choice prompt shows indices into the looked_at cards.
    // Matching cards should be ONLY qualifying (index 0 in looked_at).
    // blade_only (index 1) should NOT match the heart filter.
    if game.has_pending_choice() {
        // Select only the qualifying card (index 0 in looked_at list)
        game.select_indices(&[0]);
    }

    // Q124: blade heart DOES NOT qualify → blade_only stays in discard
    // qualifying card SHOULD be in hand
    let in_hand = game.state.player1.hand.cards.contains(&qualifying);
    let blade_in_hand = game.state.player1.hand.cards.contains(&blade_only);
    assert!(in_hand,
        "Q124: Card with base heart02/04/05 should be added to hand");
    assert!(!blade_in_hand,
        "Q124: Card with ONLY blade_heart02 should NOT be added to hand");
}

/// Play You twice (two copies). First play: 千歌 qualifies. Second play:
/// make sure blade-only cards consistently excluded.
#[test]
fn you_q124_two_plays_both_reject_blade_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let you2 = game.id("PL!S-bp2-005-P");  // second copy if available
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");
    let blade_only = game.id("PL!SP-sd1-001-SD");
    let live_card = game.id("LL-bp5-001-L");  // to satisfy reveal cost

    // Two copies of You in hand + live for cost + filler
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(you2);
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    
    // Deck: qualifying + blade_only + filler
    game.state.player1.main_deck.cards.push(qualifying);
    game.state.player1.main_deck.cards.push(blade_only);
    for _ in 0..6 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.give_energy(26);  // 13 × 2
    game.state.player1.stage.stage = [-1, -1, -1];

    // -- First You play --
    game.play_to_stage(you, rabuka_engine::zones::MemberArea::LeftSide);
    // Debut cost: reveal 1 live card from hand. Indices now: [you2(0), live(1), filler(2), filler(3)]
    // Select the live card at index 1
    if game.has_pending_choice() { game.select_indices(&[1]); }
    // Look at 7, blade_only (index 1) should NOT match heart filter
    if game.has_pending_choice() { game.select_indices(&[0]); }
    assert!(game.state.player1.hand.cards.contains(&qualifying),
        "First play: qualifying card should reach hand");
    assert!(!game.state.player1.hand.cards.contains(&blade_only),
        "First play: blade-only card should NOT reach hand");
    // -- Second You play (to RightSide) --
    game.play_to_stage(you2, rabuka_engine::zones::MemberArea::RightSide);
    // Debut cost: reveal 1 live card from hand.
    // If no live card in hand, the allow_skip choice can be skipped with empty indices.
    if game.has_pending_choice() { game.select_indices(&[]); }
    // Look at 7 from deck
    if game.has_pending_choice() { game.select_indices(&[]); }

    // After second play: only qualifying should be in hand
    // blade_only should NEVER have been added in either play
    assert!(!game.state.player1.hand.cards.contains(&blade_only),
        "Second play: blade-only card still should NOT be in hand");
}


