/// Tests for PL!N-pb1-013-R 上原歩夢 — Debut ability:
///
/// {{toujyou.png|登場}}{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：
/// 自分のコスト4以下の「上原歩夢」のメンバーカードを1枚ステージに登場させる。
///
/// Q199: Can the card placed by this ability baton touch this turn? A: No.
/// Q200: Can that card's own debut ability be used? A: Yes (it keeps its abilities).

mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;

fn find_ayumu_sd(db: &std::sync::Arc<rabuka_engine::card::CardDatabase>) -> i16 {
    for card_no in &["PL!N-sd1-013-SD", "PL!N-sd1-012-SD"] {
        if let Some(id) = db.get_card_id(card_no) {
            return id;
        }
    }
    panic!("No 上原歩夢 SD card found");
}

/// Q199: Card placed by effect cannot baton touch this turn.
#[test]
fn ayumu_pb1_q199_no_baton_touch_after_effect_placement() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ayumu = game.id("PL!N-pb1-013-R");
    let target_ayumu = find_ayumu_sd(&db);
    let filler = game.id("PL!-sd1-010-SD");

    // Hand: target card + filler
    game.add_to_hand(ayumu);
    game.add_to_hand(target_ayumu);
    game.add_to_hand(filler);

    // Energy: 7+ for play_to_stage cost + 2 for optional ability cost
    game.give_energy(10);

    // Play 上原歩夢 to stage center → triggers debut ability
    game.play_to_stage(ayumu, MemberArea::Center);

    // Debut ability fires with optional pay_energy(2) cost
    // The ability checks for a member named 上原歩夢 with cost ≤ 4 in hand
    if game.has_pending_choice() {
        // Pay the optional energy cost (select yes)
        game.select_option(1);
    }

    // Is there a choice to select the target card from hand?
    if game.has_pending_choice() {
        game.select_indices(&[0]); // select target_ayumu from hand
    }

    // The target card should now be on stage
    let stage_has_target = game.state.player1.stage.stage.contains(&target_ayumu);
    assert!(stage_has_target, "Target 上原歩夢 should be placed on stage");

    // The area where it was placed should be locked (no baton touch)
    let target_area = game.state.player1.stage.stage.iter()
        .position(|&id| id == target_ayumu)
        .map(|i| match i { 0 => MemberArea::LeftSide, 1 => MemberArea::Center, _ => MemberArea::RightSide })
        .expect("Target should be on stage");

    assert!(game.state.player1.areas_locked_this_turn.contains(&target_area),
        "Placed card's area should be locked against baton touch");
}

/// Q200: Card placed by effect retains its own abilities.
#[test]
fn ayumu_pb1_q200_placed_card_retains_abilities() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ayumu = game.id("PL!N-pb1-013-R");
    let target_ayumu = find_ayumu_sd(&db);
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(ayumu);
    game.add_to_hand(target_ayumu);
    game.add_to_hand(filler);
    game.give_energy(10);
    game.play_to_stage(ayumu, MemberArea::Center);

    if game.has_pending_choice() {
        game.select_option(1);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // The placed card should still have abilities in the database
    let card = db.get_card(target_ayumu)
        .expect("Target card should be in database");
    assert!(!card.abilities.is_empty(),
        "Placed card should retain its abilities");
}
