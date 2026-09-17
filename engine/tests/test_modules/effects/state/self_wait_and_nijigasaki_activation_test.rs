use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::game_setup::{self, ActionType};

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

fn use_ability_offers(game: &TestGame, cid: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .into_iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
        })
        .count()
}

fn is_active(game: &TestGame, cid: i16) -> bool {
    rabuka_engine::ability::util::orientation_matches_state(
        game.state.mods.get_orientation_modifier(cid),
        "active",
    )
}

fn is_waited(game: &TestGame, cid: i16) -> bool {
    game.state.mods.get_orientation_modifier(cid) == Some("wait")
}

#[test]
fn kanata_n_bp3_006_debut_active_self_becomes_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanata = game.id("PL!N-bp3-006-R");
    game.state.player1.stage.stage[1] = kanata;

    assert_ne!(
        game.state.mods.get_orientation_modifier(kanata),
        Some("wait"),
        "freshly placed member starts active"
    );
    trigger_auto(&mut game, kanata, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.mods.get_orientation_modifier(kanata),
        Some("wait"),
        "debut resolves with self-wait"
    );
}

#[test]
fn wondermates_n_sd2_025_live_start_waited_nijigasaki_activates_and_retains_ability_offer() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanata = game.id("PL!N-bp1-006-R＋"); // 虹ヶ咲, 起動
    let wondermates = game.id("PL!N-sd2-025-P");
    let outsider = game.id(FILLER); // μ's — not a legal target

    // Kanata waits (as if she used a self-wait effect earlier).
    game.state.player1.stage.stage = [kanata, wondermates, outsider];
    game.state.mods.add_orientation_modifier(kanata, "wait");
    game.state.mods.add_orientation_modifier(outsider, "wait");

    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    trigger_auto(
        &mut game,
        wondermates,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        is_active(&game, kanata),
        "ワンダーメイツ activates the waited 虹ヶ咲 member"
    );
    assert!(
        is_waited(&game, outsider),
        "non-虹ヶ咲 members are not touched"
    );
    assert!(
        use_ability_offers(&game, kanata) >= 1,
        "her 起動 remains usable this turn"
    );
}
