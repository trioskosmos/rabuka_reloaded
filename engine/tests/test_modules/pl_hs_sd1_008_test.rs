use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass();
    game.pass();
}

#[test]
fn pl_hs_sd1_008_live_start_pay_cost_select_heart01_target_ally() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let izumi = game.id("PL!HS-sd1-008-SD");
    let kozue = game.id("PL!HS-bp1-012-PR");
    let tsuzuri = game.id("PL!HS-PR-004-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kozue, izumi, tsuzuri];
    let cost1 = game.new_id("PL!HS-bp1-012-PR");
    let cost2 = game.new_id("PL!HS-PR-004-PR");
    game.add_to_hand(cost1);
    game.add_to_hand(cost2);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    let mut step = 0;
    while game.has_pending_choice() && step < 20 {
        step += 1;
        let ct = game.pending_choice_type().unwrap_or_default();
        eprintln!("[STEP {}] choice_type={}", step, ct);
        game.dbg_choice();
        match ct.as_str() {
            "SelectCard" => {
                game.select_indices(&[0]);
            }
            "SelectHeartColor" => {
                game.select_indices(&[1]);
            }
            "SelectAutoAbility" => {
                game.select_indices(&[]);
            }
            "SelectTarget" => {
                game.select_indices(&[]);
            }
            _ => {
                game.dbg_all();
                panic!("Unknown choice type at step {}: {}", step, ct);
            }
        }
    }

    for &card in &[kozue, izumi, tsuzuri] {
        for &hc in &[
            HeartColor::Heart01,
            HeartColor::Heart04,
            HeartColor::Heart05,
            HeartColor::Heart06,
        ] {
            let mod_val = game.state.mods.get_heart_modifier(card, hc);
            eprintln!("  card={} heart={:?} mod={}", game.name(card), hc, mod_val);
        }
    }

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kozue, HeartColor::Heart01),
        2,
        "kozue should have +2 heart01"
    );
}

#[test]
fn pl_hs_sd1_008_live_start_skip_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let izumi = game.id("PL!HS-sd1-008-SD");
    let kozue = game.id("PL!HS-bp1-012-PR");
    let tsuzuri = game.id("PL!HS-PR-004-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kozue, izumi, tsuzuri];
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    let mut step = 0;
    while game.has_pending_choice() && step < 10 {
        step += 1;
        let ct = game.pending_choice_type().unwrap_or_default();
        eprintln!("[SKIP STEP {}] choice_type={}", step, ct);
        if ct == "SelectAutoAbility" {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[]);
        }
    }
    eprintln!("[SKIP] done, step={}", step);

    let total: i32 = [
        HeartColor::Heart01,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ]
    .iter()
    .map(|&c| {
        game.state.mods.get_heart_modifier(kozue, c)
            + game.state.mods.get_heart_modifier(tsuzuri, c)
    })
    .sum();
    assert_eq!(total, 0, "Skipping cost should grant no hearts");
}
