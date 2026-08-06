/// BP07 優木せつ菜 PL!N-bp7-007-R＋ — all 3 abilities, gameplay tests.
///
/// ab#0 常時: このメンバーの下にあるエネルギーカード1枚につき、heart02を得る。
///   → per_unit gain_resource{heart02, location:under_member}.
///
/// ab#1 常時: 自分のエネルギーが6枚より多いかぎり、その差に等しい数のheart02を得る。
///   → dynamic_count = (energy − 6). (G6 fix: "その差" was unresolvable /
///     resolved as a live-score difference.)
///
/// ab#2 ライブ成功時: 自分のエネルギーデッキから、エネルギーカード1枚をこのメンバーの下に置く。
///   → place_energy_under_member{source:energy_deck, destination:under_member}.
///
/// The two 常時 abilities both add heart02 to せつ菜, so each test isolates its
/// ability by zeroing the other's input (ab#1 needs energy>6 with nothing under;
/// ab#0 needs energy≤6 with energy placed under).
use crate::helpers::*;
use rabuka_engine::card::{parse_heart_color, HeartColor};
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const SETSUNA: &str = "PL!N-bp7-007-R＋";
const ENERGY: &str = "LL-E-001-SD";

fn heart02(game: &TestGame, cid: i16) -> i32 {
    game.state
        .mods
        .get_heart_modifier(cid, HeartColor::Heart02)
}

fn under_count(game: &TestGame, area: MemberArea) -> usize {
    game.state.player1.stage.under_cards[area as usize].len()
}

fn place_setsuna(game: &mut TestGame) -> i16 {
    let s = game.id(SETSUNA);
    game.state.player1.stage.stage = [-1, s, -1];
    game.state.recalculate_constants();
    s
}

// ═════════════════════════════════════════════════════════════════════════
// ab#0 — 常時: heart02 per energy card under this member
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn ab0_zero_energy_under_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    // Keep total energy ≤ 6 so ab#1 contributes nothing.
    game.give_energy(4);

    let s = place_setsuna(&mut game);

    assert_eq!(heart02(&game, s), 0, "no energy under → no heart02");
}

#[test]
fn ab0_two_energy_under_two_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    game.give_energy(4);

    let s = place_setsuna(&mut game);
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, game.id(ENERGY));
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, game.id(ENERGY));
    game.state.recalculate_constants();

    assert_eq!(heart02(&game, s), 2, "2 energy cards under → 2 heart02");
}

#[test]
fn ab0_only_energy_under_counts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    game.give_energy(4);

    let s = place_setsuna(&mut game);
    // Member cards / other zones must NOT count — only energy under THIS member.
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::LeftSide, game.id(ENERGY));
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, game.id("PL!SP-sd1-001-SD"));
    game.state.recalculate_constants();

    assert_eq!(
        heart02(&game, s),
        0,
        "energy under a DIFFERENT area + a member under center → still 0"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// ab#1 — 常時: heart02 × (energy − 6) while energy > 6
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn ab1_energy_6_or_less_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    game.give_energy(6); // == 6, not "more than 6"
    let s = place_setsuna(&mut game);
    assert_eq!(heart02(&game, s), 0, "energy == 6 → no heart02");

    let mut game = TestGame::new(db.clone());
    game.give_energy(4); // < 6
    let s = place_setsuna(&mut game);
    assert_eq!(heart02(&game, s), 0, "energy < 6 → no heart02");
}

#[test]
fn ab1_energy_8_gives_two_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    game.give_energy(8);
    let s = place_setsuna(&mut game);
    assert_eq!(heart02(&game, s), 2, "8 − 6 = 2 heart02");
}

#[test]
fn ab1_energy_12_gives_six_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    game.give_energy(12);
    let s = place_setsuna(&mut game);
    assert_eq!(heart02(&game, s), 6, "12 − 6 = 6 heart02");
}

#[test]
fn ab1_scales_with_energy_recalc() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let s = place_setsuna(&mut game);

    game.give_energy(7);
    game.state.recalculate_constants();
    assert_eq!(heart02(&game, s), 1, "7 − 6 = 1");

    game.give_energy(5); // total 12
    game.state.recalculate_constants();
    assert_eq!(heart02(&game, s), 6, "12 − 6 = 6");

    // Removing energy back to ≤6 zeroes the gain.
    game.state.player1.energy_zone.cards.truncate(6);
    game.state.recalculate_constants();
    assert_eq!(heart02(&game, s), 0, "back to 6 → 0");
}

// ═════════════════════════════════════════════════════════════════════════
// ab#2 — ライブ成功時: place 1 energy from the energy deck under this member
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn ab2_live_success_places_energy_under_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let s = game.id(SETSUNA);
    game.state.player1.stage.stage = [-1, s, -1];
    // Energy deck has 2 cards.
    game.state.player1.energy_deck.cards.push(game.id(ENERGY));
    game.state.player1.energy_deck.cards.push(game.id(ENERGY));

    let card = game.db.get_card(s).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .expect("ab#2 should be a ライブ成功時 ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(s),
        None,
        None,
    );
    game.state.activating_card = Some(s);
    game.state.process_pending_auto_abilities(&pid);

    assert_eq!(
        under_count(&game, MemberArea::Center),
        1,
        "exactly 1 energy card placed under せつ菜"
    );
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        1,
        "1 energy card drawn from the energy deck"
    );

    // The newly placed energy now feeds ab#0: recalc → +1 heart02.
    game.state.recalculate_constants();
    assert_eq!(heart02(&game, s), 1, "placed energy now grants 1 heart02 via ab#0");
}

#[test]
fn ab2_live_success_no_energy_deck_places_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let s = game.id(SETSUNA);
    game.state.player1.stage.stage = [-1, s, -1];
    // Energy deck empty.

    let card = game.db.get_card(s).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(s),
        None,
        None,
    );
    game.state.activating_card = Some(s);
    game.state.process_pending_auto_abilities(&pid);

    assert_eq!(
        under_count(&game, MemberArea::Center),
        0,
        "empty energy deck → nothing placed under"
    );
}

#[test]
fn ab2_heart_colors_parse() {
    // Sanity: the heart color used by these abilities decodes to Heart02.
    assert_eq!(parse_heart_color("heart02"), HeartColor::Heart02);
}
