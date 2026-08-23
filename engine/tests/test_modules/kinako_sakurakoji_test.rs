/// Tests for 桜小路きな子 (PL!SP-pb1-006-R) — Auto ability:
///
/// 自動 このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、ブレードブレードを得る。
///
/// Q94: Debut then area move → ability triggers twice (2+2 = 4 blade total).
/// Q171: "Until live end" effects expire at LiveVictoryDetermination end.
use crate::helpers::*;

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

    assert_eq!(
        game.state.mods.get_blade_modifier(kinako),
        2,
        "Debut grants 2 blade (Q94)"
    );
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

    assert_eq!(
        game.state.mods.get_blade_modifier(kinako),
        2,
        "2 blade granted with live_end duration (Q171)"
    );
}

/// Q94 CORE SCENARIO: debut AND area-move each grant +2.
/// 桜小路きな子 (PL!SP-pb1-006-R) debuts (+2), then an area move
/// (+2 again) — 「登場か、エリアを移動するたび」 fires for BOTH events,
/// totaling exactly +4 until live end.
///
/// The area move is driven through a real effect: 桜小路きな子
/// (PL!SP-bp5-006-R)'s 起動 position-change swaps the two members.
#[test]
fn kinako_q94_debut_then_area_move_grants_4_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako_q94 = game.id("PL!SP-pb1-006-R"); // 自動: debut/move → +2 blade
    let kinako_swap = game.id("PL!SP-bp5-006-R"); // 起動: swap positions
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(kinako_q94);
    game.add_to_hand(kinako_swap);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(40);

    // Debut きな子(Q94) at LEFT: +2 blade.
    game.play_to_stage(
        kinako_q94,
        rabuka_engine::zones::MemberArea::LeftSide,
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(kinako_q94),
        2,
        "debut leg: +2"
    );

    // Play the swap-きな子 at RIGHT so the swap has a partner.
    game.play_to_stage(
        kinako_swap,
        rabuka_engine::zones::MemberArea::RightSide,
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(kinako_q94),
        2,
        "the OTHER member's debut must not touch きな子(Q94)"
    );

    // Swap: Q94-きな子 moves left → right.
    game.activate_ability(kinako_swap);
    assert!(
        game.has_pending_choice(),
        "swap ability must present a destination choice"
    );
    let actions = game.generated_actions();
    let left_idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                == Some("left")
        })
        .expect("left destination should be offered");
    game.select_generated(left_idx);

    // The swap happened…
    assert_eq!(
        game.state.player1.stage.stage[2], kinako_q94,
        "Q94-きな子 moved to RIGHT"
    );
    assert_eq!(
        game.state.player1.stage.stage[0], kinako_swap,
        "swap-きな子 moved to LEFT"
    );

    // …and the move leg fired: 2 (debut) + 2 (move) = 4.
    assert_eq!(
        game.state.mods.get_blade_modifier(kinako_q94),
        4,
        "Q94: debut + area move = exactly +4 blade"
    );
}
