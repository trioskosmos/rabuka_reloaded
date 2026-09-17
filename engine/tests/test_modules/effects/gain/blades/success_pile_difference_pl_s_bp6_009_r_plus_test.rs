use crate::helpers::*;

const LIVE_FILLER: &str = "PL!-sd1-019-SD";

#[test]
fn pl_s_bp6_009_r_plus_constant_blades_follow_opponent_success_pile_lead() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp6-009-R＋");
    let live = game.id(LIVE_FILLER);

    game.state.player1.stage.stage[1] = ruby;

    for _ in 0..3 {
        game.state.player2.success_live_card_zone.cards.push(live);
    }
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        3,
        "diff 3 → ブレード3"
    );

    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        1,
        "diff 1 → ブレード1"
    );

    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        0,
        "piles tied → 「自分より多い」 fails → no blades"
    );
}
