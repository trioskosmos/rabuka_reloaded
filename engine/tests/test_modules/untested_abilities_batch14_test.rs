/// Untested-abilities batch 14 — depth=none gaps:
/// - PL!S-bp7-014-N (常時): heart02 while own energy > opponent's
/// - PL!SP-bp7-020-N (常時): +2 blades while own energy > opponent's
/// - PL!S-sd1-022-SD (ライブ開始時): all Aqours members on stage gain a blade
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const AQOURS_MEMBER: &str = "PL!S-sd1-001-SD";

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some(trig))
            .unwrap_or_else(|| panic!("card {} lacks a '{trig}' ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

fn give_opp_energy(game: &mut TestGame, count: usize) {
    for _ in 0..count {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.add_active(count as u8);
}

// ====================================================================
// PL!S-bp7-014-N (常時):
// 「相手のエネルギーが自分より多いかぎり、{{heart_02.png|heart02}}を得る。」
// (opponent's energy strictly greater than mine)
// ====================================================================

#[test]
fn bp7014_constant_heart_while_opp_energy_ahead() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp7-014-N");
    game.state.player1.stage.stage[0] = me;
    game.give_energy(1);
    give_opp_energy(&mut game, 3);

    game.state.recalculate_constants();

    const H02: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart02;
    assert!(
        game.state.mods.get_heart_modifier(me, H02) > 0,
        "opponent energy ahead -> heart02 granted"
    );
}

#[test]
fn bp7014_constant_heart_lost_when_tied() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp7-014-N");
    game.state.player1.stage.stage[0] = me;
    game.give_energy(2);
    give_opp_energy(&mut game, 2);

    game.state.recalculate_constants();

    const H02: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart02;
    assert_eq!(
        game.state.mods.get_heart_modifier(me, H02),
        0,
        "tied energy -> no heart02"
    );
}

// ====================================================================
// PL!SP-bp7-020-N (常時):
// 「自分のエネルギーが相手より多いかぎり、{{blade}}{{blade}}を得る。」
// (mirror image of bp7-014-N: here SELF must be ahead)
// ====================================================================

#[test]
fn bp7020_constant_blades_while_energy_ahead() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp7-020-N");
    game.state.player1.stage.stage[0] = me;
    game.give_energy(3);
    give_opp_energy(&mut game, 1);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        2,
        "energy ahead -> +2 blades"
    );
}

#[test]
fn bp7020_constant_blades_off_when_behind() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp7-020-N");
    game.state.player1.stage.stage[0] = me;
    game.give_energy(1);
    give_opp_energy(&mut game, 3);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "energy behind -> no blades"
    );
}

// ====================================================================
// PL!S-sd1-022-SD (ライブ開始時):
// 「ライブ終了時まで、自分のステージにいる『Aqours』のメンバーはブレードを得る。」
// ====================================================================

#[test]
fn sd1022_live_start_grants_blade_to_all_aqours_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-sd1-022-SD");
    game.state.player1.live_card_zone.cards.push(live);

    let a1 = game.id(AQOURS_MEMBER); // Aqours
    let a2 = game.new_id("PL!S-sd1-002-SD"); // Aqours, second copy
    let non_aqours = game.id("PL!HS-bp5-004-R"); // スリーズブーケ — must NOT gain
    game.state.player1.stage.stage[0] = a1;
    game.state.player1.stage.stage[1] = a2;
    game.state.player1.stage.stage[2] = non_aqours;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert!(
        game.state.mods.get_blade_modifier(a1) >= 1,
        "Aqours member 1 gains a blade"
    );
    assert!(
        game.state.mods.get_blade_modifier(a2) >= 1,
        "Aqours member 2 gains a blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(non_aqours),
        0,
        "non-Aqours member must not gain"
    );
}
