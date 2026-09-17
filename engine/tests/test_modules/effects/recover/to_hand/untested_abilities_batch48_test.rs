/// Untested-abilities batch 48 — retrievals & Liella heart-total constant.
///
/// - PL!HS-cl1-002-CL 村野さやか (登場, opt. {E}): retrieve a 『DOLLCHESTRA』
///   card from the waitroom.
/// - PL!HS-cl1-008-CL (起動): self -> waitroom, then retrieve a 『蓮ノ空』
///   card from the waitroom.
/// - PL!SP-bp5-026-L (常時): staged Liella! members' heart total >= 11 ->
///   this live card's score +1.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

fn hspb1007_setup() -> (TestGame, i16, i16) {
    let mut game = TestGame::new(load_real_database());
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id("PL!HS-pb1-007-R");
    let card = game.db.get_card(me).unwrap();
    assert_eq!(card.card_no.as_ref(), "PL!HS-pb1-007-R");
    assert_eq!(card.name.as_ref(), "セラス 柳田 リリエンフェルト");
    assert_eq!(card.cost, Some(11));
    let debut = card
        .resolved_abilities()
        .find(|ability| ability.triggers.as_deref() == Some("登場"))
        .expect("PL!HS-pb1-007-R must have a debut ability");
    assert!(debut.full_text.contains("手札を1枚控え室に置いてもよい"));
    assert!(debut.full_text.contains("自分の控え室から『蓮ノ空』のカードを1枚手札に加える"));
    let cost_card = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(me);
    game.add_to_hand(cost_card);
    game.give_energy(13);
    (game, me, cost_card)
}

#[test]
fn hspb1007_debut_pays_two_energy_and_discards_to_recover() {
    let (mut game, me, cost_card) = hspb1007_setup();
    let target = game.id("PL!HS-bp2-004-R");
    assert_eq!(game.db.get_card(target).unwrap().card_no.as_ref(), "PL!HS-bp2-004-R");
    game.add_to_discard(target);

    game.play_to_stage(me, MemberArea::Center);

    assert_eq!(game.state.player1.stage.stage[1], me);
    assert_eq!(game.state.player1.energy_zone.active_count(), 2);
    match game.get_pending_choice() {
        Choice::SelectCard { zone, count, allow_skip, .. } => {
            assert_eq!(zone, "hand");
            assert_eq!(*count, 1);
            assert!(*allow_skip);
        }
        other => panic!("expected optional discard cost, got {:?}", other),
    }
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[cost_card]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[target]);
    game.select_indices(&[0]);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
    assert_eq!(game.state.player1.energy_zone.cards.len(), 13);
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[target]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[cost_card]);
    assert_eq!(game.state.player1.stage.stage[1], me);
}

#[test]
fn hspb1007_debut_decline_preserves_energy_hand_and_waitroom() {
    let (mut game, me, cost_card) = hspb1007_setup();
    let target = game.id("PL!HS-bp2-004-R");
    game.add_to_discard(target);

    game.play_to_stage(me, MemberArea::Center);

    game.assert_select_card("hand", 1, true);
    assert_eq!(game.state.player1.energy_zone.active_count(), 2);
    game.select_indices(&[]);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.energy_zone.active_count(), 2);
    assert_eq!(game.state.player1.energy_zone.cards.len(), 13);
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[cost_card]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[target]);
    assert_eq!(game.state.player1.stage.stage[1], me);
}

#[test]
fn hspb1007_debut_only_hasunosora_members_and_lives_are_selectable() {
    for recover_live in [false, true] {
        let (mut game, me, cost_card) = hspb1007_setup();
        let wrong_member = game.new_id("PL!-sd1-010-SD");
        let wrong_live = game.id("PL!-sd1-019-SD");
        let member = game.id("PL!HS-bp2-004-R");
        let live = game.id("PL!HS-bp2-023-L");
        for card in [wrong_member, member, wrong_live, live] {
            game.add_to_discard(card);
        }

        game.play_to_stage(me, MemberArea::Center);

        game.assert_select_card("hand", 1, true);
        game.select_indices(&[0]);

        assert_eq!(game.state.player1.energy_zone.active_count(), 0);
        assert!(game.state.player1.hand.cards.is_empty());
        assert_eq!(
            game.state.player1.waitroom.cards.as_slice(),
            &[wrong_member, member, wrong_live, live, cost_card]
        );
        match game.get_pending_choice() {
            Choice::SelectCard { zone, count, filtered_indices, .. } => {
                assert_eq!(zone, "discard");
                assert_eq!(*count, 1);
                assert_eq!(filtered_indices.as_deref(), Some(&[1, 3][..]));
            }
            other => panic!("expected Hasunosora recovery choice, got {:?}", other),
        }
        let (target, remaining) = if recover_live { (live, member) } else { (member, live) };
        game.select_waitroom_card_filtered(target);

        assert!(!game.has_pending_choice());
        assert_eq!(game.state.player1.energy_zone.active_count(), 0);
        assert_eq!(game.state.player1.energy_zone.cards.len(), 13);
        assert_eq!(game.state.player1.hand.cards.as_slice(), &[target]);
        let expected = if recover_live {
            vec![wrong_member, remaining, wrong_live, cost_card]
        } else {
            vec![wrong_member, wrong_live, remaining, cost_card]
        };
        assert_eq!(game.state.player1.waitroom.cards.as_slice(), expected.as_slice());
        assert_eq!(game.state.player1.stage.stage[1], me);
    }
}

