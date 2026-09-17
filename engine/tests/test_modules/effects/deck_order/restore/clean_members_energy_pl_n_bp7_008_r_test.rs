use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";
const KOTORI_CLEAN: &str = "PL!-pb1-021-PR";

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .expect("card should have the requested trigger ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn pl_n_bp7_008_r_bottoms_only_non_blade_heart_members_and_activates_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let emma = game.id("PL!N-bp7-008-R");
    let clean = game.id(KOTORI_CLEAN);
    let bladefill = game.id(FILLER);
    let bladefill2 = game.id("PL!-sd1-002-SD");
    game.state.player1.stage.stage[1] = emma;
    for cid in [bladefill, clean, bladefill2] {
        game.state.player1.waitroom.cards.push(cid);
    }
    let stock = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(stock);
    game.state.player1.main_deck.cards.push(stock);
    game.give_energy(4);
    game.state.player1.energy_zone.set_active_count(1);
    trigger_auto(&mut game, emma, AbilityTrigger::Debut, "登場");
    game.select_indices(&[0]);
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[]),
            _ => break,
        }
    }
    eprintln!(
        "[EMMA_DBG] deck={:?} waitroom={:?} clean={} blade={} blade2={}",
        game.state.player1.main_deck.cards,
        game.state.player1.waitroom.cards,
        clean,
        bladefill,
        bladefill2
    );
    assert_eq!(
        *game
            .state
            .player1
            .main_deck
            .cards
            .last()
            .expect("deck bottom holds the placed card"),
        clean,
        "placed card sits at the deck BOTTOM (push end)"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&clean),
        "placed card left the waitroom"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&bladefill)
            && game.state.player1.waitroom.cards.contains(&bladefill2),
        "blade-heart holders are not eligible"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "1 card placed → 1 wait energy activated (1+1)"
    );
}
