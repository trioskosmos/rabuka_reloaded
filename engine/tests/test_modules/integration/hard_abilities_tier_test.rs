/// Hard-tier untested abilities — tests written FIRST from printed text;
/// engine/parser fixes follow so these pass exactly as the cards read.
///
/// - PL!HS-bp5-005-R 徒町小鈴 (ライブ開始時): opt. discard a 『DOLLCHESTRA』
///   card from hand -> choose an own-stage 『DOLLCHESTRA』 member; until live
///   end THIS member's cost equals (chosen member's ORIGINAL cost - 1). If
///   this card's cost thereby becomes >= 10 -> gain {{heart05}} until live end.
/// - PL!SP-bp5-017-N 桜小路きな子 (常時): while ANY own-stage 『Liella!』
///   member has moved areas this turn, this card IN HAND costs -2.
/// - PL!S-bp6-003-R 松浦果南 (起動 turn1, {E}{E}+hand): mill an other『Aqours』
///   member from own stage -> then debut from waitroom an 『Aqours』 member
///   whose cost EXACTLY equals (milled cost + 2) into the vacated area.
/// - PL!HS-cl1-011-CL ド！ド！ド！ (ライブ成功時, opt. {E}): choose one —
///   retrieve a member from waitroom; OR if own live zone has >= 2 cards,
///   retrieve a 『蓮ノ空』 live card from waitroom.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

fn effective_cost(game: &TestGame, cid: i16) -> i32 {
    let printed = game
        .db
        .get_card(cid)
        .and_then(|c| c.cost)
        .expect("test card must have printed cost") as i32;
    printed + game.state.mods.get_cost_modifier(cid)
}

const FILLER: &str = "PL!N-sd1-010-SD"; // Nijigasaki filler (NOT DOLLCHESTRA/Aqours/Liella)

// ====================================================================
// IDX 227 — PL!HS-bp5-005-R 徒町小鈴 cost-copy mirror (original - 1)
// ====================================================================

fn kokoro_setup(game: &mut TestGame, stage_dolls: &[&str], hand_doll: Option<&str>) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id("PL!HS-bp5-005-R"); // 小鈴, printed cost 4, DOLLCHESTRA
    game.state.player1.stage.stage[1] = me;
    for (i, no) in stage_dolls.iter().enumerate() {
        let cid = game.new_id(no);
        game.state.player1.stage.stage[i] = cid;
    }
    if let Some(no) = hand_doll {
        let cid = game.new_id(no);
        game.add_to_hand(cid);
    }
    me
}

/// Choose さやか cost17 -> holder cost becomes 16 >= 10 -> heart05 granted.
#[test]
fn kokoro_high_member_sets_cost_and_grants_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // PR-017-PR 大沢さやか original cost 17.
    let me = kokoro_setup(&mut game, &["PL!HS-PR-017-PR"], Some("PL!HS-bp2-008-R"));

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        game.has_pending_choice(),
        "optional discard gate must be offered"
    );
    game.select_option(0); // accept the discard (this gate lists pay first)
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]); // pick the staged DOLLCHESTRA member
    }

    assert_eq!(
        effective_cost(&game, me),
        16,
        "holder cost = chosen member's original 17 - 1"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart05),
        1,
        "cost became 16 >= 10 -> heart05 until live end"
    );
}

/// Boundary: chosen member original cost 11 -> holder 10 >= 10 -> heart granted.
#[test]
fn kokoro_boundary_eleven_exactly_reaches_ten() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // bp1-002-R 瑠璃乃 original cost 11.
    let me = kokoro_setup(&mut game, &["PL!HS-bp1-002-R"], Some("PL!HS-bp2-008-R"));

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.select_option(0); // accept (pay-first ordering)
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(effective_cost(&game, me), 10, "11 - 1 = 10");
    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart05),
        1,
        "exactly 10 satisfies 「10以上」 (inclusive)"
    );
}