#[test]
fn hspb1007_debut_wrong_group_only_recovers_nothing_after_paying() {
    let (mut game, me, cost_card) = hspb1007_setup();
    let wrong_member = game.new_id("PL!-sd1-010-SD");
    let wrong_live = game.id("PL!-sd1-019-SD");
    game.add_to_discard(wrong_member);
    game.add_to_discard(wrong_live);

    game.play_to_stage(me, MemberArea::Center);

    game.assert_select_card("hand", 1, true);
    game.select_indices(&[0]);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
    assert_eq!(game.state.player1.energy_zone.cards.len(), 13);
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(
        game.state.player1.waitroom.cards.as_slice(),
        &[wrong_member, wrong_live, cost_card]
    );
    assert_eq!(game.state.player1.stage.stage[1], me);
}

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
// PL!HS-cl1-002-CL — optional energy -> DOLLCHESTRA retrieval
// ====================================================================

fn cl1002_setup(game: &mut TestGame) -> (i16, i16) {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id("PL!HS-cl1-002-CL");
    game.add_to_hand(me);
    game.give_energy(10);
    // A DOLLCHESTRA live card waits in the waitroom.
    let doll = game.id("PL!HS-bp2-023-L");
    game.state.player1.waitroom.cards.push(doll);
    (me, doll)
}

#[test]
fn cl1002_accept_energy_retrieves_dollchestra() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, doll) = cl1002_setup(&mut game);

    game.play_to_stage(me, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "optional energy cost prompted");
    game.select_option(1); // pay

    assert!(
        game.state.player1.hand.cards.contains(&doll),
        "DOLLCHESTRA card retrieved to hand"
    );
}

#[test]
fn cl1002_decline_stays_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, doll) = cl1002_setup(&mut game);

    game.play_to_stage(me, MemberArea::LeftSide);
    assert!(
        game.has_pending_choice(),
        "optional energy cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget (pay_optional_cost:skip)"
    );
    game.select_indices(&[]); // decline

    assert!(
        !game.state.player1.hand.cards.contains(&doll),
        "declined -> card stays in the waitroom"
    );
}

// ====================================================================
// PL!HS-cl1-008-CL — self-to-waitroom activation retrieves Hasunosora card
// ====================================================================

#[test]
fn cl1008_self_to_waitroom_retrieves_hasunosora() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!HS-cl1-008-CL");
    game.state.player1.stage.stage[1] = me;
    // A Hasunosora-series card waits.
    let hns_card = game.id("PL!HS-bp5-001-P"); // 日野下花帆, member
    game.state.player1.waitroom.cards.push(hns_card);

    game.activate_ability(me);

    // Drain the retrieval selection prompt.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.stage.stage.iter().any(|&c| c == me),
        "activation cost moved this member off stage"
    );
    assert!(
        game.state.player1.hand.cards.contains(&hns_card),
        "Hasunosora card retrieved to hand"
    );
}

// ====================================================================
// PL!SP-bp5-026-L — Liella heart-total constant gates live score +1
//
// aggregate=total on group_condition + Stage dispatches to
// sum_group_hearts_in_stage (base heart sums of matching members).
// The group filter 「Liella!」 matches via series containing
// スーパースター (card_series_matches_group in util.rs).
// ====================================================================

fn bp5026_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let live = game.id("PL!SP-bp5-026-L");
    game.state.player1.live_card_zone.cards.push(live);
    live
}

#[test]
fn bp5026_total_eleven_scores_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = bp5026_setup(&mut game);
    // Three Superstar-series members: 9 + 8 + any third member >= 0.
    // The two big ones alone give 17 >= 11.
    let big1 = game.id("PL!SP-pb2-005-R"); // hearts {02:3, 03:3, 06:3} = 9
    let big2 = game.id("PL!SP-bp4-004-P"); // hearts {02:3, 03:3, 06:2} = 8
    let small = game.new_id("PL!-sd1-010-SD"); // μ's — not Liella!, excluded
    game.state.player1.stage.stage[0] = big1;
    game.state.player1.stage.stage[1] = big2;
    game.state.player1.stage.stage[2] = small;

    fire_live_start(&mut game, live);

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "two Liella! members' base hearts total 17 >= 11 -> score +1"
    );
}

#[test]
fn bp5026_total_below_threshold_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = bp5026_setup(&mut game);
    // Two low-heart Liella members + one non-Liella outsider.
    // Liella total: 2 + 2 = 4 < 11.
    let l1 = game.id("PL!SP-pb2-036-N");
    let l2 = game.id("PL!SP-pb2-037-N");
    let outsider = game.new_id("PL!-sd1-010-SD"); // μ's — not counted
    game.state.player1.stage.stage[0] = l1;
    game.state.player1.stage.stage[1] = l2;
    game.state.player1.stage.stage[2] = outsider;

    fire_live_start(&mut game, live);

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "total 4 < 11 -> no bonus"
    );
}
