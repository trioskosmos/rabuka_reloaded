/// Tests for `LL-bp7-001-R＋` 国木田花丸&優木せつ菜&嵐千砂都
///
/// ab#0 (常時 プレイ時): このカードのプレイに際し、自分の手札から「国木田花丸」と「優木せつ菜」と
///   「嵐千砂都」のメンバーカードをそれぞれ1枚ずつ控え室に置いてもよい。そうしたとき、
///   このカードのコストは10になる。(base 15 → optional 10 via hand discard)
/// ab#1 (登場): 自分の控え室からライブカードを1枚手札に加える。
/// ab#2 (ライブ成功時): 自分の控え室からメンバーカードを1枚手札に加える。
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;

const HANAMARU: &str = "PL!S-bp2-016-N";
const SETSUNA: &str = "PL!N-PR-009-PR";
const CHISATO: &str = "PL!SP-pb1-014-PR";
const TRIPLE: &str = "LL-bp7-001-R＋";
const LIVE_CARD: &str = "PL!-sd1-020-SD";

fn answer_play_choice(game: &mut TestGame, accept: bool) -> bool {
    if !game.has_pending_choice() {
        return false;
    }
    if let Choice::SelectTarget { target, options, .. } = game.get_pending_choice() {
        if target == "play_time_cost_reduction" {
            // options: [No/15, Yes/10]
            let idx = if accept { 1 } else { 0 };
            if options.as_ref().map(|o| o.len() > idx).unwrap_or(false) {
                game.select_choice_option(idx);
                return true;
            }
        }
    }
    false
}

// ====================================================================
// ab#0: passive cost must NOT be set by discard state
// ====================================================================

#[test]
fn triple_passive_cost_not_set_by_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.waitroom.cards.push(hanamaru);
    game.state.player1.waitroom.cards.push(setsuna);
    game.state.player1.waitroom.cards.push(chisato);
    game.state.recalculate_constants();
    // must NOT be set — need hand discard at play time
    assert_eq!(
        game.state.mods.get_cost_modifier(triple),
        0,
        "passive cost must not be set from waitroom alone"
    );
    assert_eq!(
        game.state.mods.get_cost_modifier_set(triple),
        None,
        "no set-cost modifier without play-time choice"
    );
}

#[test]
fn triple_passive_cost_not_set_with_hand_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato);
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_cost_modifier(triple), 0);
    assert_eq!(game.state.mods.get_cost_modifier_set(triple), None);
}

// ====================================================================
// ab#0 gameplay: choice paths
// ====================================================================

#[test]
fn triple_gameplay_accept_cost10_discards_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato);
    game.give_energy(10);

    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(
        answer_play_choice(&mut game, true),
        "must offer play-time choice when hand has 3 required members"
    );

    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(remaining, 0, "cost 10 paid, 0 remain (got {remaining})");
    // triple is on stage, 3 hand cards moved to waitroom
    assert!(
        game.state.player1.waitroom.cards.contains(&hanamaru),
        "hanamaru discarded"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&setsuna),
        "setsuna discarded"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&chisato),
        "chisato discarded"
    );
    assert!(
        game.state.player1.stage.stage.contains(&triple),
        "triple is on stage"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&hanamaru),
        "hand no longer contains hanamaru"
    );
}

#[test]
fn triple_gameplay_decline_pays15_keeps_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato);
    game.give_energy(15);

    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(answer_play_choice(&mut game, false));

    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(remaining, 0, "cost 15 paid");
    // hand cards stay
    assert!(game.state.player1.hand.cards.contains(&hanamaru));
    assert!(game.state.player1.hand.cards.contains(&setsuna));
    assert!(game.state.player1.hand.cards.contains(&chisato));
    assert!(game.state.player1.waitroom.cards.is_empty());
}

#[test]
fn triple_gameplay_no_hand_cards_no_choice_pays15() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    game.state.player1.hand.cards.push(triple);
    game.give_energy(15);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(
        !game.has_pending_choice(),
        "no choice when hand lacks required members"
    );
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
}

#[test]
fn triple_gameplay_waitroom_has_three_but_hand_empty_no_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.waitroom.cards.push(hanamaru);
    game.state.player1.waitroom.cards.push(setsuna);
    game.state.player1.waitroom.cards.push(chisato);
    game.give_energy(10);
    // try with only 10 energy and no hand fodder — should fail (needs 15)
    let res = game.try_play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(res.is_err(), "should fail with 10 energy and no hand fodder: {res:?}");
}

