/// Untested-abilities batch 9 — rare-mechanic sweep from TEST_INVENTORY
/// depth=none gaps: bottom-mill tier counting (preceding_moved), look-at-bottom
/// repositioning, choose-target-player waitroom recycling, choose-one effects
/// with delayed activation suppression, or-conditions on score/original-value,
/// gained 常時 live-total abilities, alternative costs, and ability-driven
/// position changes.
use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member, cost 4

// ====================================================================
// B9-1 PL!S-bp7-021-L 僕らの旅は終わらない — ライブ開始時:
// 自分のステージにメンバーが3人以上いる場合、デッキの下から5枚控え室に置く。
// それらの中にメンバーカードが3枚以上 → draw 1。
// それらがすべてメンバーカード → さらに スコア+1。
// ====================================================================

fn tabi_board(game: &mut TestGame) -> i16 {
    let live = game.id("PL!S-bp7-021-L");
    game.state.player1.live_card_zone.cards.push(live);
    for i in 0..3usize {
        let m = game.new_id(FILLER);
        game.state.player1.stage.stage[i] = m;
    }
    live
}

#[test]
fn tabi_bp7021_mill_tiers_two_members_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = tabi_board(&mut game);

    // Bottom 5 (push order = mill order): member, member, life, life, life.
    let m1 = game.id("PL!S-sd1-001-SD");
    let m2 = game.id("PL!S-sd1-001-SD");
    let l1 = game.id("PL!-sd1-019-SD"); // lives are NOT member cards
    game.state.player1.main_deck.cards.push(m1);
    game.state.player1.main_deck.cards.push(m2);
    for _ in 0..3 {
        let l = game.new_id("PL!-sd1-019-SD");
        game.state.player1.main_deck.cards.push(l);
    }
    let _ = l1;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        5,
        "five cards milled to the waitroom"
    );
    assert_eq!(game.state.player1.hand.cards.len(), 0, "only 2 members < 3 → no draw");
    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "not all members → no score"
    );
}

#[test]
fn tabi_bp7021_mill_tiers_three_members_draw_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = tabi_board(&mut game);

    // Bottom 5: 3 members + 2 lives → draw fires, score does not.
    for _ in 0..3 {
        let m = game.id("PL!S-sd1-001-SD");
        game.state.player1.main_deck.cards.push(m);
    }
    for _ in 0..2 {
        let l = game.new_id("PL!-sd1-019-SD");
        game.state.player1.main_deck.cards.push(l);
    }

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "3 members among the milled 5 → draw 1"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "not ALL five were members → no +1"
    );
}

#[test]
fn tabi_bp7021_all_five_members_draw_and_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = tabi_board(&mut game);

    for _ in 0..5 {
        let m = game.id("PL!S-sd1-001-SD");
        game.state.player1.main_deck.cards.push(m);
    }

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(game.state.player1.hand.cards.len(), 1, "draw fired");
    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "all five were members → スコア+1"
    );
}

#[test]
fn tabi_bp7021_fewer_than_three_members_on_stage_noop() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = tabi_board(&mut game);

    game.state.player1.stage.stage[2] = -1; // only 2 members now
    for _ in 0..5 {
        let m = game.id("PL!S-sd1-001-SD");
        game.state.player1.main_deck.cards.push(m);
    }

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        0,
        "stage count 2 < 3 → nothing is milled"
    );
}

// ====================================================================
// B9-2 PL!S-bp7-010-N 高海千歌 — 登場:
// デッキの一番下のカードを見る。それをデッキの上から4番目に置いてもよい。
// ====================================================================

#[test]
fn chika_bp7010_bottom_card_to_fourth_from_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp7-010-N");
    game.add_to_stage(MemberArea::Center, chika);

    // Deck top→bottom: f0 f1 f2 f3 X (X at the bottom).
    let x = game.id("PL!S-sd1-001-SD");
    let mut deck_cards: Vec<i16> = Vec::new();
    for _ in 0..4 {
        let f = game.new_id(FILLER);
        deck_cards.push(f);
        game.state.player1.main_deck.cards.push(f);
    }
    game.state.player1.main_deck.cards.push(x);

    fire_trigger(&mut game, chika, AbilityTrigger::Debut, "登場");

    if !game.has_pending_choice() {
        panic!("expected the optional move-to-4th choice");
    }
    eprintln!("[CHIKA] choice: {}", game.pending_choice_summary());
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(1), // accept move
        Some("SelectCard") => game.select_indices(&[0]), // the looked-at card
        other => panic!("unexpected {other:?}"),
    }
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.get(3),
        Some(&x),
        "X now sits 4th from the TOP (index 3)"
    );
    assert_ne!(
        game.state.player1.main_deck.cards.last(),
        Some(&x),
        "X left the bottom"
    );
}

