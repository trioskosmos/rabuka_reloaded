use crate::helpers::*;

fn reset_both_main_decks_to_ten_fillers(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn one_unit_member_reduces_play_cost_by_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");
    let mk = game.id("PL!HS-bp1-005-R"); // unit=みらくらぱーく！ member
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(himeno);
    game.state.player1.stage.stage = [mk, -1, -1];
    reset_both_main_decks_to_ten_fillers(&mut game, filler);

    // Verify GameModifiers cost modifier
    game.state.recalculate_constant_cost_modifiers();
    let cost_mod = game.state.mods.get_cost_modifier(himeno);
    assert_eq!(cost_mod, -2, "1 Mirakura member on stage → cost -2");

    // Actually play the card: cost = 20 - 2 = 18
    game.give_energy(18);
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::Center);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "All 18 energy consumed (20 base - 2 reduction)"
    );
}

#[test]
fn no_unit_members_leaves_play_cost_unchanged() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(himeno);
    game.state.player1.stage.stage = [-1, -1, -1];
    reset_both_main_decks_to_ten_fillers(&mut game, filler);

    // Verify GameModifiers cost modifier
    game.state.recalculate_constant_cost_modifiers();
    let cost_mod = game.state.mods.get_cost_modifier(himeno);
    assert_eq!(cost_mod, 0, "0 Mirakura members on stage → cost unchanged");

    // Actually play the card: cost = 20 - 0 = 20
    game.give_energy(20);
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::Center);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "All 20 energy consumed (full base cost)"
    );
}

#[test]
fn two_unit_members_reduce_play_cost_by_four() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");
    let mk1 = game.id("PL!HS-bp1-005-R"); // unit=みらくらぱーく！
    let mk2 = game.id("PL!HS-PR-005-PR"); // unit=みらくらぱーく！
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(himeno);
    game.state.player1.stage.stage = [mk1, mk2, -1];
    reset_both_main_decks_to_ten_fillers(&mut game, filler);

    // Verify GameModifiers cost modifier
    game.state.recalculate_constant_cost_modifiers();
    let cost_mod = game.state.mods.get_cost_modifier(himeno);
    assert_eq!(cost_mod, -4, "2 Mirakura members → cost -4");

    // Actually play the card: cost = 20 - 4 = 16
    game.give_energy(16);
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::RightSide);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "All 16 energy consumed (20 base - 4 reduction)"
    );
}

/// Q249: 3 みらくらぱーく！ on stage → baton touch one to debut 姫芽 → cost = 20 - 3×2 = 14
#[test]
fn three_unit_members_reduce_cost_before_baton_touch_q249() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");
    let mk1 = game.id("PL!HS-bp1-005-R"); // unit=みらくらぱーく！
    let mk2 = game.id("PL!HS-PR-005-PR"); // unit=みらくらぱーく！
    let mk3 = game.id("PL!HS-bp1-005-R"); // another mirakura copy
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(himeno);
    game.state.player1.stage.stage = [mk1, mk2, mk3];
    reset_both_main_decks_to_ten_fillers(&mut game, filler);

    // Verify GameModifiers cost modifier
    game.state.recalculate_constant_cost_modifiers();
    let cost_mod = game.state.mods.get_cost_modifier(himeno);
    assert_eq!(cost_mod, -6, "3 Mirakura members on stage → cost -6");

    // Also verify via calculate_play_cost_reduction directly
    let reduction = rabuka_engine::ability::util::calculate_play_cost_reduction(
        &game.state.player1.stage,
        &game.state.player1.success_live_card_zone.cards,
        game.state.player1.hand.cards.len(),
        himeno,
        &game.db,
    );
    assert_eq!(
        reduction, 6,
        "calculate_play_cost_reduction = 6 for 3 mirakura"
    );

    // Play the card via baton touch (all 3 slots occupied).
    // Replacing Center (mk2, cost 10): cost = 20 - 6 - 10 = 4
    game.give_energy(4);
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::Center);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "All 4 energy consumed (20 base - 6 reduction - 10 baton touch)"
    );
}
