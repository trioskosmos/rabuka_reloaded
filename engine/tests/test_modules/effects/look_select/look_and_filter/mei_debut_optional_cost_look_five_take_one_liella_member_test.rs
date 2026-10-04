/// Tests for 米女メイ (PL!SP-bp2-007) ability #0 and 桜小路きな子 (PL!SP-pb2-017),
/// which share it:
///
///   {{toujyou.png|登場}}手札を1枚控え室に置いてもよい：
///   自分のデッキの上からカードを5枚見る。
///   その中から『Liella!』のメンバーカードを1枚公開して手札に加えてもよい。
///   残りを控え室に置く。
///
/// Parsed as `look_and_select { look_at deck_top 5, select_cards -> hand }` with an
/// OPTIONAL cost, an OPTIONAL select, and `discard_remaining: true`.
///
/// Nothing tested this ability before. `PL!SP-bp2-007-*` and `PL!SP-pb2-017-*`
/// appear in the test tree only as bystander fixtures for other cards' tests
/// (`either_or_state_change_test.rs` uses 米女メイ as "a Liella! member on stage",
/// `unit_group_recovery_excludes_same_series_test.rs` names it in a doc comment).
/// Neither is a test of this ability, so all four of its outcomes were unpinned:
///
///   * the 「残りを控え室に置く」 clause — `discard_remaining`, the sentence the
///     whole card is named for;
///   * the no-match branch, where NONE of the five is a 『Liella!』 member and the
///     optional select is never offered at all;
///   * declining the select; and
///   * declining the OPTIONAL COST, which resolves the entire ability.
///
/// The last two are the ones a "did the hand grow?" assertion cannot see. Both
/// decline paths end with the hand unchanged, so a test that only checked the hand
/// would pass on an implementation that did nothing at all.
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

const MEI: &str = "PL!SP-bp2-007-P";
/// A 『Liella!』 member, so it survives the select's group filter.
const LIELLA_MEMBER: &str = "PL!SP-pb1-001-R";
/// Not a 『Liella!』 member, and a cheap card to stand in for the optional cost.
const FILLER: &str = "PL!-sd1-010-SD";

/// Deck: [`top_card` is the Liella! member], then four fillers, then one filler
/// BELOW the look window. That sixth card is the control for "the ability touched
/// exactly five": a look of six, or a discard that ate the whole deck, fails here.
struct Deck5 {
    top: i16,
    fillers: [i16; 4],
    below_window: i16,
}

fn setup(game: &mut TestGame, liella_on_top: bool) -> (i16, Deck5) {
    let mei = game.id(MEI);
    let liella = game.id(LIELLA_MEMBER);
    let filler = game.id(FILLER);

    let mut top = filler;
    let deck_cards = if liella_on_top {
        top = liella;
        [liella, filler, filler, filler, filler]
    } else {
        [filler, filler, filler, filler, filler]
    };

    for &cid in &deck_cards {
        game.state.player1.main_deck.cards.push(cid);
    }
    // One more filler, deliberately below the five the ability looks at.
    let below_window = filler;
    game.state.player1.main_deck.cards.push(below_window);

    // The optional cost discards one card from hand, so the hand must hold one.
    game.state.player1.hand.cards.push(filler);

    game.give_energy(15);
    (mei, Deck5 { top, fillers: [filler; 4], below_window })
}

/// Play 米女メイ to the centre, which fires her 登場 ability.
fn deploy_mei(game: &mut TestGame, mei: i16) {
    game.add_to_hand(mei);
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(mei),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("play 米女メイ to stage");
}

/// Answer the optional-cost gate. `pay` names the hand card to discard; otherwise
/// the cost is declined.
///
/// The cost is a `move_cards` from hand, so the prompt is a SelectCard asking
/// WHICH card — not a pay/skip target gate. The skip is therefore an empty
/// selection, which only works if the prompt carries `allow_skip`; asserting that
/// here is what stops "the cost is optional in the text" from being true only in
/// the parser.
fn settle_optional_cost(game: &mut TestGame, pay: bool) {
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "the cost discards a card from hand, so the prompt asks which one"
    );
    assert!(
        game.get_pending_choice().allow_skip(),
        "「手札を1枚控え室に置いてもよい」 — the cost must be skippable"
    );
    game.select_indices(if pay { &[0] } else { &[] });
}

/// How many of `cards` sit in the waitroom.
fn waitroom_count(game: &TestGame, card: i16) -> usize {
    game.state.player1.waitroom.cards.iter().filter(|&&c| c == card).count()
}

