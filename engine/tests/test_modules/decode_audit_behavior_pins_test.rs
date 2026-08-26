/// Behavior pins for cards whose condition fields ride the shadow schema
/// (decode-audit triage 08-24): the JSON carries `reference_card` /
/// `card_names` on condition objects that the typed Condition struct has no
/// field for, so these tests pin the OBSERVABLE behavior demanded by the
/// Japanese text regardless of which layer implements it.
///
/// - PL!N-bp4-010-R＋ 三船栞子 ab#1 (ライブ開始時):
///   「自分のライブ中の『虹ヶ咲』のライブカードを1枚選ぶ。それと同じカード名のカードが
///    自分の成功ライブカード置き場にある場合、ライブ終了時まで、heart04を得る」
/// - PL!HS-sd1-018-SD Dream Believers（105期Ver.） ab#0 (ライブ開始時):
///   「自分のステージに『蓮ノ空』のメンバーが3人以上いて、かつ自分の控え室に
///    カード名に「DreamBelievers」を含むライブカードがある場合、このカードのスコアを＋１する」
/// - PL!SP-bp2-001-R＋ 澁谷かのん ab#0 (登場) negative:
///   無効にできる Liella! メンバーがいない場合、回収も行われない。
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

// ====================================================================
// PL!N-bp4-010-R＋ 三船栞子 ab#1 — reference_card gate on success zone
// ====================================================================

/// Same-name live card in the success zone → the selected live card gains
/// heart04 until live end.
#[test]
fn mifune_same_name_in_success_zone_gains_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mifune = game.id("PL!N-bp4-010-R\u{ff0b}");
    let niji_live_a = game.id("PL!N-sd1-025-SD");
    let niji_live_b = game.new_id("PL!N-sd1-025-SD"); // same name, different copy

    // Direct placement: we are testing the ライブ開始時 trigger in isolation,
    // not the debut pipeline.
    game.state.player1.stage.stage[1] = mifune;
    game.state.player1.live_card_zone.cards.push(niji_live_a);
    game.state.player1.success_live_card_zone.cards.push(niji_live_b);

    fire_trigger(&mut game, mifune, AbilityTrigger::LiveStart, "ライブ開始時");

    // 「ライブカードを1枚選ぶ」 — observed: a SelectCard live_card_zone
    // prompt appears even with exactly one candidate.
    assert!(
        game.has_pending_choice(),
        "live-card selection must be prompted"
    );
    assert_eq!(game.pending_choice_type().as_deref(), Some("SelectCard"));
    game.select_indices(&[0]);

    assert_eq!(
        game.state.mods.get_heart_modifier(niji_live_a, HeartColor::Heart04),
        1,
        "same-name live card in success zone -> selected live card gains heart04"
    );
}

/// Different-name live card in the success zone → no heart04 gain anywhere.
#[test]
fn mifune_different_name_in_success_zone_no_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mifune = game.id("PL!N-bp4-010-R\u{ff0b}");
    let niji_live = game.id("PL!N-sd1-025-SD");
    let other_live = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage[1] = mifune;
    game.state.player1.live_card_zone.cards.push(niji_live);
    game.state.player1.success_live_card_zone.cards.push(other_live);

    fire_trigger(&mut game, mifune, AbilityTrigger::LiveStart, "ライブ開始時");
    // Observed: SelectCard live_card_zone prompt appears even when the
    // selection would be discarded by the name gate.
    assert!(
        game.has_pending_choice(),
        "live-card selection must be prompted"
    );
    assert_eq!(game.pending_choice_type().as_deref(), Some("SelectCard"));
    game.select_indices(&[0]);

    assert_eq!(
        game.state.mods.get_heart_modifier(niji_live, HeartColor::Heart04),
        0,
        "different-name card in success zone -> no heart04"
    );
}

/// Empty success zone → no heart04 gain.
#[test]
fn mifune_empty_success_zone_no_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mifune = game.id("PL!N-bp4-010-R\u{ff0b}");
    let niji_live = game.id("PL!N-sd1-025-SD");

    game.state.player1.stage.stage[1] = mifune;
    game.state.player1.live_card_zone.cards.push(niji_live);

    fire_trigger(&mut game, mifune, AbilityTrigger::LiveStart, "ライブ開始時");
    // Observed: SelectCard live_card_zone prompt appears even with a single
    // candidate and an empty success zone.
    assert!(
        game.has_pending_choice(),
        "live-card selection must be prompted"
    );
    assert_eq!(game.pending_choice_type().as_deref(), Some("SelectCard"));
    game.select_indices(&[0]);

    assert_eq!(
        game.state.mods.get_heart_modifier(niji_live, HeartColor::Heart04),
        0,
        "empty success zone -> no heart04"
    );
}

