/// Proper-gameplay coverage for PL!SP-bp2-015-N / PL!SP-bp2-021-N
/// 自動 ターン1回 エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで heartを得る。
///
/// Prior tests in `sp_bp2_015_021_comprehensive_edge_test.rs` use synthetic
/// `revealed_cards` injection. This file drives the REAL live pipeline:
/// stage blade → yell count → deck-top reveals → auto trigger, using secondary
/// cards as deck-top yell material and stage blade providers.
///
/// Primary: PL!SP-bp2-015-N (Sumire) / PL!SP-bp2-021-N (Wien) – must be on stage.
/// Secondary stage blade: PL!-sd1-010-SD (Honoka, blade 1, b_heart03, heart01+heart03)
/// Secondary yell material:
///   no-blade – PL!S-bp2-002-R (Riko, no blade_heart, no blade)
///   blade    – PL!-pb1-014-R (Rin, b_heart01, blade 3)
///   ALL      – PL!HS-PR-010-PR (Reflection, b_all)
/// Secondary live: PL!-sd1-019-SD START:DASH!! (need 01+03+06, satisfied by Sumire+Wien+Honoka)
///
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

const SUMIRE: &str = "PL!SP-bp2-015-N";
const WIEN: &str = "PL!SP-bp2-021-N";
const HONOKA: &str = "PL!-sd1-010-SD";
const NO_BLADE: &str = "PL!S-bp2-002-R";
const BLADE: &str = "PL!-pb1-014-R";
const ALL_BLADE: &str = "PL!HS-PR-010-PR";
const LIVE: &str = "PL!-sd1-019-SD";

fn heart06(g: &TestGame, id: i16) -> i32 {
    g.state.mods.get_heart_modifier(id, HeartColor::Heart06)
}
fn heart03(g: &TestGame, id: i16) -> i32 {
    g.state.mods.get_heart_modifier(id, HeartColor::Heart03)
}

/// Advance Main → LiveCardSet (5 passes) and set live card.
/// Returns the live card id.
fn set_live_via_phase(game: &mut TestGame, live_id: i16) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(
        game.state.current_phase.to_string().contains("LiveCardSet"),
        "must be LiveCardSet, got {}",
        game.state.current_phase
    );
    game.set_live_card(live_id);
    // FirstAttacker pass (p2 sets live), SecondAttacker pass → first performance entry
    game.pass();
    game.pass();
    // One more pass lets first performance resolve (yell) and move to second performance
    game.pass();
}

/// Prepare a game where stage = [sumire, wien, honoka], decks stacked,
/// live in hand, enough energy. Does NOT start live yet.
fn setup_game_with_deck_top(deck_top: &[&str]) -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id(SUMIRE);
    let wien = game.id(WIEN);
    let honoka = game.id(HONOKA);

    // Decks: draw() removes from front (index 0), so top = front.
    // Build main_deck with controlled top at front, filler at back.
    // Note: the 5-pass advance includes a Draw phase that consumes 1 top card to hand
    // before Live, so we prepend one extra NO_BLADE to absorb that draw.
    game.state.player1.main_deck.cards.clear();
    // Fill bottom with distinct Honoka copies
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(game.new_id(HONOKA));
    }
    // Replace top N (front) with requested cards in order: deck_top[0] = topmost
    for _ in 0..deck_top.len() {
        game.state.player1.main_deck.cards.remove(0);
    }
    for &no in deck_top.iter().rev() {
        let id = game.new_id(no);
        game.state.player1.main_deck.cards.insert(0, id);
    }
    // Extra card to be drawn in Draw phase so yell sees exactly deck_top
    game.state.player1.main_deck.cards.insert(0, game.new_id(NO_BLADE));
    // Player2 deck filler distinct
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(game.new_id(HONOKA));
    }
    // Stage: sumire, wien, honoka provide hearts 01+02+03+06 covering LIVE need
    game.state.player1.stage.stage = [sumire, wien, honoka];
    // Energy: need to pay live? START:DASH!! is live card, no cost. Stage members already on stage, no play cost now.
    game.state.player1.energy_zone.cards.clear();
    game.give_energy(10);
    // Hand: live card
    let live = game.id(LIVE);
    game.state.player1.hand.cards.push(live);

    // Ensure waitroom empty
    game.state.player1.waitroom.cards.clear();
    // Ensure revealed_cards empty
    game.state.revealed_cards.clear();
    game.state.yell_occurred = false;

    (game, sumire, wien)
}

