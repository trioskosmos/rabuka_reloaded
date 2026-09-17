use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame) {
    let f1 = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(f1);
    }
    let f2 = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(f2);
    }
}

fn give_energy_p2(game: &mut TestGame, n: usize) {
    for _ in 0..n {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.add_active(n as u8);
}

fn energy_p1(g: &TestGame) -> u8 {
    g.state.player1.energy_zone.active_count()
}
fn energy_p2(g: &TestGame) -> u8 {
    g.state.player2.energy_zone.active_count()
}

fn dbg(g: &TestGame) {
    let s1 = &g.state.player1.stage.stage;
    eprintln!(
        "  P1 stage: [{},{},{}] e={} ph={:?} tp={:?} t={}",
        s1[0],
        s1[1],
        s1[2],
        energy_p1(g),
        g.state.current_phase,
        g.state.current_turn_phase,
        g.state.turn_number
    );
    let s2 = &g.state.player2.stage.stage;
    eprintln!(
        "  P2 stage: [{},{},{}] e={}",
        s2[0],
        s2[1],
        s2[2],
        energy_p2(g)
    );
}

fn drain(game: &mut TestGame, _label: &str) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 20 {
        safety += 1;
        use rabuka_engine::ability::types::Choice;
        match game.state.get_pending_choice().unwrap().clone() {
            Choice::SelectAutoAbility { .. } => {
                game.select_indices(&[]);
            }
            Choice::SelectCard { count, .. } => {
                if count > 0 && count < 10 {
                    game.select_indices(&(0..count).collect::<Vec<_>>());
                } else {
                    game.select_indices(&[0]);
                }
            }
            Choice::SelectTarget { target, .. }
                if target == "position|destination" || target == "area_select" =>
            {
                let acts = game.generated_actions();
                if acts.is_empty() {
                    game.select_indices(&[]);
                } else {
                    game.select_generated(0);
                }
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }
}

fn activate_and_drain(game: &mut TestGame, card: i16, label: &str) {
    eprintln!("--- {}: activate {}", label, card);
    dbg(game);
    game.activate_ability(card);
    drain(game, label);
    dbg(game);
}

fn pass_phase(game: &mut TestGame) {
    game.pass();
}

// ====================================================================
// Step 1: P1 Main phase position change → auto ability fires
// ====================================================================
#[test]
fn step1_p1_single() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let watcher = game.id("PL!SP-pb2-028-N");
    let mover = game.id("PL!SP-bp5-006-R");
    fill_decks(&mut game);
    game.give_energy(20);
    game.state.player1.hand.cards.push(mover);
    game.state.player1.hand.cards.push(watcher);
    game.play_to_stage(mover, MemberArea::LeftSide);
    game.play_to_stage(watcher, MemberArea::Center);
    let e_before = energy_p1(&game);
    activate_and_drain(&mut game, mover, "s1");
    assert_eq!(energy_p1(&game) - e_before, 2);
}

// ====================================================================
// Step 2: P1 then P2 position change, each on their own Main phase
// ====================================================================
#[test]
fn step2_p1_then_p2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p1_w = game.id("PL!SP-pb2-028-N");
    let p1_m = game.id("PL!SP-bp5-006-R");
    let p2_w = game.id("PL!SP-pb2-028-N");
    let p2_m = game.id("PL!SP-bp5-006-R");
    fill_decks(&mut game);
    game.give_energy(20);
    give_energy_p2(&mut game, 20);
    game.state.player1.hand.cards.push(p1_m);
    game.state.player1.hand.cards.push(p1_w);
    game.play_to_stage(p1_m, MemberArea::LeftSide);
    game.play_to_stage(p1_w, MemberArea::Center);
    game.state.player2.stage.stage = [p2_m, p2_w, -1];
    // P1
    let e1 = energy_p1(&game);
    activate_and_drain(&mut game, p1_m, "p1");
    assert_eq!(energy_p1(&game) - e1, 2);
    // Advance: Main→Active→Energy→Draw→Main (P2's turn)
    pass_phase(&mut game);
    pass_phase(&mut game);
    pass_phase(&mut game);
    pass_phase(&mut game);
    // P2
    let e2 = energy_p2(&game);
    activate_and_drain(&mut game, p2_m, "p2");
    assert_eq!(energy_p2(&game) - e2, 2);
}

