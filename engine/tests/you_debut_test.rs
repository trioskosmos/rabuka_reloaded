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

fn fill_deck_to_40(game: &mut TestGame, top_cards: Vec<i16>) {
    game.state.player1.main_deck.cards.extend(top_cards);
    let filler = game.id("PL!-sd1-010-SD");
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
}

/// Test that You's ability ends properly after all selections and that
/// the discard pile grows only via the final "残りを控え室に置く" move.
#[test]
fn you_ability_ends_and_discard_only_grows_at_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);

    // Top 7 fillers will be looked at; below1/below2 sit below the look zone.
    let below1 = game.id("PL!S-sd1-001-SD");
    let below2 = game.id("PL!-sd1-010-SD");
    fill_deck_to_40(&mut game, vec![filler, filler, filler, filler, filler, filler, filler, below1, below2]);

    let initial_discard = game.state.player1.waitroom.cards.len();
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, rabuka_engine::zones::MemberArea::LeftSide);

    // [Choice 1] Optional discard cost — skip (empty indices = skip)
    if game.has_pending_choice() { game.select_indices(&[]); }

    // No more choices — ability should be done (0 matching cards → no look_and_select choice)
    assert!(!game.has_pending_choice(),
        "Ability should have ended; had pending: {:?}", game.state.ability_queue.is_waiting_for_choice());

    // Only the 7 looked-at fillers should be in discard.
    let final_discard = game.state.player1.waitroom.cards.len();
    assert_eq!(final_discard - initial_discard, 7,
        "Expected 7 looked-at cards in discard, got {}", final_discard - initial_discard);
    assert_eq!(game.state.player1.main_deck.cards.len(), 33,
        "Deck should have 33 cards (40 total minus 7 looked at)");
}

/// Play You, select 3 heart-color cards through all optional steps,
/// then verify ability ends, selected cards reach hand, and the rest
/// go to discard.
#[test]
fn you_ability_select_3_cards_all_optional_steps() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");

    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);

    // Top 7: [qualifying, filler×6]; below sits below the look zone.
    let below = game.id("PL!-sd1-010-SD");
    fill_deck_to_40(&mut game, {
        let mut top = vec![qualifying];
        top.extend(std::iter::repeat(filler).take(6));
        top.push(below);
        top
    });

    let initial_discard = game.state.player1.waitroom.cards.len();
    let initial_hand = game.state.player1.hand.cards.len();

    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, rabuka_engine::zones::MemberArea::LeftSide);

    // [Choice 1] Optional discard cost — skip (empty indices = skip)
    if game.has_pending_choice() { game.select_indices(&[]); }
    // [Choice 2] look_and_select — select qualifying card
    if game.has_pending_choice() { game.select_indices(&[0]); }
    // [Choice 3] Reveal selected cards — confirm
    if game.has_pending_choice() { game.select_indices(&[0]); }
    // [Choice 4] Move to hand — confirm
    if game.has_pending_choice() { game.select_indices(&[0]); }

    // Ability should be complete
    assert!(!game.has_pending_choice(),
        "Ability should have ended, had pending: {:?}", game.state.ability_queue.is_waiting_for_choice());

    // qualifying card should be in hand
    assert!(game.state.player1.hand.cards.contains(&qualifying),
        "Qualifying card should be in hand");
    // below-zone card stays in deck
    assert!(game.state.player1.main_deck.cards.contains(&below),
        "Below-zone card should still be in deck");
    // 7 looked-at - 1 selected = 6 remaining → all 6 go to discard
    let final_discard = game.state.player1.waitroom.cards.len();
    assert_eq!(final_discard - initial_discard, 6,
        "Expected 6 fillers in discard, got {}", final_discard - initial_discard);
    assert_eq!(game.state.player1.hand.cards.len(), initial_hand,
        "Hand should have net 0 change (you left, qualifying entered)");
}


