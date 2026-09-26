/// 天王寺璃奈 PL!N-bp3-009-R＋ ab#0 (ライブ開始時):
///
///   控え室にあるメンバーカード2枚を好きな順番でデッキの一番下に置いてもよい：
///   それらのカードのコストの合計が、6の場合、カードを1枚引く。
///   合計が8の場合、ライブ終了時まで、ハートを得る。
///   合計が25の場合、ライブ終了時まで、「ライブの合計スコアを＋１する。」を得る。
///
/// The move itself and the cost-6 draw are covered by
/// `live_start_own_discard_two_member_bottomdeck_hearts_q164_test` and
/// `parser_issues_e2e_test`. These pin the two REWARD branches the card prints
/// and nothing else exercised: total 8 → all-heart until live end, total 25 →
/// +1 live total score until live end, plus the decline path (…してもよい).
///
/// Parser fix these tests pin: the follow-up branches drop the noun and say
/// only "合計が8の場合" / "合計が25の場合". The parser gated `cost_total` on the
/// literal コスト, so those two branches carried no threshold, the engine's
/// COST_TOTAL check read 0, and all three rewards were printed but only the
/// first could ever fire.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

const RINA: &str = "PL!N-bp3-009-R＋"; // 天王寺璃奈 — the ライブ開始時 source

/// Everything a test needs to assert on, carrying the exact card ids the setup
/// created. Re-deriving ids later with `id_ref` would compare against a
/// DIFFERENT physical copy of the same print, so the ids travel with the game.
struct RinaLiveStart {
    game: TestGame,
    rina: i16,
    cost_a: i16,
    cost_b: i16,
    /// Deck length once the live window opened, before the cost moved anything.
    deck_before: usize,
}

/// Play the live for real so 璃奈's ライブ開始時 fires from a genuine
/// performance, then pay the optional cost with the two staged waitroom members
/// (the waitroom holds ONLY those two, so the cost total is unambiguous).
/// `decline` skips the prompt instead of paying it.
fn rina_live_start(chosen: [&str; 2], decline: bool) -> RinaLiveStart {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id(RINA);
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    let cost_a = game.id(chosen[0]);
    let cost_b = game.id(chosen[1]);

    game.state.player1.waitroom.cards.clear();
    game.state.player1.waitroom.cards.push(cost_a);
    game.state.player1.waitroom.cards.push(cost_b);
    game.state.player1.stage.stage = [-1, rina, -1];
    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live);
    // Second-attacker set window, then the performance phase where ライブ開始時
    // is scanned for both attackers.
    game.pass();
    let deck_before = game.state.player1.main_deck.cards.len();
    game.pass();

    let mut guard = 0;
    let mut seen: Vec<String> = Vec::new();
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        let kind = game.pending_choice_type().unwrap_or_default();
        seen.push(format!("{kind}: {}", game.pending_choice_summary()));
        match kind.as_str() {
            "SelectAutoAbility" => game.select_indices(&[]),
            // …してもよい: an empty selection declines the optional cost.
            "SelectCard" if !decline => game.select_indices(&[0]),
            _ => game.select_indices(&[]),
        }
    }
    assert!(
        !game.has_pending_choice(),
        "the ライブ開始時 and its cost must both resolve without leftovers; \
         prompts: {seen:?}"
    );

    RinaLiveStart {
        game,
        rina,
        cost_a,
        cost_b,
        deck_before,
    }
}

/// Assert the paid cost really moved both members to the deck bottom, in
/// 好きな順番 (any order). `deck_growth` is the branch's total deck change: the
/// two cost members, minus one for the 6-total branch's own draw.
fn assert_cost_paid_to_deck_bottom(s: &RinaLiveStart, deck_growth: i32) {
    let deck = &s.game.state.player1.main_deck.cards;
    let n = deck.len();
    assert_eq!(
        n as i32,
        s.deck_before as i32 + deck_growth,
        "deck growth must be {} (the two paid members, net of this branch's own \
         reward), tail={:?}, waitroom={:?}",
        deck_growth,
        &deck[n.saturating_sub(2)..],
        s.game.state.player1.waitroom.cards
    );
    assert!(
        deck[n - 2..].contains(&s.cost_a) && deck[n - 2..].contains(&s.cost_b),
        "好きな順番でデッキの一番下に置いてもよい → the two cost members are \
         the deck's last two cards, tail={:?}",
        &deck[n - 2..]
    );
    assert!(
        !s.game.state.player1.waitroom.cards.contains(&s.cost_a)
            && !s.game.state.player1.waitroom.cards.contains(&s.cost_b),
        "the paid members left the waitroom"
    );
}

