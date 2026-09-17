use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_debut(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("登場"))
            .unwrap_or_else(|| panic!("card {} lacks a 登場 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Debut,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

fn pl_bp6_013_n_setup_waitroom_live(game: &mut TestGame) -> (i16, i16) {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id("PL!-bp6-013-N");
    game.state.player1.stage.stage[0] = me;
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);
    (me, mus_live)
}

#[test]
fn pl_bp6_013_n_success_score_nine_recovers_mus_live_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, mus_live) = pl_bp6_013_n_setup_waitroom_live(&mut game);
    let big = game.id("PL!S-pb1-023-L");
    game.state.player1.success_live_card_zone.cards.push(big);
    fire_debut(&mut game, me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "score total >= 6 -> μ's live retrieved to hand"
    );
}

#[test]
fn pl_bp6_013_n_empty_success_zone_does_not_recover_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, mus_live) = pl_bp6_013_n_setup_waitroom_live(&mut game);
    fire_debut(&mut game, me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        !game.state.player1.hand.cards.contains(&mus_live),
        "empty success zone -> no retrieval"
    );
}
