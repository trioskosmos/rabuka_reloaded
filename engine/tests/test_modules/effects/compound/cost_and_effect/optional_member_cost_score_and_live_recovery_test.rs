use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;

fn answer_optional(game: &mut TestGame, accept: bool) -> bool {
    match game.get_pending_choice() {
        Choice::SelectTarget { target, .. }
            if target == "conditional_optional"
                || target.starts_with("pay_optional_cost") =>
        {
            // options[0] = skip, options[1] = do it (WRITING_TESTS §H)
            game.select_choice_option(if accept { 1 } else { 0 });
            true
        }
        Choice::SelectCard { allow_skip, .. } => {
            if !accept && *allow_skip {
                game.select_indices(&[]);
                true
            } else {
                false
            }
        }
        _ => {
            eprintln!(
                "[answer_optional] unmatched prompt: {}",
                game.pending_choice_summary()
            );
            false
        }
    }
}

#[test]
fn pl_bp6_021_live_success_accept_member_cost_scores_and_recovers_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp6-021-L");
    let mus_member = game.new_id("PL!-sd1-010-SD"); // 『μ's』 member on stage
    let mus_live = game.id("PL!-sd1-020-SD"); // 『μ's』 live card in waitroom

    { let f = game.new_id("PL!-sd1-010-SD"); fill_decks(&mut game, f); }
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.stage.stage[0] = mus_member;
    game.add_to_discard(mus_live);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // Even with a single cost candidate the optional gate IS offered
    // (observed: SelectTarget pay_optional_cost:skip_optional_cost).
    assert!(
        game.has_pending_choice(),
        "optional μ's-member cost gate must be offered"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget optional-cost gate"
    );
    assert!(answer_optional(&mut game, true), "unexpected prompt shape");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "cost accepted -> score +1"
    );
    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "cost accepted -> μ's live card retrieved to hand"
    );
    assert!(
        !game.state.player1.stage.stage.contains(&mus_member),
        "cost accepted -> the μ's member left the stage"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&mus_member),
        "cost accepted -> the μ's member went to the waitroom"
    );
}

#[test]
fn pl_bp6_021_live_success_skip_member_cost_no_score_or_recovery() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp6-021-L");
    // TWO μ's members on stage: multiple candidates force a real prompt
    // instead of the single-target auto-pay.
    let m1 = game.new_id("PL!-sd1-010-SD");
    let m2 = game.new_id("PL!-sd1-010-SD");
    let mus_live = game.id("PL!-sd1-020-SD");

    { let f = game.new_id("PL!-sd1-010-SD"); fill_decks(&mut game, f); }
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.stage.stage[0] = m1;
    game.state.player1.stage.stage[2] = m2;
    game.add_to_discard(mus_live);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice(), "multiple candidates -> prompt expected");
    assert!(answer_optional(&mut game, false), "expected an optional-gate prompt");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "cost declined -> no score"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&mus_live),
        "cost declined -> no retrieval"
    );
    assert!(
        game.state.player1.stage.stage.contains(&m1)
            && game.state.player1.stage.stage.contains(&m2),
        "cost declined -> both members stay on stage"
    );
}
