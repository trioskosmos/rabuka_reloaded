use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

const SUMIRE: &str = "PL!SP-bp5-004-R＋";
const SHIKI: &str = "PL!SP-bp2-008-R";
const PLACER: &str = "PL!SP-pb1-005-R";
const FILLER: &str = "PL!-sd1-010-SD";
const ENERGY: &str = "LL-E-001-SD";

fn fill_main_deck(game: &mut TestGame, n: usize) {
    let filler = game.new_id(FILLER);
    for _ in 0..n {
        game.state.player1.main_deck.cards.push(filler);
    }
}

fn shiki_swap_to(game: &mut TestGame, shiki: i16, want_area: &str) {
    game.activate_ability(shiki);
    game.drain_auto_ability_choices();
    assert!(
        game.has_pending_choice(),
        "Shiki swap should offer target areas"
    );
    let actions = game.generated_actions();
    let idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .is_some_and(|area| area == want_area || area == format!("{want_area}_side"))
        })
        .expect("swap target area not offered");
    game.select_generated(idx);
    game.drain_auto_ability_choices();
}

#[test]
fn own_effect_self_swap_draws_one_and_gains_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.new_id(SUMIRE);
    let shiki = game.new_id(SHIKI);
    game.state.player1.stage.stage = [sumire, -1, -1];
    fill_main_deck(&mut game, 10);
    game.give_energy(10);
    game.add_to_hand(shiki);
    game.try_play_to_stage(shiki, MemberArea::Center)
        .expect("shiki debut");
    scan_autos_both(&mut game);

    let hand_before = game.state.player1.hand.cards.len();
    shiki_swap_to(&mut game, shiki, "left");
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.stage.stage,
        [shiki, sumire, -1],
        "swap must have moved Sumire (Center) via our own effect"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(sumire, HeartColor::Heart02),
        1,
        "compound: own area move alone fires the Sumire auto"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "compound: Sumire auto draws 1 card"
    );
}

#[test]
fn own_effect_energy_placement_draws_one_and_gains_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.new_id(SUMIRE);
    game.state.player1.stage.stage = [sumire, -1, -1];
    fill_main_deck(&mut game, 10);
    game.state
        .player1
        .energy_deck
        .cards
        .push(game.new_id(ENERGY));
    let placer = game.new_id(PLACER);
    game.state.player1.hand.cards.push(placer);
    game.give_energy(13); // Kahori cost

    let zone_before = game.state.player1.energy_zone.cards.len();
    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(placer, MemberArea::RightSide);
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "placer debut really put 1 energy into the zone"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(sumire, HeartColor::Heart02),
        1,
        "compound: own energy placement alone fires the Sumire auto"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "compound: -played placer +drawn Sumire card = same hand"
    );
}
