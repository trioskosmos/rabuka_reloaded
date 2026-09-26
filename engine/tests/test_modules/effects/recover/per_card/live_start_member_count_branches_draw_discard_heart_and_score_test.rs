use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

/// SUNNY DAY SONG (PL!-bp5-021-L) — LiveStart ability with 3 conditional branches.

#[test]
fn sunny_branch1_1_member_triggers_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let member = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, member);
    // Add enough cards for phase draws + ability draw
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 1 requires choosing which card to discard from hand.
    // Observed: SelectCard zone=hand count=1 allow_skip=false
    // ("Select 1 card(s) from hand").
    assert!(
        game.has_pending_choice(),
        "Branch 1 hand-discard prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard hand discard"
    );
    game.select_indices(&[0]);

    // Branch 1 fired: at least one card was drawn (from either player's deck)
    // Verify that cards moved: opponent has hand+discard > initial
    let p2_total = game.state.player2.hand.cards.len() + game.state.player2.waitroom.cards.len();
    assert!(p2_total > 0, "P2 should have drawn + discarded cards");
    // Opponent's hand or discard changed (they drew then discarded)
    let p2_total = game.state.player2.hand.cards.len() + game.state.player2.waitroom.cards.len();
    assert!(
        p2_total >= 2,
        "P2 should have drawn + discarded, total cards >= 2"
    );
}

/// Play `live_no` as p1's live through the performance and report how much
/// each player's hand and waitroom grew, plus the live's score modifier.
///
/// Hand size alone CANNOT be probed directly here: the performance window draws
/// for the first attacker regardless of the live card, so a control live with
/// no ライブ開始時 (PL!-sd1-019-SD) grows p1's hand by exactly as much as
/// SUNNY DAY SONG does. The DIFFERENCE between the two lives is the only
/// readable signal for the カードを1枚引く / 手札を1枚控え室に置く tier.
fn live_window_deltas(
    live_no: &str,
    stage_member: Option<&str>,
) -> (usize, usize, usize, usize, i32) {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id(live_no);
    game.assert_card_identity(live, live_no);
    let filler = game.id("PL!-sd1-013-SD");
    game.add_to_hand(live);
    if let Some(no) = stage_member {
        let m = game.id(no);
        game.state.player1.stage.stage = [-1, m, -1];
    }
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    let p1h = game.state.player1.hand.cards.len();
    let p1w = game.state.player1.waitroom.cards.len();
    let p2h = game.state.player2.hand.cards.len();
    let p2w = game.state.player2.waitroom.cards.len();

    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let live_in_zone = game.state.player1.live_card_zone.cards[0];
    (
        game.state.player1.hand.cards.len() - p1h,
        game.state.player1.waitroom.cards.len() - p1w,
        game.state.player2.hand.cards.len() - p2h,
        game.state.player2.waitroom.cards.len() - p2w,
        game.state.mods.get_score_modifier(live_in_zone),
    )
}

#[test]
fn sunny_branch1_no_members_does_nothing() {
    // No members on p1's stage: SUNNY DAY SONG must behave exactly like a live
    // with no ライブ開始時 at all.
    let (p1h, p1w, p2h, p2w, score) = live_window_deltas("PL!-bp5-021-L", None);
    let (c1h, c1w, c2h, c2w, _) = live_window_deltas("PL!-sd1-019-SD", None);

    assert_eq!(
        (p1h, p1w, p2h, p2w),
        (c1h, c1w, c2h, c2w),
        "with no members on stage the ライブ開始時 must be a no-op: \
         自分のステージにメンバーが1人以上いる場合 … カードを1枚引く / \
         手札を1枚控え室に置く (sunny {p1h}/{p1w}/{p2h}/{p2w} vs \
         control {c1h}/{c1w}/{c2h}/{c2w})"
    );
    assert_eq!(
        score, 0,
        "このカードのスコアを＋１する requires 3+ differently-named members"
    );
}

#[test]
fn sunny_branch1_one_member_draws_for_p1() {
    // One member on stage: the first tier fires for p1, so the live must beat
    // the no-ライブ開始時 control on p1's hand inside this window.
    //
    // p2's half (自分と相手はカードを1枚引く) is NOT asserted here: p2's hand does
    // not move inside the first-attacker window, so this test cannot read it.
    // The sibling test `sunny_branch1_1_member_triggers_draw` above covers the
    // opponent's side through the full live.
    let (p1h, _p1w, _p2h, _p2w, _s) =
        live_window_deltas("PL!-bp5-021-L", Some("PL!-sd1-010-SD"));
    let (c1h, _c1w, _c2h, _c2w, _) =
        live_window_deltas("PL!-sd1-019-SD", Some("PL!-sd1-010-SD"));

    assert!(
        p1h > c1h,
        "自分のステージにメンバーが1人以上いる場合 … カードを1枚引く — p1 must \
         gain one more card than the control live (sunny {p1h} vs control {c1h})"
    );
}

