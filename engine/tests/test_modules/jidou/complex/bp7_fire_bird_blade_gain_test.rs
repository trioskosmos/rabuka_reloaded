/// NSD02 parser/engine fix D19: `PL!N-sd2-026-P` / `PL!N-sd2-026-SD2` Fire Bird
/// ab#0 (ライブ開始時).
///
/// ライブ開始時：自分のステージにいるブレードを4つ以上持つ『虹ヶ咲』のメンバー1人は、
/// ライブ終了時まで、heart02×2を得る。
///
/// "At live start: one of your 虹ヶ咲 members on stage with 4 or more (current)
/// blades gains 2×heart02 until the end of the live."
///
/// The defect (D19): the parser's "concurrent heart+blade grant" dispatch rule
/// fired because the blade icon appears in the FILTER clause, emitting
/// sequential[gain_resource{blade,4}, gain_resource{heart02,4}]. The engine
/// therefore granted +4 blade AND +4 heart02 to an unfiltered member — resources
/// the text never grants, a doubled heart count, and no blade≥4 filter.
///
/// These tests pin the behavior as written: exactly ONE 虹ヶ咲 member with ≥4
/// current blades gets exactly +2 heart02 (until live end), and no blade gain.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

const FIRE_BIRD: &str = "PL!N-sd2-026-P";
const FILLER: &str = "PL!-sd1-010-SD";

fn seed_deck(game: &mut TestGame) {
    let filler = game.id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Put a member on P1's stage at the given area (no debut trigger).
fn place_member(game: &mut TestGame, area: MemberArea, card_no: &str) -> i16 {
    let id = game.id(card_no);
    game.add_to_stage(area, id);
    id
}

/// Set Fire Bird as P1's live card and fire live-start abilities.
/// Drain only auto-ability ordering prompts; gameplay choices are preserved.
fn fire_fire_bird_live_start(game: &mut TestGame) -> i16 {
    let fire_bird = game.id(FIRE_BIRD);
    game.state.player1.hand.cards.push(fire_bird);
    seed_deck(game);
    advance_to_live_card_set_p1(game);
    game.set_live_card(fire_bird);
    advance_to_live_start(game);
    game.drain_auto_ability_choices();
    fire_bird
}

// ====================================================================
// Eligible-member cases
// ====================================================================

/// One eligible 虹ヶ咲 member (宮下 愛, blade 5) on stage → auto-targeted.
/// She gains exactly +2 heart02; NO blade is gained anywhere.
#[test]
fn fire_bird_single_eligible_gains_2_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let miyashita = place_member(&mut game, MemberArea::Center, "PL!N-PR-028-PR"); // 宮下 愛, blade 5
    fire_fire_bird_live_start(&mut game);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(miyashita, HeartColor::Heart02),
        2,
        "eligible member should gain exactly +2 heart02"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(miyashita, HeartColor::Heart01),
        0,
        "no other heart color should be gained"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(miyashita),
        0,
        "Fire Bird must NOT grant blades (defect D19 granted +4 blade)"
    );
    assert!(
        !game.has_pending_choice(),
        "single eligible member should be auto-targeted without a prompt"
    );
}

/// Boundary: a member with EXACTLY 4 blades is eligible.
#[test]
fn fire_bird_exactly_4_blades_eligible() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ayumu = place_member(&mut game, MemberArea::Center, "PL!N-bp5-001-R＋"); // 上原歩夢, blade 4
    fire_fire_bird_live_start(&mut game);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(ayumu, HeartColor::Heart02),
        2,
        "exactly-4-blade member is eligible"
    );
    assert_eq!(game.state.mods.get_blade_modifier(ayumu), 0);
}

