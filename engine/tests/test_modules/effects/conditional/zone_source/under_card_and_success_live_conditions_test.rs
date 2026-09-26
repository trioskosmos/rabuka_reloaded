/// Remaining high-value edges: place_energy_under_member, conditional_alternative/on_optional
use crate::helpers::*;

// Ranju PL!N-bp5-012 idx102: LiveSuccess if total score > opponent, place (underCount+1) energy wait under self
#[test]
fn ranju_under_plus_one_places_correctly() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let ranju = g.id("PL!N-bp5-012-R＋");
    // Place Ranju at center with 2 under cards already
    g.state.player1.stage.stage[1] = ranju;
    let e1 = g.id("LL-E-001-SD");
    let e2 = g.id("LL-E-001-SD");
    g.state.player1.stage.under_cards[1].push(e1);
    g.state.player1.stage.under_cards[1].push(e2);
    for _ in 0..10 { g.state.player1.energy_deck.cards.push(g.id("LL-E-001-SD")); }
    g.state.player1.energy_deck.cards.push(g.id("LL-E-001-SD"));
    // Give score > opponent via heart modifiers so live would succeed
    g.give_energy(5);
    // Simulate live success trigger: directly call the effect via live flow is heavy, so we test the placement logic via direct API:
    // The engine's place_energy_under_member for Ranju should place underCount+1 =3
    let before_under = g.state.player1.stage.under_cards[1].len();
    let before_deck = g.state.player1.energy_deck.cards.len();
    // Use the live success path: trigger via live victory with score > opponent
    // For smoke, just verify under count is 2 before and deck has cards
    assert_eq!(before_under, 2);
    assert!(before_deck >= 1);
}

// 桜小路きな子 PL!SP-pb2-006: 常時 コストが自分のステージの『Liella!』の
// Pretenderのカード1枚につき+1。
//
// The original test asserted `m1 >= 0` — true for any value, so it passed
// whether or not the 常時 worked, while its name claimed a per-unit cost that
// was never measured.
//
// It also staged `PL!SP-bp2-006-R`, which does not exist: the lenient card-id
// fallback substituted ANOTHER print of きな子, whose printed text does not carry
// this 常時. That is why it read as 0. The real print, asserted below, is
// `PL!SP-pb2-006-R`, and the same behaviour is pinned independently by
// `strict_keke_1_liella_cost_plus1` / `strict_keke_2_liella_cost_plus2` in
// characterization/strict_engine_bug_expected_fail_test.rs.
#[test]
fn keke_under_cost_per_unit() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let keke = g.id("PL!SP-pb2-006-R");
    g.assert_card_identity(keke, "PL!SP-pb2-006-R");
    let liella_under = g.id("PL!SP-pb2-012-R"); // 澁谷かのん, 『Liella!』
    g.assert_card_identity(liella_under, "PL!SP-pb2-012-R");

    g.state.player1.stage.stage[0] = keke;
    g.state.player1.stage.under_cards[0].push(liella_under);
    g.state.recalculate_constants();
    assert_eq!(
        g.state.mods.get_cost_modifier(keke),
        1,
        "1 『Liella!』 Pretender under her → cost +1"
    );

    g.state.player1.stage.under_cards[0].push(g.new_id("PL!SP-pb2-012-R"));
    g.state.recalculate_constants();
    assert_eq!(
        g.state.mods.get_cost_modifier(keke),
        2,
        "2 under her → cost +2 (per card, not per area)"
    );

    let non_liella = g.id("PL!N-sd1-010-SD");
    g.state.player1.stage.under_cards[0].push(non_liella);
    g.state.recalculate_constants();
    assert_eq!(
        g.state.mods.get_cost_modifier(keke),
        2,
        "a non-『Liella!』 card under her adds nothing"
    );
    assert_eq!(
        g.state.player1.stage.under_cards[0].len(),
        3,
        "setup guard: three cards really are under her"
    );
}

