/// BP07 parser fixes B1/B2: `location: "under_member"` on per-unit blade gain.
///
/// B1: PL!N-bp7-003-R＋ 桜坂しずく ab#1 (ライブ開始時, ライブ終了時まで)
///   このメンバーの下に置かれている名前の異なるメンバーカード1枚につき、ブレードを得る。
///   → gain_resource{blade, per_unit, distinct:card_name, location:under_member}
///
/// B2: PL!SP-bp7-003-R＋ 嵐千砂都 ab#0 (常時)
///   このメンバーの下に置かれているメンバーカード1枚につき、ブレードを得る。
///   → gain_resource{blade, per_unit, location:under_member}  (no distinct)
///
/// The defect was the missing `location`, so the per-unit count leaked to all
/// member cards anywhere. These tests pin down that ONLY cards placed under
/// the member are counted.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn seed_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
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

fn place_member_under(game: &mut TestGame, area: MemberArea, card_no: &str) -> i16 {
    let id = game.id(card_no);
    game.state.player1.stage.place_under_card(area, id);
    id
}

// ====================================================================
// B2: PL!SP-bp7-003-R＋ 嵐千砂都 ab#0 — 常時, per member card under
// ====================================================================

/// 3 member cards under center → exactly 3 blade
#[test]
fn chika_constant_blade_per_member_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chika = game.id("PL!SP-bp7-003-R＋");
    game.state.player1.stage.stage = [-1, chika, -1];

    place_member_under(&mut game, MemberArea::Center, "PL!SP-sd1-001-SD");
    place_member_under(&mut game, MemberArea::Center, "PL!SP-sd1-001-SD");
    place_member_under(&mut game, MemberArea::Center, "PL!SP-sd1-004-SD");

    game.state.recalculate_constant_blade_modifiers();

    let blade_mod = game.state.mods.get_blade_modifier(chika);
    assert_eq!(
        blade_mod, 3,
        "3 member cards under → 3 blade (per_unit location=under_member), got {}",
        blade_mod
    );
}

/// No cards under → 0 blade
#[test]
fn chika_constant_blade_zero_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chika = game.id("PL!SP-bp7-003-R＋");
    game.state.player1.stage.stage = [-1, chika, -1];

    game.state.recalculate_constant_blade_modifiers();

    let blade_mod = game.state.mods.get_blade_modifier(chika);
    assert_eq!(
        blade_mod, 0,
        "0 member cards under → 0 blade, got {}",
        blade_mod
    );
}

/// Member cards in hand / discard / other stage areas are NOT counted —
/// only cards physically under the member. (This is the location fix.)
#[test]
fn chika_constant_blade_counts_only_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chika = game.id("PL!SP-bp7-003-R＋");
    let kanon = game.id("PL!SP-sd1-001-SD");
    let sumire = game.id("PL!SP-sd1-004-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // 1 member under; other members on stage areas, in hand, and in discard.
    game.state.player1.stage.stage = [kanon, chika, sumire];
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.waitroom.cards.push(sumire);
    game.state.player1.main_deck.cards.push(filler);

    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, kanon);

    game.state.recalculate_constant_blade_modifiers();

    let blade_mod = game.state.mods.get_blade_modifier(chika);
    assert_eq!(
        blade_mod, 1,
        "1 member under + 4 elsewhere → 1 blade (elsewhere NOT counted), got {}",
        blade_mod
    );
}

// ====================================================================
// B1: PL!N-bp7-003-R＋ 桜坂しずく ab#1 — ライブ開始時, ライブ終了時まで,
// per DISTINCT-named member card under
// ====================================================================

/// 3 member cards under with 3 distinct names → 3 blade
#[test]
fn shizuku_live_start_blade_per_distinct_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shizuku = game.id("PL!N-bp7-003-R＋");
    game.state.player1.stage.stage = [-1, shizuku, -1];

    place_member_under(&mut game, MemberArea::Center, "PL!SP-sd1-001-SD"); // 澁谷かのん
    place_member_under(&mut game, MemberArea::Center, "PL!SP-sd1-004-SD"); // 平安名すみれ
    place_member_under(&mut game, MemberArea::Center, "PL!SP-sd1-003-SD"); // 嵐千砂都

    game.give_energy(3);
    seed_deck(&mut game);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    let blade_mod = game.state.mods.get_blade_modifier(shizuku);
    assert_eq!(
        blade_mod, 3,
        "3 distinct-named member cards under → 3 blade, got {}",
        blade_mod
    );
}

/// 2 copies of the same name + 1 different name under → 2 blade (dedup)
#[test]
fn shizuku_live_start_blade_dedups_same_name() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shizuku = game.id("PL!N-bp7-003-R＋");
    game.state.player1.stage.stage = [-1, shizuku, -1];

    place_member_under(&mut game, MemberArea::Center, "PL!SP-sd1-001-SD"); // 澁谷かのん
    place_member_under(&mut game, MemberArea::Center, "PL!SP-sd1-001-SD"); // 澁谷かのん again
    place_member_under(&mut game, MemberArea::Center, "PL!SP-sd1-004-SD"); // 平安名すみれ

    game.give_energy(3);
    seed_deck(&mut game);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    let blade_mod = game.state.mods.get_blade_modifier(shizuku);
    assert_eq!(
        blade_mod, 2,
        "2 same-name + 1 different-name member under → 2 blade (distinct names), got {}",
        blade_mod
    );
}

/// No member cards under → 0 blade
#[test]
fn shizuku_live_start_blade_zero_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shizuku = game.id("PL!N-bp7-003-R＋");
    game.state.player1.stage.stage = [-1, shizuku, -1];

    game.give_energy(3);
    seed_deck(&mut game);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    let blade_mod = game.state.mods.get_blade_modifier(shizuku);
    assert_eq!(
        blade_mod, 0,
        "0 member cards under → 0 blade, got {}",
        blade_mod
    );
}

/// Member cards in hand / discard are NOT counted — only under the member.
#[test]
fn shizuku_live_start_blade_counts_only_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shizuku = game.id("PL!N-bp7-003-R＋");
    let kanon = game.id("PL!SP-sd1-001-SD");
    let sumire = game.id("PL!SP-sd1-004-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // 1 member under; 2 other members in hand + 1 in discard.
    game.state.player1.stage.stage = [-1, shizuku, -1];
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(sumire);
    game.state.player1.waitroom.cards.push(kanon);
    game.state.player1.main_deck.cards.push(filler);

    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, kanon);

    game.give_energy(3);
    seed_deck(&mut game);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    let blade_mod = game.state.mods.get_blade_modifier(shizuku);
    assert_eq!(
        blade_mod, 1,
        "1 member under + 3 elsewhere → 1 blade (elsewhere NOT counted), got {}",
        blade_mod
    );
}
