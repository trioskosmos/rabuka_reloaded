use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn pl_s_sd1_009_sd_live_start_revealed_aqours_card_moves_from_hand_to_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let ruby = game.id("PL!S-sd1-009-SD");
    game.state.player1.stage.stage[1] = ruby;

    let aqours_card = game.new_id("PL!S-sd1-003-SD");
    let non_aqours = game.new_id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.clear();
    game.state.revealed_cards.clear();
    game.state.revealed_cost_cards.clear();
    game.state.player1.hand.cards = vec![non_aqours, aqours_card].into();

    fire_live_start(&mut game, ruby);

    assert!(game.has_pending_choice(), "reveal cost gate offered");
    match game.get_pending_choice() {
        Choice::SelectTarget { target, .. } if target == "position|destination" => {
            game.select_option(0);
        }
        _ => game.select_option(1),
    }
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        if game.pending_choice_type().as_deref() != Some("SelectCard") {
            game.select_option(0);
            continue;
        }
        let hand_index = game
            .state
            .player1
            .hand
            .cards
            .iter()
            .position(|card_id| *card_id == aqours_card)
            .expect("Aqours card in hand");
        game.select_indices(&[hand_index]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&aqours_card),
        "revealed Aqours card moved onto the deck"
    );
    let in_deck = game.state.player1.main_deck.cards.contains(&aqours_card);
    assert!(in_deck, "revealed card ends up on the deck");
}

/// The negative: the hand holds no Aqours card, so the optional cost has nothing it
/// can pay with and must not be offered at all.
///
/// The old guard was inverted. `if !game.has_pending_choice() { assert!(...); return }`
/// accepted BOTH outcomes — no prompt, or a prompt that was declined — so this test
/// could not fail on the defect it exists to catch: an ability offering a cost it has
/// no candidate for. It also returned early from the no-prompt branch, skipping the
/// deck-length assertion entirely.
///
/// 「手札の『Aqours』のカードを1枚公開してもよい」 is optional, so a cost that cannot be
/// paid is skipped without a prompt (Rule 9.4.2.3) and the ability resolves with no
/// effect — no blade, hand and deck untouched.
#[test]
fn pl_s_sd1_009_sd_live_start_without_aqours_preserves_non_aqours_hand_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let ruby = game.id("PL!S-sd1-009-SD");
    game.state.player1.stage.stage[1] = ruby;
    let only_mu = game.new_id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards = vec![only_mu].into();

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start(&mut game, ruby);

    // An empty `allowed` list makes any prompt a failure, which is the assertion the
    // inverted guard was missing.
    game.drain_choices_strict(&[], &[]);
    assert!(
        game.state.player1.hand.cards.contains(&only_mu),
        "the non-Aqours hand card is not something this ability may touch — it neither \
         reveals nor moves it"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "the deck is only fed by a REVEALED card, and nothing was revealed"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        0,
        "「ライブ終了時まで、ブレードを得る」 is gated on 「これにより公開したカード」 — \
         with no Aqours card in hand there is no revealed card, so no blade"
    );
}
