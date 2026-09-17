use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";

fn fire_pl_sp_sd1_007_sd_debut(game: &mut TestGame, cid: i16) {
    fire_trigger(game, cid, AbilityTrigger::Debut, "登場");
}

fn setup_pl_sp_sd1_007_sd_member_recovery(game: &mut TestGame) -> (i16, i16) {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id("PL!SP-sd1-007-SD");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(10);
    let kanon = game.new_id("PL!SP-sd1-002-SD");
    game.state.player1.waitroom.cards.push(kanon);
    (me, kanon)
}

#[test]
fn pl_sp_sd1_007_sd_paid_energy_recovers_liella_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, kanon) = setup_pl_sp_sd1_007_sd_member_recovery(&mut game);

    fire_pl_sp_sd1_007_sd_debut(&mut game, me);
    assert!(
        game.has_pending_choice(),
        "optional pay gate must be offered"
    );
    game.select_option(1);

    assert!(
        game.state.player1.hand.cards.contains(&kanon),
        "Liella! member retrieved from the waitroom to hand"
    );
}

#[test]
fn pl_sp_sd1_007_sd_declined_energy_does_not_recover_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, kanon) = setup_pl_sp_sd1_007_sd_member_recovery(&mut game);

    fire_pl_sp_sd1_007_sd_debut(&mut game, me);
    assert!(
        game.has_pending_choice(),
        "optional pay gate must be offered"
    );
    game.select_option(0);

    assert!(
        !game.state.player1.hand.cards.contains(&kanon),
        "declined -> Kanata stays in the waitroom"
    );
}
