use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};

const FILLER: &str = "PL!N-sd1-010-SD";
const TOO_EXPENSIVE: &str = "PL!HS-bp6-002-R"; // 村野さやか, cost 9 Hasunosora
const NON_HASU: &str = "PL!-sd1-010-SD"; // μ's, not Hasunosora

// PL!HS-bp6-016-R 桂城泉 (was positive-path only)
// {{起動}}{{ターン1回}}4E: waitroom → deploy 1 cost≤4 Hasunosora member to
// an EMPTY area.
fn izumi_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id("PL!HS-bp6-016-R");
    assert_eq!(
        game.db.get_card(me).unwrap().card_no,
        "PL!HS-bp6-016-R",
        "staged the R print of Izumi"
    );
    game.state.player1.stage.stage[1] = me;
    game.give_energy(6);
    me
}

fn use_offers(game: &TestGame, cid: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
        })
        .count()
}

fn drain(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        !game.has_pending_choice(),
        "prompts should terminate after answering"
    );
}

#[test]
fn pl_hs_bp6_016_r_activation_moves_low_cost_hasunosora_member_from_waitroom_to_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let me = game.id("PL!HS-bp6-016-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(6);
    let kamaru = game.new_id("PL!HS-bp2-004-R");
    game.state.player1.waitroom.cards.push(kamaru);
    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        game.state.player1.stage.stage.contains(&kamaru),
        "cost<=4 『蓮ノ空』 member debuted into an empty area"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&kamaru),
        "deployed member left the waitroom"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "4E of 6 paid"
    );
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(me, 0, game.state.turn_number)),
        "ターン1回 use recorded"
    );
    assert_eq!(use_offers(&game, me), 0, "no re-offer after consuming");
}

/// Edge: only a cost-9 Hasunosora member waits — ineligible. The 4E cost is
/// still payable, so the effect (not the cost) fizzles: offered, energy
/// paid, nothing deployed, terminates.
#[test]
fn pl_hs_bp6_016_r_too_expensive_member_not_deployed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = izumi_setup(&mut game);
    let pricey = game.new_id(TOO_EXPENSIVE);
    game.state.player1.waitroom.cards.push(pricey);

    assert_eq!(use_offers(&game, me), 1, "4E payable: offered");
    game.activate_ability(me);
    drain(&mut game);

    assert!(
        game.state.player1.waitroom.cards.contains(&pricey),
        "cost-9 member stays in waitroom"
    );
    assert!(
        !game.state.player1.stage.stage.contains(&pricey),
        "cost-9 member never deploys"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "cost paid even though the effect found no target"
    );
}

/// Edge: eligible + ineligible wait together — only the eligible deploys.
#[test]
fn pl_hs_bp6_016_r_non_hasunosora_ignored() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = izumi_setup(&mut game);
    let kamaru = game.new_id("PL!HS-bp2-004-R");
    let outsider = game.new_id(NON_HASU);
    game.state.player1.waitroom.cards.push(kamaru);
    game.state.player1.waitroom.cards.push(outsider);

    game.activate_ability(me);
    drain(&mut game);

    assert!(
        game.state.player1.stage.stage.contains(&kamaru),
        "eligible member deploys"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&outsider),
        "non-Hasunosora member ignored"
    );
}

/// Edge: stage full (no empty area) → terminates with nothing deployed.
#[test]
fn pl_hs_bp6_016_r_full_stage_terminates_without_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = izumi_setup(&mut game);
    let kamaru = game.new_id("PL!HS-bp2-004-R");
    game.state.player1.waitroom.cards.push(kamaru);
    let l = game.new_id(FILLER);
    let r = game.new_id(FILLER);
    game.state.player1.stage.stage = [l, me, r];

    game.activate_ability(me);
    drain(&mut game);

    assert!(
        game.state.player1.waitroom.cards.contains(&kamaru),
        "no empty area: member stays in waitroom"
    );
    assert_eq!(
        game.state.player1.stage.stage,
        [l, me, r],
        "stage unchanged"
    );
}
