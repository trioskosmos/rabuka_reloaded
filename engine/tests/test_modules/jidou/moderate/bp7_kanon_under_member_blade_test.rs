/// BP07 parser fix B3: PL!SP-bp7-001-R 澁谷かのん ab#0.
///
///  常時：このカードが『Liella!』のメンバーの下に置かれているかぎり、そのメンバーは
/// ブレードを得る。
///
/// (Continuous) As long as THIS card (澁谷かのん) is placed under a Liella!
/// member, THAT member gains a blade.
///
/// This card is normally placed under another member by her own ab#1 (自動, baton
/// touch): when she leaves the stage while having baton-touched, she is placed
/// under the member that arrived via that baton-touch. The scenario tested here is
/// the end state of those two abilities together: 澁谷かのん is under a Liella!
/// host, so the host gains a blade.
///
/// The parser defect: the `condition` node lacked `location: "under_member"`, so the
/// "…の下に置かれているかぎり" scope was dropped and the blade could apply even when
/// the card was NOT under a member. These tests pin the host-granting behavior.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Put 澁谷かのん (the bp7 ability card) UNDER the member at `area`.
fn place_kanon_under(game: &mut TestGame, area: MemberArea) -> i16 {
    let kanon = game.id("PL!SP-bp7-001-R");
    game.state.player1.stage.place_under_card(area, kanon);
    kanon
}

// ====================================================================
// ab#0 — the host member (the one 澁谷かのん is under) gains 1 blade
// ====================================================================

/// 澁谷かのん under a Liella! host (平安名すみれ) → host gains 1 blade.
#[test]
fn kanon_under_liella_host_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Host = 平安名すみれ, a Liella! member on center stage.
    let host = game.id("PL!SP-sd1-004-SD");
    game.state.player1.stage.stage = [-1, host, -1];

    place_kanon_under(&mut game, MemberArea::Center);

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(host);
    assert_eq!(
        blade_mod, 1,
        "host with 澁谷かのん underneath → 1 blade, got {}",
        blade_mod
    );
}

/// 澁谷かのん on the stage (not under anyone) → no blade granted.
#[test]
fn kanon_on_stage_grants_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let host = game.id("PL!SP-sd1-004-SD");
    let kanon = game.id("PL!SP-bp7-001-R");
    // 澁谷かのん herself on stage, host on stage; neither is under the other.
    game.state.player1.stage.stage = [kanon, host, -1];

    game.state.recalculate_constants();

    let host_blade = game.state.mods.get_blade_modifier(host);
    let kanon_blade = game.state.mods.get_blade_modifier(kanon);
    assert_eq!(
        host_blade, 0,
        "host not underneath anything → 0 blade, got {}",
        host_blade
    );
    assert_eq!(
        kanon_blade, 0,
        "澁谷かのん on stage (not under a member) → 0 blade, got {}",
        kanon_blade
    );
}

/// 澁谷かのん under a NON-Liella member (Aqours 津島善子) → no blade (group scope).
#[test]
fn kanon_under_non_liella_host_grants_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Host = Aqours 津島善子 (not Liella!).
    let host = game.id("PL!S-sd1-015-SD");
    game.state.player1.stage.stage = [-1, host, -1];

    place_kanon_under(&mut game, MemberArea::Center);

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(host);
    assert_eq!(
        blade_mod, 0,
        "host not a Liella! member → 0 blade, got {}",
        blade_mod
    );
}

/// No card under the host at all → 0 blade.
#[test]
fn kanon_absent_grants_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let host = game.id("PL!SP-sd1-004-SD");
    game.state.player1.stage.stage = [-1, host, -1];

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(host);
    assert_eq!(
        blade_mod, 0,
        "nothing under host → 0 blade, got {}",
        blade_mod
    );
}

/// Two copies of 澁谷かのん under the SAME Liella! host → host gains 2 blade
/// (each copy's constant ability is independent).
#[test]
fn two_kanon_copies_under_one_liella_host() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let host = game.id("PL!SP-sd1-004-SD");
    game.state.player1.stage.stage = [-1, host, -1];

    let k1 = game.id("PL!SP-bp7-001-R");
    let k2 = game.id("PL!SP-bp7-001-R");
    game.state.player1.stage.place_under_card(MemberArea::Center, k1);
    game.state.player1.stage.place_under_card(MemberArea::Center, k2);

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(host);
    assert_eq!(
        blade_mod, 2,
        "2 copies of 澁谷かのん under one Liella! host → 2 blade, got {}",
        blade_mod
    );
}

/// 澁谷かのん under one host in one slot; the OTHER (empty) slot host gets no
/// blade — the grant is scoped to the very member she is stacked under.
#[test]
fn kanon_under_center_does_not_leak_to_other_slots() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let center_host = game.id("PL!SP-sd1-004-SD");
    let right_host = game.id("PL!SP-sd1-001-SD");
    game.state.player1.stage.stage = [-1, center_host, right_host];

    place_kanon_under(&mut game, MemberArea::Center);

    game.state.recalculate_constants();

    let center_blade = game.state.mods.get_blade_modifier(center_host);
    let right_blade = game.state.mods.get_blade_modifier(right_host);
    assert_eq!(
        center_blade, 1,
        "center host with 澁谷かのん under → 1 blade, got {}",
        center_blade
    );
    assert_eq!(
        right_blade, 0,
        "right host (some Li' but nothing under) → 0 blade, got {}",
        right_blade
    );
}

/// 澁谷かのん in HAND (not stacked under any member) → contributes nothing.
#[test]
fn kanon_in_hand_contributes_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let host = game.id("PL!SP-sd1-004-SD");
    game.state.player1.stage.stage = [-1, host, -1];

    let kanon = game.id("PL!SP-bp7-001-R");
    game.state.player1.hand.cards.push(kanon);

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(host);
    assert_eq!(
        blade_mod, 0,
        "host with 澁谷かのん only in hand → 0 blade, got {}",
        blade_mod
    );
}