// ====================================================================
// PL!HS-sd1-018-SD Dream Believers（105期Ver.） — compound gate:
// 蓮ノ空 members ≥ 3 AND waitroom has live card named *containing*
// 「DreamBelievers」 → score +1
// ====================================================================

fn hs_member(game: &mut TestGame) -> i16 {
    // 日野下花帆 (PL!HS series → 蓮ノ空 group match via series).
    game.new_id("PL!HS-bp5-001-P")
}

/// Both clauses met → score +1.
#[test]
fn dream_believers_members_and_name_match_score_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-sd1-018-SD");

    game.state.player1.live_card_zone.cards.push(live);
    for slot in 0..3 {
        game.state.player1.stage.stage[slot] = hs_member(&mut game);
    }
    // Base "Dream Believers" — a different entry whose NAME CONTAINS the
    // substring (「カード名に「DreamBelievers」を含む」), not an exact match.
    game.add_to_discard(game.id("PL!HS-bp1-019-L"));

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "3 蓮ノ空 members + DreamBelievers-named live card in waitroom -> score +1"
    );
}

/// Name clause fails: waitroom live card without「DreamBelievers」→ no bonus,
/// even though the member count is satisfied.
#[test]
fn dream_believers_wrong_name_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-sd1-018-SD");

    game.state.player1.live_card_zone.cards.push(live);
    for slot in 0..3 {
        game.state.player1.stage.stage[slot] = hs_member(&mut game);
    }
    // A live card whose name does not contain the substring.
    game.add_to_discard(game.id("PL!-sd1-020-SD"));

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "waitroom live card without 'DreamBelievers' in its name -> no score"
    );
}

/// Member-count clause fails: only 2 蓮ノ空 members despite a name match.
#[test]
fn dream_believers_two_members_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-sd1-018-SD");

    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.stage.stage[0] = hs_member(&mut game);
    game.state.player1.stage.stage[1] = hs_member(&mut game);
    game.add_to_discard(game.id("PL!HS-bp1-019-L"));

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "only 2 蓮ノ空 members -> no score even with a name match"
    );
}

/// Empty waitroom → no bonus despite full stage.
#[test]
fn dream_believers_empty_waitroom_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-sd1-018-SD");

    game.state.player1.live_card_zone.cards.push(live);
    for slot in 0..3 {
        game.state.player1.stage.stage[slot] = hs_member(&mut game);
    }
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler); // keep refresh mechanics away from the assertion

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "empty waitroom -> no score"
    );
}

/// Substring edge: 「Dream Believers（104期Ver.）」 also CONTAINS the substring
/// (Q236/Q237 treat the variants as same-family names), so it satisfies the
/// gate.
#[test]
fn dream_believers_variant_name_still_matches() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-sd1-018-SD"); // 105期Ver.

    game.state.player1.live_card_zone.cards.push(live);
    for slot in 0..3 {
        game.state.player1.stage.stage[slot] = hs_member(&mut game);
    }
    game.add_to_discard(game.id("PL!HS-bp5-017-L")); // 104期Ver.

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "'Dream Believers（104期Ver.）' contains the substring -> score +1"
    );
}

// ====================================================================
// PL!SP-bp2-001-R＋ 澁谷かのん ab#0 negative: nothing invalidatable →
// 「これにより無効にした場合」 never fires → no waitroom recovery.
// (Positive path covered by kanon_invalidate_test; Q106 covers the
// already-nullified re-pick ruling at the rules level.)
// ====================================================================

#[test]
fn kanon_alone_on_stage_no_recovery() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp2-001-R\u{ff0b}");
    let liella_card = game.id("PL!SP-sd1-001-SD"); // Liella! card in waitroom

    game.add_to_hand(kanon);
    game.add_to_discard(liella_card);
    game.give_energy(13);
    // No other member on stage: kanon debuts into an empty Center.
    game.play_to_stage(kanon, rabuka_engine::zones::MemberArea::Center);

    assert!(
        !game.state.player1.hand.cards.contains(&liella_card),
        "no invalidatable Liella! member -> 「これにより無効にした場合」 cannot fire -> no recovery"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&liella_card),
        "the Liella! card stays in the waitroom"
    );
}
