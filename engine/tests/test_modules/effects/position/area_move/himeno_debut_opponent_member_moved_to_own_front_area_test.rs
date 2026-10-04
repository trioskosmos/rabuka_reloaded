use crate::helpers::*;
use rabuka_engine::types::{Phase, TurnPhase};
use rabuka_engine::zones::MemberArea;

const HIMENO: &str = "PL!HS-pb1-014-R";
const FILLER: &str = "PL!-sd1-010-SD";

/// Debut: position change an opponent member to the front area of this member.
/// Himeno at center, opponent has 3 members.
/// Front of center = center → selected left member swaps to opponent center.
#[test]
fn himeno_front_3_opponents_select_left_moves_to_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp_left = game.id(FILLER);
    let opp_center = game.id(FILLER);
    let opp_right = game.id(FILLER);

    game.state.player2.stage.stage = [opp_left, opp_center, opp_right];
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_option(0);
    }

    assert_eq!(game.state.player2.stage.stage[1], opp_left);
}

/// Himeno at center, opponent has 1 member at left.
#[test]
fn himeno_front_1_opponent_at_left_moves_to_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp = game.id(FILLER);

    game.state.player2.stage.stage[0] = opp;
    game.state.player2.stage.stage[1] = -1;
    game.state.player2.stage.stage[2] = -1;
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_option(0);
    }

    assert_eq!(game.state.player2.stage.stage[1], opp);
    assert_eq!(game.state.player2.stage.stage[0], -1);
}

/// Himeno on left. Front of left = right (mirrored). Select opponent center → moves to right.
#[test]
fn himeno_front_on_left_opponent_moves_to_right() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp_center = game.id(FILLER);
    let opp_right = game.id(FILLER);

    game.state.player2.stage.stage[0] = -1;
    game.state.player2.stage.stage[1] = opp_center;
    game.state.player2.stage.stage[2] = opp_right;
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::LeftSide);

    while game.has_pending_choice() {
        game.select_option(0);
    }

    assert_eq!(game.state.player2.stage.stage[2], opp_center);
    assert_eq!(game.state.player2.stage.stage[1], opp_right);
}

/// Himeno on right. Front of right = left (mirrored). Select opponent center → moves to left.
#[test]
fn himeno_front_on_right_opponent_moves_to_left() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp_center = game.id(FILLER);
    let opp_left = game.id(FILLER);

    game.state.player2.stage.stage = [opp_left, opp_center, -1];
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::RightSide);

    while game.has_pending_choice() {
        // First option is "left" (index 0), which IS the front of right.
        // Select "center" (index 1) as the source instead.
        game.select_option(1);
    }

    assert_eq!(game.state.player2.stage.stage[0], opp_center);
}

/// No opponent members → no valid source → ability does nothing.
#[test]
fn himeno_front_no_opponent_skill_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);

    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::Center);

    assert!(
        !game.has_pending_choice(),
        "No choice when no opponent target"
    );
}

/// Constant ability (ab#1): gain heart01 when front opponent has higher cost.
#[test]
fn himeno_front_constant_evaluates() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO); // cost 9

    game.state.player1.stage.stage[1] = himeno;
    // Opponent member at front with HIGHER cost (条件: 相手のコスト > 姫芽のコスト)
    let opp = game.id("PL!HS-sd1-006-SD"); // cost 15
    game.state.player2.stage.stage[1] = opp;
    game.state.recalculate_constants();

    let m = game
        .state
        .mods
        .get_heart_modifier(himeno, rabuka_engine::card::parse_heart_color("heart01"));
    assert_eq!(
        m, 1,
        "higher-cost opponent at front must grant exactly 1 heart01"
    );
}

/// No opponent in front → no heart01.
#[test]
fn himeno_front_constant_uses_modified_front_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp = game.id("PL!HS-sd1-006-SD");

    game.state.player1.stage.stage[1] = himeno;
    game.state.player2.stage.stage[1] = opp;
    game.state.mods.add_cost_modifier(opp, -7);
    game.state.recalculate_constants();

    let m = game
        .state
        .mods
        .get_heart_modifier(himeno, rabuka_engine::card::parse_heart_color("heart01"));
    assert_eq!(m, 0);
}

