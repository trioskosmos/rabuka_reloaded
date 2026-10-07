//! 黒澤ルビィ `PL!S-PR-048-PR` / 平安名すみれ `PL!SP-PR-027-PR` (ライブ開始時)
//!
//! 自分の控え室にあるカードが9枚以下の場合、自分の控え室にあるライブカードを3枚まで
//! 選び、自分のデッキの上に好きな順番で置く。
//!
//! Three things this pins, all of which the parser used to drop:
//!  * the 「9枚以下」 gate (card_count_condition on the waitroom) — 9 fires, 10 does not;
//!  * 「ライブカードを…選び」 — a member in the same waitroom is NOT selectable;
//!  * 「デッキの上に好きな順番で置く」 — the select carried no destination at all, so the
//!    chosen cards never left the waitroom. Now: select → move to deck_top → the
//!    「好きな順番で」 ORDER prompt.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

const RUBII: &str = "PL!S-PR-048-PR";
const SUMIRE: &str = "PL!SP-PR-027-PR";
/// The ライブカード this ability moves.
const LIVE: &str = "PL!S-bp2-019-L";
/// A member: banks in the same waitroom but is NOT a ライブカード.
const MEMBER: &str = "PL!-sd1-010-SD";

fn fill_decks(game: &mut TestGame) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        let f = game.id(MEMBER);
        game.state.player1.main_deck.cards.push(f);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..10 {
        let o = game.id(MEMBER);
        game.state.player2.main_deck.cards.push(o);
    }
}

/// Stand `ability` on stage, bank `lives` ライブカード followed by `pads` members in
/// the waitroom, then walk to ライブ開始時. Returns the waitroom as it stood when the
/// live started — `lives` live cards first, then the pads.
fn to_live_start(ability: &str, lives: usize, pads: usize) -> (TestGame, Vec<i16>) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);

    let me = game.id(ability);
    game.assert_card_identity(me, ability);
    game.add_to_hand(me);
    game.give_energy(30);
    game.play_to_stage(me, MemberArea::Center);

    let mut banked = Vec::new();
    for i in 0..lives {
        let c = if i == 0 {
            game.id(LIVE)
        } else {
            game.new_id(LIVE)
        };
        game.add_to_discard(c);
        banked.push(c);
    }
    for _ in 0..pads {
        let c = game.id(MEMBER);
        game.add_to_discard(c);
        banked.push(c);
    }

    // A live card in hand is what actually gets performed.
    let live = game.new_id(LIVE);
    game.add_to_hand(live);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live);
    game.pass();
    game.pass();
    // 自動 offered at live start are declined so the prompt under test is on top.
    while game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
        game.select_indices(&[]);
    }
    // NOTE: the deck is NOT still 30 here — walking to ライブ開始時 runs the turn's
    // draw, so every deck assertion below is measured against the length captured
    // at this point, never against a hard-coded 30.
    (game, banked)
}

/// Happy path: three lives to deck top in the order the player picks, member left banked.
#[test]
fn the_three_lives_go_to_deck_top_in_the_chosen_order() {
    let (mut game, banked) = to_live_start(RUBII, 3, 1);
    let lives: Vec<i16> = banked[..3].to_vec();
    let member = banked[3];
    let deck_before = game.state.player1.main_deck.cards.len();

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "9枚以下 → the select prompt opens"
    );
    let choice = game.get_pending_choice().clone();
    let (count, filtered) = match &choice {
        Choice::SelectCard {
            count,
            filtered_indices,
            zone,
            ..
        } => {
            assert_eq!(zone, "discard", "選ぶ is off the waitroom");
            (*count, filtered_indices.clone())
        }
        _ => panic!("Expected SelectCard"),
    };
    assert_eq!(count, 3, "3枚まで選び");
    let filtered = filtered.expect("card_type filter produces filtered_indices");
    assert_eq!(
        filtered,
        vec![0, 1, 2],
        "only the three ライブカード are offered; the member at index 3 is filtered out"
    );

    game.try_select_indices(&[0, 1, 2]).unwrap();
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "好きな順番で置く → the ORDER prompt"
    );
    // The second-selected card goes on top.
    game.select_option(1);
    game.drain_choices();

    let deck = game.state.player1.main_deck.cards.to_vec();
    assert_eq!(
        deck.len(),
        deck_before + 3,
        "three cards joined the deck"
    );
    assert_eq!(
        deck[0], lives[1],
        "好きな順番で — the card the player chose as top is on top"
    );
    let mut top_three: Vec<i16> = deck[..3].to_vec();
    top_three.sort_unstable();
    let mut expect: Vec<i16> = lives.clone();
    expect.sort_unstable();
    assert_eq!(top_three, expect, "the three lives occupy deck slots 0..3");
    assert_eq!(
        game.state.player1.waitroom.cards.to_vec(),
        vec![member],
        "残り — the non-live member stayed banked"
    );
}