/// Real live yell with 3 no-blade cards → both Sumire and Wien gain.
#[test]
fn yell_proper_no_blade_gains_via_live() {
    // total_blade = 1+1+1 =3 → yell reveals 3 cards
    let (mut game, sumire, wien) = setup_game_with_deck_top(&[NO_BLADE, NO_BLADE, NO_BLADE]);
    let live = game.state.player1.hand.cards[0];
    let tb = game.state.player1.stage.total_blades(
        &game.db,
        &game.state.mods.blade_modifiers,
        &game.state.mods.orientation_modifiers,
        false,
    );
    eprintln!("pre-live total_blade={} stage={:?}", tb, game.state.player1.stage.stage);
    eprintln!("pre-live hearts stage available: {:?}", game.state.player1.stage.get_available_hearts(&game.db, &game.state.mods.heart_override, &game.state.mods.heart_modifiers, &game.state.mods.heart_color_multiplier, &game.state.mods.heart_copy));
    eprintln!("live need: {:?}", game.db.get_card(live).unwrap().need_heart);
    set_live_via_phase(&mut game, live);

    // After performance, yell occurred and autos resolved.
    // Drain any remaining choices (e.g., live success look)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // Debug verdicts on failure
    if heart06(&game, sumire) != 1 || heart03(&game, wien) != 1 {
        eprintln!("rule_log: {:?}", game.state.rule_log);
        eprintln!("revealed: {:?}", game.state.revealed_cards.iter().map(|id| game.name(*id)).collect::<Vec<_>>());
        for &id in &game.state.revealed_cards {
            let card = game.db.get_card(id).unwrap();
            eprintln!("revealed id {} {} has_blade_heart={} blade_heart={:?} blade={}", id, card.card_no, card.has_blade_heart(), card.blade_heart, card.blade);
        }
        eprintln!("yell_occurred: {}", game.state.yell_occurred);
        eprintln!("live_zone: {:?}", game.state.player1.live_card_zone.cards.iter().map(|id| game.name(*id)).collect::<Vec<_>>());
        eprintln!("stage: {:?}", game.state.player1.stage.stage.iter().map(|id| if *id==-1 {"empty".to_string()} else {game.name(*id)}).collect::<Vec<_>>());
        eprintln!("deck len: {}", game.state.player1.main_deck.cards.len());
        eprintln!("phase: {}", game.state.current_phase);
        eprintln!("player1 wait: {:?}", game.state.player1.waitroom.cards.iter().map(|id| game.name(*id)).collect::<Vec<_>>());
        eprintln!("mods heart06 sumire: {} heart03 wien: {}", heart06(&game, sumire), heart03(&game, wien));
        eprintln!("{}", crate::helpers::ability_verdicts(&mut game, "p1"));
    }
    // One more pass to reach LiveVictoryDetermination / cleanup if needed
    // Blade heart absence should have triggered both autos (Turn1)
    assert_eq!(
        heart06(&game, sumire),
        1,
        "Sumire should gain heart06 when yell had no blade (proper gameplay)"
    );
    assert_eq!(
        heart03(&game, wien),
        1,
        "Wien should gain heart03 when yell had no blade (proper gameplay)"
    );
}

/// Yell contains a blade heart → both blocked.
#[test]
fn yell_proper_with_blade_blocks_via_live() {
    // 2 no-blade + 1 blade (blade at top position, will be among 3 reveals)
    let (mut game, sumire, wien) =
        setup_game_with_deck_top(&[NO_BLADE, NO_BLADE, BLADE]);
    let live = game.state.player1.hand.cards[0];
    set_live_via_phase(&mut game, live);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        heart06(&game, sumire),
        0,
        "Sumire must NOT gain when yell contained a blade heart"
    );
    assert_eq!(
        heart03(&game, wien),
        0,
        "Wien must NOT gain when yell contained a blade heart"
    );
}

/// ALL blade counts as blade → blocked (Q112).
#[test]
fn yell_proper_all_blade_blocks_via_live() {
    let (mut game, sumire, wien) =
        setup_game_with_deck_top(&[NO_BLADE, NO_BLADE, ALL_BLADE]);
    let live = game.state.player1.hand.cards[0];
    set_live_via_phase(&mut game, live);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(heart06(&game, sumire), 0, "ALL blade must block Sumire");
    assert_eq!(heart03(&game, wien), 0, "ALL blade must block Wien");
}
