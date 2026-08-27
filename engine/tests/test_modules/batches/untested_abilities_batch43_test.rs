/// Untested-abilities batch 43 — center-area grants & score-gated retrieval.
///
/// - PL!-bp4-011-N (ライブ開始時, opt. self-wait): CENTER-area 『μ's』 members
///   gain +2 blades until live end.
/// - PL!-bp4-017-N twin: same gate -> +1 blade.
/// - PL!-bp6-013-N (登場): own success-zone score total ≥6 -> retrieve one
///   『μ's』 live card from the waitroom.
use crate::helpers::*;
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

// ====================================================================
// PL!-bp4-011-N — center-area μ's +2 blades behind self-wait cost
// ====================================================================

#[test]
fn bp4011_wait_self_center_mus_member_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp4-011-N");
    // The holder sits LEFT; a μ's mate holds the CENTER.
    game.state.player1.stage.stage[0] = me;
    let center = game.new_id("PL!-sd1-001-SD"); // 高坂穂乃果, μ's
    game.state.player1.stage.stage[1] = center;
    let left_mu = game.new_id("PL!-sd1-007-SD"); // 東條希, μ's
    game.state.player1.stage.stage[2] = left_mu;

    fire_live_start(&mut game, me);

    // Optional self-wait cost gate is always presented; answer "Yes: wait self".
    assert!(
        game.has_pending_choice(),
        "pay-optional-cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget pay_optional_cost gate"
    );
    game.select_option(1); // Yes: wait self

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        Some("wait"),
        "cost waits this member"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(center),
        2,
        "center-area μ's member gains +2 blades"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(left_mu),
        0,
        "non-center member gains nothing"
    );
}

// ====================================================================
// PL!-bp4-017-N twin — +1 blade variant
// ====================================================================

#[test]
fn bp4017_twin_center_mus_member_one_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp4-017-N");
    game.state.player1.stage.stage[0] = me;
    let center = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage[1] = center;

    fire_live_start(&mut game, me);
    assert!(
        game.has_pending_choice(),
        "pay-optional-cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget pay_optional_cost gate"
    );
    game.select_option(1);

    assert_eq!(
        game.state.mods.get_blade_modifier(center),
        1,
        "twin grants +1 blade to center-area μ's member"
    );
}

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

// ====================================================================
// PL!-bp6-013-N — success-zone score sum gates μ's live retrieval
// ====================================================================

fn bp6013_setup(game: &mut TestGame) -> (i16, i16) {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id("PL!-bp6-013-N");
    game.state.player1.stage.stage[0] = me;
    // A μ's live card waits in the waitroom for retrieval.
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);
    (me, mus_live)
}

#[test]
fn bp6013_success_score_six_retrieves_mus_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, mus_live) = bp6013_setup(&mut game);

    // One success-zone live with score 9 -> total >= 6.
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
fn bp6013_empty_success_zone_no_retrieval() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, mus_live) = bp6013_setup(&mut game);

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