/// Chosen member cost 10 -> holder 9 < 10 -> cost set but NO heart.
#[test]
fn kokoro_below_threshold_sets_cost_without_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // bp6-005-R+ 小鈴 original cost 10.
    let me = kokoro_setup(&mut game, &["PL!HS-bp6-005-R＋"], Some("PL!HS-bp2-004-R"));

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.select_option(0); // accept (pay-first ordering)
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(effective_cost(&game, me), 9, "10 - 1 = 9");
    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart05),
        0,
        "9 < 10 -> no heart05"
    );
}

/// Declining the optional discard does nothing at all.
#[test]
fn kokoro_decline_leaves_everything_untouched() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = kokoro_setup(&mut game, &["PL!HS-PR-017-PR"], Some("PL!HS-bp2-008-R"));

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice(), "gate offered");
    game.select_option(1); // decline (skip)

    assert_eq!(
        effective_cost(&game, me),
        4,
        "declined -> holder cost unchanged"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart05),
        0,
        "declined -> no heart"
    );
    assert!(
        !game.has_pending_choice(),
        "declined -> no member selection follows"
    );
}

/// No 『DOLLCHESTRA』 card in hand -> no gate, nothing happens.
#[test]
fn kokoro_no_dollchestra_in_hand_no_gate() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = kokoro_setup(&mut game, &["PL!HS-PR-017-PR"], None);

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        !game.has_pending_choice(),
        "no payable cost target -> no gate"
    );
    assert_eq!(effective_cost(&game, me), 4);
}

// ====================================================================
// IDX 726 — PL!SP-bp5-017-N 桜小路きな子 常時 hand cost -2 while moved
// ====================================================================

fn kinako_hand(game: &mut TestGame) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let k = game.id("PL!SP-bp5-017-N"); // printed cost 9
    game.add_to_hand(k);
    k
}

#[test]
fn kinako_hand_cost_minus_two_while_liella_moved() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let k = kinako_hand(&mut game);
    // A Liella! member on stage that moved areas this turn.
    let liella = game.new_id("PL!SP-sd1-002-SD"); // 澁谷かのん, Liella!
    game.state.player1.stage.stage[0] = liella;
    game.state.cards_moved_this_turn.push(liella);
    game.state.position_change_occurred_this_turn = true;

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(k),
        -2,
        "a Liella! member moved this turn -> this hand card costs -2"
    );
}

#[test]
fn kinako_hand_cost_normal_when_liella_did_not_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let k = kinako_hand(&mut game);
    let liella = game.new_id("PL!SP-sd1-002-SD");
    game.state.player1.stage.stage[0] = liella; // staged, never moved

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(k),
        0,
        "no movement this turn -> normal cost"
    );
}

/// Group-filter edge: only a NON-Liella! member moved -> no discount.
#[test]
fn kinako_non_liella_movement_gives_no_discount() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let k = kinako_hand(&mut game);
    let outsider = game.new_id(FILLER); // Nijigasaki member
    game.state.player1.stage.stage[0] = outsider;
    game.state.cards_moved_this_turn.push(outsider);
    game.state.position_change_occurred_this_turn = true;

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(k),
        0,
        "the moved member must be 『Liella!』 — others don't count"
    );
}

// ====================================================================
// IDX 461 — PL!S-bp6-003-R 松浦果南 mill -> exact-cost+2 rebirth
// ====================================================================

fn kanan_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id("PL!S-bp6-003-R"); // 松浦果南, Aqours
    game.state.player1.stage.stage[1] = me;
    game.give_energy(10);
    game.add_to_hand(game.new_id(FILLER)); // hand-discard cost target
    me
}

#[test]
fn kanan_mill_then_debut_exact_cost_plus_two_same_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = kanan_setup(&mut game);
    // Sacrifice: 千歌 cost 9 on left side. Rebirth target: ダイヤ cost 11 = 9+2.
    let chika = game.new_id("PL!S-bp2-001-R");
    game.state.player1.stage.stage[0] = chika;
    let dia = game.new_id("PL!S-bp2-004-R");
    game.state.player1.waitroom.cards.push(dia);

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]); // pay cost, pick sacrifice, pick rebirth
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&chika),
        "sacrificed Aqours member went to the waitroom"
    );
    assert_eq!(
        game.state.player1.stage.stage[0], dia,
        "cost-11 Aqours debuted into the vacated LEFT area"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&dia),
        "reborn member left the waitroom"
    );
}