#[test]
fn chika_bp7010_decline_leaves_bottom_card_alone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp7-010-N");
    game.add_to_stage(MemberArea::Center, chika);

    let x = game.id("PL!S-sd1-001-SD");
    game.state.player1.main_deck.cards.push(x); // only card = top AND bottom

    fire_trigger(&mut game, chika, AbilityTrigger::Debut, "登場");

    // The optional reposition shows up as a looked_at SelectCard; SKIP it.
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[]), // decline
            _ => break,
        }
    }
    assert!(
        game.state.player1.main_deck.cards.contains(&x),
        "declined → X must STAY in the deck (5.7.1: 見る only informs)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.last(),
        Some(&x),
        "declined → X stays at the BOTTOM"
    );
}

// ====================================================================
// B9-3 PL!S-bp7-013-N 黒澤ダイヤ — 登場:
// 自分か相手を選ぶ。そのプレイヤーの控え室にあるメンバーカードを2枚まで
// 好きな順番でデッキの下に置く。
// ====================================================================

#[test]
fn dia_bp7013_own_waitroom_members_to_deck_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dia = game.id("PL!S-bp7-013-N");
    game.add_to_stage(MemberArea::Center, dia);

    let m1 = game.id("PL!S-sd1-001-SD");
    let m2 = game.id("PL!S-sd1-001-SD");
    let life = game.id("PL!-sd1-019-SD"); // non-member stays put
    game.state.player1.waitroom.cards.push(m1);
    game.state.player1.waitroom.cards.push(life);
    game.state.player1.waitroom.cards.push(m2);

    fire_trigger(&mut game, dia, AbilityTrigger::Debut, "登場");

    assert!(game.has_pending_choice(), "自分/相手 player choice expected");
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(0), // 自分
        other => panic!("expected SelectTarget for player pick, got {other:?}"),
    }
    // Then select both member cards from the waitroom.
    let mut picked = 0;
    while game.has_pending_choice() && picked < 4 {
        let idxs: Vec<usize> = game
            .state
            .player1
            .waitroom
            .cards
            .iter()
            .enumerate()
            .filter(|(_, &c)| game.db.get_card(c).is_some_and(|cc| matches!(cc.card_type, rabuka_engine::card::CardType::Member)))
            .map(|(i, _)| i)
            .collect();
        if idxs.is_empty() {
            break;
        }
        game.select_indices(&idxs);
        picked += 1;
    }

    assert_eq!(
        game.state.player1.main_deck.cards.last().copied(),
        Some(m2),
        "member cards recycled to the BOTTOM of the deck"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        2,
        "both members moved under each other"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&life),
        "the live card was not a legal target and stayed"
    );
    assert_eq!(game.state.player1.waitroom.cards.len(), 1);
}

// ====================================================================
// B9-4 PL!SP-bp7-017-N 桜小路きな子 — 登場:
// エネルギーデッキから1枚ウェイト状態で置く。そのエネルギーは次のターンの
// アクティブフェイズにアクティブしない。
// ====================================================================

#[test]
fn kinako_spbp7017_placed_energy_wont_activate_next_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako = game.id("PL!SP-bp7-017-N");
    game.add_to_stage(MemberArea::Center, kinako);
    fill_energy_deck(&mut game, 0, 2);

    fire_trigger(&mut game, kinako, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        1,
        "one energy placed from the energy deck"
    );
    let placed = *game.state.player1.energy_zone.cards.last().unwrap();
    assert!(
        game.state.mods.is_delayed_cannot_active(placed),
        "Q280-family: the placed energy must NOT activate next turn"
    );
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        1,
        "energy deck shrank by one"
    );
}

// ====================================================================
// B9-5 PL!S-bp7-025-L Guilty Night, Guilty Kiss! — ライブ成功時:
// 選ぶ：①相手のコスト4以下メンバーを2人までウェイト＋次ターン非アクティブ
//       ②カードを1枚引く。
// ====================================================================

fn gn_gk_board(game: &mut TestGame) -> i16 {
    let live = game.id("PL!S-bp7-025-L");
    game.state.player1.live_card_zone.cards.push(live);
    // Opponent stages two cheap (cost 4 boundary) actives.
    let o1 = game.new_id(FILLER);
    let o2 = game.new_id(FILLER);
    game.state.player2.stage.stage[0] = o1;
    game.state.player2.stage.stage[1] = o2;
    live
}

#[test]
fn guilty_night_option_a_waits_and_suppresses() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = gn_gk_board(&mut game);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice(), "option choice expected");
    game.select_option(0); // option A

    // Option A then asks WHICH opponent members to wait (up to 2, cost≤4).
    let mut picked = false;
    for _ in 0..3 {
        if !game.has_pending_choice() {
            break;
        }
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                game.select_indices(&[0, 1]);
                picked = true;
            }
            _ => game.select_indices(&[]),
        }
    }
    assert!(picked, "the wait-target selection must be offered");

    for i in 0..2usize {
        let opp = game.state.player2.stage.stage[i];
        assert_eq!(
            game.state.mods.get_orientation_modifier(opp),
            Some("wait"),
            "opponent member {i} waited"
        );
        assert!(
            game.state.mods.is_delayed_cannot_active(opp),
            "…and will NOT activate next turn"
        );
    }
}

