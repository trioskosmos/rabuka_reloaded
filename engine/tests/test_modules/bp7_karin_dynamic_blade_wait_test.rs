/// BP07 parser/engine fix C5: `PL!N-bp7-004-R` / `PL!N-bp7-004-P` 朝香果林 ab#0 (起動).
///
/// 起動[ターン1回]エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く：
/// 相手のステージにいる、元々持つブレードの数がこのメンバーの下にあるエネルギー
/// カードの枚数に1を足した数以下のメンバー1人をウェイトにする。
///
/// "起動(turn 1): place 1 energy from the energy zone under this member: wait 1
/// opponent member whose ORIGINAL blade count ≤ (energy cards under this
/// member) + 1."
///
/// The defect (C5): the cost was parsed with card_type=member_card and no source,
/// and the effect was parsed as `place_energy_under_member` instead of
/// `change_state(wait)` — the dynamic blade limit
/// `(energy under this member) + 1` (≤, original value) was dropped entirely.
///
/// These tests pin the behavior as written: the cost moves exactly one energy
/// from the energy zone under 朝香果林, then exactly one opponent member whose
/// ORIGINAL (printed) blade is at most `(energy under her) + 1` is waited.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const ENERGY: &str = "LL-E-001-SD";

/// 朝香果林, the activating member (blade 4). Center area = under_cards[1].
fn place_karin(game: &mut TestGame) -> i16 {
    let karin = game.id("PL!N-bp7-004-P");
    game.add_to_stage(MemberArea::Center, karin);
    karin
}

/// Pre-place `n` energy cards under 朝香果林 (center slot 1) to control the
/// dynamic threshold = (energy under) + 1 at effect resolution.
fn seed_energy_under(game: &mut TestGame, karin: i16, n: usize) {
    let center_idx = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .position(|&id| id == karin)
        .expect("karin on stage");
    for _ in 0..n {
        game.state.player1.stage.under_cards[center_idx].push(game.id(ENERGY));
    }
}

/// Put an opponent member on P2's stage (center).
fn place_opponent(game: &mut TestGame, card_no: &str) -> i16 {
    let id = game.id(card_no);
    game.state.player2.stage.stage[1] = id;
    id
}

/// Activate 朝香果林's 起動 ability and drain all pending choices. Returns the
/// number of energy cards under her afterwards.
fn activate_karin(game: &mut TestGame, karin: i16) -> usize {
    game.give_energy(1);
    game.activate_ability(karin);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        game.select_indices(&[0]);
    }
    let center_idx = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .position(|&id| id == karin)
        .expect("karin on stage");
    game.state.player1.stage.under_cards[center_idx].len()
}

// ====================================================================
// Cost behavior
// ====================================================================

/// The cost moves exactly one energy from the energy zone under 朝香果林.
#[test]
fn c5_cost_moves_one_energy_from_zone_under_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = place_karin(&mut game);
    seed_energy_under(&mut game, karin, 1);
    let opp = place_opponent(&mut game, "PL!-sd1-002-SD"); // 絢瀬 絵里, blade 1
    game.give_energy(1);
    let energy_zone_before = game.state.player1.energy_zone.cards.len();
    game.activate_ability(karin);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        game.select_indices(&[0]);
    }

    let under_after = game.state.player1.stage.under_cards[1].len();

    assert_eq!(
        under_after,
        2,
        "exactly one energy is placed under 朝香果林 (1 seeded + 1 cost)"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_zone_before - 1,
        "the cost energy comes from the energy zone"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(opp),
        Some("wait"),
        "opponent within the dynamic blade limit should be waited"
    );
}

// ====================================================================
// Dynamic blade limit = (energy under 朝香果林) + 1, original blade
// ====================================================================

/// 0 seeded + 1 cost = 1 under → threshold 2. Opponent blade 3 is NOT waited.
#[test]
fn c5_opponent_over_limit_not_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = place_karin(&mut game);
    let sumire = place_opponent(&mut game, "PL!SP-PR-024-PR"); // 平安名すみれ, blade 3
    activate_karin(&mut game, karin);

    assert!(
        game.state
            .mods
            .get_orientation_modifier(sumire)
            .is_none(),
        "blade-3 member must NOT be waited when threshold is 2 (1 under + 1)"
    );
}