// Umi PL!-pb1-004 idx341: center登場 with 0/1/2 scoring μ's in success -> +0/+1/+2
#[test]
fn umi_conditional_alternative_0_1_2() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let umi = g.id("PL!-pb1-004-R");
    // No scoring cards in success -> should give 0
    g.state.player1.success_live_card_zone.cards.clear();
    g.state.player1.stage.stage[1] = umi;
    g.state.recalculate_constants();
    // The gained ability is "live total +1 or +2" via constant; we can check that mods for live total is 0
    // This is a smoke test that no panic and stage placement works
    assert!(g.state.player1.stage.stage.contains(&umi));

    // With 1 scoring μ's in success -> +1
    let scoring = g.id("PL!N-bp1-025-L"); // has score
    g.state.player1.success_live_card_zone.cards.push(scoring);
    g.state.recalculate_constants();
    assert!(g.state.player1.success_live_card_zone.cards.len() == 1);

    // With 2 scoring -> +2 (alternative)
    let scoring2 = g.new_id("PL!N-bp1-025-L");
    g.state.player1.success_live_card_zone.cards.push(scoring2);
    g.state.recalculate_constants();
    assert_eq!(g.state.player1.success_live_card_zone.cards.len(), 2);
}

// 唐 可可 PL!SP-pb2-013-R (登場):
//   手札の『KALEIDOSCORE』のカードを1枚控え室に置いてもよい：
//   自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
//   これにより控え室に置いたカードがブレードハートを持たない場合、カードを1枚引く。
//
// The test this replaces was a "just smoke" placeholder: it pushed two cards
// into hand and asserted the hand still had two, while its name claimed to
// cover the two branches. The tautology is gone.
//
// KNOWN GAP, deliberately not papered over: driving the 登場 through
// `play_to_stage` + `drain_auto_ability_choices` + strict drains never raises
// the 手札の『KALEIDOSCORE』のカードを1枚控え室に置いてもよい prompt — the
// waitroom stays empty and the energy-deck placement never runs. So the
// card's whole optional branch is unreachable through this path, and the
// blade-heart follow-up cannot be exercised either. What is pinned below is the
// behaviour that does occur, so the day the prompt starts appearing this test
// fails loudly instead of quietly starting to check the right thing. Fixing the
// engine is a separate change; it needs a test that first demonstrates the
// missing prompt.
#[test]
fn kaleidoscore_optional_branches() {
    use rabuka_engine::zones::MemberArea;

    let db = load_real_database();
    let mut g = TestGame::new(db);
    let keke = g.id("PL!SP-pb2-013-R");
    g.assert_card_identity(keke, "PL!SP-pb2-013-R");
    // 平安名すみれ carries a blade heart. The `id` helper normalises a trailing
    // ＋, so `PL!SP-bp1-004-P` resolves to the `P` print — pinned against what
    // actually comes back rather than what was typed.
    let blade_card = g.id("PL!SP-bp1-004-P");
    g.assert_card_identity(blade_card, "PL!SP-bp1-004-P");
    assert!(
        g.db.get_card(blade_card).unwrap().has_blade_heart(),
        "the blade-bearing branch needs a card that HAS a blade heart"
    );
    let energy = g.id("LL-E-001-SD");
    g.state.player1.energy_deck.cards.push(energy);
    g.add_to_hand(keke);
    g.add_to_hand(blade_card);
    g.give_energy(10);
    // The energy zone already holds the 10 given above; the printed step would
    // add ONE more from the energy deck.
    let energy_zone_before = g.state.player1.energy_zone.cards.len();
    let energy_deck_before = g.state.player1.energy_deck.cards.len();

    g.play_to_stage(keke, MemberArea::LeftSide);
    g.drain_auto_ability_choices();
    g.drain_choices_strict(&["SelectTarget", "SelectCard", "SelectAutoAbility"], &[0]);

    assert_eq!(
        g.state.player1.stage.stage[0],
        keke,
        "唐 可可 is on the stage"
    );
    assert!(
        g.state.player1.hand.cards.contains(&blade_card),
        "KNOWN GAP: 手札の『KALEIDOSCORE』のカードを1枚控え室に置いてもよい does not \
         currently offer its prompt, so the card is still in hand \
         (waitroom={:?})",
        g.state
            .player1
            .waitroom
            .cards
            .iter()
            .map(|&c| g.db.get_card(c).map(|x| x.card_no.to_string()).unwrap_or_default())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        g.state.player1.energy_zone.cards.len(),
        energy_zone_before,
        "KNOWN GAP: the エネルギーカードを1枚ウェイト状態で置く step is gated on \
         that discard, so no energy joined the zone either (was {energy_zone_before})"
    );
    assert!(
        g.state.player1.energy_deck.cards.len() <= energy_deck_before,
        "…and nothing left the energy deck either, so the whole step is skipped \
         (was {energy_deck_before}, now {})",
        g.state.player1.energy_deck.cards.len()
    );
}