#[test]
fn himeno_front_constant_uses_modified_own_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp = game.id(FILLER);

    game.state.player1.stage.stage[1] = himeno;
    game.state.player2.stage.stage[1] = opp;
    game.state.mods.add_cost_modifier(himeno, -6);
    game.state.recalculate_constants();

    let m = game
        .state
        .mods
        .get_heart_modifier(himeno, rabuka_engine::card::parse_heart_color("heart01"));
    assert_eq!(m, 1);
}

#[test]
fn himeno_front_constant_no_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);

    game.state.player1.stage.stage[1] = himeno;
    game.state.player2.stage.stage[1] = -1;
    game.state.recalculate_constants();

    let m = game
        .state
        .mods
        .get_heart_modifier(himeno, rabuka_engine::card::parse_heart_color("heart01"));
    assert_eq!(m, 0);
}

/// ab#1 read from her LEFT slot: 「このメンバーの正面のエリア」 is the opponent's
/// RIGHT, and only that area counts.
///
/// Every other ab#1 test on this file stages 姫芽 at `stage[1]`. On the centre slot
/// "the area in front of her" and "the centre" are the same area, so the host-relative
/// mapping is not exercised at all — an implementation that read `front` as `center`
/// unconditionally would pass all five of them.
///
/// The mapping is not invented here: `himeno_front_on_left_opponent_moves_to_right`
/// above already establishes that 姫芽 on the left puts her front at the opponent's
/// right. This test asks only whether the 常時 clause reads the same area.
///
/// 姫芽 costs 9 and `PL!HS-sd1-006-SD` costs 15, so the opponent member satisfies
/// 「よりコストが高い」 and the heart is decided purely by WHICH area it stands in.
#[test]
fn himeno_front_constant_from_left_reads_the_opponents_right() {
    let db = load_real_database();

    // Her front is the opponent's right: the heart is granted.
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let dear = game.id("PL!HS-sd1-006-SD"); // cost 15 > 9
    game.state.player1.stage.stage[0] = himeno; // LEFT side
    game.state.player2.stage.stage[2] = dear; // opponent RIGHT = her front
    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(himeno, rabuka_engine::card::parse_heart_color("heart01")),
        1,
        "姫芽 on the left: 「このメンバーの正面のエリア」 is the opponent's right, where \
         the 15-cost member stands"
    );

    // Same two cards, the opponent's member moved one area along. Now it is not in
    // front of her, so the heart is withheld — a `front == center` reading would
    // still grant it here if the centre happened to be occupied, which is why the
    // centre slot is left empty rather than filled.
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let dear = game.id("PL!HS-sd1-006-SD");
    game.state.player1.stage.stage[0] = himeno;
    game.state.player2.stage.stage[0] = dear; // opponent LEFT — not her front
    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(himeno, rabuka_engine::card::parse_heart_color("heart01")),
        0,
        "the same member one area away is not 「正面のエリア」, so no heart01 — this is \
         the half a centre-bound reading cannot produce"
    );
}

/// The mirror: on her RIGHT slot, her front is the opponent's LEFT.
///
/// Together with the test above this walks both mirrored slots, so 「正面」 cannot be
/// implemented as either "always the centre" or "always the same side as her".
#[test]
fn himeno_front_constant_from_right_reads_the_opponents_left() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let dear = game.id("PL!HS-sd1-006-SD"); // cost 15 > 9
    game.state.player1.stage.stage[2] = himeno; // RIGHT side
    game.state.player2.stage.stage[0] = dear; // opponent LEFT = her front
    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(himeno, rabuka_engine::card::parse_heart_color("heart01")),
        1,
        "姫芽 on the right: 「正面のエリア」 is the opponent's left"
    );

    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let dear = game.id("PL!HS-sd1-006-SD");
    game.state.player1.stage.stage[2] = himeno;
    game.state.player2.stage.stage[2] = dear; // her OWN side, not her front
    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(himeno, rabuka_engine::card::parse_heart_color("heart01")),
        0,
        "「このメンバーの正面」 is mirrored, so the opponent member on her own side does \
         not count"
    );
}

