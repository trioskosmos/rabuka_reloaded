/// Tests for PL!-sd1-005-SD 星空凛 — Activation ability (ab#0):
///
/// {{kidou.png|起動}}このメンバーをステージから控え室に置く：
/// 自分の控え室からライブカードを1枚手札に加える。
///
/// Cost: put this member from stage to waitroom (self_cost).
/// Effect: recover 1 live card from waitroom to hand.
///
/// Q123: Can you activate with no live cards in waitroom? A: Yes.
///       (Effect does nothing, but cost is still paid.)
/// Q79:  After activating, can a new member be placed in the vacated area? A: Yes.
use crate::helpers::*;

/// Q123: Activate ability when waitroom has no live cards — cost still paid.
#[test]
fn q123_self_cost_is_paid_without_recoverable_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let rin = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: 星空凛 at center
    game.state.player1.stage.stage = [-1, rin, -1];

    // Hand: a card for hand count (not relevant here)
    game.add_to_hand(filler);

    // No live cards in waitroom
    game.give_energy(3);

    let stage_before = game.state.player1.stage.stage[1];
    assert_eq!(
        stage_before, rin,
        "Rin should be on stage before activation"
    );

    game.activate_ability(rin);

    // Self_cost moved Rin off stage → stage center should be empty
    assert_eq!(
        game.state.player1.stage.stage[1], -1,
        "Rin should be removed from stage after self_cost"
    );

    // Rin should be in waitroom
    assert!(
        game.state.player1.waitroom.cards.contains(&rin),
        "Rin should be in waitroom after self_cost"
    );

    // No live card was recovered (waitroom had none), but ability succeeded
    // The effect gracefully did nothing
}

/// The sacrificed member is NOT a candidate here — the exact mirror of the member
/// variant.
///
/// Two abilities in this family share the self_cost and differ only in the recovery's
/// `card_type`, and that one field decides whether the cost's own card is eligible:
///
///   * 「…控え室から**メンバーカード**を1枚」 — the sacrificed member IS a member card,
///     so it qualifies and can be recovered.
///     (`self_to_waitroom_recover_member_q79_test.rs`)
///   * 「…控え室から**ライブカード**を1枚」 — the sacrificed member is not a live card,
///     so it cannot be, whatever else is in the waitroom. This file.
///
/// Pinning only the first would leave `card_type: live_card` unverified in the
/// direction that actually excludes something: a recovery that ignored the filter
/// would pick Rin out of a waitroom containing only her, and pass every other test in
/// this file.
///
/// With Rin and one live card in the waitroom there is exactly one candidate, so the
/// ability resolves without asking. `drain_choices_strict` makes a prompt a failure:
/// two candidates would mean Rin was offered.
#[test]
fn q123_the_sacrificed_member_is_not_a_candidate_for_the_live_recovery() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!-sd1-005-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [-1, rin, -1];
    game.add_to_discard(live_card);
    game.give_energy(3);

    game.activate_ability(rin);
    game.drain_choices_strict(&[], &[]);

    assert!(
        game.state.player1.hand.cards.contains(&live_card),
        "the live card is the only 『ライブカード』 in the waitroom and is recovered"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&rin),
        "Rin is a member, and 「ライブカードを1枚」 excludes her — she stays in the \
         waitroom; waitroom is {:?}",
        game.state.player1.waitroom.cards
    );
    assert!(
        !game.state.player1.hand.cards.contains(&rin),
        "and specifically does not come back to hand"
    );
}

/// Q79: After self_cost removes the member, the vacated area is empty and
/// can receive a new card.
#[test]
fn q79_self_cost_vacates_center_and_recovers_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let rin = game.id("PL!-sd1-005-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    // Stage: 星空凛 at center
    game.state.player1.stage.stage = [-1, rin, -1];

    // The live card goes in the WAITROOM and nowhere else.
    //
    // It used to be added to the hand as well, from the same `game.id()` call, which
    // put one card instance in two zones at once — and made the assertion below
    // unfalsifiable, since `hand.contains(&live_card)` was already true before the
    // ability ran. The comment claimed the hand copy was there "so waitroom has a
    // target for recovery", but the waitroom copy is the target; the hand copy only
    // guaranteed a pass.
    game.add_to_discard(live_card);
    game.give_energy(3);

    assert!(
        !game.state.player1.hand.cards.contains(&live_card),
        "fixture: the live card must start in the waitroom only, or the recovery \
         assertion below proves nothing"
    );

    game.activate_ability(rin);

    // Center stage should be empty after self_cost
    assert_eq!(
        game.state.player1.stage.stage[1], -1,
        "Center area should be empty after self_cost"
    );

    // Waitroom should have Rin (from self_cost) — live card was recovered
    assert!(
        game.state.player1.waitroom.cards.contains(&rin),
        "Rin should be in waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&live_card),
        "Live card should be recovered to hand"
    );

    // The vacated center area can receive a new card (empty slot exists)
}