/// Two Li' hosts each with their own 澁谷かのん underneath → BOTH get +1.
#[test]
fn kanon_copies_under_two_liella_hosts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let center_host = game.id("PL!SP-sd1-004-SD");
    let right_host = game.id("PL!SP-sd1-001-SD");
    game.state.player1.stage.stage = [-1, center_host, right_host];

    let ka = game.id("PL!SP-bp7-001-R");
    let kb = game.id("PL!SP-bp7-001-R");
    game.state.player1.stage.place_under_card(MemberArea::Center, ka);
    game.state.player1.stage.place_under_card(MemberArea::RightSide, kb);

    game.state.recalculate_constants();

    let center_blade = game.state.mods.get_blade_modifier(center_host);
    let right_blade = game.state.mods.get_blade_modifier(right_host);
    assert_eq!(
        center_blade, 1,
        "center host with 澁谷かのん under → 1 blade, got {}",
        center_blade
    );
    assert_eq!(
        right_blade, 1,
        "right host with 澁谷かのん under → 1 blade, got {}",
        right_blade
    );
}

/// 澁谷かのん under an OPPONENT (player2) Liella! host → that host also gains
/// a blade. The constant effect is not player-scoped: "そのメンバー" is whoever
/// she is stacked under.
#[test]
fn kanon_under_opponent_liella_host_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let p2_host = game.id("PL!SP-sd1-004-SD");
    game.state.player2.stage.stage = [-1, p2_host, -1];

    let kanon = game.id("PL!SP-bp7-001-R");
    game.state.player2.stage.place_under_card(MemberArea::Center, kanon);

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(p2_host);
    assert_eq!(
        blade_mod, 1,
        "opponent host with 澁谷かのん under → 1 blade, got {}",
        blade_mod
    );
}

/// Dynamic removal: place 澁谷かのん under a Liella! host (→1 blade), then she
/// leaves (recycled). Recalculation must REMOVE the blade — no stale modifier.
#[test]
fn kanon_leaving_removes_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let host = game.id("PL!SP-sd1-004-SD");
    game.state.player1.stage.stage = [-1, host, -1];

    let kanon = game.id("PL!SP-bp7-001-R");
    game.state.player1.stage.place_under_card(MemberArea::Center, kanon);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(host),
        1,
        "with 澁谷かのん under → 1 blade"
    );

    // 澁谷かのん leaves the stage with her host (under-cards are recycled).
    let (_, _) = game
        .state
        .player1
        .stage
        .recycle_under_cards(MemberArea::Center, &game.db);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(host),
        0,
        "after 澁谷かのん leaves → 0 blade (no stale modifier)"
    );
}

// ====================================================================
// Full flow of BOTH abilities (ab#1 自動 + ab#0 常時)
// ====================================================================
// ab#1: 自動 — このメンバーがステージから控え室に置かれたとき、バトンタッチして
//   いた場合、このカードをそのバトンタッチで登場したメンバーの下に置く。
//   (When this member goes stage→waitroom and was baton-touched, place this card
//    under the member that appeared via that baton-touch.)
// ab#0: 常時 — このカードが『Liella!』のメンバーの下に置かれているかぎり、
//   そのメンバーはブレードを得る。 (While under a Liella! member, that host gains
//   a blade.)
//
// Full scenario: baton-touch a Liella! arriver over 澁谷かのん → ab#1 places her
// under the arriver → ab#0 grants the arriver a blade.

/// Play `arriver` onto 澁谷かのん's occupied center area (baton-touch), draining
/// any auto-ability prompts.
fn baton_touch_over(game: &mut TestGame, kanon: i16, arriver: i16) {
    let _ = kanon;
    game.state.player1.hand.cards.push(arriver);
    game.play_to_stage(arriver, MemberArea::Center);
    while game.has_pending_choice() {
        // Default: skip any optional choice; required discards pick the first card.
        let required = game.state.get_pending_choice().is_some_and(|c| {
            matches!(
                c,
                rabuka_engine::ability::types::Choice::SelectCard {
                    count: 1,
                    allow_skip: false,
                    ..
                }
            )
        });
        if required {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
    }
}

#[test]
fn kanon_full_flow_baton_touch_places_under_and_grants_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // 澁谷かのん on center stage.
    let kanon = game.id("PL!SP-bp7-001-R");
    game.state.player1.stage.stage[1] = kanon;

    // Filler deck so debut "draw" prompts resolve cleanly.
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(25);

    // Baton-touch a Liella! arriver (平安名すみれ) over 澁谷かのん.
    let arriver = game.id("PL!SP-sd1-004-SD");
    baton_touch_over(&mut game, kanon, arriver);

    assert_eq!(
        game.state.player1.stage.stage[1], arriver,
        "arriver should occupy center after baton-touch"
    );

    // ab#1: 澁谷かのん should now be UNDER the arriver, not sitting in the waitroom.
    let under = game.state.player1.stage.get_under_cards(MemberArea::Center);
    assert!(
        under.contains(&kanon),
        "ab#1 should place 澁谷かのん under the arriving member; under={:?}",
        under
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&kanon),
        "澁谷かのん must be under the arriver (ab#1), not in the waitroom"
    );

    // ab#0: the Liella! arriver host now gains a blade.
    game.state.recalculate_constants();
    let blade_mod = game.state.mods.get_blade_modifier(arriver);
    assert_eq!(
        blade_mod, 1,
        "host Liella! member with 澁谷かのん underneath → 1 blade, got {}",
        blade_mod
    );
}
