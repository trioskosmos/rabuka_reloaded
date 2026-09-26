use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn drain_skippable_choices(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { allow_skip: true, .. } => game.select_indices(&[]),
            _ => break,
        }
    }
}

/// SENTIMENTAL StepS ライブ (PL!-pb1-032-L) ab#0:
///   ライブ成功時 自分の成功ライブカード置き場に『μ's』のカードがある場合、
///   カードを1枚引く。
///
/// These tests are a DIFFERENTIAL: one fixture, one live, and the only
/// difference is which card is in the success live zone. The outcome is the deck
/// size across the ライブ成功時 window.
///
/// They used to assert only `!has_pending_choice()`, which holds whether the
/// ability fired or not — a draw is not a prompt, so all three passed while
/// proving nothing.
fn setup() -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-pb1-032-L");
    let filler = game.id_ref("PL!-sd1-010-SD");
    // Three DISTINCT members that print heart01/heart03/heart06 so the live's
    // requirement (h01 + h03 + h06 + h0×2) is MET and the live succeeds.
    let m = game.new_id("PL!-sd1-001-SD");
    let m2 = game.new_id("PL!-sd1-001-SD");
    let m3 = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [m, m2, m3];
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(live);
    (game, live, filler)
}

/// Cards the deck loses across the ライブ成功時 window, counted from AFTER the
/// yell.
///
/// The baseline is taken at LiveVictoryDetermination, not before the live runs:
/// the yell discards the cards it did not reveal, so a count that starts at the
/// live card set carries 9-10 cards of noise (measured) against the 1 card the
/// ability moves. ライブ成功時 then resolves on the steps inside
/// LiveVictoryDetermination, and the live closes on the → Active transition.
fn deck_delta_across_live_success(game: &mut TestGame) -> i32 {
    game.advance_to_phase(rabuka_engine::game_state::Phase::LiveVictoryDetermination);
    drain_skippable_choices(game);
    let before = game.state.player1.main_deck.cards.len() as i32;
    game.advance_to_phase(rabuka_engine::game_state::Phase::Active);
    drain_skippable_choices(game);
    game.state.player1.main_deck.cards.len() as i32 - before
}

/// A μ's card in each of μ's two eras. Per `engine/rules/rules.txt` the
/// グループ名称 list is μ's / Aqours / 虹ヶ咲 / Liella! / 蓮ノ空 / … and the
/// ユニット名称 list is Printemps / BiBi / … / スリーズブケ / … /
/// みらくらぱーく！ / Edel Note / AiScReam — so Printemps and みらくらぱーく！
/// are both μ's UNIT names, and neither is Liella!. (Liella! is
/// 5yncri5e! / QU4RTZ / A・ZU・NA / R3BIRTH / Sunny Passion, the
/// スーパースター series.)
const MU_PRINTEMPS: &str = "PL!-sd1-001-SD";
const MU_MUREN: &str = "PL!HS-bp1-023-L";
/// A 蓮ノ空 member — the negative twin's group.
const HASUNOSORA: &str = "PL!HS-bp1-001-R";

