/// Q266 — 鬼塚夏美 PL!SP-pb2-009 ab#0 (登場/ライブ開始時).
///
/// 登場/ライブ開始時：『Liella!』のメンバー1人をウェイトにしてもよい：相手のステージにいる
/// 元々持つブレードの数がこれによりウェイトにしたメンバーが元々持つブレードの数より2つ以上
/// 少ないメンバー1人をウェイトにする。
///
/// Official QA Q266: paying the wait cost with a 0-blade Liella! member makes it impossible
/// to wait even a 0-blade opponent member, because the opponent must have original blade
/// <= (costed member's original blade − 2). No legal target exists.
///
/// So the wait target's ORIGINAL blade must be at most (costed member's original blade − 2).
/// We exercise the real boundary with real cards (blade-0 members exist, e.g. 澁谷かのん
/// PL!SP-PR-003-PR):
///   costed B=0 → target must be <= −2 → NO opponent waitable (Q266 exact case)
///   costed B=1 → target must be <= −1 → NO opponent waitable
///   costed B=2 → target must be <= 0  → blade-0 waitable, blade-1 NOT
///   costed B=4 → target must be <= 2  → blade-1/2 waitable, blade-3 NOT
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const NATSUMI: &str = "PL!SP-pb2-009-PP"; // 鬼塚夏美 — Liella, the effect owner
// Liella! members with known ORIGINAL blades, used to pay the wait cost.
const COST_B0: &str = "PL!SP-PR-003-PR"; // 澁谷かのん, blade 0
const COST_B1: &str = "PL!SP-bp1-016-PR"; // 葉月 恋, blade 1
const COST_B2: &str = "PL!SP-bp1-004-PR"; // 平安名すみれ, blade 2
const COST_B4: &str = "PL!SP-pb1-001-PR"; // 澁谷かのん, blade 4
// Opponent members with known ORIGINAL blades (μ's).
const OPP_B0: &str = "PL!-sd1-008-SD"; // 小泉 花陽, blade 0
const OPP_B1: &str = "PL!-sd1-010-SD"; // 高坂 穂乃果, blade 1
const OPP_B2: &str = "PL!-sd1-006-SD"; // 西木野 真姫, blade 2
const OPP_B3: &str = "PL!-sd1-001-SD"; // 高坂 穂乃果, blade 3

fn opponent_waited(game: &TestGame, id: i16) -> bool {
    game.state.mods.get_orientation_modifier(id).as_deref() == Some("wait")
}

/// Place 鬼塚夏美 on p1 center and an optional-costable Liella! member on p1 stage
/// so the wait cost can be paid. Returns (natsumi, cost_member).
fn setup(game: &mut TestGame, cost_member: &str, opp_ids: &[i16; 3]) -> (i16, i16) {
    let natsumi = game.id(NATSUMI);
    let cost = game.id(cost_member);
    // Costed Liella! member already on p1 stage (waited as the cost).
    game.state.player1.stage.stage[0] = cost;
    // 鬼塚夏美 in hand, played during run_debut (plays to center, triggering 登場).
    game.add_to_hand(natsumi);
    game.give_energy(8);
    for (i, &id) in opp_ids.iter().enumerate() {
        game.state.player2.stage.stage[i] = id;
    }
    (natsumi, cost)
}

/// Trigger 鬼塚夏美's 登場 ability and drive the optional wait cost + target wait.
/// `want_cost` = whether to actually pay the cost (wait the Liella! member).
fn run_debut(game: &mut TestGame, natsumi: i16, want_cost: bool) {
    game.play_to_stage(natsumi, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 25 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            // The optional wait cost is a pay/skip choice. Index 1 = pay, index 0 = skip.
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "pay_optional_cost:skip_optional_cost" =>
            {
                if want_cost {
                    game.select_choice_option(1); // pay: wait the Liella! member
                } else {
                    game.select_choice_option(0); // skip the optional cost
                }
            }
            // Then pick which Liella! member to wait (SelectCard), or the wait target.
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            _ => {
                game.select_choice_option(0);
            }
        }
    }
}

// ====================================================================
// costed B=0 → target must be <= −2 → nothing is waitable (Q266 exact)
// ====================================================================
#[test]
fn q266_cost_b0_waits_nothing_incl_blade0() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let b0 = game.id(OPP_B0); // blade 0, but needs <= -2 → not waitable
    let (natsumi, _cost) = setup(&mut game, COST_B0, &[b0, -1, -1]);
    run_debut(&mut game, natsumi, true);

    assert!(
        !opponent_waited(&game, b0),
        "costed 0-blade member → target needs <= -2; a 0-blade opponent must NOT be waited (Q266)"
    );
}

// ====================================================================
// costed B=1 → target must be <= −1 → nothing waitable
// ====================================================================
#[test]
fn q266_cost_b1_waits_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let b0 = game.id(OPP_B0);
    let (natsumi, _cost) = setup(&mut game, COST_B1, &[b0, -1, -1]);
    run_debut(&mut game, natsumi, true);

    assert!(
        !opponent_waited(&game, b0),
        "costed 1-blade member → target needs <= -1; a 0-blade opponent must NOT be waited"
    );
}

// ====================================================================
// costed B=2 → target must be <= 0 → blade-0 waitable, blade-1 NOT
// ====================================================================
#[test]
fn q266_cost_b2_waits_blade0_not_blade1() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let b0 = game.id(OPP_B0); // blade 0 <= 0 → waitable
    let b1 = game.id(OPP_B1); // blade 1 > 0 → NOT waitable
    let (natsumi, _cost) = setup(&mut game, COST_B2, &[b0, b1, -1]);
    run_debut(&mut game, natsumi, true);

    assert!(opponent_waited(&game, b0), "blade-0 opponent (<= 0) should be waited");
    assert!(
        !opponent_waited(&game, b1),
        "blade-1 opponent (> 0) must NOT be waited"
    );
}

// ====================================================================
// costed B=4 → target must be <= 2 → blade-1/2 waitable, blade-3 NOT
// ====================================================================
#[test]
fn q266_cost_b4_waits_blade2_not_blade3() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let b2 = game.id(OPP_B2); // blade 2 <= 2 → waitable
    let b3 = game.id(OPP_B3); // blade 3 > 2 → NOT waitable
    let (natsumi, _cost) = setup(&mut game, COST_B4, &[b2, b3, -1]);
    run_debut(&mut game, natsumi, true);

    assert!(opponent_waited(&game, b2), "blade-2 opponent (<= 2) should be waited");
    assert!(
        !opponent_waited(&game, b3),
        "blade-3 opponent (> 2) must NOT be waited"
    );
}

// ====================================================================
// Skipping the optional cost → nothing is waited
// ====================================================================
#[test]
fn q266_skip_cost_waits_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let b0 = game.id(OPP_B0);
    let (natsumi, _cost) = setup(&mut game, COST_B1, &[b0, -1, -1]);
    run_debut(&mut game, natsumi, false);

    assert!(
        !opponent_waited(&game, b0),
        "when the optional wait cost is declined, no opponent is waited"
    );
}
