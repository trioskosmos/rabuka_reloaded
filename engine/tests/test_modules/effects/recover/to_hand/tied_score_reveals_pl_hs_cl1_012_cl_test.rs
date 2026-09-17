use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const TIED_LIVE: &str = "PL!S-bp7-024-L";
const SATISFYING_MEMBER: &str = "PL!S-PR-014-PR";

fn pl_hs_cl1_012_cl_setup_live(game: &mut TestGame) {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let live = game.id("PL!HS-cl1-012-CL");
    game.state.player1.live_card_zone.cards.push(live);
}

fn place_tied_live(game: &mut TestGame, player: u8) {
    let l = game.new_id(TIED_LIVE);
    let member = game.id(SATISFYING_MEMBER);
    let (p, stage_idx) = if player == 1 {
        (&mut game.state.player1, 2usize)
    } else {
        (&mut game.state.player2, 2usize)
    };
    p.live_card_zone.cards.push(l);
    p.stage.stage[stage_idx] = member;
    p.stage_hearts = Some(p.calculate_stage_hearts(
        &game.db,
        &game.state.mods.heart_color_multiplier,
        &game.state.mods.heart_override,
        &game.state.mods.heart_modifiers,
        &game.state.mods.heart_copy,
    ));
}

#[test]
fn pl_hs_cl1_012_cl_tied_scores_retrieve_expensive_revealed_member_not_cheap_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    pl_hs_cl1_012_cl_setup_live(&mut game);
    place_tied_live(&mut game, 1);
    place_tied_live(&mut game, 2);
    let expensive = game.new_id("PL!HS-bp5-004-R");
    let cheap = game.new_id("PL!SP-PR-003-PR");
    game.state.revealed_cards.push(expensive);
    game.state.revealed_cards.push(cheap);
    let live_id = game.id_ref("PL!HS-cl1-012-CL");
    fire_trigger(
        &mut game,
        live_id,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        game.state.player1.hand.cards.contains(&expensive),
        "tie -> cost>=9 member retrieved to hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&cheap),
        "cost-2 member not eligible"
    );
}

#[test]
fn pl_hs_cl1_012_cl_unequal_scores_do_not_retrieve_revealed_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    pl_hs_cl1_012_cl_setup_live(&mut game);
    place_tied_live(&mut game, 1);
    let expensive = game.new_id("PL!HS-bp5-004-R");
    game.state.revealed_cards.push(expensive);
    let live_id = game.id_ref("PL!HS-cl1-012-CL");
    fire_trigger(
        &mut game,
        live_id,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert!(
        !game.state.player1.hand.cards.contains(&expensive),
        "scores not tied -> no retrieval"
    );
}