#[test]
fn guilty_night_option_b_draws_instead() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    let live = gn_gk_board(&mut game);

    let o1 = game.state.player2.stage.stage[0];
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice());
    game.select_option(1); // option B

    assert_eq!(game.state.player1.hand.cards.len(), 1, "option B → draw 1");
    assert_eq!(
        game.state.mods.get_orientation_modifier(o1),
        None,
        "opponent untouched by option B"
    );
}

// ====================================================================
// B9-6 PL!SP-pb2-004-R 平安名すみれ — ライブ成功時 (or_condition):
// ①ライブカード置き場に元々のスコアより高いスコアのライブカードがある、または
// ②エールで公開した自分のカードにスコアを持つライブカードがある → draw 1。
// ====================================================================

#[test]
fn sumire_pbp2004_original_score_beaten_or_revealed_live_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-pb2-004-R");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.add_to_stage(MemberArea::Center, sumire);
    let live6 = game.id("PL!SP-bp1-027-L"); // original score 6
    game.state.player1.live_card_zone.cards.push(live6);

    // Neither condition: score NOT raised, nothing revealed.
    fire_trigger(&mut game, sumire, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert_eq!(game.state.player1.hand.cards.len(), 0, "no condition met → no draw");

    // Condition ①: current score pushed ABOVE its original 6.
    game.state.mods.add_score_modifier(live6, 2);
    fire_trigger(&mut game, sumire, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "元々のスコアより高いスコアのライブカード → draw 1"
    );

    // Condition ②: a revealed live card with a score icon.
    game.state.mods.add_score_modifier(live6, -2); // undo ①
    game.state.revealed_cards.push(game.id("PL!-sd1-019-SD")); // score-1 live
    fire_trigger(&mut game, sumire, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "revealed scored live → draw 1 via the OR branch"
    );
}

// ====================================================================
// B9-7 PL!-PR-020-PR 高坂穂乃果 — ライブ開始時 (center):
// ライブカード置き場のライブカードのスコア合計が8以上の場合、
// 「常時：ライブの合計スコア+1」を得る。(gain_ability)
// ====================================================================

#[test]
fn honoka_pr0020_gains_constant_live_total_at_eight() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let honoka = game.id("PL!-PR-020-PR");
    game.add_to_stage(MemberArea::Center, honoka); // center required

    // Live zone score total EXACTLY 8 = Sing！Shine！Smile！(6) + START:DASH!!×2 (1+1).
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!SP-bp1-027-L"));
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-019-SD"));
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));

    // Both live cards carry need_heart requirements; inject a Heart00 wildcard
    // stage_hearts so calculate_live_score counts their scores (8 >= 8 gate).
    let mut heart_map = HeartMap::new();
    heart_map.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts: heart_map });

    fire_trigger(&mut game, honoka, AbilityTrigger::LiveStart, "ライブ開始時");
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "score total ≥ 8 → gained 【常時】ライブの合計スコア+1"
    );
}

// ====================================================================
// B9-8 PL!S-bp6-007-R 国木田花丸 — ライブ開始時:
// コスト：E2支払うか手札2枚控え室（代替コスト）：自分の成功置き場が空かつ
// 相手の成功置き場2枚以上 → 自分の『Aqours』メンバー2人まで
// 「常時：ライブの合計スコア+1」を得る（ライブ終了時まで）。
// ====================================================================

#[test]
fn hanamaru_bp6007_alt_cost_gain_constant_to_aqours() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-bp6-007-R");
    game.add_to_stage(MemberArea::Center, hanamaru);
    let aqours_friend = game.id("PL!S-pb1-007-R"); // Aqours cost 9
    game.add_to_stage(MemberArea::LeftSide, aqours_friend);
    let outsider = game.id(FILLER); // μ's — never gains
    game.add_to_stage(MemberArea::RightSide, outsider);

    // Own success zone EMPTY, opponent has 2 cards.
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id("PL!-sd1-019-SD"));
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.new_id("PL!-sd1-019-SD"));

    // Pay with ENERGY (2 active available).
    game.give_energy(2);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, hanamaru, AbilityTrigger::LiveStart, "ライブ開始時");

    // Drain any cost/effect choices (energy-vs-discard prompt etc.).
    // The gain_ability stage selection must pick BOTH Aqours members
    // (花丸 herself + 国木田花丸 pb1-007); the μ's member is filtered out.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectTarget") => game.select_option(0),
            Some("SelectCard") => game.select_indices(&[0, 1]),
            _ => game.select_indices(&[]),
        }
    }

    assert!(
        game.state.player1.energy_zone.active_count() <= 2,
        "sanity"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "energy payment leaves hand alone"
    );
    game.state.recalculate_constants();
    // The gained constants feed the live-total accumulator (per gaining member).
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 2,
        "up to TWO 『Aqours』 members each gain ライブの合計スコア+1 (μ's member excluded)"
    );
}
