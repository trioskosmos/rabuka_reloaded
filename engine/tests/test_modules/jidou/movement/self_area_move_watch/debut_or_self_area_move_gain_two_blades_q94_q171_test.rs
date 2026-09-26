/// Tests for 桜小路きな子 (PL!SP-pb1-006-R) — Auto ability:
///
/// 自動 このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、ブレードブレードを得る。
///
/// Q94: Debut then area move → ability triggers twice (2+2 = 4 blade total).
/// Q171: "Until live end" effects expire at LiveVictoryDetermination end.
use crate::helpers::*;

/// 桜小路きな子 PL!SP-pb1-006-R (cost 9 — the energy budget below depends on it).
/// Pinned because PL!SP-bp5-006-R and PL!SP-pb2-006-R are two more printings
/// of the same character, at costs 11 and 2.
fn kinako_q94_id(game: &mut TestGame) -> i16 {
    let id = game.id("PL!SP-pb1-006-R");
    game.assert_card_identity(id, "PL!SP-pb1-006-R");
    game.assert_card_cost(id, 9);
    id
}

/// Q94: Debut triggers the auto ability, granting 2 blade.
#[test]
fn debut_grants_two_blades_until_live_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako = kinako_q94_id(&mut game);
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

#[test]
fn debut_blades_persist_until_live_victory_then_expire_without_performing() {
    use rabuka_engine::game_state::Phase;

    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = kinako_q94_id(&mut game);
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.add_to_hand(member);
    game.give_energy(9);
    game.play_to_stage(member, rabuka_engine::zones::MemberArea::LeftSide);
    assert_eq!(game.state.mods.get_blade_modifier(member), 2);

    for _ in 0..20 {
        if game.state.current_phase == Phase::LiveVictoryDetermination {
            break;
        }
        assert!(!game.has_pending_choice());
        assert_eq!(game.state.mods.get_blade_modifier(member), 2);
        game.pass();
    }
    assert_eq!(game.state.current_phase, Phase::LiveVictoryDetermination);
    assert!(game.state.player1.live_card_zone.cards.is_empty());
    assert!(game.state.player2.live_card_zone.cards.is_empty());
    assert_eq!(game.state.mods.get_blade_modifier(member), 2);
    let turn_before = game.state.turn_number;
    game.pass();
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.current_phase, Phase::Active);
    assert!(game.state.turn_number > turn_before);
    assert_eq!(game.state.player1.stage.stage[0], member);
    assert_eq!(game.state.mods.get_blade_modifier(member), 0);
}

/// Q94 CORE SCENARIO: debut AND area-move each grant +2.
/// 桜小路きな子 (PL!SP-pb1-006-R) debuts (+2), then an area move
/// (+2 again) — 「登場か、エリアを移動するたび」 fires for BOTH events,
/// totaling exactly +4 until live end.
///
/// The area move is driven through a real effect: 桜小路きな子
/// (PL!SP-bp5-006-R)'s 起動 position-change swaps the two members.
#[test]
fn debut_then_effect_swap_stacks_two_blade_grants() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako_q94 = kinako_q94_id(&mut game); // 自動: debut/move → +2 blade
    let kinako_swap = game.id("PL!SP-bp5-006-R"); // 起動: swap positions
    // Two printings of the same character, deliberately: the auto watcher and
    // the 起動 swapper are different cards on stage.
    game.assert_card_identity(kinako_swap, "PL!SP-bp5-006-R");
    game.assert_same_card_name(kinako_q94, kinako_swap, "two きな子 printings");
    assert_ne!(kinako_q94, kinako_swap, "two separate card instances");
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
