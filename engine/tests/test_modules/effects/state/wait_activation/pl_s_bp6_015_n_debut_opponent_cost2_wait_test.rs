/// PL!S-bp6-015-N (西木野真姫) 登場: wait one opponent member of cost 2 or less.
///
/// Every test here is the same shape: a コスト2以下 filter decides whether the
/// opponent's member is waited. So the two things that can silently break it are
/// the COST of the fixture member and the wait itself, and both are now pinned:
/// the print identity, the cost the filter reads, and an exact `wait` / not-wait
/// result rather than a double negative.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const YOSHIKO: &str = "PL!S-bp6-015-N"; // 西木野真姫 — the 登場 source
const OPP_COST_2: &str = "PL!N-bp7-017-N"; // cost 2 → inside the gate
const OPP_COST_13: &str = "PL!-sd1-003-SD"; // cost 13 → outside the gate

/// 真姫 in hand, `opp` as p2's only stage member, a filled deck, 15 energy.
fn setup(opp: &str) -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id(YOSHIKO);
    let opp_member = game.id(opp);
    game.assert_card_identity(yoshiko, YOSHIKO);
    game.state.player2.stage.stage = [opp_member, -1, -1];
    game.state.player1.hand.cards.push(yoshiko);
    game.give_energy(15);
    for _ in 0..5 {
        let f = game.id("PL!-sd1-010-SD");
        game.state.player1.main_deck.cards.push(f);
    }
    (game, yoshiko, opp_member)
}

fn orientation_of(game: &TestGame, cid: i16) -> Option<String> {
    game.state
        .mods
        .get_orientation_modifier(cid)
        .map(|m| m.to_string())
}

#[test]
fn s_bp6_015_wait_opponent_cost2_succeeds() {
    let (mut game, yoshiko, opp_low2) = setup(OPP_COST_2);
    game.assert_card_cost(opp_low2, 2);

    game.play_to_stage(yoshiko, MemberArea::Center);

    assert_eq!(
        orientation_of(&game, opp_low2).as_deref(),
        Some("wait"),
        "コスト2以下のメンバーの1人をウェイト状態にする — the cost-2 opponent \
         member qualifies"
    );
}

#[test]
fn s_bp6_015_wait_opponent_cost_high_no_wait() {
    let (mut game, yoshiko, opp_high) = setup(OPP_COST_13);
    game.assert_card_cost(opp_high, 13);

    game.play_to_stage(yoshiko, MemberArea::Center);

    // Exactly no modifier: a cost-13 member is not a candidate at all, so the
    // choice must never have been offered and the card must be pristine. The
    // old `is_none() || != Some("wait")` said the same thing but read as if it
    // allowed other states, and never checked the cost that decides it.
    assert_eq!(
        orientation_of(&game, opp_high),
        None,
        "コスト2以下 only — a cost-13 opponent member gets no orientation modifier"
    );
    assert_eq!(
        game.state.player2.stage.stage[0],
        opp_high,
        "the opponent's member must still be on their stage"
    );
}

#[test]
fn s_bp6_015_wait_opponent_empty_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id(YOSHIKO);
    game.assert_card_identity(yoshiko, YOSHIKO);
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(yoshiko);
    game.give_energy(15);
    for _ in 0..5 {
        let f = game.id("PL!-sd1-010-SD");
        game.state.player1.main_deck.cards.push(f);
    }

    game.play_to_stage(yoshiko, MemberArea::Center);

    assert!(
        !game.has_pending_choice(),
        "an empty opponent stage has no candidate, so no choice may be offered"
    );
}

#[test]
fn s_bp6_015_wait_opponent_multiple_cost2_choose_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id(YOSHIKO);
    let opp1 = game.id(OPP_COST_2);
    let opp2 = game.new_id(OPP_COST_2);
    game.assert_card_identity(opp1, OPP_COST_2);
    game.assert_card_cost(opp1, 2);
    game.assert_same_card_name(opp1, opp2, "two copies of the same cost-2 print");
    assert_ne!(opp1, opp2, "two separate card instances");
    game.state.player2.stage.stage = [opp1, opp2, -1];
    game.state.player1.hand.cards.push(yoshiko);
    game.give_energy(15);
    for _ in 0..5 {
        let f = game.id("PL!-sd1-010-SD");
        game.state.player1.main_deck.cards.push(f);
    }

    game.play_to_stage(yoshiko, MemberArea::Center);

    // Two qualifying members → the printed 「1人」 must offer a choice.
    assert!(game.has_pending_choice(), "expected SelectCard for multiple cost2 targets");
    assert_eq!(game.pending_choice_type().as_deref(), Some("SelectCard"));
    game.select_indices(&[0]);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);

    // Exactly one, and it must be the one that was chosen.
    let waited1 = orientation_of(&game, opp1).as_deref() == Some("wait");
    let waited2 = orientation_of(&game, opp2).as_deref() == Some("wait");
    assert_eq!(
        [waited1, waited2],
        [true, false],
        "1人 — exactly the chosen member (index 0) is waited, the other is not"
    );
}
