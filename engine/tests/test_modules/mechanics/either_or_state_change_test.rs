/// Either/or state changes — parser now splits 「AかBをアクティブにする」 into
/// a player choice and 「メンバーと、エネルギーを…」 into sequential steps,
/// replacing the old effect-text sniffing that silently resolved BOTH sides
/// of an either/or wording.
///
/// Cards covered:
///   - PL!N-bp4-008-R エマ・ヴェルデ ab#0 起動: 手札1枚控え室：
///     エネルギー1枚か『虹ヶ咲』のメンバー1人をアクティブにする。
///   - PL!N-pb1-008-R エマ・ヴェルデ ab#1 登場:
///     ステージのメンバー1人か、エネルギーを2枚アクティブにする。
///   - PL!SP-bp5-003-R＋ 嵐千砂都 ab#1 ライブ開始時 センター:
///     すべての『Liella!』のメンバーと、すべてのエネルギーをアクティブにする。(AND)
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::ability::util::orientation_matches_state;

/// Active means no wait modifier OR an explicit "active" modifier.
fn is_active(game: &TestGame, cid: i16) -> bool {
    orientation_matches_state(game.state.mods.get_orientation_modifier(cid), "active")
}

fn is_waited(game: &TestGame, cid: i16) -> bool {
    game.state.mods.get_orientation_modifier(cid) == Some("wait")
}

const FILLER: &str = "PL!-sd1-010-SD";

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

// ====================================================================
// PL!N-bp4-008-R — choosing the MEMBER side must not touch energy.
// ====================================================================
#[test]
fn emma_bp4008_member_side_leaves_energy_alone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let emma = game.id("PL!N-bp4-008-R");
    let nijigasaki = game.id("PL!N-bp3-006-R"); // 近江彼方, 虹ヶ咲

    // Waited 虹ヶ咲 member on the left; emma center.
    game.state.player1.stage.stage[0] = nijigasaki;
    game.state.mods.add_orientation_modifier(nijigasaki, "wait");
    game.state.player1.stage.stage[1] = emma;
    game.add_to_hand(game.id(FILLER));
    game.give_energy(3);
    let energy_before = game.state.player1.energy_zone.active_count();

    game.activate_ability(emma);

    // Cost: discard 1 hand card.
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    // Either/or choice: [energy, member] per parse order → member = index 1.
    assert!(
        game.has_pending_choice(),
        "either/or wording must ask the player which side to resolve"
    );
    game.select_option(1);

    assert!(
        is_active(&game, nijigasaki),
        "chosen side: the waited 虹ヶ咲 member is activated"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before,
        "the OTHER side (energy) must not resolve — 「AかB」 never both"
    );
}

// ====================================================================
// PL!N-bp4-008-R — choosing the ENERGY side leaves the member waited.
// ====================================================================
#[test]
fn emma_bp4008_energy_side_leaves_member_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let emma = game.id("PL!N-bp4-008-R");
    let nijigasaki = game.id("PL!N-bp3-006-R");

    game.state.player1.stage.stage[0] = nijigasaki;
    game.state.mods.add_orientation_modifier(nijigasaki, "wait");
    game.state.player1.stage.stage[1] = emma;
    game.add_to_hand(game.id(FILLER));
    game.give_energy(3);
    let energy_before = game.state.player1.energy_zone.active_count();

    game.activate_ability(emma);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }
    // Energy option = index 0.
    game.select_option(0);

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before + 1,
        "chosen side: exactly 1 energy activated"
    );
    assert!(
        is_waited(&game, nijigasaki),
        "the OTHER side (member) must stay waited"
    );
}

// ====================================================================
// PL!N-pb1-008-R 登場 — member-first ordering splits correctly too.
// ====================================================================
#[test]
fn emma_pb1008_debut_choice_picks_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let emma = game.id("PL!N-pb1-008-R");
    let waited = game.id("PL!N-bp3-006-R");

    game.state.player1.stage.stage[0] = waited;
    game.state.mods.add_orientation_modifier(waited, "wait");
    game.state.player1.stage.stage[1] = emma;
    game.give_energy(4);
    let energy_before = game.state.player1.energy_zone.active_count();

    trigger_auto(&mut game, emma, AbilityTrigger::Debut, "登場");

    assert!(
        game.has_pending_choice(),
        "debut either/or must present the choice"
    );
    game.select_option(0); // member side

    assert!(
        is_active(&game, waited),
        "member side chosen → waited member activated"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before,
        "energy side untouched"
    );
}

// ====================================================================
// PL!SP-bp5-003-R＋ ライブ開始時 センター — AND semantics: ALL Liella!
// members AND ALL energy become active.
// ====================================================================
#[test]
fn chisato_bp5003_live_start_activates_all_members_and_energy() {
    use crate::helpers::put_on_deck_top;
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-bp5-003-R＋");
    let liella_waited = game.id("PL!SP-bp2-007-R"); // 米女メイ Liella!
    let non_liella = game.id("PL!N-bp3-006-R"); // 虹ヶ咲 — must NOT be touched

    game.state.player1.stage.stage[0] = liella_waited;
    game.state.mods.add_orientation_modifier(liella_waited, "wait");
    game.state.player1.stage.stage[2] = non_liella;
    game.state.mods.add_orientation_modifier(non_liella, "wait");
    game.state.player1.stage.stage[1] = chisato;

    // 5 energy: 3 active, 2 wait.
    game.give_energy(5);
    game.state.player1.energy_zone.set_active_count(3);

    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    put_on_deck_top(&mut game, 0, filler);

    trigger_auto(
        &mut game,
        chisato,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );

    assert!(
        is_active(&game, liella_waited),
        "AND wording: every Liella! member activates"
    );
    assert!(
        is_waited(&game, non_liella),
        "non-Liella! members are not targeted"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        5,
        "AND wording: ALL energy becomes active (3+2)"
    );
}