#[test]
fn kanan_no_matching_cost_only_the_mill_happens() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = kanan_setup(&mut game);
    let chika = game.new_id("PL!S-bp2-001-R"); // cost 9
    game.state.player1.stage.stage[0] = chika;
    // Only a WRONG-cost Aqours member in the waitroom (9 != 9+2).
    let wrong = game.new_id("PL!S-PR-016-PR"); // ダイヤ cost 9
    game.state.player1.waitroom.cards.push(wrong);

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&chika),
        "the sacrifice still happened"
    );
    assert_eq!(
        game.state.player1.stage.stage[0], -1,
        "no member debuted into the vacated area (no cost-11 available)"
    );
}

/// exclude_self edge: the ability holder can't be its own sacrifice.
#[test]
fn kanan_cannot_mill_itself() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = kanan_setup(&mut game);
    // Holder is the ONLY own Aqours member on stage.
    let dia = game.new_id("PL!S-bp2-004-R"); // cost 11 = 9(holder)+2
    game.state.player1.waitroom.cards.push(dia);

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.stage.stage[1], me,
        "holder stays on stage — exclude_self prevents self-milling"
    );
    assert!(
        !game.state.player1.stage.stage.contains(&dia),
        "without a sacrifice there is no rebirth either"
    );
}

// ====================================================================
// IDX 840 — PL!HS-cl1-011-CL ド！ド！ド！ choice after optional {E}
// ====================================================================

fn dokidoki_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let live = game.id("PL!HS-cl1-011-CL");
    game.give_energy(5);
    live
}

#[test]
fn ddd_decline_pay_does_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = dokidoki_setup(&mut game);
    let mate = game.new_id(FILLER);
    game.state.player1.waitroom.cards.push(mate);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice(), "optional pay gate offered");
    game.select_option(0); // decline

    assert!(
        game.state.player1.waitroom.cards.contains(&mate),
        "declined -> waitroom untouched"
    );
}

#[test]
fn ddd_option_a_retrieves_any_member_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = dokidoki_setup(&mut game);
    let mate = game.new_id(FILLER);
    game.state.player1.waitroom.cards.push(mate);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    game.select_option(1); // pay 1 energy
    game.select_option(0); // option A: member retrieval
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]); // pick the member
    }

    assert!(
        game.state.player1.hand.cards.contains(&mate),
        "option A retrieved the member to hand"
    );
}

#[test]
fn ddd_option_b_requires_two_cards_in_live_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = dokidoki_setup(&mut game);
    // Exactly ONE card in the live zone -> option B's gate fails.
    game.add_to_hand(live);
    game.set_live_card(live);
    let hs_live = game.new_id("PL!HS-bp1-020-L"); // 蓮ノ空 live in waitroom
    game.state.player1.waitroom.cards.push(hs_live);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    game.select_option(1); // pay
    game.select_option(1); // option B
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&hs_live),
        "live zone has <2 cards -> option B must not retrieve anything"
    );
}

#[test]
fn ddd_option_b_with_two_live_cards_fetches_hasunosora_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = dokidoki_setup(&mut game);
    game.add_to_hand(live);
    game.set_live_card(live);
    // Second card in the live zone satisfies 「2枚以上」.
    let second = game.new_id("PL!HS-bp1-021-L");
    game.state.player1.live_card_zone.cards.push(second);
    let hs_live = game.new_id("PL!HS-bp1-020-L");
    game.state.player1.waitroom.cards.push(hs_live);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    game.select_option(1); // pay
    game.select_option(1); // option B
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&hs_live),
        "gate met -> 『蓮ノ空』 live card retrieved to hand"
    );
}
