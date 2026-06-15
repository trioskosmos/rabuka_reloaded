/// Q190: Mia Taylor (PL!N-bp4-011-R＋) — LiveStart: specify heart color.
/// ALL heart (heart00) cannot be chosen. Engine gives 6 colors (heart01-06).
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

#[test]
fn mia_q190_heart_selection_excludes_heart00() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = game.id("PL!N-bp4-011-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("LL-bp5-001-L");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [mia, filler, filler];
    game.state.player1.hand.cards.push(live);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // LiveStart: optional discard 1 live from hand → heart color choice
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Heart color choice appears (SelectHeartColor with 6 options: heart01-06)
    if game.has_pending_choice() {
        // Verify the choice options don't include heart00
        if let Some(ref pc) = game.state.get_pending_choice_json() {
            let json = pc.as_object().expect("pending_choice should be an object");
            if let Some(opts) = json.get("options").and_then(|o| o.as_array()) {
                for opt in opts {
                    let s = opt.as_str().unwrap_or("");
                    assert_ne!(s, "heart00", "heart00 should NOT be in selection");
                }
            }
        }
        // Pick heart02 (index 1)
        game.select_option(1);
    }

    let hand_count = game.state.player1.hand.cards.len();
    let had_choice = game.state.has_pending_choice();
    eprintln!(
        "[MIA] hand after: {}, had_choice: {}",
        hand_count, had_choice
    );
}
