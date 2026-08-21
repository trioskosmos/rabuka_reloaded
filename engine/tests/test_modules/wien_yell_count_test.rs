/// Tests for 繧ｦ繧｣繝ｼ繝ｳ繝ｻ繝槭Ν繧ｬ繝ｬ繝ｼ繝・(PL!SP-bp2-010-R+) ab#1 窶・LiveStart:
///   閾ｪ蛻・・繧ｹ繝・・繧ｸ縺ｫ縺薙・繝｡繝ｳ繝舌・莉･螟悶・繝｡繝ｳ繝舌・縺・莠ｺ莉･荳翫＞繧句ｴ蜷医・///   繝ｩ繧､繝也ｵゆｺ・凾縺ｾ縺ｧ縲√お繝ｼ繝ｫ縺ｫ繧医▲縺ｦ蜈ｬ髢九＆繧後ｋ閾ｪ蛻・・繧ｫ繝ｼ繝峨・譫壽焚縺・譫壽ｸ帙ｋ縲・///
/// Parsed: condition card_count_condition(>=1, exclude_self, stage)
///         action modify_yell_count(operation=subtract, value=8)
///
/// The old gameplay_test.rs coverage only asserted "members remain on stage"
/// and never verified the yell-count modification itself. These tests assert
/// gs.cheer_checks_required directly.
///
/// Edge cases:
///   1. Partner on stage 竊・cheer checks reduced by exactly 8
///   2. Wien alone 竊・no reduction (condition unmet)
///   3. Opponent's cheer checks unaffected
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const WIEN: &str = "PL!SP-bp2-010-R\u{ff0b}"; // full-width plus
const FILLER: &str = "PL!-sd1-010-SD";

fn fill_decks(game: &mut TestGame) {
    let fill = game.id(FILLER);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
        game.state.player2.main_deck.cards.push(fill);
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_past_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        game.select_indices(&[]);
        guard += 1;
    }
}

/// Baseline blade count used for the cheer check (must exceed 8 so the
/// subtraction is observable).
const BASE_BLADE: u8 = 10;

// =========================================================================
// 1. Partner on stage 竊・required checks = BASE_BLADE - 8.
// =========================================================================
#[test]
fn wien_with_partner_reduces_cheer_checks_by_8() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wien = game.id(WIEN);
    let partner = game.id(FILLER);

    game.state.player1.stage.stage = [-1, wien, partner];
    let live_card = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_past_live_start(&mut game);

    // Trigger the cheer check with a known blade count.
    let p1_id = game.state.player1.id.clone();
    let _ = game
        .state
        .perform_cheer_check(&p1_id, BASE_BLADE);

    assert_eq!(
        game.state.cheer_checks_required, BASE_BLADE - 8,
        "Wien + partner must reduce required cheer checks by 8 (got {})",
        game.state.cheer_checks_required
    );
}

// =========================================================================
// 2. Wien alone 竊・condition unmet 竊・no reduction.
// =========================================================================
#[test]
fn wien_alone_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wien = game.id(WIEN);

    game.state.player1.stage.stage = [-1, wien, -1];
    let live_card = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_past_live_start(&mut game);

    let p1_id = game.state.player1.id.clone();
    let _ = game
        .state
        .perform_cheer_check(&p1_id, BASE_BLADE);

    assert_eq!(
        game.state.cheer_checks_required, BASE_BLADE,
        "Alone Wien must not reduce cheer checks (got {})",
        game.state.cheer_checks_required
    );
}

// =========================================================================
// 3. Opponent's cheer checks are unaffected by Wien's reduction.
// =========================================================================
#[test]
fn opponent_cheer_checks_unaffected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wien = game.id(WIEN);
    let partner = game.id(FILLER);

    game.state.player1.stage.stage = [-1, wien, partner];
    let live_card = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_past_live_start(&mut game);

    let p2_id = game.state.player2.id.clone();
    let _ = game
        .state
        .perform_cheer_check(&p2_id, BASE_BLADE);

    assert_eq!(
        game.state.cheer_checks_required, BASE_BLADE,
        "Opponent cheer checks must not be reduced (got {})",
        game.state.cheer_checks_required
    );
}

// =========================================================================
// 4. Q117: the "other member" may be ANOTHER COPY of Wien herself 窶・//    same card name does not matter. Reduction still applies.
// =========================================================================
#[test]
fn q117_second_wien_copy_counts_as_other_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wien1 = game.id(WIEN);
    let wien2 = game.new_id(WIEN);

    game.state.player1.stage.stage = [-1, wien1, wien2];
    let live_card = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_past_live_start(&mut game);

    let p1_id = game.state.player1.id.clone();
    let _ = game
        .state
        .perform_cheer_check(&p1_id, BASE_BLADE);

    assert_eq!(
        game.state.cheer_checks_required, 0,
        "Q117: second Wien copy satisfies 'this member以外'; both copies fire \
         so the reduction stacks (-16 on a base of 10, saturating to 0), got {}",
        game.state.cheer_checks_required
    );
}