/// Multiple eligible members → the ability prompts to select exactly one;
/// the chosen member gets +2 heart02 and the unchosen gets 0.
#[test]
fn fire_bird_multiple_eligible_selects_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ayumu = place_member(&mut game, MemberArea::LeftSide, "PL!N-bp5-001-R＋"); // 上原歩夢, blade 4
    let miyashita = place_member(&mut game, MemberArea::Center, "PL!N-PR-028-PR"); // 宮下 愛, blade 5
    fire_fire_bird_live_start(&mut game);

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "multiple eligible members should require a selection prompt"
    );
    let count = game.pending_choice_count();
    assert_eq!(count, 1, "exactly one member should be selectable");

    game.select_indices(&[0]);

    let ayumu_heart = game
        .state
        .mods
        .get_heart_modifier(ayumu, HeartColor::Heart02);
    let miyashita_heart = game
        .state
        .mods
        .get_heart_modifier(miyashita, HeartColor::Heart02);
    assert!(
        ayumu_heart == 2 || miyashita_heart == 2,
        "the chosen member should gain +2 heart02 (ayumu={}, miyashita={})",
        ayumu_heart,
        miyashita_heart
    );
    assert!(
        ayumu_heart == 0 || miyashita_heart == 0,
        "the unchosen member should gain nothing (ayumu={}, miyashita={})",
        ayumu_heart,
        miyashita_heart
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(ayumu) + game.state.mods.get_blade_modifier(miyashita),
        0,
        "no blade gain regardless of selection"
    );
}

/// A waited eligible member still counts as "ステージにいる" (no active
/// requirement in the text) and is auto-targetable.
#[test]
fn fire_bird_waited_member_still_eligible() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let miyashita = place_member(&mut game, MemberArea::Center, "PL!N-PR-028-PR"); // 宮下 愛, blade 5
    game.state.mods.add_orientation_modifier(miyashita, "wait");
    fire_fire_bird_live_start(&mut game);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(miyashita, HeartColor::Heart02),
        2,
        "waited members are still on the stage and eligible"
    );
}

// ====================================================================
// Ineligible-member cases
// ====================================================================

/// Members below 4 blades are NOT eligible → no gain, no prompt.
#[test]
fn fire_bird_below_4_blades_not_eligible() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shiori = place_member(&mut game, MemberArea::Center, "PL!N-sd1-012-SD"); // 鐘 嵐珠, blade 3, no ability
    fire_fire_bird_live_start(&mut game);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(shiori, HeartColor::Heart02),
        0,
        "blade-3 member must NOT receive heart02"
    );
    assert_eq!(game.state.mods.get_blade_modifier(shiori), 0);
    assert!(
        !game.has_pending_choice(),
        "no selection prompt when no member is eligible"
    );
}

/// No eligible members on stage → nothing happens, no prompt.
#[test]
fn fire_bird_no_eligible_member_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kasumi = place_member(&mut game, MemberArea::Center, "PL!N-bp1-002-R＋"); // 中須かすみ, blade 2
    fire_fire_bird_live_start(&mut game);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kasumi, HeartColor::Heart02),
        0,
        "no gain with no eligible member"
    );
    assert!(!game.has_pending_choice());
}

/// A 4-blade member from a DIFFERENT group is NOT eligible (虹ヶ咲 filter).
#[test]
fn fire_bird_wrong_group_not_eligible() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chika = place_member(&mut game, MemberArea::Center, "PL!S-bp2-001-R"); // 高海千歌 (Aqours), blade 4
    fire_fire_bird_live_start(&mut game);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(chika, HeartColor::Heart02),
        0,
        "non-虹ヶ咲 4-blade member must be excluded by the group filter"
    );
    assert!(!game.has_pending_choice());
}

/// The blade count is the CURRENT total (base + modifiers), not the printed
/// value: a blade-3 虹ヶ咲 member with a +1 blade modifier reaches 4 and is
/// eligible. (Defect D19's filter was dropped entirely; this pins the semantic
/// used once it is restored.)
#[test]
fn fire_bird_current_blade_modifiers_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shiori = place_member(&mut game, MemberArea::Center, "PL!N-sd1-012-SD"); // 鐘 嵐珠, printed blade 3
    game.state.mods.add_blade_modifier(shiori, 1); // current total → 4
    fire_fire_bird_live_start(&mut game);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(shiori, HeartColor::Heart02),
        2,
        "current blade total (3 printed + 1 modifier = 4) satisfies the ≥4 filter"
    );
}
