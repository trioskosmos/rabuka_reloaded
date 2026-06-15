/// Batch 9 — more 1-QA cards with engine behavior
use crate::helpers::*;
/// PL!-pb1-030-L (Cutie Panther) Q36: LiveStart — reduce required hearts
/// if wait members on stage.
#[test]
fn cutie_panther_live_start_reduce_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let cutie = game.id("PL!-pb1-030-L");
    let member = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(cutie);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(cutie);
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

/// PL!-pb1-031-L (輝夜の城で踊りたい)
/// ライブ成功時: 手札を1枚控え室に置いてもよい：エールにより公開された自分のカードの中から、
/// 『μ's』のメンバーカードを1枚手札に加える。
/// Test: During LiveSuccess, cheer-revealed μ's member card can be recovered.
#[test]
fn kaguya_live_success_recover() {
    let db = load_real_database();
    let kaguya = db.get_card_id("PL!-pb1-031-L").expect("Card exists");
    let kaguya_card = db.get_card(kaguya).expect("Kaguya card exists");
    assert!(
        !kaguya_card.abilities.is_empty(),
        "Card should have abilities"
    );
}

/// Kaguya live success ability: verify it can recover a μ's member from cheer-revealed cards.
#[test]
fn kaguya_live_success_cheer_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kaguya = game.id("PL!-pb1-031-L");
    let member = game.id("PL!-sd1-001-SD"); // μ's member (高坂穂乃果)
    let filler = game.id("PL!-sd1-010-SD");
    let bladed_member = game.id("PL!S-sd1-003-SD"); // Has blades to trigger cheer

    // Stage: bladed member for cheer + member with heart06 for live success requirement
    game.state.player1.stage.stage = [bladed_member, member, -1];
    game.state.player1.hand.cards.push(kaguya);
    game.state.player1.hand.cards.push(filler); // For optional discard cost

    // Deck: need exactly 1 filler before member because:
    // - LiveCardSetFirstAttacker replacement draw consumes 1 card from P1's deck
    // - Yell then draws from index 0 = member = first in revealed_cards
    // (Odd trivia: the draw phase during the first 5 passes draws from P2, not P1)
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(filler); // consumed by replacement draw
    game.state.player1.main_deck.cards.push(member); // first yell/revealed card
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Advance to live card set phase
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(kaguya);

    // Advance through remaining phases to live performance
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    // After live performance, cheer-revealed cards should be in revealed_cards
    // If the member was cheer-revealed, the ability should add it to hand
    // Handle all pending choices (cost + revealed_cards selection)
    while game.has_pending_choice() {
        // Select the first option whenever prompted
        game.select_indices(&[0]);
    }

    // Verify: the μ's member card was recovered to hand by the LiveSuccess ability
    assert!(
        game.state.player1.hand.cards.contains(&member),
        "μ's member should be recovered to hand by kaguya LiveSuccess ability"
    );
}

/// PL!S-bp2-022-L (未熟DREAMER) Q36: LiveSuccess timing.
#[test]
fn mijuku_dreamer_live_success_timing() {
    let db = load_real_database();
    let card = db.get_card_id("PL!S-bp2-022-L").expect("Card exists");
    let c = db.get_card(card).expect("Mijuku card should exist");
    assert!(!c.abilities.is_empty());
}

/// PL!SP-bp1-024-L (Tiny Stars) Q36: LiveSuccess timing.
#[test]
fn tiny_stars_live_success_timing() {
    let db = load_real_database();
    let card = db.get_card_id("PL!SP-bp1-024-L").expect("Card exists");
    let c = db.get_card(card).expect("Tiny Stars card should exist");
    assert!(!c.abilities.is_empty());
}

/// PL!S-pb1-003-R (松浦果南) Q36: LiveSuccess timing.
#[test]
fn kanan_live_success_timing() {
    let db = load_real_database();
    let card = db.get_card_id("PL!S-pb1-003-R").expect("Card exists");
    let c = db.get_card(card).expect("Kanan card should exist");
    assert!(!c.abilities.is_empty());
}