/// 合計が8の場合 — 4 + 4. The reward is an all-heart until live end, and the
/// 25-total live score must NOT also fire.
#[test]
fn live_start_cost_total_8_gains_all_heart_until_live_end() {
    // PL!-sd1-008-SD cost 4 + PL!-sd1-010-SD cost 4.
    let s = rina_live_start(["PL!-sd1-008-SD", "PL!-sd1-010-SD"], false);

    assert_cost_paid_to_deck_bottom(&s, 2);
    assert_eq!(
        s.game.state.mods.get_heart_modifier(s.rina, HeartColor::All),
        1,
        "合計が8の場合、ライブ終了時まで、ハートを得る"
    );
    assert_eq!(
        s.game.state.mods.p1_constant_total_score_bonus,
        0,
        "the 8-total branch grants a heart, NOT the 25-total live score"
    );
}

/// 合計が25の場合 — 15 + 10. The reward is a 常時 「ライブの合計スコアを＋１する。」
/// until live end, not a heart.
#[test]
fn live_start_cost_total_25_gains_live_total_score_until_live_end() {
    // PL!-sd1-009-SD cost 15 + PL!HS-PR-001-PR cost 10.
    let s = rina_live_start(["PL!-sd1-009-SD", "PL!HS-PR-001-PR"], false);

    assert_cost_paid_to_deck_bottom(&s, 2);
    assert_eq!(
        s.game.state.mods.p1_constant_total_score_bonus,
        1,
        "合計が25の場合、ライブ終了時まで、「ライブの合計スコアを＋１する。」を得る"
    );
    assert_eq!(
        s.game.state.mods.get_heart_modifier(s.rina, HeartColor::All),
        0,
        "the 25-total branch grants live total score, NOT the 8-total heart"
    );
}

/// 合計が6の場合: neither other reward may leak, and カードを1枚引く is what this
/// branch pays — two members reach the deck bottom, one is drawn back out.
#[test]
fn live_start_cost_total_6_draws_and_grants_neither_reward() {
    // PL!-sd1-002-SD cost 2 + PL!-sd1-008-SD cost 4.
    let s = rina_live_start(["PL!-sd1-002-SD", "PL!-sd1-008-SD"], false);

    assert_cost_paid_to_deck_bottom(&s, 1);
    assert_eq!(
        s.game.state.mods.get_heart_modifier(s.rina, HeartColor::All),
        0,
        "6 is not 8: no all-heart"
    );
    assert_eq!(
        s.game.state.mods.p1_constant_total_score_bonus,
        0,
        "6 is not 25: no live total score"
    );
    assert_eq!(
        s.game.state.player1.stage.stage[1], s.rina,
        "天王寺璃奈 stays on stage"
    );
}

/// …してもよい — the move is optional. Declining leaves both cards in the
/// waitroom, the deck untouched, and grants nothing (proved with a 25-total
/// pair, so a missing reward can only mean the cost was declined).
#[test]
fn live_start_declined_optional_keeps_waitroom_and_grants_nothing() {
    let s = rina_live_start(["PL!-sd1-009-SD", "PL!HS-PR-001-PR"], true);

    assert_eq!(
        s.game.state.player1.waitroom.cards.as_slice(),
        &[s.cost_a, s.cost_b],
        "declining …してもよい must leave both members in the waitroom"
    );
    assert_eq!(
        s.game.state.player1.main_deck.cards.len(),
        s.deck_before,
        "a declined cost must not move anything to the deck"
    );
    assert_eq!(
        s.game.state.mods.p1_constant_total_score_bonus,
        0,
        "no cost paid → no cost total → no 25-total live score"
    );
    assert_eq!(
        s.game.state.mods.get_heart_modifier(s.rina, HeartColor::All),
        0,
        "no cost paid → no 8-total heart"
    );
}
