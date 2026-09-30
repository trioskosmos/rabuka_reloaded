use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";
/// 鐘 嵐珠 — {{toujyou.png|登場}}自分のステージに『R3BIRTH』のメンバーが**3人いる場合**、
/// ライブ終了時まで、heart01を得る。
const ME: &str = "PL!N-bp7-024-N";

/// Fixtures use the CLEAN card numbers.
///
/// The corpus also contains 13 keys with text fragments spliced into the card_no —
/// `PL!N-bp1-023-PRproteinbar`, `PL!N-sd1-012-PRproteinbar`, `LL-PR-007-PRLoveLive!Days` and
/// ten more. Each of the 13 is a **byte-exact duplicate** of a legitimate entry, identical in
/// ability, cost, base_heart, blade_heart, need_heart, special_heart and series. For example
/// `PL!N-bp1-023-PRproteinbar` ≡ `PL!N-bp1-023-N`. They are phantom duplicates, and this file
/// previously reached for one of them.
///
/// These fixtures use the clean keys, so they name real pool entries and stay resolvable if the
/// 13 duplicates are ever deleted.
const R3BIRTH_A: &str = "PL!N-bp1-023-N";
const R3BIRTH_B: &str = "PL!N-bp1-024-PR";
const R3BIRTH_C: &str = "PL!N-PR-012-PR";

/// The group the parser carries for the printed 「『R3BIRTH』」.
const PRINTED_GROUP: &str = "R3BIRTH";

/// **Premise audit.** This test's comment claims "3 R3BIRTH members on stage". That claim is
/// **not supported by `cards/cards.json`**, and it is checked here rather than assumed:
///
/// * The only `series` values in the corpus are スーパースター!!, 虹ヶ咲学園スクールアイドル同好会,
///   μ's, サンシャイン!! and 蓮ノ空女学院スクールアイドルクラブ. **None contains `R3BIRTH`** —
///   it is a sub-group the `series` field does not model.
/// * All four fixtures used here are ラブライブ！虹ヶ咲学園スクールアイドル同好会.
/// * The only `R3BIRTH` string anywhere in `engine/src` is inside the generated
///   `core/cards_gen.rs` card-number table; there is no group→series alias for it.
///
/// So what these tests actually prove is **not** established: either an alias maps
/// 『R3BIRTH』 onto 咲学園 (in which case the comment is wrong but the behaviour may be right),
/// or the group is unsatisfiable and the count is met vacuously — which would be a severe
/// defect, since **every** condition gated on a sub-group name would then fire
/// unconditionally.
///
/// What the print below adds: the two behavioural boards differ by exactly one card, and the
/// fixtures carry **heart01 blade hearts** (鐘嵐珠 ×2), while the replacing card has none.
/// Whether that is the real discriminator, or a coincidence of this fixture, is left open —
/// see the note at the bottom of this test.
#[test]
fn n_bp7_024_n_fixture_group_membership_is_documented() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    for card_no in [ME, R3BIRTH_A, R3BIRTH_B, R3BIRTH_C] {
        let id = game.id(card_no);
        let c = game
            .db
            .get_card(id)
            .unwrap_or_else(|| panic!("{card_no} must resolve in the baked database"));
        println!(
            "{card_no:<30} {:<10} series={:?} blade={:?}",
            c.name, c.series, c.blade_heart
        );
        assert!(
            !format!("{:?}", c.series).contains(PRINTED_GROUP),
            "If this now reports series containing {PRINTED_GROUP}, the corpus gained a real \
             R3BIRTH series and the 3人 premise of the tests below can finally be checked \
             properly instead of being assumed. Update those tests when it happens."
        );
    }

// The corpus's 13 contaminated keys are NOT used here — they are byte-exact duplicates
    // of clean entries (see the fixture note at the top), so reaching for one would pin a
    // phantom. Recorded rather than asserted: if `PL!N-bp1-023-PRproteinbar` ever stops being
    // an exact duplicate, that is a corpus change worth knowing about.
    //
    // What is deliberately NOT asserted: what actually satisfies the 『R3BIRTH』 condition.
    // The two behavioural tests below differ by exactly one card — the positive stages
    // `PL!N-bp1-024-PR` 鐘嵐珠 where the negative stages `PL!-sd1-010-SD` 高坂穂乃果 — and both
    // boards hold three 虹ヶ咲 members. The printed series above shows no card is tagged
    // R3BIRTH, and the discriminating pair is the one that carries a **heart01 blade heart**
    // (鐘嵐珠 ×2 prints) against one that carries none. Whether the group maps onto that, onto
    // a character match, or onto something else is **not established**, and guessing would be
    // worse than leaving it open. Resolving it needs the group→series alias table, which does
    // not appear to exist for this name.
    assert!(
        format!("{:?}", game.db.get_card(game.id(ME)).unwrap().series)
            .contains("虹ヶ咲"),
        "If 鐘嵐珠's series is no longer 虹ヶ咲, the premise of these tests has changed and the \
         『R3BIRTH』 question needs reopening."
    );
}

/// The positive case, with the printed amount pinned exactly.
///
/// The parse is `gain_resource { count: 1, heart_colors: [heart01], duration: live_end }` — one
/// heart, not "some". The original assertion was `> 0`, which accepted 2 or 3; a
/// double-counting or per-unit defect on a single heart01 was invisible. The sibling
/// two-member case already asserted `== 0`, so the pair now brackets the value exactly.
#[test]
fn n_bp7_024_n_debut_gains_one_heart01_with_three_r3birth_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(ME);
    let r1 = game.id(R3BIRTH_A);
    let r2 = game.id(R3BIRTH_B);
    let r3 = game.id(R3BIRTH_C);
    game.state.player1.stage.stage = [me, r1, r2];
    game.state.player2.stage.stage[0] = r3;

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    const H01: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart01;
    assert_eq!(
        game.state.mods.get_heart_modifier(me, H01),
        1,
        "ライブ終了時まで、heart01を得る — the parse carries count: 1, so the grant is EXACTLY one \
         heart01, not a positive amount. A value of 2+ means the amount is not read from the \
         printed text. NB the 『R3BIRTH』 premise of this fixture is unverified — see \
         n_bp7_024_n_fixture_group_membership_is_documented."
    );
}

#[test]
fn n_bp7_024_n_debut_gains_no_heart01_with_two_r3birth_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(ME);
    let r1 = game.id(R3BIRTH_A);
    let other = game.new_id(FILLER);
    game.state.player1.stage.stage = [me, r1, other];

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    const H01: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart01;
    assert_eq!(
        game.state.mods.get_heart_modifier(me, H01),
        0,
        "only 2 qualifying members -> no heart01"
    );
}



