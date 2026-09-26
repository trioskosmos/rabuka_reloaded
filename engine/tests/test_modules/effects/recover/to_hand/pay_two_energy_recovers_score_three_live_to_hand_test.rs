/// 日野下花帆 PL!HS-bp2-001-R ab#0 (起動):
///
/// {{kidou.png/起動}}{{turn1.png/ターン1回}}{{icon_energy.png/E}}{{icon_energy.png/E}}：
/// 自分の控え室からスコア3以下の『蓮ノ空』のライブカードを1枚手札に加える。
///
/// (Activation, once per turn, costs 2 energy: add 1 『蓮ノ空』 live card of
/// score 3 or less from your waitroom to your hand.)
///
/// These five used to be `assert!(true)` placeholders whose only purpose was to
/// raise the L0 coverage number — the file said so. Each one now pins the rule
/// its name claims: the cost, the score ceiling, the group filter, the
/// ターン1回 limit, and the two ways a recovery can fail to happen.
use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};

const HANANO: &str = "PL!HS-bp2-001-R"; // 日野下花帆 — the 起動 source
const DREAM_BELIEVERS: &str = "PL!HS-bp1-019-L"; // 蓮ノ空 live, score 1 → eligible
const AURORA_FLOWER: &str = "PL!HS-bp5-018-L"; // score 7 → excluded by SCORE

/// The card under test, on stage, with identity pinned. PL!HS-bp1-019-L and
/// the other HS-019 prints are one bp number apart, and 「スコア3以下」 is a
/// score gate, so both the print and its score have to be the intended ones.
fn setup() -> (TestGame, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanano = game.id(HANANO);
    game.assert_card_identity(hanano, HANANO);
    game.state.player1.stage.stage[1] = hanano;
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    (game, hanano)
}

fn use_offers(game: &TestGame, cid: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
        })
        .count()
}

/// Answer the recover prompt if the engine raised one. With a single eligible
/// card the engine auto-aims and asks nothing — which is the interesting fact
/// this file pins — so both drains may legitimately find nothing to do.
///
/// The recover itself is a NON-SKIPPABLE SelectCard, so it must be answered with
/// a selection; `drain_choices_strict` rejects an empty answer there, which is
/// the strictness the SOFT_GUARD ratchet wants (the guarded form silently
/// absorbs a missing prompt — exactly the failure this file exists to catch).
fn accept_recover(game: &mut TestGame) {
    game.drain_choices_strict(&["SelectCard"], &[0]);
    game.drain_choices_strict(&["SelectAutoAbility"], &[]);
}

/// Happy path: the cost is paid, the eligible live leaves the waitroom for hand.
#[test]
fn hs_bp2_001_pay_2e_recovers_score3_live() {
    let (mut game, hanano) = setup();
    let live = game.id(DREAM_BELIEVERS);
    game.assert_card_identity(live, DREAM_BELIEVERS);
    game.assert_card_score(live, 1);
    game.state.player1.waitroom.cards.push(live);
    game.give_energy(3);

    assert_eq!(use_offers(&game, hanano), 1, "the 起動 must be offered");
    assert!(
        game.try_activate_ability(hanano).is_ok(),
        "activation must succeed with 2 energy available"
    );
    accept_recover(&mut game);

    assert!(
        game.state.player1.hand.cards.contains(&live),
        "スコア3以下の『蓮ノ空』のライブカードを1枚手札に加える"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "the recovered card left the waitroom"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        1,
        "E E — exactly two energy were paid"
    );
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(hanano, 0, game.state.turn_number)),
        "ターン1回 use recorded"
    );
}

/// The recover is NOT optional — the printed text carries no てもよい, so
/// paying E E must recover the one eligible live with no prompt at all.
/// (An earlier version of this test assumed a decline path existed and proved
/// nothing: an empty selection on a single candidate accepts it.)
#[test]
fn hs_bp2_001_recover_is_mandatory_once_the_cost_is_paid() {
    let (mut game, hanano) = setup();
    let live = game.id(DREAM_BELIEVERS);
    game.state.player1.waitroom.cards.push(live);
    game.give_energy(3);

    game.try_activate_ability(hanano).expect("activation");

    assert!(
        !game.has_pending_choice(),
        "one eligible card and no てもよい → the recover resolves on its own"
    );
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "paying E E must recover the eligible live"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        1,
        "E E — exactly two energy were paid"
    );
}

/// ターン1回 — the second activation in the same turn is neither offered nor
/// accepted, and nothing is recovered a second time.
#[test]
fn hs_bp2_001_turn1_blocks_second() {
    let (mut game, hanano) = setup();
    let live1 = game.id(DREAM_BELIEVERS);
    let live2 = game.new_id(DREAM_BELIEVERS);
    game.assert_same_card_name(live1, live2, "two copies of the same live");
    game.state.player1.waitroom.cards.push(live1);
    game.state.player1.waitroom.cards.push(live2);
    game.give_energy(5);

    game.try_activate_ability(hanano).expect("first activation");
    accept_recover(&mut game);
    let hand_after_first = game.state.player1.hand.cards.len();
    assert!(
        hand_after_first >= 1,
        "the first activation must actually recover something"
    );

    assert_eq!(
        use_offers(&game, hanano),
        0,
        "ターン1回 — the 起動 must not be offered again this turn"
    );
    assert!(
        game.try_activate_ability(hanano).is_err(),
        "the second activation must be refused"
    );
    game.drain_choices_strict(&["SelectAutoAbility", "SelectCard"], &[]);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_after_first,
        "a refused second activation recovers nothing more"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live2),
        "the second live is still in the waitroom"
    );
}

/// E E is the cost: with one energy the 起動 is not offered and the card is
/// refused outright — no recover, no energy spent.
#[test]
fn hs_bp2_001_insufficient_energy_no_recover() {
    let (mut game, hanano) = setup();
    let live = game.id(DREAM_BELIEVERS);
    game.state.player1.waitroom.cards.push(live);
    game.give_energy(1);

    assert_eq!(use_offers(&game, hanano), 0, "1 energy cannot pay E E");
    assert!(
        game.try_activate_ability(hanano).is_err(),
        "activation must be refused, not silently resolve"
    );
    game.drain_choices_strict(&["SelectAutoAbility", "SelectCard"], &[]);
    assert_eq!(
        game.state.player1.waitroom.cards.as_slice(),
        &[live],
        "nothing was recovered"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        1,
        "a refused activation spends no energy"
    );
}

/// スコア3以下 is a real gate: the only waitroom live scores 7, so it stays.
/// (AURORA FLOWER is also 『蓮ノ空』 via its set_card_identity 常時 — it is the
/// SCORE, not the group, that excludes it, which is what this pins.)
#[test]
fn hs_bp2_001_no_eligible_live_no_recover() {
    let (mut game, hanano) = setup();
    let live_high = game.id(AURORA_FLOWER);
    game.assert_card_identity(live_high, AURORA_FLOWER);
    game.assert_card_score(live_high, 7);
    game.state.player1.waitroom.cards.push(live_high);
    game.give_energy(3);

    game.try_activate_ability(hanano).expect("activation");
    game.drain_choices_strict(&["SelectAutoAbility", "SelectCard"], &[]);

    assert!(
        !game.state.player1.hand.cards.contains(&live_high),
        "スコア7 is above the スコア3 ceiling, so it must not be recovered"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.as_slice(),
        &[live_high],
        "the ineligible live stayed in the waitroom"
    );
}