#[test]
fn success_zone_group_draw_live_known_gap_mu_printemps_card_in_zone_draws_nothing() {
    let (mut game, live, _filler) = setup();
    let mu = game.id(MU_PRINTEMPS);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(mu);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    let delta = deck_delta_across_live_success(&mut game);

    assert!(
        game.state.player1.success_live_card_zone.cards.contains(&live),
        "precondition: this live SUCCEEDED, so ライブ成功時 resolved at all"
    );
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&mu),
        "precondition: a μ's card IS in the success zone"
    );
    // KNOWN GAP. The ability resolves — the debug log shows
    //   [CONDITION] source=…(PL!-pb1-032-L) action=draw_card passed=false
    //             type=GroupCondition location="success_live_card_zone"
    //             group=["μ's"]
    // — and no card is drawn, even though the zone holds a μ's card.
    //
    // Cause, in the engine's own terms. `Card.group` is empty in the card data,
    // so `card_matches_group_str` falls back to (a) `unit == "μ's"`, which
    // cannot hold because every `unit` is a UNIT NAME (Printemps,
    // みらくらぱーく！), and (b) `card_series_matches_group(series, "μ's")`
    // (engine/src/ability/util.rs), which accepts a ラブライブ！ series that
    // contains none of サンシャイン / 虹ヶ咲 / スーパースター / 蓮ノ空.
    //
    // A みらくらぱーく！ card is printed in ラブライブ！蓮ノ空女学院スクール
    // アイドルクラブ — μ's renamed their UNIT, not their school — so (b)
    // rejects all of them. Measured over cards.json: of the 215 cards whose
    // `unit` is Printemps or みらくらぱーく！, 119 match and 96 do not, and every
    // one of the 96 is a みらくらぱーく！ card in the 蓮ノ空 series. So a
    // 『μ's』 filter silently ignores 45% of μ's cards.
    //
    // The fix belongs in the matcher: a unit→group table is what rules.txt
    // already prints (Printemps/みらくらぱーく → μ's, BiBi/lily white/CYaRon →
    // Aqours, AZALEA/Guilty Kiss/QU4RTZ/A・ZU・NA → 虹ヶ咲, 5yncri5e!/DiverDiva/
    // R3BIRTH/CatChu!/KALEIDOSCORE → Liella!, スリーズブケ/DOLLCHESTRA/
    // Edel Note → 蓮ノ空, AiScReam → Sunny Passion). This test is the tripwire:
    // it goes red the moment that lands, and then it becomes the real positive
    // twin.
    assert_eq!(
        delta, 0,
        "KNOWN GAP: a Printemps (μ's) card is in the success zone and nothing is \
         drawn"
    );
}

#[test]
fn success_zone_group_draw_live_known_gap_mu_muren_card_in_zone_draws_nothing() {
    let (mut game, live, _filler) = setup();
    // The same group under μ's later unit name, みらくらぱーく！. This is the
    // half of μ's the series rule cannot see at all, because these cards print
    // in the 蓮ノ空 school series.
    let mu = game.id(MU_MUREN);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(mu);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    let delta = deck_delta_across_live_success(&mut game);

    assert!(
        game.state.player1.success_live_card_zone.cards.contains(&live),
        "precondition: this live SUCCEEDED"
    );
    assert_eq!(
        delta, 0,
        "KNOWN GAP: also nothing for a みらくらぱーく！ card — 96 of 215 μ's \
         cards are invisible to this filter because their series contains 蓮ノ空"
    );
}

#[test]
fn success_zone_group_draw_live_draws_nothing_with_non_mu_card_in_success_zone() {
    let (mut game, live, _filler) = setup();
    // Same fixture, same live, same success: the zone holds a 蓮ノ空 member
    // instead. Identical to the twins above in every respect but the card, so
    // "0 cards drawn" here is attributable to the group filter and not to a
    // failed live or a mis-measured window.
    let hasu = game.id(HASUNOSORA);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(hasu);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    let delta = deck_delta_across_live_success(&mut game);

    assert!(
        game.state.player1.success_live_card_zone.cards.contains(&live),
        "precondition: this live SUCCEEDED — the negative must be the FILTER, \
         not a failed live"
    );
    assert_eq!(
        delta, 0,
        "a 蓮ノ空 card in the success zone leaves the deck untouched"
    );
    assert!(!game.has_pending_choice());
}

#[test]
fn success_zone_group_draw_live_draws_nothing_with_empty_success_zone() {
    let (mut game, live, _filler) = setup();
    game.state
        .player1
        .success_live_card_zone
        .cards
        .clear();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    let delta = deck_delta_across_live_success(&mut game);

    assert_eq!(delta, 0, "『μ's』のカードがない: nothing is drawn");
    assert!(!game.has_pending_choice());
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.advance_to_phase(rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker);
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}