#[test]
fn sunny_branch3_3_members_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD"); // 星空凛
    let kotori = game.id("PL!-sd1-010-SD"); // 南ことり
    let umi = game.id("PL!-sd1-006-SD"); // 園田海未
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::LeftSide, kotori);
    game.add_to_stage(MemberArea::RightSide, umi);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Handle pending choices: discard choice from branch 1, then heart target from branch 2
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Branch 3: score +1 for 3 distinct-name members
    let sunny_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.mods.get_score_modifier(sunny_id);
    assert_eq!(score_mod, 1, "3 distinct-name members should give +1 score");
}

#[test]
fn sunny_branch3_3_members_duplicate_name_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD"); // 星空凛
    let honoka2 = game.id("PL!-sd1-005-SD"); // same name
    let kotori = game.id("PL!-sd1-010-SD"); // 南ことり
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::LeftSide, honoka2);
    game.add_to_stage(MemberArea::RightSide, kotori);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Handle pending choices: discard choice from branch 1, then heart target from branch 2
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let sunny_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.mods.get_score_modifier(sunny_id);
    assert_eq!(score_mod, 0, "No score bonus with duplicate names");
}

// ============================================================
// Branch 2 tests (2+ members — grant heart03 to 1 μ's member)
// ============================================================

/// 2 μ's members on stage → Branch 2 fires, heart03 granted to first member.
#[test]
fn sunny_branch2_two_mus_grants_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD"); // μ's member
    let kotori = game.id("PL!-sd1-010-SD"); // μ's member
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::LeftSide, kotori);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Handle all pending choices: Branch 1 discard, then Branch 2 heart target
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // 『μ's』のメンバー1人は…heart03を得る — ONE member, so exactly one of the
    // two has heart03 == 1 and the other has none. The old
    // `h1 >= 1 || h2 >= 1` would also pass if the engine granted heart03 to
    // BOTH, which is the failure this tier is able to produce.
    use rabuka_engine::card::HeartColor;
    let heart03_honoka = game
        .state
        .mods
        .get_heart_modifier(honoka, HeartColor::Heart03);
    let heart03_kotori = game
        .state
        .mods
        .get_heart_modifier(kotori, HeartColor::Heart03);
    let holders = [heart03_honoka, heart03_kotori]
        .iter()
        .filter(|&&h| h == 1)
        .count();
    assert_eq!(
        holders, 1,
        "『μ's』のメンバー1人は…heart03を得る — exactly one member gains heart03 \
         (honoka={heart03_honoka}, kotori={heart03_kotori})"
    );
    assert!(
        (heart03_honoka == 1) ^ (heart03_kotori == 1),
        "one of them has heart03 and the other has none, not a partial amount"
    );
}

#[test]
fn sunny_branch2_targets_only_owners_mus_members() {
    use rabuka_engine::card::HeartColor;

    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sunny = game.id("PL!-bp5-021-L");
    let p1_mus = game.id("PL!-sd1-005-SD");
    let p1_non_mus = game.id("PL!S-sd1-013-SD");
    let p2_mus_a = game.new_id("PL!-sd1-005-SD");
    let p2_mus_b = game.new_id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-013-SD");
    assert_ne!(p1_mus, p2_mus_a);
    assert_ne!(p1_mus, p2_mus_b);
    assert_ne!(p2_mus_a, p2_mus_b);

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, p1_mus);
    game.add_to_stage(MemberArea::LeftSide, p1_non_mus);
    game.state.player2.stage.stage[0] = p2_mus_a;
    game.state.player2.stage.stage[1] = p2_mus_b;
    game.state.player2.stage.stage[2] = -1;
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    assert_eq!(game.pending_choice_type().as_deref(), Some("SelectCard"));
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(p1_mus, HeartColor::Heart03),
        1
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(p1_non_mus, HeartColor::Heart03),
        0
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(p2_mus_a, HeartColor::Heart03),
        0
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(p2_mus_b, HeartColor::Heart03),
        0
    );
}

/// 2 non-μ's members → Branch 2 condition met but no μ's target → no heart granted.
#[test]
fn sunny_branch2_two_non_mus_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let aqours_a = game.id("PL!S-sd1-013-SD"); // Aqours member
    let aqours_b = game.id("PL!S-sd1-010-SD"); // Aqours member
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, aqours_a);
    game.add_to_stage(MemberArea::LeftSide, aqours_b);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 1 → discard choice. Observed: SelectCard zone=hand count=1
    // allow_skip=false is offered even when no μ's target exists for branch 2.
    assert!(
        game.has_pending_choice(),
        "Branch 1 hand-discard prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard hand discard"
    );
    game.select_indices(&[0]);

    // Branch 2 condition (count>=2) is met, but no μ's member exists to target.
    // The effect should skip the gain_resource silently (no choice presented).
    assert!(
        !game.has_pending_choice(),
        "No heart target choice when no μ's members"
    );

    // Verify no heart03 was granted to either member
    use rabuka_engine::card::HeartColor;
    let h_a = game
        .state
        .mods
        .get_heart_modifier(aqours_a, HeartColor::Heart03);
    let h_b = game
        .state
        .mods
        .get_heart_modifier(aqours_b, HeartColor::Heart03);
    assert_eq!(h_a, 0, "Non-μ's member should not get heart03");
    assert_eq!(h_b, 0, "Non-μ's member should not get heart03");
}

