use crate::helpers::*;

// ====================================================================
// PL!SP-pb2-020-R 鬼塚夏美 (was NEGATIVE-ONLY: Q264 no-fire case)
// {{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、手札にある
// 『Liella!』のライブカードを1枚控え室に置いてもよい。そうした場合、追加で
// 2枚エールを行う。
//
// On yell: you may discard a Liella! live from hand; if you do, yell 2 more
// (re-yell reveals 2 more cards). The Q264 zero-yell negative is covered in
// zero_yell_optional_live_discard_q264_test.rs.
// ====================================================================

const LIELLA_LIVE: &str = "PL!SP-sd1-023-SD";

fn natsumi_yell_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let natsumi = game.id("PL!SP-pb2-020-R");
    assert_eq!(
        game.db.get_card(natsumi).unwrap().card_no,
        "PL!SP-pb2-020-R",
        "staged the R print of Natsumi"
    );
    game.state.player1.stage.stage[1] = natsumi;
    natsumi
}

fn fire_yell_watchers(game: &mut TestGame) {
    // The parsed trigger condition is revealed_cards non-empty (Q264: a
    // zero-card yell does NOT count as "yell").
    game.state.yell_occurred = true;
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
}

#[test]
fn pb2_020_yell_discards_liella_live_for_two_extra_yells() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = natsumi_yell_setup(&mut game);
    let _ = natsumi;
    let live = game.new_id(LIELLA_LIVE);
    game.state.player1.hand.cards.push(live);
    // The yell that fires the watcher: Natsumi's 2 blades reveal 2.
    game.state.revealed_cards.push(game.new_id("PL!-sd1-010-SD"));
    game.state.revealed_cards.push(game.new_id("PL!-sd1-010-SD"));

    fire_yell_watchers(&mut game);
    assert!(
        game.has_pending_choice(),
        "optional discard of the Liella! live prompted"
    );
    game.select_indices(&[0]);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "paid: Liella! live discarded"
    );
    // 2 triggering reveals + 2 extra yells x 2 blades each.
    assert_eq!(
        game.state.revealed_cards.len(),
        6,
        "triggering yell + exactly 2 additional yells revealed"
    );
    assert!(
        game.state
            .turn_limited_abilities_used
            .values()
            .sum::<u8>()
            >= 1,
        "ターン1回 use recorded"
    );
}

#[test]
fn pb2_020_decline_discard_no_extra_yells() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    natsumi_yell_setup(&mut game);
    let live = game.new_id(LIELLA_LIVE);
    game.state.player1.hand.cards.push(live);
    game.state.revealed_cards.push(game.new_id("PL!-sd1-010-SD"));
    game.state.revealed_cards.push(game.new_id("PL!-sd1-010-SD"));

    fire_yell_watchers(&mut game);
    assert!(game.has_pending_choice(), "optional discard prompted");
    game.select_indices(&[]);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    assert!(
        game.state.player1.hand.cards.contains(&live),
        "declined: live stays in hand"
    );
    assert_eq!(
        game.state.revealed_cards.len(),
        2,
        "declined: only the triggering yell stands"
    );
}

#[test]
fn pb2_020_no_liella_live_in_hand_no_prompt() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    natsumi_yell_setup(&mut game);
    game.add_to_hand(game.new_id("PL!-sd1-010-SD"));
    game.state.revealed_cards.push(game.new_id("PL!-sd1-010-SD"));
    game.state.revealed_cards.push(game.new_id("PL!-sd1-010-SD"));

    fire_yell_watchers(&mut game);
    assert!(
        !game.has_pending_choice(),
        "no eligible live: auto-skip, no prompt"
    );
    assert_eq!(
        game.state.revealed_cards.len(),
        2,
        "no eligible live: only the triggering yell stands"
    );
}

#[test]
fn pb2_020_use_limit_blocks_second_yell() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    natsumi_yell_setup(&mut game);
    let live1 = game.new_id(LIELLA_LIVE);
    let live2 = game.new_id(LIELLA_LIVE);
    game.state.player1.hand.cards.push(live1);
    game.state.player1.hand.cards.push(live2);
    game.state.revealed_cards.push(game.new_id("PL!-sd1-010-SD"));
    game.state.revealed_cards.push(game.new_id("PL!-sd1-010-SD"));

    fire_yell_watchers(&mut game);
    assert!(game.has_pending_choice(), "first yell prompts");
    game.select_indices(&[0]);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    let revealed_after_first = game.state.revealed_cards.len();

    fire_yell_watchers(&mut game);
    assert!(
        !game.has_pending_choice(),
        "second yell same turn: use_limit blocks"
    );
    assert_eq!(
        game.state.revealed_cards.len(),
        revealed_after_first,
        "no further yells from the blocked firing"
    );
    assert!(
        game.state.player1.hand.cards.contains(&live2),
        "second live never touched"
    );
}
