use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

const KOTORI: &str = "PL!-bp5-003-R＋";
const FILLER: &str = "PL!-sd1-010-SD";
const MUS: &str = "PL!-bp6-001-R＋"; // 高坂穂乃果, μ's
const NON_MUS: &str = "PL!N-bp1-012-R＋"; // 鐘嵐珠, 虹ヶ咲
const LIVE: &str = "PL!-sd1-019-SD";

/// Answer whatever comes up, taking TWO cards when the select asks for two.
///
/// The loop is here to stay, and the drain-loop ratchet budget for this file
/// records it: ことり has TWO 登場 abilities, so her debut raises a
/// `SelectAutoAbility` order prompt BEFORE the cost and the select. The sequence is
/// therefore multi-typed and order-dependent, and a fixed list of arms gets it wrong —
/// an earlier attempt assumed the cost came first and failed on the order prompt.
///
/// The `guard` is the point: a plain `while has_pending_choice()` with no bound hangs
/// forever if the ability stops resolving, and a bound that silently gives up turns a
/// hang into a green test that asserted nothing.
fn resolve(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() {
        guard += 1;
        assert!(guard < 10, "prompts did not settle — 10 is far past the 4 this ability asks");
        match game.get_pending_choice() {
            // The select asks for TWO, and answers [0, 1] — the branch under test is
            // 「カードを2枚手札に加える」, so taking one would not exercise it.
            Choice::SelectCard { count, .. } if *count >= 2 => game.select_indices(&[0, 1]),
            _ => game.select_indices(&[0]),
        }
    }
}

/// Four distinct filler instances on top of the deck, plus one BELOW the look
/// window so "the ability looked at four" is distinguishable from "it emptied the
/// deck".
///
/// The deck is CLEARED first. `TestGame::new` ships it pre-populated, so appending
/// puts these cards at positions 5-8 — the look at the top four would never see
/// them, and the assertions below would fail for a reason that has nothing to do
/// with the ability. (`discard_placed_group_live_or_blade_condition_test.rs` clears
/// for the same reason.)
fn deck_with_look_window(game: &mut TestGame) -> ([i16; 4], i16) {
    game.state.player1.main_deck.cards.clear();
    let mut top = [0; 4];
    for slot in top.iter_mut() {
        *slot = game.id(FILLER);
        game.state.player1.main_deck.cards.push(*slot);
    }
    let below = game.id(FILLER);
    game.state.player1.main_deck.cards.push(below);
    (top, below)
}

/// 「カードを2枚手札に加える。残りを控え室に置く」 — taken, and milled, by exact
/// count.
///
/// `matching_group_discard_look_four_increases_hand` asserts only that the hand GREW.
/// That is satisfied by taking one card, by taking two, and by running the wrong
/// branch as well as the right one. Nothing in the file pins either number the card
/// prints.
///
/// A live card sits in the waitroom throughout, so a `conditional_alternative` that
/// ran BOTH effects would still satisfy every existing assertion in this file while
/// handing the player a free live card.
#[test]
fn matching_group_discard_takes_two_and_mills_the_rest() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    let mus_card = game.id(MUS);
    game.state.player1.stage.stage = [kotori, -1, -1];
    game.state.player1.hand.cards.push(mus_card);

    let (top, below) = deck_with_look_window(&mut game);
    let live_in_waitroom = game.id(LIVE);
    game.state.player1.waitroom.cards.push(live_in_waitroom);
    game.give_energy(10);

    game.activate_ability(kotori);
    resolve(&mut game);

    let hand: Vec<i16> = game.state.player1.hand.cards.iter().copied().collect();
    let taken = top.iter().filter(|c| hand.contains(c)).count();
    assert_eq!(taken, 2, "「カードを2枚手札に加える」 — exactly two, not one; hand={hand:?} top={top:?} below={below} mus={mus_card}");

    let milled: Vec<i16> = top.iter().copied().filter(|c| game.state.player1.waitroom.cards.contains(c)).collect();
    assert_eq!(
        milled.len(),
        2,
        "「残りを控え室に置く」 — the other two of the four looked at are milled; \
         waitroom is {:?}",
        game.state.player1.waitroom.cards
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        1,
        "exactly four left the deck; the fifth sits below the look window"
    );
    assert!(
        game.state.player1.main_deck.cards.contains(&below),
        "the card below the look window is untouched"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_in_waitroom),
        "the 『μ's』のカードの場合 branch does not also recover a live card — that is \
         the 『μ's』のカード以外の場合 branch, and a live card is sitting right there"
    );
}

