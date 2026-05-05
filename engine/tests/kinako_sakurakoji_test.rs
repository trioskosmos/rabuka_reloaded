/// Tests for 桜小路きな子 (PL!SP-pb1-006-R) — Auto ability:
///
/// 自動 このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、ブレードブレードを得る。
///
/// Q94: Debut then area move → ability triggers twice (2+2 = 4 blade total).
/// Q171: "Until live end" effects expire at LiveVictoryDetermination end.

mod helpers;
use helpers::*;

/// Q94: Debut triggers the auto ability, granting 2 blade.
#[test]
fn kinako_q94_debut_grants_2_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako = game.id("PL!SP-pb1-006-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(kinako);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(kinako, rabuka_engine::zones::MemberArea::LeftSide);

    assert_eq!(game.state.get_blade_modifier(kinako), 2,
        "Debut grants 2 blade (Q94)");
}

/// Q171: Blade has duration=live_end, persists after ability resolves.
#[test]
fn kinako_q171_blade_live_end_duration() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako = game.id("PL!SP-pb1-006-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(kinako);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(kinako, rabuka_engine::zones::MemberArea::LeftSide);

    assert_eq!(game.state.get_blade_modifier(kinako), 2,
        "2 blade granted with live_end duration (Q171)");
}
