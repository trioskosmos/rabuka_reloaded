/// Tests for エマ・ヴェルデ (PL!N-bp3-008-R＋) — Activation: wait a にこ member
/// other than this member → draw 1.
///
/// Q163: "This member" (the ability user) cannot be selected for the wait cost
/// because of exclude_self. With no other qualifying members, the cost fails.

mod helpers;
use helpers::*;

/// Only エマ on stage (a にこ member). exclude_self=true means no valid target
/// for the wait cost → activation should fail/not prompt.
#[test]
fn emma_q163_self_excluded_no_other_niko_cost_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[2] = filler;

    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }

    // Activate ability
    game.activate_ability(emma);

    // Cost: wait a にこ member other than self.
    // With only エマ (a にこ member) on stage and exclude_self=true,
    // no valid candidates → cost should fail silently
    // (the ability should not proceed to draw)

    // Drain any pending choices
    while game.has_pending_choice() { game.select_indices(&[]); }

    // Cost should fail since exclude_self leaves no candidates.
    // The failed cost should not proceed to draw.
    let hand_count = game.state.player1.hand.cards.len();
    eprintln!("[EMMA] hand after failed activation: {}", hand_count);
    // hand started with 1 filler, never drew because ability cost failed
    assert_eq!(hand_count, 1, "No draw happened because cost couldn't be paid");
}

/// Put エマ alongside a 虹ヶ咲 member. The group_names: ["虹ヶ咲"] from the
/// parser should match the 虹ヶ咲 member. exclude_self excludes エマ.
/// → the 虹ヶ咲 member is the only valid candidate → it gets waited → draw 1.
#[test]
fn emma_q163_nijigasaki_member_pays_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    // 虹ヶ咲 member: any 虹ヶ咲 series member card
    let niji = game.id("PL!N-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[2] = niji;

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }

    // Activate Emma's ability
    game.activate_ability(emma);

    // Cost: select 1 stage member to wait (valid: niji, excluded: emma, filler not 虹ヶ咲)
    while game.has_pending_choice() { game.select_indices(&[0]); }

    let hand_count = game.state.player1.hand.cards.len();
    eprintln!("[EMMA] hand after activation: {}", hand_count);
    let niji_waited = game.state.mods.get_orientation_modifier(niji)
        .map_or(false, |o| o == "wait");
    eprintln!("[EMMA] niji waited: {}", niji_waited);
    assert!(hand_count > 0, "Should have drawn 1 card");
    assert!(niji_waited, "虹ヶ咲 member should be waited");
}