/// 「自分のステージにいるメンバーが『みらくらぱーく！』のみの場合」 — one outsider
/// blocks it.
///
/// Every other debut test on this file stages 姫芽 **alone**, so the condition is only
/// ever evaluated in the direction that lets the ability run. A stage-gate read as
/// "at least one みらくらぱーく！" instead of "only みらくらぱーく！" — or dropped
/// entirely — passes all of them.
///
/// AZALEA is not みらくらぱーく！, and the assertion says so rather than trusting the
/// comment: the same mistake in `pb2_018` (a Printemps card annotated as BiBi) produced
/// a confident, wrong diagnosis in that file's own header.
#[test]
fn himeno_debut_one_non_miracle_ally_blocks_the_position_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id(HIMENO);
    let outsider = game.id("PL!S-bp5-012-N"); // 松浦果南, AZALEA
    let opp = game.id(FILLER);

    assert!(
        !rabuka_engine::ability::util::card_matches_group_str(
            &game.db,
            outsider,
            Some("みらくらぱーく！")
        ),
        "fixture: the ally that breaks exclusivity must NOT be 『みらくらぱーく！』, or \
         this test decides on something other than group"
    );

    // The ally occupies a side area, so 姫芽 still debuts to the centre beside it.
    game.state.player1.stage.stage[0] = outsider;
    game.state.player2.stage.stage = [opp, -1, -1];
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::Center);

    // Explicit arms, not the drain loop the four tests above use. This board offers
    // no target at all, so the correct outcome is that NOTHING is asked — and
    // `drain_choices_strict` with an empty `allowed` list turns a stray prompt into a
    // failure rather than answering it. A loop would have passed either way, which is
    // the whole hazard docs/EDGE_CASE_FINDINGS.md C9 describes.
    game.drain_choices_strict(&[], &[]);

    assert_eq!(
        game.state.player2.stage.stage[1],
        -1,
        "『みらくらぱーく！』のみの場合 — an AZALEA ally on my stage means the opponent's \
         member is not position-changed; their row is {:?}",
        game.state.player2.stage.stage
    );
}

/// The control: a SECOND 『みらくらぱーく！』 member still qualifies.
///
/// Without it, the test above would also pass on an implementation that required the
/// stage to hold exactly one member, or that mis-scanned the stage and saw nothing.
/// Two instances of 姫芽 herself are two members and both are 『みらくらぱーく！』, so the
/// gate is satisfied by a multi-member stage — 「のみ」 is about group, not about count.
#[test]
fn himeno_debut_a_second_miracle_member_still_qualifies() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id(HIMENO);
    let ally = game.new_id(HIMENO); // a second 姫芽 — same card, so same group
    let opp = game.id(FILLER);

    game.state.player1.stage.stage[0] = ally;
    game.state.player2.stage.stage = [opp, -1, -1];
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::Center);

    // One prompt, answered by name: the area to move the chosen member into. Asserting
    // the type first means a mistyped or mis-ordered prompt fails here instead of
    // being silently absorbed by a drain loop.
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "the gate holds, so the position change is offered"
    );
    game.select_option(0);
    game.drain_choices_strict(&[], &[]);

    assert_eq!(
        game.state.player2.stage.stage[1],
        opp,
        "a second 『みらくらぱーく！』 member keeps the stage all-みらくらぱーく！, so the \
         opponent's left member moves to the front of 姫芽"
    );
    assert_eq!(game.state.player1.stage.stage[0], ally, "the ally stayed put");
}

/// Opponent at side not front → no heart01.
#[test]
fn himeno_front_constant_opponent_at_side() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp = game.id(FILLER);

    game.state.player1.stage.stage[1] = himeno;
    game.state.player2.stage.stage[0] = opp;
    game.state.player2.stage.stage[1] = -1;
    game.state.recalculate_constants();

    let m = game
        .state
        .mods
        .get_heart_modifier(himeno, rabuka_engine::card::parse_heart_color("heart01"));
    assert_eq!(m, 0);
}
