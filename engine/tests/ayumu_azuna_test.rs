/// Tests for PL!N-bp3-001-R+ (上原歩夢 / A.ZU.NA) — LiveStart energy-under-member blade gain
///
/// Ab#0 (ライブ開始時): 自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい。
///   そうした場合、カードを1枚引き、ライブ終了時まで、自分のステージにいるメンバーは
///   {{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。
///
/// Parsed as conditional_on_optional:
///   optional_action: place_energy_under_member(1)
///   conditional_action: sequential[ draw_card(1), gain_resource(blade, 2, all, live_end) ]
///
/// Q158: Blade +2 applies to ALL members on stage, not just this member
/// Q157: Wait/weighed energy can be placed under the member
/// Q184: Energy under member does NOT count toward energy count
//=====================================================================

mod helpers;
use helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass(); assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn seed_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
}

#[test]
fn azuna_ability_parsed() {
    let db = load_real_database();
    let ayumu = db.get_card_by_no("PL!N-bp3-001-R\u{ff0b}")
        .expect("Ayumu A.ZU.NA should exist");
    let ab0 = &ayumu.abilities[0];

    assert_eq!(ab0.triggers.as_deref(), Some("ライブ開始時"));

    let effect = ab0.effect.as_ref().expect("Effect should exist");
    eprintln!("[PARSER]  effect.action = '{}'", effect.action);
    eprintln!("[PARSER]  optional_action: {:?}", effect.optional_action.as_ref().map(|a| &a.action));
    eprintln!("[PARSER]  conditional_action: {:?}", effect.conditional_action.as_ref().map(|a| &a.action));
    if let Some(ref cond) = effect.conditional_action {
        eprintln!("[PARSER]  cond.actions count = {:?}", cond.actions.as_ref().map(|a| a.len()));
    }
    assert_eq!(effect.action, "sequential", "Expected sequential (parser no longer outputs conditional_on_optional)");
    assert_eq!(effect.conditional, Some(true), "Should have conditional=true");
    assert!(effect.actions.is_some(), "Should have actions");

    if let Some(ref actions) = effect.actions {
        assert!(actions.len() >= 2, "Should have at least 2 actions");
        assert_eq!(actions[0].action, "place_energy_under_member");
        assert_eq!(actions[0].optional, Some(true));
        assert_eq!(actions[0].energy_count, Some(1));

        // Second action should be inner sequential with [draw_card, do_nothing(gain_resource)]
        let inner_seq = &actions[1];
        assert_eq!(inner_seq.action, "sequential");
        if let Some(ref inner_actions) = inner_seq.actions {
            assert!(inner_actions.len() >= 2, "Inner sequential should have at least 2 actions");
            assert!(inner_actions[0].action == "draw_card" || inner_actions[0].action == "draw");
            assert_eq!(inner_actions[0].count, Some(1));

            // Find gain_resource in inner actions (may be at index 2 after do_nothing)
            let blade_action = inner_actions.iter().find(|a| a.action == "gain_resource");
            assert!(blade_action.is_some(), "Should have gain_resource action");
            let blade = blade_action.unwrap();
            assert_eq!(blade.resource.as_deref(), Some("blade"));
            assert_eq!(blade.count, Some(2));
            assert_eq!(blade.card_type.as_deref(), Some("member_card"));
        }
    }
}

/// Q158: All members on stage gain +2 blade when energy is placed
#[test]
fn azuna_q158_blade_all_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ayumu = game.id("PL!N-bp3-001-R\u{ff0b}");
    let member_left = game.id("PL!-sd1-010-SD");
    let member_right = game.id("PL!-sd1-013-SD");
    let filler_live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [member_left, ayumu, member_right];
    game.state.player1.hand.cards.push(filler_live);
    game.give_energy(3);
    seed_deck(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);

    if game.has_pending_choice() {
        game.select_option(1);
    }

    assert_eq!(game.state.get_blade_modifier(member_left), 2,
        "Left member +2 blade (Q158)");
    assert_eq!(game.state.get_blade_modifier(ayumu), 2,
        "Center member +2 blade (Q158)");
    assert_eq!(game.state.get_blade_modifier(member_right), 2,
        "Right member +2 blade (Q158)");
}

#[test]
fn azuna_q158_blade_single_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ayumu = game.id("PL!N-bp3-001-R\u{ff0b}");
    let filler_live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [-1, ayumu, -1];
    game.state.player1.hand.cards.push(filler_live);
    game.give_energy(3);
    seed_deck(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);

    if game.has_pending_choice() {
        game.select_option(1);
    }

    assert_eq!(game.state.get_blade_modifier(ayumu), 2);
}

/// select_option(0) = skip → no place_energy → no conditional (blade stays 0)
#[test]
fn azuna_q158_blade_not_gained_if_energy_not_placed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ayumu = game.id("PL!N-bp3-001-R\u{ff0b}");
    let filler_live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [-1, ayumu, -1];
    game.state.player1.hand.cards.push(filler_live);
    game.give_energy(3);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);

    if game.has_pending_choice() {
        game.select_option(0);
    }

    assert_eq!(game.state.get_blade_modifier(ayumu), 0);
}

/// Q157: Wait energy CAN be placed (any energy state works)
#[test]
fn azuna_q157_energy_under_member_uses_any_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ayumu = game.id("PL!N-bp3-001-R\u{ff0b}");
    let filler_live = game.id("PL!-sd1-019-SD");
    let energy_id = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [-1, ayumu, -1];
    game.state.player1.hand.cards.push(filler_live);
    seed_deck(&mut game);
    game.give_energy(1);

    advance_to_live_card_set_p1(&mut game);

    // Add wait energy (doesn't increment active_energy_count)
    game.state.player1.energy_zone.cards.push(energy_id);

    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);

    if game.has_pending_choice() {
        game.select_option(1);
    }

    assert_eq!(game.state.get_blade_modifier(ayumu), 2,
        "Q157: Blade works with any energy type");
}

/// Q184: Energy zone decreases by 1 when energy is placed under member
#[test]
fn azuna_q184_energy_under_member_not_counted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ayumu = game.id("PL!N-bp3-001-R\u{ff0b}");
    let filler_live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [-1, ayumu, -1];
    game.state.player1.hand.cards.push(filler_live);
    game.give_energy(3);
    seed_deck(&mut game);

    let energy_before = game.state.player1.energy_zone.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);

    if game.has_pending_choice() {
        game.select_option(1);
    }

    assert_eq!(game.state.player1.energy_zone.cards.len(), energy_before - 1,
        "Q184: 1 energy removed from zone");
}
