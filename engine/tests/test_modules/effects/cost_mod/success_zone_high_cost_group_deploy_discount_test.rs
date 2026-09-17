use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Music S.T.A.R.T!! (PL!-bp6-019-L)
// 常時: このカードが自分の成功ライブカード置き場にあるかぎり、
// 元々のコストが17以上の『μ's』のメンバーカードを
// 自分の手札から登場させるためのコストは2減る。この効果は重複しない。
//
// While this card is in the success live card zone, μ's members with
// original cost >= 17 cost 2 less to deploy from hand. Non-stackable.
// ====================================================================

/// μ's member with cost >= 17 in hand → cost reduced by 2
#[test]
fn success_zone_discount_reduces_cost_seventeen_group_member_by_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let music_start = game.id("PL!-bp6-019-L");
    let maki = game.id("PL!-PR-015-PR"); // μ's/BiBi, cost 17

    // Music S.T.A.R.T!! in success live zone
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(music_start);

    let base_cost = game.db.get_card(maki).unwrap().cost.unwrap_or(0);
    assert_eq!(base_cost, 17);

    game.add_to_hand(maki);
    game.give_energy(20);

    game.play_to_stage(maki, MemberArea::Center);

    // Should have paid 17 - 2 = 15 energy
    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(
        remaining,
        20 - (17 - 2),
        "μ's member cost 17 should be reduced by 2 → paid 15"
    );
}

/// μ's member with cost < 17 in hand → NO reduction
#[test]
fn success_zone_discount_does_not_reduce_low_cost_group_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let music_start = game.id("PL!-bp6-019-L");
    let honoka = game.id("PL!-PR-001-PR"); // μ's/Printemps, cost 4

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(music_start);

    let base_cost = game.db.get_card(honoka).unwrap().cost.unwrap_or(0);
    assert_eq!(base_cost, 4);

    game.add_to_hand(honoka);
    game.give_energy(10);

    game.play_to_stage(honoka, MemberArea::Center);

    // Should have paid full cost 4 (no reduction)
    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(
        remaining,
        10 - 4,
        "μ's member cost 4 should NOT be reduced → paid 4"
    );
}

/// Non-μ's member with cost >= 17 → NO reduction (group_names filter)
#[test]
fn success_zone_discount_does_not_reduce_wrong_group_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let music_start = game.id("PL!-bp6-019-L");
    let mari = game.id("PL!S-bp2-008-P"); // Aqours/GuiltyKiss, cost 17

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(music_start);

    let base_cost = game.db.get_card(mari).unwrap().cost.unwrap_or(0);
    assert_eq!(base_cost, 17);

    game.add_to_hand(mari);
    game.give_energy(20);

    game.play_to_stage(mari, MemberArea::Center);

    // Should have paid full cost 17 (not μ's group)
    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(
        remaining,
        20 - 17,
        "Aqours member cost 17 should NOT be reduced (not μ's) → paid 17"
    );
}

/// Remove Music S.T.A.R.T!! from success live zone → reduction stops
#[test]
fn success_zone_discount_absent_after_source_removed_before_play() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let music_start = game.id("PL!-bp6-019-L");
    let maki = game.id("PL!-PR-015-PR");

    // Put in success live zone
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(music_start);

    // Now remove it
    game.state.player1.success_live_card_zone.cards.clear();

    let base_cost = game.db.get_card(maki).unwrap().cost.unwrap_or(0);
    assert_eq!(base_cost, 17);

    game.add_to_hand(maki);
    game.give_energy(20);

    game.play_to_stage(maki, MemberArea::Center);

    // Should have paid full cost 17 (no Music S.T.A.R.T!! in zone)
    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(
        remaining,
        20 - 17,
        "No Music S.T.A.R.T!! in success zone → paid full cost 17"
    );
}
