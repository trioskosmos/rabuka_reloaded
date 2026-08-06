/// BP07 CLEAN-G3: PL!S-bp7-005-R＋ 渡辺 曜 ab#1 (常時).
///
/// 常時：自分のステージにいる、メンバーカードが下に置かれている『Aqours』の
/// メンバーは、ブレードを得る。
///
/// (Constant) Aqours members on your stage THAT HAVE a member card placed under
/// them gain 1 blade. The under-card filter must gate the grant — Aqours members
/// with no member card underneath gain nothing, and the grant only applies to
/// Aqours members.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const WATANABE: &str = "PL!S-bp7-005-R＋"; // 渡辺 曜 (Aqours) — the ability card
const HOST_AQOURS: &str = "PL!S-bp7-006-R"; // 津島善子 (Aqours) — a host
const HOST_NON_AQOURS: &str = "PL!SP-sd1-001-SD"; // 澁谷かのん (Liella!, not Aqours)
const MEMBER_UNDER: &str = "PL!-sd1-001-SD"; // a generic member card to place under
const ENERGY_UNDER: &str = "LL-E-001-SD";

fn blade_of(game: &TestGame, id: i16) -> i32 {
    game.state.mods.get_blade_modifier(id)
}

/// Put 渡辺 曜 (the constant-ability source) plus `host` on stage, and place
/// `under` (if Some) under `host`'s area. `host_area` is where `host` sits.
fn setup(game: &mut TestGame, host: i16, host_area: MemberArea, under: Option<i16>) {
    // 渡辺 曜 at the opposite slot so the constant ability is active.
    let watanabe = game.id(WATANABE);
    let other_area = match host_area {
        MemberArea::LeftSide => MemberArea::RightSide,
        MemberArea::Center => MemberArea::RightSide,
        MemberArea::RightSide => MemberArea::LeftSide,
    };
    game.state.player1.stage.set_area(other_area, watanabe);
    game.state.player1.stage.set_area(host_area, host);
    if let Some(u) = under {
        game.state.player1.stage.place_under_card(host_area, u);
    }
}

/// An Aqours host WITH a member card underneath gains 1 blade.
#[test]
fn watanabe_aqours_host_with_member_under_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let host = game.id(HOST_AQOURS);
    let u = game.id(MEMBER_UNDER); setup(&mut game, host, MemberArea::Center, Some(u));

    game.state.recalculate_constant_blade_modifiers();

    assert_eq!(
        blade_of(&game, host),
        1,
        "Aqours host with a member card underneath → 1 blade, got {}",
        blade_of(&game, host)
    );
}

/// An Aqours host WITHOUT any card underneath gains nothing.
#[test]
fn watanabe_aqours_host_without_under_gains_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let host = game.id(HOST_AQOURS);
    setup(&mut game, host, MemberArea::Center, None);

    game.state.recalculate_constant_blade_modifiers();

    assert_eq!(
        blade_of(&game, host),
        0,
        "Aqours host with nothing underneath → 0 blade, got {}",
        blade_of(&game, host)
    );
}

/// An Aqours host with only an ENERGY card underneath gains nothing (the text
/// requires a MEMBER card under).
#[test]
fn watanabe_aqours_host_with_energy_under_gains_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let host = game.id(HOST_AQOURS);
    let u = game.id(ENERGY_UNDER); setup(&mut game, host, MemberArea::Center, Some(u));

    game.state.recalculate_constant_blade_modifiers();

    assert_eq!(
        blade_of(&game, host),
        0,
        "energy under the host does NOT count (needs a member card) → 0 blade, got {}",
        blade_of(&game, host)
    );
}

/// A NON-Aqours host with a member card underneath gains nothing (group scope).
#[test]
fn watanabe_non_aqours_host_with_member_under_gains_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let host = game.id(HOST_NON_AQOURS);
    let u = game.id(MEMBER_UNDER); setup(&mut game, host, MemberArea::Center, Some(u));

    game.state.recalculate_constant_blade_modifiers();

    assert_eq!(
        blade_of(&game, host),
        0,
        "non-Aqours host → 0 blade, got {}",
        blade_of(&game, host)
    );
}

/// 渡辺 曜 herself on stage with no card under → no self-grant.
#[test]
fn watanabe_self_no_under_gains_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let watanabe = game.id(WATANABE);
    game.state.player1.stage.stage[1] = watanabe;

    game.state.recalculate_constant_blade_modifiers();

    assert_eq!(
        blade_of(&game, watanabe),
        0,
        "渡辺 曜 with no card under → 0 blade, got {}",
        blade_of(&game, watanabe)
    );
}

/// Two Aqours hosts each with a member card under → BOTH gain 1 blade.
#[test]
fn watanabe_two_hosts_with_members_under_both_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let watanabe = game.id(WATANABE);
    let a = game.id(HOST_AQOURS);
    let b = game.id("PL!S-bp7-004-R"); // 黒澤ダイヤ (Aqours)
    game.state.player1.stage.stage = [watanabe, a, b];
    game.state.player1.stage.place_under_card(MemberArea::Center, game.id(MEMBER_UNDER));
    game.state.player1.stage.place_under_card(MemberArea::RightSide, game.id(MEMBER_UNDER));

    game.state.recalculate_constant_blade_modifiers();

    assert_eq!(blade_of(&game, a), 1, "center Aqours host → 1 blade");
    assert_eq!(blade_of(&game, b), 1, "right Aqours host → 1 blade");
}
