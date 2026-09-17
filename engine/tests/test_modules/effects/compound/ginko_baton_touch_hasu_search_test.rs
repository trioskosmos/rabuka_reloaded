/// Tests for PL!HS-bp2-007-R+/P/P+/SEC (百生 吟子) ab#0 — Appearance:
///   このメンバーよりコストが低い『スリーズブーケ』のメンバーから
///   バトンタッチして登場した場合、自分の控え室から『蓮ノ空』のライブカードを
///   1枚手札に加える。
///
/// Parsed ability:
///   trigger: 登場 (appearance via baton touch)
///   condition: movement_condition(baton_touch, group_names=[スリーズブーケ],
///              cost_comparison < relative_to=activating)
///   effect: move_cards(source=discard, destination=hand, count=1,
///                      card_type=live_card, group_names=[蓮ノ空])
///
/// Regression guard: the parser used to strip ALL group_names off the action
/// (baton-touch gate leak fix), which made the engine search the whole
/// waitroom instead of only 『蓮ノ空』 live cards. The non-蓮ノ空 tests here
/// fail if group_names is ever dropped from the move_cards again.
///
/// Cards:
///   PL!HS-bp2-007-R+      百生 吟子 cost 11 (this card)
///   PL!HS-bp1-012-PR      乙宗 梢 cost 4 スリーズブーケ (cheap replace target)
///   PL!HS-PR-031-PR       日野下花帆 cost 11 スリーズブーケ (equal-cost target)
///   PL!-sd1-006-SD        cost 9 μ's (wrong-group replace target)
///   PL!HS-bp1-019-L       Dream Believers — 蓮ノ空 series live card
///   PL!-sd1-020-SD        きっと青春が聞こえる — μ's live card (control)
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const GINKO: &str = "PL!HS-bp2-007-R+";
const CHEAP_SUZU: &str = "PL!HS-bp1-012-PR";
const EQUAL_COST_SUZU: &str = "PL!HS-PR-031-PR";
const WRONG_GROUP_MEMBER: &str = "PL!-sd1-006-SD";
const HASU_LIVE: &str = "PL!HS-bp1-019-L";
const NON_HASU_LIVE: &str = "PL!-sd1-020-SD";

fn setup_game(game: &mut TestGame, replaced: &str) -> i16 {
    let replaced_id = game.id(replaced);
    game.state.player1.stage.stage[1] = replaced_id;
    game.give_energy(25);
    replaced_id
}

/// Play Ginko over `replaced` and resolve all choices, selecting the first
/// candidate each time (the group filter guarantees only 蓮ノ空 lives are
/// candidates).
fn baton_touch_ginko(game: &mut TestGame, ginko: i16) {
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(ginko);
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(ginko, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        game.select_indices(&[0]);
        guard += 1;
    }
}

// =========================================================================
// 1. Happy path: baton touch over cheaper スリーズブーケ → 蓮ノ空 live
//    moves from waitroom to hand.
// =========================================================================
#[test]
fn baton_touch_search_adds_hasu_live_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_game(&mut game, CHEAP_SUZU);

    let ginko = game.id(GINKO);
    let hasu = game.id(HASU_LIVE);
    game.state.player1.waitroom.cards.push(hasu);

    baton_touch_ginko(&mut game, ginko);

    assert!(
        game.state.player1.stage.stage.contains(&ginko),
        "Ginko must be on stage after baton touch"
    );
    assert!(
        game.state.player1.hand.cards.contains(&hasu),
        "蓮ノ空 live card must be added to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&hasu),
        "蓮ノ空 live card must leave the waitroom"
    );
}

// =========================================================================
// 2. REGRESSION: a non-蓮ノ空 live card in the waitroom must NOT be added.
//    Before the parser fix (group_names stripped from the action), the
//    engine searched the waitroom unfiltered and grabbed any live card.
// =========================================================================
#[test]
fn non_hasu_live_not_added() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_game(&mut game, CHEAP_SUZU);

    let ginko = game.id(GINKO);
    let mus_live = game.id(NON_HASU_LIVE);
    game.state.player1.waitroom.cards.push(mus_live);

    baton_touch_ginko(&mut game, ginko);

    assert!(
        !game.state.player1.hand.cards.contains(&mus_live),
        "Non-蓮ノ空 live card must NOT be added to hand (group_names filter)"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&mus_live),
        "Non-蓮ノ空 live card must stay in the waitroom"
    );
}

