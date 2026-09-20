use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

// ====================================================================
// PL!SP-bp7-026-L "Dears" (L0 gap — previously zero test coverage)
// ライブ開始時：エネルギー置き場にあるエネルギー1枚をエネルギーデッキに
// 置いてもよい：自分のステージに「葉月恋」がいる場合、カードを2枚引き、
// 手札を1枚控え室に置く。
//
// Live start: optionally return 1 energy from the energy zone to the
// energy deck; if Ren Hazuki (葉月 恋) is on your stage, draw 2 then
// discard 1 from hand.
// ====================================================================

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

fn dears_setup(game: &mut TestGame, with_ren: bool) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let live = game.id("PL!SP-bp7-026-L");
    assert_eq!(
        game.db.get_card(live).unwrap().card_no,
        "PL!SP-bp7-026-L",
        "staged the SP print of Dears, not the N print"
    );
    game.state.player1.live_card_zone.cards.push(live);
    let left = if with_ren {
        game.id("PL!SP-sd1-005-SD")
    } else {
        game.id("PL!-sd1-010-SD")
    };
    let mid = game.id("PL!-sd1-010-SD");
    let right = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [left, mid, right];
    if with_ren {
        assert!(
            game.db
                .get_card(left)
                .unwrap()
                .name
                .contains("葉月"),
            "staged member must be Ren Hazuki"
        );
    }
    live
}

fn drain_prompts(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        !game.has_pending_choice(),
        "prompts should terminate after accepting"
    );
}

#[test]
fn dears_accept_cost_with_ren_draws_two_discards_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = dears_setup(&mut game, true);
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 2);

    let deck_before = game.state.player1.energy_deck.cards.len();
    let zone_before = game.state.player1.energy_zone.cards.len();
    let hand_before = game.state.player1.hand.cards.len();
    let wait_before = game.state.player1.waitroom.cards.len();

    fire_live_start(&mut game, live);
    assert!(game.has_pending_choice(), "optional energy cost prompted");
    game.select_option(1);
    drain_prompts(&mut game);

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before + 1,
        "accepted: one energy moved from zone back to the energy deck"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before - 1,
        "zone lost exactly the moved card"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "Ren on stage: drew 2 then discarded 1 (net +1)"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        wait_before + 1,
        "the discarded hand card reached the waitroom"
    );
}

#[test]
fn dears_decline_cost_nothing_happens() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = dears_setup(&mut game, true);
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 2);

    let deck_before = game.state.player1.energy_deck.cards.len();
    let zone_before = game.state.player1.energy_zone.cards.len();
    let hand_before = game.state.player1.hand.cards.len();
    let wait_before = game.state.player1.waitroom.cards.len();

    fire_live_start(&mut game, live);
    assert!(game.has_pending_choice(), "optional energy cost prompted");
    game.select_indices(&[]);

    assert!(
        !game.has_pending_choice(),
        "declining ends the ability with no follow-up"
    );
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before,
        "declined: energy deck untouched"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "declined: energy zone untouched"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "declined: no draw"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        wait_before,
        "declined: no discard"
    );
}

#[test]
fn dears_accept_cost_without_ren_pays_but_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = dears_setup(&mut game, false);
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 2);

    let deck_before = game.state.player1.energy_deck.cards.len();
    let zone_before = game.state.player1.energy_zone.cards.len();
    let hand_before = game.state.player1.hand.cards.len();
    let wait_before = game.state.player1.waitroom.cards.len();

    fire_live_start(&mut game, live);
    assert!(game.has_pending_choice(), "optional energy cost prompted");
    game.select_option(1);
    drain_prompts(&mut game);

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before + 1,
        "cost still payable without Ren"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before - 1,
        "zone lost exactly the moved card"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no Ren on stage: no draw"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        wait_before,
        "no Ren on stage: no discard"
    );
}
