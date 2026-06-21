use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn setup_p1_deck(game: &mut TestGame, live_ids: &[i16]) {
    let filler = game.id_ref("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for (i, &id) in live_ids.iter().enumerate() {
        game.state.player1.main_deck.cards.insert(1 + i, id);
    }
}

fn dbg_revealed(game: &TestGame) {
    eprintln!(
        "  revealed_cards: {:?}",
        game.state
            .revealed_cards
            .iter()
            .map(|&id| (game.name(id), id))
            .collect::<Vec<_>>()
    );
}

/// Debug test to trace revealed_cards state throughout the flow.
#[test]
fn q253_debug_revealed_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let galaxy = game.id("PL!S-bp6-023-L");
    let yell_live = game.id("PL!-sd1-020-SD");

    game.add_to_stage(MemberArea::LeftSide, game.id("PL!S-pb1-003-R"));
    game.add_to_stage(MemberArea::Center, game.id("PL!S-bp2-017-N"));
    game.add_to_stage(MemberArea::RightSide, game.id("PL!S-bp3-015-N"));

    setup_p1_deck(&mut game, &[yell_live]);

    eprintln!("=== Before live card set ===");
    eprintln!("  phase: {}", game.state.current_phase);
    eprintln!(
        "  deck top 4: {:?}",
        game.state.player1.main_deck.peek_top(4)
    );
    eprintln!("  deck len: {}", game.state.player1.main_deck.len());

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(galaxy);
    game.set_live_card(galaxy);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    eprintln!("\n=== After live start ===");
    eprintln!("  phase: {}", game.state.current_phase);
    dbg_revealed(&game);
    eprintln!("  deck len: {}", game.state.player1.main_deck.len());
    eprintln!(
        "  hand: {:?}",
        game.state
            .player1
            .hand
            .cards
            .iter()
            .map(|&id| game.name(id))
            .collect::<Vec<_>>()
    );

    // First performance (P1 yell)
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    eprintln!("\n=== After P1 performance ===");
    eprintln!("  phase: {}", game.state.current_phase);
    dbg_revealed(&game);
    eprintln!("  deck len: {}", game.state.player1.main_deck.len());

    // Second performance (P2 yell)
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    eprintln!("\n=== After P2 performance ===");
    eprintln!("  phase: {}", game.state.current_phase);
    dbg_revealed(&game);

    // Live Victory Determination (Live Success triggers here)
    game.pass();
    eprintln!("\n=== After LVD pass ===");
    eprintln!("  phase: {}", game.state.current_phase);
    dbg_revealed(&game);

    while game.has_pending_choice() {
        eprintln!("  pending: {:?}", game.pending_choice_type());
        dbg_revealed(&game);
        game.select_indices(&[]);
    }

    eprintln!("\n=== After all choices ===");
    eprintln!("  phase: {}", game.state.current_phase);
    dbg_revealed(&game);

    eprintln!("\n=== Results ===");
    let sm = game.state.mods.get_score_modifier(galaxy);
    eprintln!("  GALAXY score_modifier: {}", sm);
}
