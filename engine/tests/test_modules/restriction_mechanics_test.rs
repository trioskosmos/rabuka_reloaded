/// Restriction & aura mechanics — mined from ability texts, zero prior test
/// coverage:
///   - LL-bp2-001-R＋ 常時: cannot be baton-touched out at all
///   - PL!HS-bp6-006-R＋ 常時: baton-touchable ONLY by みらくらぱーく！ members
///   - PL!-pb1-009-R 登場: this turn, NO member may be activated by effects
///   - PL!S-bp7-009-R 常時: front-area opponent (cost≤4) loses 1 blade
///   - PL!HS-pb1-014-R 常時: front opponent cost > own → heart01
///   - PL!S-bp2-022-L ライブ成功時: deck refreshed this turn → score +2
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // cost 4, blade 1

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| {
            a.triggers
                .as_deref()
                .is_some_and(|t| t.contains(trigger_str))
        })
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
// LL-bp2-001-R＋ — absolute baton-touch protection.
// ====================================================================
#[test]
fn ll_bp2001_cannot_be_baton_touched_out() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let protected = game.id("LL-bp2-001-R＋"); // cost 20
    let attacker = game.id("PL!-pb1-021-PR"); // cost 5 — would normally baton for free

    game.add_to_stage(MemberArea::Center, protected);
    game.state.mods.add_orientation_modifier(protected, "wait");
    game.add_to_hand(attacker);
    game.give_energy(6);

    // Baton touch onto her area must be rejected.
    let res = game.try_play_to_stage(attacker, MemberArea::Center);
    assert!(
        res.is_err(),
        "cannot_baton_touch protection must block the play"
    );
    assert!(
        game.state.player1.waitroom.cards.is_empty(),
        "protected member was not sent to the waitroom"
    );
}

// ====================================================================
// PL!HS-bp6-006-R＋ — conditional protection: only みらくらぱーく！ partners.
// ====================================================================
#[test]
fn himena_bp6006_baton_only_with_murasakipark() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himena = game.id("PL!HS-bp6-006-R＋"); // cost 20, みらくらぱーく！
    let outsider = game.id("PL!-pb1-021-PR"); // cost 5, NOT みらくらぱーく！

    game.add_to_stage(MemberArea::LeftSide, himena);
    game.state.mods.add_orientation_modifier(himena, "wait");
    game.add_to_hand(outsider);
    game.give_energy(6);

    let res = game.try_play_to_stage(outsider, MemberArea::LeftSide);
    assert!(
        res.is_err(),
        "non-みらくらぱーく！ baton partner must be blocked"
    );
}

// ====================================================================
// PL!-pb1-009-R 矢澤にこ — suppression aura: no effect-based activations.
// ====================================================================
#[test]
fn niko_pb1009_suppresses_effect_activations_for_the_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-009-R");
    // bp3-005's 登場 activates ALL own stage members — a pure effect activation.
    let mass_activator = game.id("PL!-bp3-005-R");
    let victim = game.new_id(FILLER);

    game.state.player1.stage.stage[0] = victim;
    game.state.mods.add_orientation_modifier(victim, "wait");
    game.state.player1.stage.stage[1] = nico;
    game.state.player1.stage.stage[2] = mass_activator;

    trigger_auto(
        &mut game,
        nico,
        AbilityTrigger::Debut,
        "登場",
    );

    trigger_auto(
        &mut game,
        mass_activator,
        AbilityTrigger::Debut,
        "登場",
    );
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.mods.get_orientation_modifier(victim) == Some("wait"),
        "効果によってはアクティブにならない — effect activation must be blocked"
    );
}

// ====================================================================
// PL!S-bp7-009-R ルビィ — front opponent cost≤4 loses one blade.
// Front mirroring: my RIGHT faces OPPONENT'S LEFT.
// ====================================================================
#[test]
fn ruby_bpb7009_front_blade_loss_respects_mirror_and_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp7-009-R"); // cost 2
    let cheap = game.new_id(FILLER); // cost 4, blade 1
    let pricey = game.id(BIG_RUBY_S); // cost 15, blade 5

    // Ruby on MY right → front is OPPONENT'S LEFT slot.
    game.state.player1.stage.stage[2] = ruby;
    game.state.player2.stage.stage = [cheap, -1, pricey];
    let _ = pricey;

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(cheap),
        -1,
        "front opponent with cost ≤4 loses 1 blade"
    );
    assert_eq!(game.state.mods.get_blade_modifier(ruby), 0);

    // Move ruby to LEFT → front becomes opponent's RIGHT (pricey, cost 15).
    game.state.player1.stage.stage[2] = -1;
    game.state.player1.stage.stage[0] = ruby;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(pricey),
        0,
        "cost-15 opponent exceeds コスト4以下 → no loss"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(cheap),
        0,
        "no longer in front → previous loss removed"
    );
}
const BIG_RUBY_S: &str = "PL!S-bp5-009-R";