/// 0 seeded + 1 cost = 1 under → threshold 2. Opponent blade 2 is waited.
#[test]
fn c5_boundary_exact_blade_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = place_karin(&mut game);
    let chisato = place_opponent(&mut game, "PL!SP-pb1-014-PR"); // 嵐 千砂都, blade 2
    activate_karin(&mut game, karin);

    assert_eq!(
        game.state.mods.get_orientation_modifier(chisato),
        Some("wait"),
        "exactly-(threshold) original blade is within the limit"
    );
}

/// 1 seeded + 1 cost = 2 under → threshold 3. Opponent blade 3 IS waited.
#[test]
fn c5_threshold_scales_with_energy_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = place_karin(&mut game);
    seed_energy_under(&mut game, karin, 1);
    let sumire = place_opponent(&mut game, "PL!SP-PR-024-PR"); // 平安名すみれ, blade 3
    activate_karin(&mut game, karin);

    assert_eq!(
        game.state.mods.get_orientation_modifier(sumire),
        Some("wait"),
        "threshold scales with energy under: 2 under → ≤3 blades"
    );
}

/// The limit counts ORIGINAL (元々持つ) blade — a modifier that raises the
/// current blade does not disqualify the member.
#[test]
fn c5_uses_original_blade_not_current() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = place_karin(&mut game);
    let chisato = place_opponent(&mut game, "PL!SP-pb1-014-PR"); // 嵐 千砂都, printed blade 2
    game.state.mods.add_blade_modifier(chisato, 1); // current = 3
    activate_karin(&mut game, karin);

    assert_eq!(
        game.state.mods.get_orientation_modifier(chisato),
        Some("wait"),
        "original blade 2 ≤ threshold 2, current 3 modifier is ignored (元々持つ)"
    );
}

/// Blade under-1 members are the minimum eligible group; only ORIGINAL value.
#[test]
fn c5_original_blade_zero_plus_modifier_still_ineligible_if_original_over() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = place_karin(&mut game);
    let sumire = place_opponent(&mut game, "PL!SP-PR-024-PR"); // 平安名すみれ, printed blade 3
    game.state.mods.add_blade_modifier(sumire, -2); // current = 1
    activate_karin(&mut game, karin);

    assert!(
        game.state
            .mods
            .get_orientation_modifier(sumire)
            .is_none(),
        "printed blade 3 exceeds threshold 2 even though the current total is 1"
    );
}

// ====================================================================
// Targeting: exactly 1 member
// ====================================================================

/// Multiple opponents within the limit → prompt to choose exactly one.
#[test]
fn c5_multiple_candidates_selects_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = place_karin(&mut game);
    seed_energy_under(&mut game, karin, 1); // threshold 3
    let eri = game.id("PL!-sd1-002-SD"); // 絢瀬 絵里, blade 1
    let sumire = game.id("PL!SP-PR-024-PR"); // 平安名すみれ, blade 3
    game.state.player2.stage.stage[0] = eri;
    game.state.player2.stage.stage[1] = sumire;
    game.give_energy(1);
    game.activate_ability(karin);

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "two eligible opponents require a selection prompt"
    );
    let count = game.pending_choice_count();
    assert_eq!(count, 1, "exactly one member should be selected");
    game.select_indices(&[0]);

    let eri_wait = game.state.mods.get_orientation_modifier(eri) == Some("wait");
    let sumire_wait = game.state.mods.get_orientation_modifier(sumire) == Some("wait");
    assert!(
        eri_wait || sumire_wait,
        "the chosen member should be waited"
    );
    assert!(
        !(eri_wait && sumire_wait),
        "only ONE member may be waited"
    );
}

/// A single eligible opponent → auto-waited, no prompt.
#[test]
fn c5_single_candidate_auto_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = place_karin(&mut game);
    let eri = place_opponent(&mut game, "PL!-sd1-002-SD"); // 絢瀬 絵里, blade 1
    game.give_energy(1);
    game.activate_ability(karin);

    // Only 1 eligible candidate → the effect resolves without a prompt.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(eri),
        Some("wait"),
        "single eligible opponent should be auto-waited"
    );
}

/// No opponent within the limit → nothing is waited, no prompt.
#[test]
fn c5_no_eligible_opponent_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = place_karin(&mut game);
    let sumire = place_opponent(&mut game, "PL!SP-PR-024-PR"); // 平安名すみれ, blade 3
    game.give_energy(1);
    game.activate_ability(karin); // threshold 2, blade 3 > 2

    assert!(
        game.state
            .mods
            .get_orientation_modifier(sumire)
            .is_none(),
        "no opponent within the limit should be waited"
    );
    assert!(
        !game.has_pending_choice(),
        "no selection prompt when nothing is eligible"
    );
}