/// The other branch: no look at all.
///
/// The mirror of the test above on the deck. `nonmatching_group_discard_recovers_live_to_hand`
/// asserts the live card reached hand, which an implementation that ran both
/// branches would also satisfy — so the assertion that actually separates them is
/// that the deck is UNTOUCHED.
#[test]
fn nonmatching_group_discard_does_not_look_at_the_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    let non_mus = game.id(NON_MUS);
    game.state.player1.stage.stage = [kotori, -1, -1];
    game.state.player1.hand.cards.push(non_mus);

    let (top, _below) = deck_with_look_window(&mut game);
    let live_in_waitroom = game.id(LIVE);
    game.state.player1.waitroom.cards.push(live_in_waitroom);
    game.give_energy(10);
    let deck_before = game.state.player1.main_deck.cards.len();

    game.activate_ability(kotori);
    resolve(&mut game);

    assert!(
        game.state.player1.hand.cards.contains(&live_in_waitroom),
        "「自分の控え室からライブカードを1枚手札に加える」"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live_in_waitroom),
        "the live card left the waitroom"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "「『μ's』のカード以外の場合」 does not look at the deck — this is what \
         separates the two branches, since recovering the live card would satisfy \
         the other test either way"
    );
    let hand: Vec<i16> = game.state.player1.hand.cards.iter().copied().collect();
    assert_eq!(
        top.iter().filter(|c| hand.contains(c)).count(),
        0,
        "and takes nothing from it; hand={hand:?}"
    );
}

fn append_fillers_to_player_main_deck(game: &mut TestGame, player: &str, count: usize) {
    let ids: Vec<i16> = (0..count).map(|_| game.id(FILLER)).collect();
    let deck = if player == "p1" {
        &mut game.state.player1.main_deck.cards
    } else {
        &mut game.state.player2.main_deck.cards
    };
    for f in ids {
        deck.push(f);
    }
}

/// ab#1: discard μ's card → look 4, pick 2, rest to discard
#[test]
fn matching_group_discard_look_four_increases_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    let mus_card = game.id("PL!-bp6-001-R\u{ff0b}"); // 高坂穂乃果, μ's card

    game.state.player1.stage.stage = [kotori, -1, -1];
    game.state.player1.hand.cards.push(mus_card);

    append_fillers_to_player_main_deck(&mut game, "p1", 10);
    game.give_energy(10);

    let hand_before = game.state.player1.hand.cards.len();

    game.activate_ability(kotori);

    // Resolve all choices
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        let choice = game.get_pending_choice();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectCard { count, zone, .. } if *count > 1 && zone == "revealed_cards" => {
                let indices: Vec<usize> = (0..*count).collect();
                game.select_indices(&indices);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    let hand_after = game.state.player1.hand.cards.len();
    assert!(
        hand_after > hand_before,
        "Should have gained cards from look_and_select: before={}, after={}",
        hand_before,
        hand_after
    );
}

/// ab#1: discard non-μ's card → add 1 live card from discard to hand
#[test]
fn nonmatching_group_discard_recovers_live_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    // Use a 虹ヶ咲 card as the non-μ's discard — 虹ヶ咲 is NOT μ's
    let non_mus = game.id("PL!N-bp1-012-R\u{ff0b}"); // 鐘嵐珠, 虹ヶ咲 (not μ's)
    let live_in_discard = game.id("PL!-sd1-019-SD"); // a live card

    game.state.player1.stage.stage = [kotori, -1, -1];
    game.state.player1.hand.cards.push(non_mus);
    game.state.player1.waitroom.cards.push(live_in_discard);

    append_fillers_to_player_main_deck(&mut game, "p1", 10);
    game.give_energy(10);

    game.activate_ability(kotori);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        let choice = game.get_pending_choice();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectCard { count, zone, .. } if *count > 1 && zone == "revealed_cards" => {
                let indices: Vec<usize> = (0..*count).collect();
                game.select_indices(&indices);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    assert!(
        game.state.player1.hand.cards.contains(&live_in_discard),
        "Should have recovered live card from discard to hand"
    );
}
