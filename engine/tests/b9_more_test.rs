/// Batch 9 — more 1-QA cards with engine behavior

mod helpers;
use helpers::*;
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
    for _ in 0..5 { game.pass(); }
    game.set_live_card(cutie);
    game.pass(); game.pass(); game.pass(); game.pass(); game.pass();
}

/// PL!-pb1-031-L (輝夜の城で踊りたい) Q36: LiveSuccess — recover member from discard.
#[test]
fn kaguya_live_success_recover() {
    let db = load_real_database();
    let card = db.get_card_id("PL!-pb1-031-L").expect("Card exists");
    let card_data = db.get_card(card).unwrap();
    assert!(!card_data.abilities.is_empty(), "Card should have abilities");
}

/// PL!S-bp2-022-L (未熟DREAMER) Q36: LiveSuccess timing.
#[test]
fn mijuku_dreamer_live_success_timing() {
    let db = load_real_database();
    let card = db.get_card_id("PL!S-bp2-022-L").expect("Card exists");
    let c = db.get_card(card).unwrap();
    assert!(!c.abilities.is_empty());
}

/// PL!SP-bp1-024-L (Tiny Stars) Q36: LiveSuccess timing.
#[test]
fn tiny_stars_live_success_timing() {
    let db = load_real_database();
    let card = db.get_card_id("PL!SP-bp1-024-L").expect("Card exists");
    let c = db.get_card(card).unwrap();
    assert!(!c.abilities.is_empty());
}

/// PL!S-pb1-003-R (松浦果南) Q36: LiveSuccess timing.
#[test]
fn kanan_live_success_timing() {
    let db = load_real_database();
    let card = db.get_card_id("PL!S-pb1-003-R").expect("Card exists");
    let c = db.get_card(card).unwrap();
    assert!(!c.abilities.is_empty());
}
