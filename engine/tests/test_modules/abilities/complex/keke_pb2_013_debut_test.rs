use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Tests for PL!SP-pb2-013-R / PL!SP-pb2-013-P＋ 唐 可可
///
/// 登場: 手札の『KALEIDOSCORE』のカードを1枚控え室に置いてもよい：
///   自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
///   これにより控え室に置いたカードがブレードハートを持たない場合、カードを1枚引く。

/// Discard KALEIDOSCORE card WITH blade_heart → energy placed, no draw
#[test]
fn keke_discard_blade_heart_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let keke = game.id("PL!SP-pb2-013-R");
    let blade_heart_fodder = game.id("PL!SP-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(keke);
    game.add_to_hand(blade_heart_fodder);
    game.give_energy(10);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let energy_card = game.id("LL-E-001-SD");
    game.state.player1.energy_deck.cards.push(energy_card);

    let energy_before = game.player().energy_zone.cards.len();
    let hand_before = game.player().hand.cards.len();
    let energy_deck_before = game.state.player1.energy_deck.cards.len();

    game.play_to_stage(keke, MemberArea::Center);

    game.drain_auto_ability_choices();
    assert!(game.has_pending_choice());
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[0]);

    assert!(!game.has_pending_choice());

    assert!(game.state.player1.stage.stage.contains(&keke));
    assert!(!game.player().hand.cards.contains(&blade_heart_fodder));
    assert!(game.player().waitroom.cards.contains(&blade_heart_fodder));

    let energy_after = game.player().energy_zone.cards.len();
    assert_eq!(energy_after, energy_before + 1);
    let energy_deck_after = game.state.player1.energy_deck.cards.len();
    assert_eq!(energy_deck_before - energy_deck_after, 1);
    assert_eq!(game.player().hand.cards.len(), hand_before - 2);
}

/// Discard KALEIDOSCORE card WITHOUT blade_heart → energy placed + draw 1
#[test]
fn keke_discard_no_blade_heart_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let keke = game.id("PL!SP-pb2-013-R");
    let no_blade_heart_fodder = game.id("PL!SP-bp1-021-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(keke);
    game.add_to_hand(no_blade_heart_fodder);
    game.give_energy(10);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let energy_card = game.id("LL-E-001-SD");
    game.state.player1.energy_deck.cards.push(energy_card);

    let energy_before = game.player().energy_zone.cards.len();
    let hand_before = game.player().hand.cards.len();
    let energy_deck_before = game.state.player1.energy_deck.cards.len();

    game.play_to_stage(keke, MemberArea::Center);

    game.drain_auto_ability_choices();
    assert!(game.has_pending_choice());
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[0]);

    assert!(!game.has_pending_choice());

    assert!(game.state.player1.stage.stage.contains(&keke));
    assert!(!game.player().hand.cards.contains(&no_blade_heart_fodder));
    assert!(game
        .player()
        .waitroom
        .cards
        .contains(&no_blade_heart_fodder));

    let energy_after = game.player().energy_zone.cards.len();
    assert_eq!(energy_after, energy_before + 1);
    let energy_deck_after = game.state.player1.energy_deck.cards.len();
    assert_eq!(energy_deck_before - energy_deck_after, 1);
    assert_eq!(game.player().hand.cards.len(), hand_before - 1);
}