/// Boundary: exactly nine banked still fires.
#[test]
fn nine_cards_in_the_waitroom_still_fires() {
    let (mut game, _banked) = to_live_start(RUBII, 3, 6);
    let deck_before = game.state.player1.main_deck.cards.len();
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "9枚以下 — nine is inside the gate"
    );
    game.try_select_indices(&[0, 1, 2]).unwrap();
    if game.pending_choice_type().as_deref() == Some("SelectTarget") {
        game.select_option(0);
    }
    game.drain_choices();
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before + 3,
        "the three lives moved to deck top"
    );
}

/// Boundary: ten banked does not fire.
#[test]
fn ten_cards_in_the_waitroom_does_not_fire() {
    let (game, _banked) = to_live_start(RUBII, 3, 7);
    assert!(
        !game.has_pending_choice(),
        "10枚 → the gate is 9枚以下, so nothing is offered (pending {:?})",
        game.pending_choice_type()
    );
}

/// Fewer than the cap: stopping after one still places it on deck top.
#[test]
fn taking_one_still_places_it_on_deck_top() {
    let (mut game, banked) = to_live_start(RUBII, 2, 1);
    let first = banked[0];
    let deck_before = game.state.player1.main_deck.cards.len();

    assert_eq!(game.pending_choice_type().as_deref(), Some("SelectCard"));
    game.try_select_indices(&[0]).unwrap();
    // 「3枚まで」 — a partial take may be followed up, and stopping is legal.
    if game.pending_choice_type().as_deref() == Some("SelectCard") {
        game.select_indices(&[]);
    }
    if game.pending_choice_type().as_deref() == Some("SelectTarget") {
        game.select_option(0);
    }
    game.drain_choices();

    let deck = game.state.player1.main_deck.cards.to_vec();
    assert_eq!(
        deck.len(),
        deck_before + 1,
        "exactly one card reached the deck"
    );
    assert_eq!(
        deck[0], first,
        "the single chosen life goes to deck top (好きな順番で is moot with one card)"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.to_vec(),
        banked[1..].to_vec(),
        "the unselected life and the member stay banked"
    );
}

/// The second print carries the identical ability, so it behaves identically.
#[test]
fn the_second_print_behaves_the_same() {
    let (mut game, banked) = to_live_start(SUMIRE, 3, 1);
    let lives: Vec<i16> = banked[..3].to_vec();
    let member = banked[3];
    let deck_before = game.state.player1.main_deck.cards.len();

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "PL!SP-PR-027-PR shares the printed text and must open the same prompt"
    );
    game.try_select_indices(&[0, 1, 2]).unwrap();
    if game.pending_choice_type().as_deref() == Some("SelectTarget") {
        game.select_option(0);
    }
    game.drain_choices();

    let deck = game.state.player1.main_deck.cards.to_vec();
    assert_eq!(
        deck.len(),
        deck_before + 3,
        "three lives moved onto the deck"
    );
    assert_eq!(
        deck[0], lives[0],
        "the first-selected card is on top when the player picks it"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.to_vec(),
        vec![member],
        "only the non-live member is left banked"
    );
}