#[test]
fn triple_gameplay_energy10_with_hand_can_play_for10() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato);
    game.give_energy(10);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    // accept
    assert!(answer_play_choice(&mut game, true));
    assert!(game.state.player1.stage.stage.contains(&triple));
}

#[test]
fn triple_gameplay_incomplete_hand_no_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    // missing chisato
    game.state.player1.hand.cards.push(triple);
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(setsuna);
    game.give_energy(15);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
}

// ====================================================================
// ab#1 (登場): add 1 live card from waitroom to hand
// ====================================================================

#[test]
fn triple_debut_adds_live_card_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let live = game.id(LIVE_CARD);
    game.state.player1.waitroom.cards.push(live);
    game.state.player1.hand.cards.push(triple);
    game.give_energy(15);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "debut should add a live card from waitroom to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "the live card should leave the waitroom"
    );
}

#[test]
fn triple_debut_no_live_card_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    game.state.player1.hand.cards.push(triple);
    let member = game.id("PL!-sd1-010-SD");
    game.state.player1.waitroom.cards.push(member);
    game.give_energy(15);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(
        game.state.player1.waitroom.cards.contains(&member),
        "non-live cards in waitroom are not touched by ab#1"
    );
}

// ====================================================================
// ab#2 (ライブ成功時): add 1 member card from waitroom to hand
// ====================================================================

fn trigger_live_success(game: &mut TestGame, card_id: i16) {
    fire_trigger(
        game,
        card_id,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    game.drain_auto_ability_choices();
}

#[test]
fn triple_live_success_adds_member_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    game.state.player1.stage.stage = [-1, triple, -1];
    let member = game.id("PL!-sd1-010-SD");
    game.state.player1.waitroom.cards.push(member);
    trigger_live_success(&mut game, triple);
    assert!(
        game.state.player1.hand.cards.contains(&member),
        "live success should add a member card from waitroom to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&member),
        "the member card should leave the waitroom"
    );
}

#[test]
fn triple_live_success_ignores_live_cards_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    game.state.player1.stage.stage = [-1, triple, -1];
    let live_in_waitroom = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(live_in_waitroom);
    trigger_live_success(&mut game, triple);
    assert!(
        game.state
            .player1
            .waitroom
            .cards
            .contains(&live_in_waitroom),
        "live cards in waitroom are not touched by ab#2"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&live_in_waitroom),
        "live card must not be added to hand by ab#2"
    );
}

// ====================================================================
// Multi-name / softlock edge cases
// ====================================================================

/// The triple card itself (another copy) can be used as fodder for ONE slot
/// only, not for multiple. Using a second copy of the triple as the hanamaru
/// fodder should succeed.
#[test]
fn triple_second_copy_can_be_used_as_one_fodder() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple_play = game.id(TRIPLE);
    let triple_fodder = game.id(TRIPLE); // distinct copy
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    game.state.player1.hand.cards.push(triple_play);
    game.state.player1.hand.cards.push(triple_fodder);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato);
    game.give_energy(10);
    game.play_to_stage(triple_play, rabuka_engine::zones::MemberArea::Center);
    assert!(
        answer_play_choice(&mut game, true),
        "second triple copy should satisfy one required character"
    );
    assert!(game.state.player1.stage.stage.contains(&triple_play));
    assert!(game.state.player1.waitroom.cards.contains(&triple_fodder));
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
}

/// One card cannot satisfy two required characters. Hand: [triple to play,
/// second triple (covers all 3), setsuna]. Only 2 distinct fodder cards -> no choice.
#[test]
fn triple_one_card_cannot_cover_two_slots() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple_play = game.id(TRIPLE);
    let triple_fodder = game.id(TRIPLE);
    let setsuna = game.id(SETSUNA);
    game.state.player1.hand.cards.push(triple_play);
    game.state.player1.hand.cards.push(triple_fodder);
    game.state.player1.hand.cards.push(setsuna);
    // missing chisato as distinct card — second triple alone is not enough for both hanamaru+chisato
    game.give_energy(15);
    game.play_to_stage(triple_play, rabuka_engine::zones::MemberArea::Center);
    assert!(
        !game.has_pending_choice(),
        "single multi-name card must not count for two slots"
    );
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
}