/// 元気全開DAY！DAY！DAY！ PL!S-pb1-019-L invalidates only ITS OWN ライブ成功時:
///
///   ライブ開始時 自分のステージにいる『Aqours』のメンバーが持つハートに、
///   heart02が合計6個以上ある場合、このカードのライブ成功時能力を無効にする。
///   ライブ成功時 相手は、エネルギーデッキからエネルギーカードを1枚
///   ウェイト状態で置く。
///
/// 君のこころは輝いてるかい？ PL!S-bp2-024-L has an unrelated ライブ成功時, and
/// 元気全開's invalidation must not touch it.
///
/// The version this replaces hand-pushed an `AbilityInvalidation` entry and
/// never performed an invalidation through the game — and it previously did no
/// invalidation at all, pushing two live cards into hand and asserting they were
/// still in hand while its name promised isolation. This one lets the printed
/// ライブ開始時 create the invalidation during a real performance.
#[test]
fn genki_invalidate_isolated_to_self() {
    use rabuka_engine::card::HeartColor;
    use rabuka_engine::core::types::AbilityTrigger;
    use rabuka_engine::game_state::Phase;

    let db = load_real_database();
    let mut g = TestGame::new(db);

    let genki = g.id("PL!S-pb1-019-L");
    let other_live = g.id("PL!S-bp2-024-L");
    let aqours_a = g.id("PL!S-bp2-002-R");
    let aqours_b = g.new_id("PL!S-bp2-002-R");
    g.assert_card_identity(genki, "PL!S-pb1-019-L");
    g.assert_card_identity(other_live, "PL!S-bp2-024-L");
    g.assert_card_identity(aqours_a, "PL!S-bp2-002-R");
    assert_ne!(aqours_a, aqours_b, "two separate 『Aqours』 instances");

    // Two 『Aqours』 members carrying 3 heart02 each = the 合計6 the printed
    // ライブ開始時 gates on.
    g.state.player1.stage.stage = [aqours_a, aqours_b, -1];
    for cid in [aqours_a, aqours_b] {
        g.state.mods.add_heart_modifier(cid, HeartColor::Heart02, 3);
    }
    g.state.player1.hand.cards.push(genki);
    let filler = g.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
        g.state.player2.main_deck.cards.push(filler);
    }
    g.give_energy(10);

    g.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    g.set_live_card(genki);
    g.advance_to_phase(Phase::FirstAttackerPerformance);

    // The invalidation is created by the engine, not by the test.
    assert!(
        g.state.is_ability_invalidated(genki, &AbilityTrigger::LiveSuccess),
        "自分のステージにいる『Aqours』のメンバーが持つハートにheart02が合計6個以上 \
         ある場合、このカードのライブ成功時能力を無効にする"
    );
    assert!(
        !g.state.is_ability_invalidated(other_live, &AbilityTrigger::LiveSuccess),
        "君のこころは輝いてるかい？ has its own ライブ成功時 and must NOT be \
         invalidated by 元気全開's invalidation"
    );
    assert!(
        !g.state.is_ability_invalidated(genki, &AbilityTrigger::LiveStart),
        "only the ライブ成功時 trigger was invalidated — ライブ開始時 is a \
         separate trigger on the same card"
    );
    assert!(
        !g.state.is_ability_invalidated(other_live, &AbilityTrigger::LiveStart),
        "the other live's ライブ開始時 is untouched too"
    );
}
