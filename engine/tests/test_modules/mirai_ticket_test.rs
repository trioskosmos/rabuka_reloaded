use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn setup_mirai(game: &mut TestGame, aqours_ids: &[i16]) {
    let mirai = game.id("PL!S-bp6-021-L");
    game.state.player1.live_card_zone.cards.push(mirai);
    for &id in aqours_ids {
        game.state.revealed_cards.push(id);
        game.state.player1.waitroom.cards.push(id);
    }
}

fn fire(game: &mut TestGame) {
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
}

fn drain(game: &mut TestGame) {
    while game.has_pending_choice() {
        match game.get_pending_choice().clone() {
            Choice::SelectCard { .. } => {
                game.select_indices(&[]);
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// MIRAI TICKET (PL!S-bp6-021-L) — Live card, auto ability:
//
// 自分がエールしたとき、エールにより公開された自分のカードの中から
// ブレードハートを持たない『Aqours』のメンバーカードを1枚まで
// 控え室に置いてもよい。そうした場合、これにより控え室に置いた
// カードのコスト5につき、追加で1枚エールを行う。
// この能力では4枚までしか追加でエールできない。
//
// Setup: stage is empty → total_blade=0 → additional yells draw 0 cards.
// The test asserts the card was moved (left revealed_cards).
// ═══════════════════════════════════════════════════════════════

/// Single valid card → auto-selects (no choice prompt). Card leaves revealed_cards.
#[test]
fn mirai_ticket_one_card_moved() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq = game.id("PL!S-PR-013-PR"); // cost 11
    setup_mirai(&mut game, &[aq]);
    fire(&mut game);

    assert!(
        !game.state.revealed_cards.contains(&aq),
        "Card must leave revealed_cards after MIRAI TICKET"
    );
}

/// Two valid cards → SelectCard(revealed_cards) with allow_skip=true appears.
#[test]
fn mirai_ticket_two_cards_shows_skippable_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq1 = game.id("PL!S-PR-013-PR");
    let aq2 = game.id("PL!S-bp2-002-R");
    setup_mirai(&mut game, &[aq1, aq2]);
    fire(&mut game);

    assert!(
        game.has_pending_choice(),
        "MIRAI TICKET must show SelectCard with 2+ valid cards"
    );

    let c = game.get_pending_choice().clone();
    match &c {
        Choice::SelectCard {
            zone,
            count,
            allow_skip,
            ..
        } => {
            assert_eq!(zone, "revealed_cards");
            assert_eq!(*count, 1);
            assert!(*allow_skip, "Must allow skipping");
        }
        _ => panic!("Expected SelectCard(revealed_cards), got: {:?}", c),
    }
}

/// Skip the choice → no card moves, both stay in revealed_cards.
#[test]
fn mirai_ticket_skip_leaves_cards_in_revealed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq1 = game.id("PL!S-PR-013-PR");
    let aq2 = game.id("PL!S-bp2-002-R");
    setup_mirai(&mut game, &[aq1, aq2]);
    fire(&mut game);

    assert!(game.has_pending_choice(), "MIRAI TICKET must show a choice");
    game.select_indices(&[]); // skip

    drain(&mut game);

    assert!(
        game.state.revealed_cards.contains(&aq1) && game.state.revealed_cards.contains(&aq2),
        "Both cards must stay in revealed_cards after skipping"
    );
}

/// No revealed_cards → condition fails → MIRAI TICKET does NOT fire.
#[test]
fn mirai_ticket_no_revealed_cards_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!S-bp6-021-L"));
    fire(&mut game);

    if game.has_pending_choice() {
        let c = game.get_pending_choice().clone();
        panic!(
            "MIRAI TICKET must NOT fire without revealed_cards. Got: {:?}",
            c
        );
    }
}

/// Select a card → it is moved from revealed_cards.
#[test]
fn mirai_ticket_select_moves_one_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq1 = game.id("PL!S-PR-013-PR");
    let aq2 = game.id("PL!S-bp2-002-R");
    setup_mirai(&mut game, &[aq1, aq2]);
    fire(&mut game);

    assert!(game.has_pending_choice(), "MIRAI TICKET must show a choice");
    game.select_indices(&[0]);

    drain(&mut game);

    // At least one card left revealed_cards (the selected one was moved)
    let moved =
        !game.state.revealed_cards.contains(&aq1) || !game.state.revealed_cards.contains(&aq2);
    assert!(moved, "Selected card must leave revealed_cards");
}

/// Cost 11 → floor(11/5) = 2 additional yells. With stage empty (blade=0),
/// each yell draws 0 cards. Card is moved and stays in waitroom.
#[test]
fn mirai_ticket_cost11_yell_count_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq = game.id("PL!S-PR-013-PR"); // cost 11
    setup_mirai(&mut game, &[aq]);
    fire(&mut game);

    // Card was moved from revealed_cards (move_cards ran)
    assert!(
        !game.state.revealed_cards.contains(&aq),
        "Cost-11 card must leave revealed_cards"
    );
    // With empty stage (blade=0), perform_yell draws 0 cards per yell.
    // The card stays in waitroom since the yell doesn't refill from deck.
    assert!(
        game.state.player1.waitroom.cards.contains(&aq),
        "Cost-11 card must be in waitroom after MIRAI TICKET"
    );
}

/// Cost 4 → floor(4/5) = 0 additional yells. Only move_cards fires.
#[test]
fn mirai_ticket_cost4_zero_yells() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq = game.id("PL!S-bp2-002-R"); // Riko, cost 4
    setup_mirai(&mut game, &[aq]);
    fire(&mut game);

    assert!(
        !game.state.revealed_cards.contains(&aq),
        "Cost-4 card must leave revealed_cards"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&aq),
        "Cost-4 card must be in waitroom"
    );
}
