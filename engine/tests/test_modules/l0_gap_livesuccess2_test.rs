/// L0 gap coverage: additional LiveSuccess abilities — score modifiers,
/// per-unit scoring, and card retrieval from revealed cards.
use crate::helpers::*;

/// PL!N-bp3-031-L: ライブ成功時 自分のステージにいるウェイト状態の
/// メンバー1人につき、このカードのスコアを＋１する。
#[test]
fn bp3_031_per_waited_member_score_plus1() {
    use rabuka_engine::core::types::AbilityTrigger;

    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!N-bp3-031-L");
    game.state.player1.live_card_zone.cards.push(live);
    // Two waited members on stage + one active member (must not count)
    let m1 = game.new_id("PL!N-sd1-002-SD");
    let m2 = game.new_id("PL!N-sd1-003-SD");
    let active = game.new_id("PL!N-sd1-001-SD");
    game.state.player1.stage.stage = [m1, m2, active];
    game.state.mods.add_orientation_modifier(m1, "wait");
    game.state.mods.add_orientation_modifier(m2, "wait");

    // Fire the LiveSuccess trigger through the real ability pipeline.
    let ability_id = {
        let card = game.db.get_card(live).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
            .expect("card lacks ライブ成功時 ability");
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(game.db.get_card(live).unwrap().card_no.to_string()),
        Some(live),
        None,
        None,
    );
    game.state.activating_card = Some(live);
    game.state.process_pending_auto_abilities(&pid);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let score = game.state.mods.get_score_modifier(live);
    assert_eq!(
        score, 2,
        "two waited members → exactly +2 (active member must not count)"
    );
}

/// PL!SP-bp4-003-R: Constant center → +2 blade.
#[test]
fn sp_bp4_003_center_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m = game.id("PL!SP-bp4-003-R");
    game.state.player1.stage.stage = [-1, m, -1];
    game.state.recalculate_constants();
    assert!(
        game.state.mods.get_blade_modifier(m) >= 2,
        "center constant grants blade"
    );
}
