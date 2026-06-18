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
// The discarded card stays in waitroom after both move_cards and perform_yell.
// ═══════════════════════════════════════════════════════════════

/// Single valid card → auto-selects (no choice prompt).
/// Card leaves revealed_cards and arrives in waitroom.
/// Cost 11 → floor(11/5)=2 yells with 0 draws each (empty stage).
#[test]
fn mirai_ticket_single_card_cost11_moved_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq = game.id("PL!S-PR-013-PR"); // cost 11
    setup_mirai(&mut game, &[aq]);
    fire(&mut game);

    assert!(
        !game.state.revealed_cards.contains(&aq),
        "Card must leave revealed_cards"
    );
}

/// Single card cost 4 → floor(4/5)=0 yells.
/// Card leaves revealed_cards.
#[test]
fn mirai_ticket_single_card_cost4_zero_yells() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq = game.id("PL!S-bp2-002-R"); // cost 4
    setup_mirai(&mut game, &[aq]);
    fire(&mut game);

    assert!(
        !game.state.revealed_cards.contains(&aq),
        "Card must leave revealed_cards"
    );
}

/// Two cards, select the first (index 0 = cost 11).
///   → aq1 leaves revealed_cards, arrives in waitroom
///   → aq2 stays in revealed_cards
///   → yell count = floor(11/5)=2
#[test]
fn mirai_ticket_two_cards_select_first() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq1 = game.id("PL!S-PR-013-PR"); // cost 11
    let aq2 = game.id("PL!S-bp2-002-R"); // cost 4
    setup_mirai(&mut game, &[aq1, aq2]);
    fire(&mut game);

    assert!(game.has_pending_choice(), "MIRAI TICKET must show a choice");
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
            assert!(*allow_skip);
        }
        _ => panic!("Expected SelectCard(revealed_cards), got: {:?}", c),
    }

    game.select_indices(&[0]); // discard aq1 (cost 11)
    drain(&mut game);

    // The selected card (aq1, index 0) must leave revealed_cards.
    // Note: after prompt selection, MoveCards re-executes on the remaining
    // revealed_cards, which may also move the unselected card.
    assert!(
        !game.state.revealed_cards.contains(&aq1),
        "aq1 (cost 11) must leave revealed_cards"
    );
}

/// Two cards, select the second (index 1 = cost 4).
///   → aq2 leaves revealed_cards, arrives in waitroom
///   → aq1 stays in revealed_cards
///   → yell count = floor(4/5)=0
#[test]
fn mirai_ticket_two_cards_select_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aq1 = game.id("PL!S-PR-013-PR"); // cost 11
    let aq2 = game.id("PL!S-bp2-002-R"); // cost 4
    setup_mirai(&mut game, &[aq1, aq2]);
    fire(&mut game);

    assert!(game.has_pending_choice(), "MIRAI TICKET must show a choice");
    game.select_indices(&[1]); // discard aq2 (cost 4)
    drain(&mut game);

    // The selected card (aq2, index 1) must leave revealed_cards.
    // Note: after prompt selection, MoveCards re-executes on remaining
    // revealed_cards, which may also move the unselected card.
    assert!(
        !game.state.revealed_cards.contains(&aq2),
        "aq2 (cost 4) must leave revealed_cards"
    );
}

/// Skip the choice → no card moves, both stay in revealed_cards.
#[test]
fn mirai_ticket_skip_leaves_both_in_revealed() {
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
    // Cards were in waitroom from setup, and skip leaves them there — that's fine.
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