/// Softlock avoidance: hand contains played triple + 3 fodders where fodder
/// assignment needs optimal matching. If greedy picks wrong, it would fail.
/// Hand: triple_play, triple_fodder (hanamaru+setsuna+chisato), hanamaru, chisato.
/// Greedy hanamaru->triple_fodder would leave setsuna unmatched -> must assign triple_fodder to setsuna instead.
#[test]
fn triple_optimal_assignment_with_multi_name() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple_play = game.id(TRIPLE);
    let triple_fodder = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let chisato = game.id(CHISATO);
    // hanamaru and chisato singles + triple_fodder to cover setsuna
    game.state.player1.hand.cards.push(triple_play);
    game.state.player1.hand.cards.push(triple_fodder);
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(chisato);
    game.give_energy(10);
    game.play_to_stage(triple_play, rabuka_engine::zones::MemberArea::Center);
    assert!(
        answer_play_choice(&mut game, true),
        "optimal assignment should find hanamaru->hanamaru, chisato->chisato, triple->setsuna"
    );
    assert!(game.state.player1.stage.stage.contains(&triple_play));
}

/// Playing triple does not consume itself as fodder — ensures the played card
/// is excluded from the hand check. Hand: only the triple, no fodder -> no choice.
#[test]
fn triple_played_card_not_counted_as_fodder() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    game.state.player1.hand.cards.push(triple);
    game.give_energy(15);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(!game.has_pending_choice());
    assert!(game.state.player1.stage.stage.contains(&triple));
}

/// With 10 energy, declining the alternative cost would require 15 -> insufficient.
/// The choice is still offered (both options shown), but declining correctly
/// results in a payment error. This verifies the minimum-cost path (10) is available.
#[test]
fn triple_10_energy_minimum_cost_is_10() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato);
    game.give_energy(10);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(game.has_pending_choice(), "choice must be offered with 10 energy");
    // Minimum cost path: accept 10
    assert!(answer_play_choice(&mut game, true));
    assert!(game.state.player1.stage.stage.contains(&triple));
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
}

// ====================================================================
// Combinatorial / fuzz stress tests - trying to break the implementation
// ====================================================================

/// Non-member cards with matching names must NOT count (e.g. energy/live with same name).
/// PL!SP-pb1-038-SRE is an energy card named "澁谷かのん＆嵐 千砂都" – must not count as chisato.
#[test]
fn triple_non_member_dual_name_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    // Energy dual-name, not a member – use PL!SP-pb1-038-SRE
    let chisato_energy = game.id("PL!SP-pb1-038-SRE"); // 澁谷かのん＆嵐 千砂都 (energy)
    game.state.player1.hand.cards.push(triple);
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato_energy);
    game.give_energy(15);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(
        !game.has_pending_choice(),
        "energy card with chisato name must not count as member fodder"
    );
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
}

#[test]
fn triple_with_extra_unrelated_cards_still_offers_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    let extra1 = game.id("PL!-sd1-010-SD"); // unrelated μ's member
    let extra2 = game.id("PL!HS-bp1-005-PR"); // unrelated Hasunosora member
    game.state.player1.hand.cards.push(triple);
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato);
    game.state.player1.hand.cards.push(extra1);
    game.state.player1.hand.cards.push(extra2);
    game.give_energy(10);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(answer_play_choice(&mut game, true));
    assert!(game.state.player1.stage.stage.contains(&triple));
    // extra cards must remain in hand
    assert!(game.state.player1.hand.cards.contains(&extra1));
    assert!(game.state.player1.hand.cards.contains(&extra2));
    // exactly 3 discarded
    assert_eq!(game.state.player1.waitroom.cards.len(), 3);
}

