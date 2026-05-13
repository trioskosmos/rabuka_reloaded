/// Tests for PL!SP-bp5-007-R (米女メイ) ab#0 — Q235
///
/// Ability (登場):
///   手札を1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。
///   その中から各グループにつき1枚まで公開し、3枚まで手札に加えてもよい。
///   残りを控え室に置く。
///
/// Q235: LL-bp1-001-R+ (上原歩夢&澁谷かのん&日野下花帆) を
///        個別名として手札に加えられるか？
/// Answer: はい、マルチネームカードは個別名でも検索可能。
use crate::helpers::*;

/// Debut ability triggers look_and_select from deck top 5.
/// With multi-name cards in deck, verify engine processes the
/// look → reveal → add sequence without crashing.
#[test]
fn mei_bp5_q235_debut_look_and_select_with_multiname() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let mei = game.id("PL!SP-bp5-007-R");
    let filler = game.id("PL!-sd1-010-SD");
    let filler2 = game.id("PL!-sd1-013-SD");
    // Multi-name card: 上原歩夢&澁谷かのん&日野下花帆
    let multiname = game.id("LL-bp1-001-R\u{ff0b}");

    // Mei in hand + filler
    game.state.player1.hand.cards.push(mei);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler2);

    // Deck top: put multiname card + filler
    game.state.player1.main_deck.cards.push(multiname);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler2);
    game.state.player1.main_deck.cards.push(filler2);
    game.state.player1.main_deck.cards.push(filler2);

    game.give_energy(15); // Mei cost=15

    // Play Mei to stage — debut triggers look_and_select
    game.play_to_stage(mei, rabuka_engine::zones::MemberArea::LeftSide);

    // Debut fires with optional cost (discard 1 from hand) + look_and_select
    // Handle optional cost choice
    if game.has_pending_choice() {
        game.select_indices(&[0]); // accept the optional discard
    }

    // Look_and_select: look at top 5 → reveal → add up to 3 → discard rest
    // The choice flow depends on engine implementation.
    // For now, verify no crash and the ability framework fires.
    let on_stage = game.state.player1.stage.stage[0];
    assert_eq!(on_stage, mei, "Mei should be on stage");

    // Resolve the look_and_select choice: accept all cards to hand
    if game.has_pending_choice() {
        game.select_indices(&[0, 1, 2, 3, 4]); // select all 5 looked-at cards
    }
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Q235: Multi-name card should either be in hand (added) or in waitroom (discarded)
    let multiname_in_hand = game.state.player1.hand.cards.contains(&multiname);
    let multiname_in_waitroom = game.state.player1.waitroom.cards.contains(&multiname);
    assert!(
        multiname_in_hand,
        "Multi-name card should have been added to hand (was first card selected for add)"
    );
    // Deck should have fewer cards (look_and_select removes from deck top)
    assert!(
        !game.state.player1.main_deck.cards.contains(&multiname),
        "Multi-name card should no longer be in the deck (removed by look_and_select)"
    );
}
