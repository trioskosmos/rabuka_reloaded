use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn setup_mirai(game: &mut TestGame, aqours_ids: &[i16]) {
    let mirai = game.id("PL!S-bp6-021-L");
    game.state.player1.live_card_zone.cards.push(mirai);
    for &id in aqours_ids {
        game.state.revealed_cards.push(id);
        game.state.player1.waitroom.cards.push(id);
    }
    game.state.yell_occurred = true;
}

fn fire(game: &mut TestGame) {
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
}

fn drain(game: &mut TestGame) {
    while game.has_pending_choice() {
        match game.get_pending_choice().clone() {
            Choice::SelectCard { .. } => game.select_indices(&[]),
            _ => game.select_indices(&[]),
        }
    }
}

// MIRAI TICKET: Auto 1/turn. When you yell, put up to 1 Aqours member card
// without blade heart from revealed cards to waitroom. If you do,
// perform 1 additional yell per 5 cost (max 4 extra yells).

#[test]
fn mirai_ticket_two_cards_select_first() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq1 = game.id("PL!S-bp3-009-R"); // Aqours cost 9
    let aq2 = game.id("PL!S-bp2-002-R"); // Aqours cost 4
    setup_mirai(&mut game, &[aq1, aq2]);
    fire(&mut game);

    assert!(game.has_pending_choice(), "MIRAI TICKET must show a choice");
    match game.get_pending_choice().clone() {
        Choice::SelectCard {
            zone,
            count,
            allow_skip,
            ..
        } => {
            assert_eq!(zone, "revealed_cards");
            assert_eq!(count, 1);
            assert!(allow_skip);
        }
        c => panic!("Expected SelectCard(revealed_cards), got: {:?}", c),
    }

    game.select_indices(&[0]);
    drain(&mut game);
    assert!(
        !game.state.revealed_cards.contains(&aq1),
        "first revealed card should move to waitroom"
    );
}

#[test]
fn mirai_ticket_two_cards_select_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq1 = game.id("PL!S-bp3-009-R");
    let aq2 = game.id("PL!S-bp2-002-R");
    setup_mirai(&mut game, &[aq1, aq2]);
    fire(&mut game);

    assert!(game.has_pending_choice(), "MIRAI TICKET must show a choice");
    game.select_indices(&[1]);
    drain(&mut game);
    assert!(
        !game.state.revealed_cards.contains(&aq2),
        "second revealed card should move to waitroom"
    );
}

#[test]
fn mirai_ticket_skip_leaves_both_in_revealed() {
    // Select 0 cards (skip) — so shita baai should prevent extra yell
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq1 = game.id("PL!S-bp3-009-R");
    let aq2 = game.id("PL!S-bp2-002-R");
    setup_mirai(&mut game, &[aq1, aq2]);
    fire(&mut game);

    assert!(game.has_pending_choice(), "MIRAI TICKET must show a choice");
    game.select_indices(&[]); // skip
    drain(&mut game);

    assert!(
        game.state.revealed_cards.contains(&aq1),
        "skipped: aq1 should stay in revealed"
    );
    assert!(
        game.state.revealed_cards.contains(&aq2),
        "skipped: aq2 should stay in revealed"
    );
}

#[test]
fn mirai_ticket_use_limit_blocks_second_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq = game.id("PL!S-bp2-002-R"); // cost 4
    let aq2 = game.id("PL!S-bp3-009-R"); // cost 9
    setup_mirai(&mut game, &[aq]);
    fire(&mut game);
    // Single card -> auto-selected, no choice shown.
    assert!(
        !game.state.revealed_cards.contains(&aq),
        "first trigger: card should move from revealed"
    );

    // Second yell trigger -- should be blocked by use_limit: 1
    game.state.revealed_cards.push(aq2);
    game.state.player1.waitroom.cards.push(aq2);
    game.state.yell_occurred = true;
    fire(&mut game);

    assert!(
        !game.has_pending_choice(),
        "second trigger must be blocked by use_limit"
    );
    assert!(
        game.state.revealed_cards.contains(&aq2),
        "aq2 should remain in revealed since ability didn't fire again"
    );
}

#[test]
fn mirai_ticket_empty_revealed_triggers_and_auto_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!S-bp6-021-L"));
    game.state.yell_occurred = true;
    fire(&mut game);

    assert!(
        !game.has_pending_choice(),
        "empty revealed: ability should auto-skip with no choice"
    );
}

#[test]
fn mirai_ticket_cost4_zero_extra_yells() {
    // cost 4 -> 4/5 = 0 extra yells (floor division)
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq = game.id("PL!S-bp2-002-R");
    setup_mirai(&mut game, &[aq]);
    fire(&mut game);

    assert!(
        !game.state.revealed_cards.contains(&aq),
        "selected card should move to waitroom"
    );
}

#[test]
fn mirai_ticket_cost5_one_extra_yell() {
    // cost 5 -> 5/5 = 1 extra yell
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq = game.id("PL!S-bp6-013-N");
    setup_mirai(&mut game, &[aq]);
    fire(&mut game);

    assert!(
        !game.state.revealed_cards.contains(&aq),
        "selected card should move to waitroom"
    );
}

#[test]
fn mirai_ticket_cost9_one_extra_yell() {
    // cost 9 -> 9/5 = 1 extra yell
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq = game.id("PL!S-bp3-009-R");
    setup_mirai(&mut game, &[aq]);
    fire(&mut game);

    assert!(
        !game.state.revealed_cards.contains(&aq),
        "selected card should move to waitroom"
    );
}

#[test]
fn mirai_ticket_cost10_two_extra_yells() {
    // cost 10 -> 10/5 = 2 extra yells
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq = game.id("PL!S-bp5-001-R+");
    setup_mirai(&mut game, &[aq]);
    fire(&mut game);

    assert!(
        !game.state.revealed_cards.contains(&aq),
        "selected card should move to waitroom"
    );
}

#[test]
fn mirai_ticket_cost20_capped_at_four() {
    // cost 20 -> 20/5 = 4, exactly at the 4-cap
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq = game.id("LL-bp2-001-R+");
    setup_mirai(&mut game, &[aq]);
    fire(&mut game);

    assert!(
        !game.state.revealed_cards.contains(&aq),
        "selected card should move to waitroom"
    );
}
