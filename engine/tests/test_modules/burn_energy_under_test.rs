/// PL!N-bp7-029-L Burn!! idx900 live_success with under_member source
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const BURN: &str = "PL!N-bp7-029-L";
const MEMBER: &str = "PL!N-bp1-007-R";
const ENERGY: &str = "LL-E-001-SD";

fn stage_with_under(member_card: &str, under_count: usize) -> (TestGame, i16) {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let mem = g.id(member_card);
    g.add_to_stage(MemberArea::Center, mem);
    for _ in 0..under_count {
        let e = g.id(ENERGY);
        g.state.player1.stage.place_under_card(MemberArea::Center, e);
    }
    (g, mem)
}

fn trigger_burn_success(game: &mut TestGame, burn_id: i16) {
    let card = game.db.get_card(burn_id).unwrap();
    let ab = card.resolved_abilities().find(|a| a.triggers.as_deref() == Some("ライブ成功時")).unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(burn_id),
        None,
        None,
    );
    game.state.activating_card = Some(burn_id);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
}

fn select_burn_move(game: &mut TestGame) {
    use rabuka_engine::ability::types::Choice;
    while game.has_pending_choice() {
        let ch = game.state.get_pending_choice().cloned().unwrap();
        match ch {
            Choice::SelectTarget { target, .. } if target.contains("pay_optional_cost") => {
                game.select_choice_option(1); // Move
            }
            Choice::SelectCard { zone, .. } if zone == "stage" => {
                // filtered_indices = [1] for center, so index 0 selects center
                game.select_indices(&[0]);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
        game.drain_auto_ability_choices();
    }
}

fn select_burn_skip(game: &mut TestGame) {
    use rabuka_engine::ability::types::Choice;
    if game.has_pending_choice() {
        let ch = game.state.get_pending_choice().cloned().unwrap();
        match ch {
            Choice::SelectTarget { target, .. } if target.contains("pay_optional_cost") => {
                game.select_choice_option(0); // Skip
            }
            _ => game.select_indices(&[]),
        }
    }
    game.drain_auto_ability_choices();
}

#[test]
fn burn_no_under_no_move_no_score() {
    let (mut g, _mem) = stage_with_under(MEMBER, 0);
    let burn = g.id(BURN);
    g.state.player1.success_live_card_zone.cards.push(burn);
    for _ in 0..10 { g.give_energy(1); }
    let score_before = g.state.mods.get_score_modifier(burn);
    trigger_burn_success(&mut g, burn);
    select_burn_skip(&mut g);
    let score_after = g.state.mods.get_score_modifier(burn);
    assert_eq!(score_after, score_before, "0 under +10 total should be 0 – needs 1+ moved");
}

#[test]
fn burn_one_under_moves_and_scores_when_total_10() {
    let (mut g, mem) = stage_with_under(MEMBER, 1);
    for _ in 0..9 { g.give_energy(1); }
    assert_eq!(g.state.player1.energy_zone.cards.len(), 9);
    assert_eq!(g.state.player1.stage.under_cards[1].len(), 1);
    let burn = g.id(BURN);
    g.state.player1.success_live_card_zone.cards.push(burn);
    trigger_burn_success(&mut g, burn);
    select_burn_move(&mut g);
    let score = g.state.mods.get_score_modifier(burn);
    assert_eq!(score, 1, "1 under+9=10 with 1 moved should be 1");
    let _ = mem;
}

#[test]
fn burn_one_under_but_total_9_no_score() {
    let (mut g, _mem) = stage_with_under(MEMBER, 1);
    for _ in 0..8 { g.give_energy(1); }
    let burn = g.id(BURN);
    g.state.player1.success_live_card_zone.cards.push(burn);
    trigger_burn_success(&mut g, burn);
    select_burn_move(&mut g);
    let score = g.state.mods.get_score_modifier(burn);
    assert_eq!(score, 0, "1 under+8=9 total should be 0");
}

#[test]
fn burn_skip_optional_no_score_even_with_10_total() {
    let (mut g, _mem) = stage_with_under(MEMBER, 3);
    for _ in 0..10 { g.give_energy(1); }
    let burn = g.id(BURN);
    g.state.player1.success_live_card_zone.cards.push(burn);
    trigger_burn_success(&mut g, burn);
    select_burn_skip(&mut g);
    let score = g.state.mods.get_score_modifier(burn);
    assert_eq!(score, 0, "skip optional -> 0 moved => no score even with 10 total");
}

#[test]
fn burn_multiple_under_all_moved_and_scores() {
    let (mut g, _mem) = stage_with_under(MEMBER, 3);
    for _ in 0..7 { g.give_energy(1); }
    let burn = g.id(BURN);
    g.state.player1.success_live_card_zone.cards.push(burn);
    trigger_burn_success(&mut g, burn);
    select_burn_move(&mut g);
    let score = g.state.mods.get_score_modifier(burn);
    assert_eq!(score, 1, "3 under+7=10 with 3 moved should be 1");
}

#[test]
fn burn_wait_state_verification() {
    let (mut g, _mem) = stage_with_under(MEMBER, 2);
    for _ in 0..8 { g.give_energy(1); }
    let burn = g.id(BURN);
    g.state.player1.success_live_card_zone.cards.push(burn);
    trigger_burn_success(&mut g, burn);
    select_burn_move(&mut g);
    assert!(g.state.player1.energy_zone.active_count() <= 8);
}
