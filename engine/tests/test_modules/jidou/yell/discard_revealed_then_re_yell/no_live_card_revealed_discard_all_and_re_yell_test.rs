/// 黒澤ダイヤ (PL!S-bp2-004-R) ab#0
///
/// 自動 [1/ターン]: エールにより公開された自分のカードの中にライブカードがないとき、
/// それらのカードをすべて控え室に置いてもよい。これにより1枚以上のカードが控え室に
/// 置かれた場合、そのエールで得たブレードハートを失い、もう一度エールを行う。
use crate::helpers::*;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_p1_performance(game: &mut TestGame, dia: i16) {
    game.state.player1.stage.stage[2] = dia;
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live);
    game.pass();
    game.pass();
    game.pass();
}

/// Q107: re-yell works — optional discard accepted → followup runs.
#[test]
fn q107_dia_re_yell_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");

    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    advance_to_p1_performance(&mut game, dia);

    assert!(game.has_pending_choice());
    let count = if let rabuka_engine::ability::types::Choice::SelectCard { count, .. } =
        game.get_pending_choice()
    {
        *count
    } else {
        0
    };
    game.select_indices(&(0..count).collect::<Vec<_>>());

    assert!(!game.state.initial_yell_revealed_cards.is_empty());
    assert!(!game.state.re_yell_revealed_cards.is_empty());
}

/// Q107: skip optional discard → followup does NOT run.
///
/// This test previously had no assertion at all. Its own comment described the
/// hazard correctly — "a mandatory prompt left parked mid-ability produces exactly
/// the absence the next assertion checks, so nothing fails" — and then no next
/// assertion existed. It would have passed against an engine that always re-yells.
///
/// The field to check is `re_yell_occurred`, not `re_yell_revealed_cards`: the latter
/// is only repopulated if the followup ran far enough to yell, so a partial bug could
/// leave it empty and satisfy a weaker check. The sibling card's equivalent test
/// (`blade_heart_two_or_fewer_discard_all_and_re_yell_test.rs::discard_skip_no_re_yell`)
/// uses `re_yell_occurred`, so the two cards now assert the same thing the same way.
#[test]
fn q107_dia_skip_discard_no_followup() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");

    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    advance_to_p1_performance(&mut game, dia);

    assert!(game.has_pending_choice(), "Optional discard prompt");
    let revealed_before = game.state.initial_yell_revealed_cards.len();
    assert!(
        revealed_before > 0,
        "fixture: the yell must reveal something, or there is nothing to decline"
    );
    game.select_indices(&[]);
    // Answer what cannot be declined, decline what can.
    game.drain_choices();

    assert!(
        !game.state.re_yell_occurred,
        "declining places no cards, so 「これにより1枚以上のカードが控え室に置かれた\
         場合」 is false and the followup must not run"
    );
}

/// 「ライブカードがないとき」 — a live card among the reveals blocks everything.
///
/// The gate on this ability is a NEGATED location check: the effect only exists when
/// none of the yell's reveals is a live card. This file is named for that clause and,
/// before this test, never exercised it — `fill_decks` fills both decks with
/// `PL!-sd1-010-SD`, a member, so every reveal was non-live and the condition was
/// vacuously true in all three tests.
///
/// One live card in the reveal pool is enough to close the gate: the discard is not
/// offered (there is nothing to decline) and no re-yell follows. Both halves matter,
/// because an engine that offered the prompt anyway would still need the followup
/// gated — a re-yell here would hand the player an extra yell for milling their own
/// live card.
#[test]
fn q107_a_live_card_among_the_reveals_blocks_the_whole_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");

    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");

    // Same shape as fill_decks, but the deck holds LIVE cards, so the yell reveals
    // them and 「公開された自分のカードの中にライブカードがない」 is false.
    let live = game.id("PL!-sd1-019-SD");
    assert_eq!(
        game.db.get_card(live).map(|c| c.card_type.clone()),
        Some(rabuka_engine::card::CardType::Live),
        "fixture: the blocker must be a live card"
    );
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(live);
        game.state.player2.main_deck.cards.push(game.new_id("PL!-sd1-010-SD"));
    }
    game.give_energy(15);

    advance_to_p1_performance(&mut game, dia);

    assert!(
        !game.state.initial_yell_revealed_cards.is_empty(),
        "fixture: the yell still has to reveal cards for the gate to mean anything"
    );
    assert!(
        !game.has_pending_choice(),
        "「ライブカードがないとき」 is false, so Dia offers no discard prompt; got {:?}",
        game.pending_choice_summary()
    );
    assert!(
        !game.state.re_yell_occurred,
        "and no re-yell: the followup is gated on cards having been placed"
    );
}
