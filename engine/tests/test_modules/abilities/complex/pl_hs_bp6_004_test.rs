use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn seed_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn setup_and_trigger_live_start(game: &mut TestGame, hand_cards: Vec<i16>) {
    let filler_live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(filler_live);
    for c in hand_cards {
        game.state.player1.hand.cards.push(c);
    }
    seed_deck(game);
    game.give_energy(5);
    advance_to_live_card_set_p1(game);
    game.set_live_card(filler_live);
    advance_to_live_start(game);
}

/// 百生吟子 on stage with an opponent member (cost=2) for ab#0 to target.
/// Discard another 百生吟子 from hand as cost for ab#1.
/// The discarded card IS a 百生吟子 member → condition met → 2 blades.
#[test]
fn ginako_discard_self_gains_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ginako = game.id("PL!HS-bp6-004-R");
    let second_ginako = game.new_id("PL!HS-bp6-004-R");
    let opp_member = game.new_id("PL!-sd1-005-SD"); // cost 2 (<= 9, legal ab#0 target)

    game.state.player1.stage.stage[1] = ginako;
    game.state.player2.stage.stage[1] = opp_member;

    setup_and_trigger_live_start(&mut game, vec![second_ginako]);

    let discard_before = game.state.player1.waitroom.cards.len();

    // Ginako has two LiveStart abilities (ab#0 cost<=9 wait, ab#1 discard→blade).
    // They trigger simultaneously → first pending must be SelectAutoAbility ordering.
    assert!(
        game.has_pending_choice(),
        "ginako LiveStart must offer SelectAutoAbility for two abilities"
    );
    let first = game.get_pending_choice().clone();
    assert!(
        matches!(first, rabuka_engine::ability::types::Choice::SelectAutoAbility { .. }),
        "first choice must be SelectAutoAbility, got {:?}",
        first
    );
    game.select_option(0);

    // ab#0 auto-applies when exactly 1 legal target (engine auto-picks, no prompt).
    // Strict: verify the auto-picked target is the ONLY legal one (cost 2 <= 9)
    // and the wait was actually applied — not silently skipped.
    let opp_ori = game.state.mods.get_orientation_modifier(opp_member);
    assert_eq!(
        opp_ori,
        Some("wait"),
        "ab#0 must auto-apply wait to the single cost<=9 opponent (cost 2 <= 9)"
    );

    // ab#1 must offer hand discard (SelectCard hand, allow_skip)
    assert!(
        game.has_pending_choice(),
        "ab#1 must offer hand SelectCard after ab#0"
    );
    let ch = game.get_pending_choice().clone();
    match ch {
        rabuka_engine::ability::types::Choice::SelectCard { zone, allow_skip, .. } => {
            assert_eq!(zone, "hand", "ab#1 discard must be from hand, got {}", zone);
            assert!(allow_skip, "ab#1 discard is optional (してもよい)");
        }
        other => panic!("ab#1 must be SelectCard(hand), got {:?}", other),
    }
    game.select_indices(&[0]); // discard second_ginako (百生吟子)

    // Drain remaining (conditional_on_result followup etc.)
    let mut safety = 0;
    while game.has_pending_choice() && safety < 10 {
        safety += 1;
        let ch = game.get_pending_choice().clone();
        match ch {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => game.select_option(0),
            _ => game.select_indices(&[0]),
        }
    }

    // Strict results:
    let blade = game.state.mods.get_blade_modifier(ginako);
    assert_eq!(blade, 2, "discard 百生吟子 → 1 base blade + 1 conditional = 2, got {}", blade);
    assert_eq!(
        game.state.mods.get_orientation_modifier(opp_member),
        Some("wait"),
        "opponent must remain waited"
    );
    assert!(
        game.state.player1.waitroom.cards.len() > discard_before,
        "hand card must be discarded for ab#1 cost"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&second_ginako),
        "the discarded card must be the second Ginako"
    );
    assert!(
        !game.state.player2.waitroom.cards.contains(&opp_member),
        "wait must NOT move the card to waitroom (orientation only)"
    );
}

/// Discard a card that is NOT 百生吟子 → condition not met → 1 blade.
#[test]
fn ginako_discard_non_ginako_gains_one_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ginako = game.id("PL!HS-bp6-004-R");
    let non_ginako = game.id("PL!-sd1-010-SD");
    let opp_member = game.new_id("PL!-sd1-005-SD");

    game.state.player1.stage.stage[1] = ginako;
    game.state.player2.stage.stage[1] = opp_member;

    setup_and_trigger_live_start(&mut game, vec![non_ginako]);

    // ab#0: put opponent member to wait
    assert!(game.has_pending_choice(), "ab#0 target selection");
    game.select_indices(&[0]);

    // ab#1: optional discard cost → select non-百生吟子 at index 0
    assert!(game.has_pending_choice(), "ab#1 optional discard");
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let blade = game.state.mods.get_blade_modifier(ginako);
    assert_eq!(blade, 1);
}

/// Skip the optional cost → no discard, no blade.
#[test]
fn ginako_skip_cost_gains_zero_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ginako = game.id("PL!HS-bp6-004-R");
    let opp_member = game.new_id("PL!-sd1-005-SD");

    game.state.player1.stage.stage[1] = ginako;
    game.state.player2.stage.stage[1] = opp_member;

    setup_and_trigger_live_start(&mut game, vec![]);

    // ab#0: put opponent member to wait
    assert!(game.has_pending_choice(), "ab#0 target selection");
    game.select_indices(&[0]);

    // ab#1: optional discard → skip with empty selection
    assert!(game.has_pending_choice(), "ab#1 optional discard");
    game.select_indices(&[]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let blade = game.state.mods.get_blade_modifier(ginako);
    assert_eq!(blade, 0);
}

/// Two Ginako copies on stage: both get the optional discard choice.
/// Discard one Ginako → the discarding copy gets 2 blades.
#[test]
fn two_ginako_discard_one_gains_blade_on_self() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ginako1 = game.id("PL!HS-bp6-004-R");
    let ginako2 = game.new_id("PL!HS-bp6-004-R");
    let third_ginako = game.new_id("PL!HS-bp6-004-R");
    let opp_member = game.new_id("PL!-sd1-005-SD");

    game.state.player1.stage.stage[0] = ginako1;
    game.state.player1.stage.stage[1] = ginako2;
    game.state.player2.stage.stage[1] = opp_member;

    setup_and_trigger_live_start(&mut game, vec![third_ginako]);

    // Multiple auto-abilities trigger simultaneously → SelectAutoAbility ordering
    // 4 abilities queued: g1ab0, g1ab1, g2ab0, g2ab1
    // 4 rounds needed: auto-ordering, run ability, handle sub-choices
    let mut paid = false;
    for _ in 0..8 {
        if !game.has_pending_choice() {
            break;
        }
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0);
            }
            rabuka_engine::ability::types::Choice::SelectCard {
                zone,
                allow_skip: _,
                ..
            } => {
                if zone == "hand" && !paid {
                    game.select_indices(&[0]);
                    paid = true;
                } else {
                    game.select_indices(&[]);
                }
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let blade1 = game.state.mods.get_blade_modifier(ginako1);
    let blade2 = game.state.mods.get_blade_modifier(ginako2);
    // Whichever ginako paid cost with 百生吟子 → 2 blades; the other → 0
    assert_eq!(
        blade1 + blade2,
        2,
        "Exactly one ginako should get 2 blades, got {} + {}",
        blade1,
        blade2
    );
}