// ====================================================================
// Step 3: use_limit blocks second activation in same turn
// ====================================================================
#[test]
fn step3_use_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let watcher = game.id("PL!SP-pb2-028-N");
    let mover = game.id("PL!SP-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(20);
    game.state.player1.hand.cards.push(mover);
    game.state.player1.hand.cards.push(watcher);
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(mover, MemberArea::LeftSide);
    game.play_to_stage(watcher, MemberArea::Center);
    game.play_to_stage(filler, MemberArea::RightSide);
    let e1 = energy_p1(&game);
    activate_and_drain(&mut game, mover, "s3a");
    assert_eq!(energy_p1(&game) - e1, 2);
    let result = game.try_activate_ability(mover);
    assert!(result.is_err(), "use_limit=1 blocks 2nd activation");
}

// ====================================================================
// Step 4: Live → Turn 2 → P1 Main + P2 Main position changes (use_limits reset)
// ====================================================================
#[test]
fn step4_turn2_reset() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let watcher = game.id("PL!SP-pb2-028-N");
    let mover = game.id("PL!SP-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");
    let p2_mover = game.id("PL!SP-bp5-006-R");
    let p2_watcher = game.id("PL!SP-pb2-028-N");
    fill_decks(&mut game);
    game.give_energy(20);
    give_energy_p2(&mut game, 20);
    game.state.player1.hand.cards.push(mover);
    game.state.player1.hand.cards.push(watcher);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(live_card);
    game.play_to_stage(mover, MemberArea::LeftSide);
    game.play_to_stage(watcher, MemberArea::Center);
    game.play_to_stage(filler, MemberArea::RightSide);
    game.state.player2.stage.stage = [p2_mover, p2_watcher, -1];

    // T1 P1 Main
    activate_and_drain(&mut game, mover, "t1p1");
    assert_eq!(energy_p1(&game), 2);

    // T1 P2 Main
    pass_phase(&mut game);
    pass_phase(&mut game);
    pass_phase(&mut game);
    pass_phase(&mut game);
    let p2e = energy_p2(&game);
    activate_and_drain(&mut game, p2_mover, "t1p2");
    assert_eq!(energy_p2(&game) - p2e, 2);

    // Advance through Live to Turn 2
    // P2 Main → LiveCardSet → LiveCardSet2 → Perf1 → Perf2 → LiveVictory → T2 Active
    pass_phase(&mut game); // Main→LiveCardSet
    game.set_live_card(live_card);
    pass_phase(&mut game); // LiveCardSet→LiveCardSet2
    drain(&mut game, "ls");
    pass_phase(&mut game); // LiveCardSet2→Perf1
    pass_phase(&mut game); // Perf1→Perf2
    pass_phase(&mut game); // Perf2→LiveVictory
    pass_phase(&mut game); // LiveVictory→T2 Active (P1)

    assert_eq!(game.state.turn_number, 2);
    assert_eq!(
        game.state.current_turn_phase,
        rabuka_engine::types::TurnPhase::FirstAttackerNormal
    );

    // T2 P1 Main
    pass_phase(&mut game);
    pass_phase(&mut game);
    pass_phase(&mut game);
    let e_before = energy_p1(&game);
    activate_and_drain(&mut game, mover, "t2p1");
    assert_eq!(energy_p1(&game) - e_before, 2);

    // T2 P2 Main
    pass_phase(&mut game);
    pass_phase(&mut game);
    pass_phase(&mut game);
    pass_phase(&mut game);
    let p2e_before = energy_p2(&game);
    activate_and_drain(&mut game, p2_mover, "t2p2");
    assert_eq!(energy_p2(&game) - p2e_before, 2);
}

// ====================================================================
// Step 5: each_time:area_move trigger (鬼塚夏美) fires when she moves via position change
// ====================================================================
#[test]
fn step5_each_time_area_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // 鬼塚夏美 pb1-020: each_time:area_move → draw 1 (only from other card's effect)
    let natsumi = game.id("PL!SP-pb1-020-N");
    let mover = game.id("PL!SP-bp5-006-R");
    fill_decks(&mut game);
    game.give_energy(20);
    // Mover at Left, natsumi at Center → swap moves natsumi
    game.state.player1.hand.cards.push(mover);
    game.state.player1.hand.cards.push(natsumi);
    game.play_to_stage(mover, MemberArea::LeftSide);
    game.play_to_stage(natsumi, MemberArea::Center);
    let hand_before = game.state.player1.hand.cards.len();
    activate_and_drain(&mut game, mover, "s5");
    let hand_delta = game.state.player1.hand.cards.len() - hand_before;
    assert_eq!(
        hand_delta, 1,
        "natsumi's each_time:area_move draws 1 when she's swapped"
    );
}