// ====================================================================
// PL!S-bp2-022-L 未熟DREAMER — refresh-happened score condition.
// ====================================================================
#[test]
fn mijuku_dreamer_refresh_condition_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp2-022-L");

    game.state.player1.live_card_zone.cards.push(live);

    // No refresh this turn → no bonus.
    game.state.trigger_auto_ability(
        format!("{}_refresh", live),
        AbilityTrigger::LiveSuccess,
        game.state.player1.id.clone(),
        Some("PL!S-bp2-022-L".to_string()),
        Some(live),
        None,
        None,
    );
    game.state.activating_card = Some(live);
    let pid = game.state.player1.id.clone();
    game.state.process_pending_auto_abilities(&pid);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "deck did not refresh → no +2"
    );

    // Refresh happened this turn → +2.
    game.state.player1.deck_refreshed_this_turn = true;
    game.state.trigger_auto_ability(
        format!("{}_refresh2", live),
        AbilityTrigger::LiveSuccess,
        game.state.player1.id.clone(),
        Some("PL!S-bp2-022-L".to_string()),
        Some(live),
        None,
        None,
    );
    game.state.activating_card = Some(live);
    game.state.process_pending_auto_abilities(&pid);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.mods.get_score_modifier(live),
        2,
        "リフレッシュしていた場合 → スコア+2"
    );
}

// ====================================================================
// HS-bp6-006-R＋ ALLOWED side: a みらくらぱーく！ partner CAN baton her out.
// ====================================================================
#[test]
fn himena_bp6006_murasakipark_partner_baton_succeeds() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himena = game.id("PL!HS-bp6-006-R＋"); // cost 20
    let partner = game.id("PL!HS-PR-018-PR"); // 大沢瑠璃乃 みらくらぱーく！ cost 4

    game.add_to_stage(MemberArea::LeftSide, himena);
    game.state.mods.add_orientation_modifier(himena, "wait");
    game.add_to_hand(partner);
    game.give_energy(0); // baton covers 20−4=16 > 0 → needs energy? no: clamp, pay 0

    let res = game.try_play_to_stage(partner, MemberArea::LeftSide);
    assert!(res.is_ok(), "allowed partner group must pass the restriction");
    assert!(
        !game.state.player1.stage.stage.contains(&himena),
        "protected member replaced by allowed partner"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&himena),
        "baton-touched member goes to the waitroom"
    );
    assert!(
        game.state.player1.stage.stage.contains(&partner),
        "partner now occupies the area"
    );
}

// ====================================================================
// PL!SP-bp4-004-R＋ すみれ — DOUBLE baton: two occupants removed, combined
// cost subtracted (Q26: no refunds past zero).
// ====================================================================
#[test]
fn sumire_bpb4004_double_baton_removes_both_and_clamps_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp4-004-R＋"); // cost 22 — double baton constant
    let big_a = game.id("PL!S-bp5-009-R"); // cost 15
    let big_b = game.id("PL!HS-bp6-006-R＋"); // cost 20

    // Two occupied areas with heavy members; third area empty.
    game.state.player1.stage.stage = [big_a, big_b, -1];
    game.state.mods.add_orientation_modifier(big_a, "wait");
    game.state.mods.add_orientation_modifier(big_b, "wait");
    game.add_to_hand(sumire);
    game.give_energy(3);

    // Combined occupant cost 35 ≥ her 22 → pair_cost clamps to 0.
    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let double_plays: Vec<_> = actions
        .iter()
        .filter(|a| {
            a.action_type == rabuka_engine::game_setup::ActionType::PlayMemberToStage
                && a.parameters
                    .as_ref()
                    .and_then(|p| p.card_id)
                    .is_some_and(|cid| cid == sumire)
        })
        .collect();
    assert!(
        !double_plays.is_empty(),
        "double-baton play must be offered when two eligible occupants exist"
    );

    let res = game.try_play_to_stage(sumire, MemberArea::Center);
    assert!(res.is_ok(), "double baton resolves");
    assert!(
        !game.state.player1.stage.stage.contains(&big_a)
            && !game.state.player1.stage.stage.contains(&big_b),
        "both occupants removed"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&big_a)
            && game.state.player1.waitroom.cards.contains(&big_b),
        "both batoned members in the waitroom"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        3,
        "combined cost 35 ≥ 22 → pair cost clamps to 0 (Q26: no refunds)"
    );
}