/// Many duplicate hanamaru variants: hand has 3 different hanamaru cards + chisato
/// but only one setsuna copy duplicated as triple – must still find distinct assignment.
#[test]
fn triple_many_duplicates_still_finds_assignment() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple_play = game.id(TRIPLE);
    let triple_fodder = game.id(TRIPLE);
    let hanamaru2 = game.id("PL!S-bp2-007-R＋"); // another hanamaru variant
    let hanamaru3 = game.id("PL!S-bp3-016-N"); // yet another hanamaru
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    // Hand: triple_play, triple_fodder, hanamaru2, hanamaru3, setsuna, chisato
    // Fodder needs 3 distinct: could use hanamaru2+setsuna+chisato, ignoring extras
    game.state.player1.hand.cards.push(triple_play);
    game.state.player1.hand.cards.push(triple_fodder);
    game.state.player1.hand.cards.push(hanamaru2);
    game.state.player1.hand.cards.push(hanamaru3);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato);
    game.give_energy(10);
    game.play_to_stage(triple_play, rabuka_engine::zones::MemberArea::Center);
    assert!(answer_play_choice(&mut game, true));
    assert!(game.state.player1.stage.stage.contains(&triple_play));
    assert_eq!(game.state.player1.waitroom.cards.len(), 3);
    // triple_fodder should stay (not needed) OR be one of the three – either is valid, but distinctness holds
    let hand_len = game.state.player1.hand.cards.len();
    assert_eq!(hand_len, 2, "6 hand -1 played -3 discarded =2 remain, got {hand_len}");
}

/// Exhaustive-ish: try every hanamaru/setsuna/chisato variant combination for offer correctness.
/// This will catch any variant where name contains extra spaces or is mis-detected.
#[test]
fn triple_combinatorial_variant_fuzz() {
    let db = load_real_database();
    let hanamaru_variants = [
        "PL!S-bp2-016-N",
        "PL!S-bp2-007-R＋",
        "PL!S-bp3-016-N",
        "PL!S-bp7-016-N",
    ];
    let setsuna_variants = [
        "PL!N-PR-009-PR",
        "PL!N-bp5-007-R＋",
        "PL!N-bp7-019-N",
        "PL!N-sd1-007-SD",
    ];
    let chisato_variants = [
        "PL!SP-pb1-014-PR",
        "PL!SP-bp5-014-N",
        "PL!SP-bp7-014-N",
        "PL!SP-pb2-014-R",
    ];
    for &h in &hanamaru_variants {
        for &s in &setsuna_variants {
            for &c in &chisato_variants {
                let mut game = TestGame::new(db.clone());
                let triple = game.id(TRIPLE);
                let hanamaru = game.id(h);
                let setsuna = game.id(s);
                let chisato = game.id(c);
                game.state.player1.hand.cards.push(triple);
                game.state.player1.hand.cards.push(hanamaru);
                game.state.player1.hand.cards.push(setsuna);
                game.state.player1.hand.cards.push(chisato);
                game.give_energy(10);
                game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
                assert!(
                    game.has_pending_choice(),
                    "variant combo {h} {s} {c} should offer choice"
                );
                assert!(answer_play_choice(&mut game, true));
                assert!(
                    game.state.player1.stage.stage.contains(&triple),
                    "variant combo {h} {s} {c} should succeed with 10"
                );
            }
        }
    }
}

#[test]
fn triple_cost_cleared_after_play_and_second_play_costs_15() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple1 = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    // First play with discount
    game.state.player1.hand.cards.push(triple1);
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato);
    game.give_energy(10);
    game.play_to_stage(triple1, rabuka_engine::zones::MemberArea::Center);
    assert!(answer_play_choice(&mut game, true));
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
    // Second play: need a fresh triple in hand with no fodder left -> should cost 15, no alternative
    let triple2 = game.id(TRIPLE);
    game.state.player1.hand.cards.push(triple2);
    game.give_energy(15);
    let res = game.try_play_to_stage(triple2, rabuka_engine::zones::MemberArea::LeftSide);
    assert!(res.is_ok(), "second play should succeed at cost 15: {res:?}");
    assert!(!game.has_pending_choice(), "second play should have no alternative (fodder exhausted)");
    assert_eq!(game.state.player1.energy_zone.active_count(), 0, "15 paid for second");
    // Verify set-cost was cleared: triple2's stage entry has no lingering set modifier affecting future checks
    assert_eq!(game.state.mods.get_cost_modifier_set(triple2), None);
}

/// Trying to cheat by using stage member as fodder must not work (hand only).
#[test]
fn triple_stage_member_not_counted_as_hand_fodder() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let hanamaru_stage = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    // Hanamaru on stage, not in hand
    game.state.player1.stage.stage[0] = hanamaru_stage;
    game.state.player1.hand.cards.push(triple);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato);
    // Missing hand hanamaru – only stage has it
    game.give_energy(15);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(
        !game.has_pending_choice(),
        "stage hanamaru must not count for hand discard"
    );
}