/// Taking the 『Liella!』 member keeps it and discards the other four.
///
/// This is the whole card: 5 looked at, 1 「公開して手札に加える」, 「残りを控え室に
/// 置く」. The assertions are per-card rather than on hand and waitroom sizes,
/// because the played card has already left the hand and a count-only assertion
/// would be satisfied by any implementation that moved the same number of cards
/// around.
#[test]
fn taking_one_liella_member_discards_the_other_four() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mei, deck) = setup(&mut game, true);
    let deck_before = game.state.player1.main_deck.cards.len();

    deploy_mei(&mut game, mei);
    settle_optional_cost(&mut game, true);

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "the 『Liella!』 member among the five is offered"
    );
    game.select_indices(&[0]);
    game.drain_choices_strict(&[], &[]);

    assert!(
        game.state.player1.hand.cards.contains(&deck.top),
        "the selected 『Liella!』 member is in hand"
    );
    // The cost card plus the four unselected fillers.
    assert_eq!(
        waitroom_count(&game, deck.fillers[0]),
        5,
        "1 optional-cost discard + 「残りを控え室に置く」 x4; waitroom is {:?}",
        game.state.player1.waitroom.cards
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 5,
        "the ability looks at exactly five, so five leave the deck and the card \
         below the window stays"
    );
    assert!(
        game.state.player1.main_deck.cards.contains(&deck.below_window),
        "the sixth card was below the look window and must not be touched"
    );
}

/// None of the five is a 『Liella!』 member: nothing is offered and all five are
/// discarded.
///
/// The select is 「…してもよい」, so with no eligible card the engine has nothing to
/// ask about and must not manufacture a choice. This is a separate code path from
/// declining — `offer_looked_at_selection` discards the whole pool when the filter
/// matches nothing, without ever building a prompt — and a test that only covered
/// declining would leave it unpinned.
#[test]
fn no_liella_member_among_the_five_discards_all_five_without_prompting() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mei, deck) = setup(&mut game, false);
    let deck_before = game.state.player1.main_deck.cards.len();

    deploy_mei(&mut game, mei);
    settle_optional_cost(&mut game, true);
    // Baseline AFTER the cost. The cost is a hand discard, so a hand size taken
    // before it is off by one however the play itself shuffled cards around —
    // the same trap as the waitroom counts below, where the cost discards into the
    // zone being counted.
    let hand_after_cost = game.state.player1.hand.cards.len();

    assert!(
        !game.has_pending_choice(),
        "with no 『Liella!』 member among the five there is nothing to select, so \
         the optional select must not be offered at all; got {:?}",
        game.pending_choice_summary()
    );

    assert_eq!(
        waitroom_count(&game, deck.fillers[0]),
        6,
        "1 optional-cost discard + all five revealed cards; waitroom is {:?}",
        game.state.player1.waitroom.cards
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_after_cost,
        "the hand gained nothing: 「公開して手札に加える」 never ran"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 5,
        "the five leave the deck even though none could be taken"
    );
}

/// Declining the select discards all five, not four.
///
/// 「公開して手札に加えてもよい。残りを控え室に置く。」 — 「残り」 is "the rest",
/// measured against all five looked at. Taking nothing must not leave the pool in
/// the looked-at buffer, which is shared game state the next look-based card would
/// read.
#[test]
fn declining_the_select_discards_all_five() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mei, deck) = setup(&mut game, true);
    let deck_before = game.state.player1.main_deck.cards.len();

    deploy_mei(&mut game, mei);
    settle_optional_cost(&mut game, true);
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "the select is offered because a 『Liella!』 member is among the five"
    );
    game.select_indices(&[]); // 「してもよい」 — decline
    game.drain_choices_strict(&[], &[]);

    assert_eq!(
        waitroom_count(&game, deck.fillers[0]),
        5,
        "1 optional-cost discard + all five; waitroom is {:?}",
        game.state.player1.waitroom.cards
    );
    assert!(
        game.state.revealed_cards.is_empty() && game.state.looked_at_cards.is_empty(),
        "declining must still resolve the pool: revealed={:?} looked_at={:?}",
        game.state.revealed_cards,
        game.state.looked_at_cards
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 5,
        "all five leave the deck whether or not one is taken"
    );
}

/// Declining the optional cost resolves the WHOLE ability.
///
/// 「手札を1枚控え室に置いてもよい：」 — the colon makes everything after it
/// conditional on the cost. Declining must not leave a free look-and-take: the deck
/// is untouched, the hand is untouched, and the waited-on card stays out of the
/// waitroom.
///
/// This is the test that catches the classic bug here. "Hand unchanged" is true both
/// when declining correctly and when the ability did nothing at all, so the deck
/// and waitroom are what make the assertion mean anything.
#[test]
fn declining_the_optional_cost_resolves_the_whole_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mei, deck) = setup(&mut game, true);
    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    deploy_mei(&mut game, mei);
    settle_optional_cost(&mut game, false);
    game.drain_choices_strict(&[], &[]);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declining the cost must not look at anything: 「…てもよい：」 gates the \
         whole sentence"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&deck.top),
        "declining the cost must not take the 『Liella!』 member either"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before,
        "and must not discard the cost card, nor 「残り」; waitroom is {:?}",
        game.state.player1.waitroom.cards
    );
}