// =========================================================================
// 3. Mixed waitroom: only the 蓮ノ空 live is a legal candidate. The μ's
//    live is placed FIRST so a broken filter would select it at index 0.
// =========================================================================
#[test]
fn mixed_waitroom_only_hasu_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_game(&mut game, CHEAP_SUZU);

    let ginko = game.id(GINKO);
    let hasu = game.id(HASU_LIVE);
    let mus_live = game.id(NON_HASU_LIVE);
    game.state.player1.waitroom.cards.push(mus_live);
    game.state.player1.waitroom.cards.push(hasu);

    baton_touch_ginko(&mut game, ginko);

    assert!(
        game.state.player1.hand.cards.contains(&hasu),
        "蓮ノ空 live must be selectable and added to hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&mus_live),
        "μ's live must remain in the waitroom"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&mus_live),
        "μ's live must never be selected"
    );
}

// =========================================================================
// 4. Replaced member is NOT スリーズブーケ → condition fails, no search.
// =========================================================================
#[test]
fn wrong_group_replaced_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_game(&mut game, WRONG_GROUP_MEMBER);

    let ginko = game.id(GINKO);
    let hasu = game.id(HASU_LIVE);
    game.state.player1.waitroom.cards.push(hasu);

    baton_touch_ginko(&mut game, ginko);

    assert!(
        game.state.player1.waitroom.cards.contains(&hasu),
        "No trigger over a non-スリーズブーケ member: waitroom untouched"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&hasu),
        "No trigger over a non-スリーズブーケ member: nothing added to hand"
    );
}

// =========================================================================
// 5. Replaced member has EQUAL cost (11, not lower) → condition fails.
// =========================================================================
#[test]
fn equal_cost_replaced_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_game(&mut game, EQUAL_COST_SUZU);

    let ginko = game.id(GINKO);
    let hasu = game.id(HASU_LIVE);
    game.state.player1.waitroom.cards.push(hasu);

    baton_touch_ginko(&mut game, ginko);

    assert!(
        game.state.player1.waitroom.cards.contains(&hasu),
        "Equal-cost replacement is not 'よりコストが低い': waitroom untouched"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&hasu),
        "Equal-cost replacement must not add cards to hand"
    );
}

// =========================================================================
// 6. Empty waitroom → ability resolves without crash, nothing added.
// =========================================================================
#[test]
fn empty_waitroom_resolves_cleanly() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_game(&mut game, CHEAP_SUZU);

    let ginko = game.id(GINKO);
    let hand_before = game.state.player1.hand.cards.clone();

    baton_touch_ginko(&mut game, ginko);

    assert!(
        game.state.player1.stage.stage.contains(&ginko),
        "Ginko still enters play with an empty waitroom"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before.len() + 1,
        "Ginko+filler pushed, Ginko played: net +1 card in hand"
    );
}

// =========================================================================
// 7. Only a MEMBER card in the waitroom → card_type=live_card filter holds.
// =========================================================================
#[test]
fn member_in_waitroom_ignored() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_game(&mut game, CHEAP_SUZU);

    let ginko = game.id(GINKO);
    let member = game.new_id(CHEAP_SUZU);
    game.state.player1.waitroom.cards.push(member);

    baton_touch_ginko(&mut game, ginko);

    assert!(
        game.state.player1.waitroom.cards.contains(&member),
        "Member card must stay in the waitroom (card_type=live_card filter)"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&member),
        "Member card must not be added to hand"
    );
}

// =========================================================================
// 8. Two 蓮ノ空 lives in the waitroom → exactly one is taken (count=1).
// =========================================================================
#[test]
fn two_hasu_lives_exactly_one_taken() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_game(&mut game, CHEAP_SUZU);

    let ginko = game.id(GINKO);
    let hasu1 = game.id(HASU_LIVE);
    let hasu2 = game.new_id(HASU_LIVE);
    game.state.player1.waitroom.cards.push(hasu1);
    game.state.player1.waitroom.cards.push(hasu2);

    baton_touch_ginko(&mut game, ginko);

    let in_hand = game.state.player1.hand.cards.iter().filter(|&&c| c == hasu1 || c == hasu2).count();
    let in_waitroom = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .filter(|&&c| c == hasu1 || c == hasu2)
        .count();
    assert_eq!(in_hand, 1, "Exactly one 蓮ノ空 live added to hand");
    assert_eq!(in_waitroom, 1, "Exactly one 蓮ノ空 live remains in waitroom");
}