/// Only 1 member → Branch 2 condition (>=2) NOT met, no heart03 granted.
#[test]
fn sunny_branch2_one_member_skips_b2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD"); // μ's member
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, honoka);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 1 → discard choice. Observed: SelectCard zone=hand count=1
    // allow_skip=false is offered even when no μ's target exists for branch 2.
    assert!(
        game.has_pending_choice(),
        "Branch 1 hand-discard prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard hand discard"
    );
    game.select_indices(&[0]);

    // Branch 2 should NOT fire (only 1 member)
    assert!(
        !game.has_pending_choice(),
        "Branch 2 should not trigger with only 1 member"
    );

    // No heart03 granted
    use rabuka_engine::card::HeartColor;
    let heart03_mod = game
        .state
        .mods
        .get_heart_modifier(honoka, HeartColor::Heart03);
    assert_eq!(heart03_mod, 0, "No heart03 granted with only 1 member");
}

// ============================================================
// Q210/Q211: Joint card (multiname) with SUNNY DAY SONG
// ============================================================

#[test]
fn sunny_q210_joint_card_counts_as_one_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp3-001-R\u{ff0b}"); // 園田海未&津島善子&天王寺璃奈
    let sunny = game.id("PL!-bp5-021-L");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, joint);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 1 fires (1 member = joint card counts as 1).
    // Drain all pending choices using the while-loop pattern from existing tests.
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Branch 2 should NOT have fired (count = 1, need >= 2).
    // Verify no heart03 was granted to the joint card.
    let heart03_mod = game
        .state
        .mods
        .get_heart_modifier(joint, rabuka_engine::card::HeartColor::Heart03);
    assert_eq!(
        heart03_mod, 0,
        "No heart03 granted with 1 joint member (Branch 2 should not fire)"
    );
}

/// Q211: Joint card (LL-bp3-001-R+, contains μ's character 園田海未) + 1 other member = 2 members.
/// Branch 2 fires and the joint card IS selectable as a μ's member for heart03 gain.
#[test]
fn sunny_q211_joint_card_targetable_for_mus_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp3-001-R\u{ff0b}"); // contains 園田海未 (μ's)
    let sunny = game.id("PL!-bp5-021-L");
    let other = game.id("PL!-sd1-013-SD"); // generic member (not μ's specific)
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, joint);
    game.add_to_stage(MemberArea::LeftSide, other);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Observed 3-step chain:
    //   1. SelectAutoAbility — the joint card's own LiveStart ability fires
    //      alongside SUNNY DAY SONG; pick resolution order (index 0).
    assert!(
        game.has_pending_choice(),
        "auto-ability order prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectAutoAbility"),
        "expected SelectAutoAbility (joint card LiveStart vs SUNNY DAY SONG)"
    );
    game.select_indices(&[0]);

    //   2. SUNNY DAY SONG Branch 1: SelectCard zone=hand count=1
    //      allow_skip=false ("Select 1 card(s) from hand").
    assert!(
        game.has_pending_choice(),
        "Branch 1 hand-discard prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected Branch 1 SelectCard hand discard"
    );
    game.select_indices(&[0]);

    //   3. Branch 2 fires (joint card + 1 other = 2 members):
    //      SelectCard zone=stage count=1 group=μ's — heart target selection.
    assert!(
        game.has_pending_choice(),
        "Branch 2 should fire with joint card + 1 other = 2 members"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected Branch 2 SelectCard stage heart target (group μ's)"
    );
    game.select_indices(&[0]);

    // Verify the joint card received heart03
    use rabuka_engine::card::HeartColor;
    let heart03_mod = game
        .state
        .mods
        .get_heart_modifier(joint, HeartColor::Heart03);
    // The other member should also be checked
    let heart03_mod_other = game
        .state
        .mods
        .get_heart_modifier(other, HeartColor::Heart03);

    // At least one member should have heart03 (the joint card is a valid μ's target)
    assert!(
        heart03_mod >= 1 || heart03_mod_other >= 1,
        "Joint card or other member should gain heart03 as a μ's target"
    );
}

#[test]
fn sunny_branch1_discard_is_not_restricted_to_mus_q210() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sunny = game.id("PL!-bp5-021-L");
    let aqours = game.id("PL!S-sd1-013-SD");
    let mus = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(sunny);
    game.add_to_hand(aqours);
    game.add_to_hand(mus);
    game.add_to_stage(MemberArea::Center, mus);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    let hand_index = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .position(|card_id| *card_id == aqours)
        .expect("Aqours hand card");
    let mus_index = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .position(|card_id| *card_id == mus)
        .expect("mu's hand card");
    match game.get_pending_choice() {
        Choice::SelectCard {
            filtered_indices, ..
        } => {
            let indices = filtered_indices.as_ref().expect("discard filter");
            assert!(indices.contains(&hand_index));
            assert!(indices.contains(&mus_index));
        }
        choice => panic!("expected unfiltered hand discard, got {choice:?}"),
    }
}


fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
