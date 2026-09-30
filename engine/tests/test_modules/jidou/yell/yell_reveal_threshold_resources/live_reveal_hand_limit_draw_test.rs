use crate::helpers::*;

fn setup_live_reveal_draw(game: &mut TestGame, revealed_ids: &[i16]) {
    let source = game.id("PL!S-bp2-007-R\u{ff0b}");
    game.state.player1.stage.stage = [-1, source, -1];
    for &id in revealed_ids {
        game.state.revealed_cards.push(id);
        game.state.player1.waitroom.cards.push(id);
    }
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
}

#[test]
fn live_revealed_in_yell_with_empty_hand_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp3-026-L");
    setup_live_reveal_draw(&mut game, &[live]);

    let hand_before = game.state.player1.hand.cards.len();
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let hand_after = game.state.player1.hand.cards.len();

    assert_eq!(
        hand_after,
        hand_before + 1,
        "Live reveal draws 1 with hand at most 7"
    );
}

#[test]
fn live_revealed_in_yell_with_eight_hand_cards_does_not_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp3-026-L");
    setup_live_reveal_draw(&mut game, &[live]);

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
        "Live reveal must not draw when hand exceeds 7"
    );
}

#[test]
fn yell_without_live_card_does_not_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    setup_live_reveal_draw(&mut game, &[filler]);

    let hand_before = game.state.player1.hand.cards.len();
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let hand_after = game.state.player1.hand.cards.len();

    assert_eq!(
        hand_after, hand_before,
        "No live card in yell must not draw"
    );
}

/// 「自分の手札が**7枚以下**の場合」 — the bound itself.
///
/// The tests above stage an empty hand (0) and 8, which straddle the bound with nothing on
/// it. `hand <= 7`, `hand < 7`, and any off-by-one in between all pass them. Only a hand of
/// exactly 7 separates those: with 7 the clause is satisfied and the card is drawn, so the
/// hand ends at 8.
#[test]
fn live_revealed_in_yell_with_exactly_seven_hand_cards_still_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp3-026-L");
    setup_live_reveal_draw(&mut game, &[live]);

    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..7 {
        game.state.player1.hand.cards.push(filler);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        7,
        "precondition: the hand must start at exactly the printed 7枚以下 bound"
    );

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        8,
        "手札が7枚以下 — 7 satisfies 7枚以下, so exactly one card is drawn and the hand ends \
         at 8. Ending at 7 means the comparison is exclusive (`< 7`)."
    );
}

/// Q120 (qa_data.json, 2025.06.06) — the ORDERING rule, which nothing here covers.
///
/// 国木田花丸: 「…我的.liveが1枚以上[公開]されているとき、**自分の手札が7枚以下の場合**、
/// カードを1枚引く。」 Q120 asks: with 7 cards in hand, a ドロー blade-heart live is revealed.
/// Can her ability still draw?
///
/// 「いいえ。発動した自動能力を使うのは、エールで公開されたドローブレードハートの効果を
/// 解決したあとです。」 — the draw blade heart resolves FIRST: hand 7 → 8. Only then is
/// 「手札が7枚以下」 evaluated, and by then it is false, so no card is drawn.
///
/// The observable is the final hand size, and it discriminates the two orderings exactly:
///
///   * blade heart first (ruled): 7 → 8, then 8 > 7 → **no draw** → hand ends at **8**
///   * auto checked first:        7 ≤ 7 → draw → then blade heart → hand ends at **9**
///
/// `PL!-bp3-021-L` 愛してるばんざーい! is the draw blade heart: `blade_heart {b_heart06: 1}`
/// and `special_heart {draw: 1}` — the draw icon, which Q112 confirms counts as a blade heart.
#[test]
fn q120_the_draw_blade_heart_resolves_before_the_hand_limit_is_checked() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let draw_blade = game.id("PL!-bp3-021-L");
    game.assert_card_identity(draw_blade, "PL!-bp3-021-L");

    // Pin the premise from card data: this live really does print the ドロー icon.
    let card = game.db.get_card(draw_blade).unwrap();
    assert!(card.has_draw_icon(), "precondition: this live prints the ドロー icon");
    assert!(
        card.blade_heart.is_some(),
        "precondition: the draw icon lives alongside a real blade heart (Q112)"
    );

    setup_live_reveal_draw(&mut game, &[draw_blade]);

    // A known deck size, so the assertions below can count draws without depending on which
    // card came out — an identity assertion here proved unreliable, because the card that
    // reaches hand is not the one staged at index 0.
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..7 {
        game.state.player1.hand.cards.push(filler);
    }
    assert_eq!(game.state.player1.hand.cards.len(), 7, "precondition: hand starts at 7");

    let deck = &mut game.state.player1.main_deck.cards;
    deck.clear();
    for _ in 0..12 {
        deck.push(filler);
    }
    let deck_before = deck.len();

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        8,
        "Q120 — the ドロー blade heart resolves first (7 → 8), and only then is 手札が7枚以下 \
         checked, which by then is false, so 国木田花丸 draws nothing. A hand of 9 means her \
         ability was resolved BEFORE the blade heart, contradicting the ruling."
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "and exactly ONE card left the deck. Two draws would mean both the blade heart and \
         her ability resolved ({} before, {} after)",
        deck_before,
        game.state.player1.main_deck.cards.len()
    );
}
