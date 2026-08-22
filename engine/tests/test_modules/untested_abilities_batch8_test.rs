/// Untested-abilities batch 8 — mined from TEST_INVENTORY depth=none gaps,
/// assertions derived strictly from printed text + rules.txt/qa_data.json.
/// Focus mechanics: lone-member constants (blade gain AND loss), escalating
/// thresholds, conditional_alternative, sequential double-threshold constants,
/// per-unit score from an optional wait-cost, and deck-bottom mill with a
/// preceding_moved condition.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member, cost 4

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    fire_trigger_nth(game, cid, trigger, trig, 0);
}

/// Fire the NTH ability matching `trig` (cards like HAPPY PARTY TRAIN carry
/// TWO ライブ開始時 abilities — ab#0 all-active check, ab#1 deck-bottom mill).
fn fire_trigger_nth(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str, nth: usize) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .filter(|a| a.triggers.as_deref() == Some(trig))
            .nth(nth)
            .unwrap_or_else(|| panic!("card {} lacks '{trig}' ability #{nth}", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// B1 PL!HS-pb1-015-R セラス柳田リリエンフェルト — 常時:
// 自分のステージにほかのメンバーがいないかぎり、ブレードを３つ失う。
// Base blade 5. Lone-member NEGATIVE constant (sign=negative).
// ====================================================================

#[test]
fn seras_pb1015_loses_three_blade_when_alone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let seras = game.id("PL!HS-pb1-015-R"); // base blade 5
    assert_eq!(game.db.get_card(seras).unwrap().blade, 5);
    game.add_to_stage(MemberArea::Center, seras);

    // Alone → −3 blade modifier.
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(seras),
        -3,
        "lone on stage → ブレードを３つ失う"
    );

    // A friend arrives → loss stops.
    game.add_to_stage(MemberArea::LeftSide, game.id(FILLER));
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(seras),
        0,
        "ほかのメンバーがいる → no loss"
    );

    // Friend leaves → loss returns.
    game.state.player1.stage.stage[0] = -1;
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(seras), -3);
}

// ====================================================================
// B2 PL!HS-bp6-002-R 村野さやか — 常時:
// 自分のステージにほかのメンバーがいないかぎり、ブレードを２つ得る。
// Mirror of B1 with positive sign.
// ====================================================================

#[test]
fn sayaka_bp6002_gains_two_blade_when_alone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sayaka = game.id("PL!HS-bp6-002-R");
    assert_eq!(game.db.get_card(sayaka).unwrap().blade, 2);
    game.add_to_stage(MemberArea::Center, sayaka);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(sayaka),
        2,
        "lone on stage → blade+2"
    );

    game.add_to_stage(MemberArea::RightSide, game.new_id(FILLER));
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(sayaka),
        0,
        "not alone anymore → bonus off"
    );
}

// ====================================================================
// B3 PL!SP-pb1-002-R 唐可可 — 常時:
// 自分のエネルギーが12枚以上ある場合、ライブの合計スコアを＋１する。
// ====================================================================

#[test]
fn keke_spbp1002_twelve_energy_live_total_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let keke = game.id("PL!SP-pb1-002-R");
    game.add_to_stage(MemberArea::Center, keke);

    // 11 < 12 → nothing.
    game.give_energy(11);
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);

    // Exactly 12 → live total +1.
    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "エネルギーが12枚以上 → live total +1"
    );
}

// ====================================================================
// B4 PL!-bp4-004-R 園田海未 — 登場:
// 自分の成功ライブカード置き場にあるカードのスコアの合計が６以上の場合、
// エネルギーを2枚アクティブにする。
// ====================================================================

#[test]
fn umi_bp4004_success_score_six_activates_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp4-004-R");
    let live6 = game.id("PL!SP-bp1-027-L"); // Sing！Shine！Smile！ score 6
    game.add_to_stage(MemberArea::Center, umi);
    game.state.player1.success_live_card_zone.add_card(live6);

    // 5 energies: 2 active / 3 waiting → activate 2 → 4 active.
    for _ in 0..5 {
        let e = game.new_id("LL-E-001-SD");
        game.state.player1.energy_zone.cards.push(e);
    }
    game.state.player1.energy_zone.set_active_count(2);

    fire_trigger(&mut game, umi, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        4,
        "score total 6 ≥ 6 → activate 2"
    );
}

#[test]
fn umi_bp4004_below_threshold_activates_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp4-004-R");
    let live5 = game.id("PL!S-PR-024-PR"); // 勇気はどこに? score 5
    game.add_to_stage(MemberArea::Center, umi);
    game.state.player1.success_live_card_zone.add_card(live5);
    for _ in 0..4 {
        let e = game.new_id("LL-E-001-SD");
        game.state.player1.energy_zone.cards.push(e);
    }
    game.state.player1.energy_zone.set_active_count(1);

    fire_trigger(&mut game, umi, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        1,
        "score total 5 < 6 → nothing"
    );
}

// ====================================================================
// B5 PL!S-bp7-002-R 桜内梨子 — 登場:
// 自分のステージにコスト9以上の『Aqours』のメンバーがいる場合、カードを1枚引く。
// ====================================================================

#[test]
fn riko_bp7002_cost_nine_aqours_on_stage_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let riko = game.id("PL!S-bp7-002-R");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.add_to_stage(MemberArea::Center, riko);
    // Boundary case: EXACTLY cost 9 Aqours member (国木田花丸 pb1-007).
    game.add_to_stage(MemberArea::LeftSide, game.id("PL!S-pb1-007-R"));

    let hand_before = game.state.player1.hand.cards.len();
    fire_trigger(&mut game, riko, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "cost 9 (boundary >=) Aqours member present → draw 1"
    );
}

#[test]
fn riko_bp7002_group_and_cost_filters() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let riko = game.id("PL!S-bp7-002-R");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.add_to_stage(MemberArea::Center, riko);

    // Non-Aqours member at exactly cost 9 (村野さやか) → group filter fails.
    game.add_to_stage(MemberArea::LeftSide, game.id("PL!HS-bp6-002-R"));
    let hand_before = game.state.player1.hand.cards.len();
    fire_trigger(&mut game, riko, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "cost 9 but NOT 『Aqours』 → no draw"
    );

    // Cheap Aqours member (高海千歌 cost 2) → cost filter fails.
    game.add_to_stage(MemberArea::LeftSide, game.id("PL!S-PR-025-PR"));
    fire_trigger(&mut game, riko, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "『Aqours』 but cost 2 < 9 → no draw"
    );
}

// ====================================================================
// B6/B7 PL!S-bp7-020-L HAPPY PARTY TRAIN (score 3,
// need {heart02:1, heart04:2, heart05:2, heart0:3}).
// ab#0 ライブ開始時: 自分のステージにいるすべてのメンバーがアクティブ状態の場合、
//   このカードの必要ハートを {{heart0}} 減らす。
// ab#1 ライブ開始時: 自分のデッキの下からカードを1枚控え室に置く。
//   それが『Aqours』のメンバーカードの場合、このカードの必要ハートを {{heart0}} 減らす。
// ====================================================================

fn hpt_in_live_zone(game: &mut TestGame) -> i16 {
    let hpt = game.id("PL!S-bp7-020-L");
    game.state.player1.live_card_zone.cards.push(hpt);
    hpt
}

#[test]
fn happy_party_train_all_active_reduces_need_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hpt = hpt_in_live_zone(&mut game);

    let m1 = game.new_id("PL!S-sd1-001-SD");
    let m2 = game.new_id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = m1;
    game.state.player1.stage.stage[1] = m2;
    game.state.mods.add_orientation_modifier(m1, "active");
    game.state.mods.add_orientation_modifier(m2, "active");

    fire_trigger(&mut game, hpt, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(hpt, HeartColor::Heart00),
        -1,
        "all members active → 必要ハート heart0 −1"
    );
}

#[test]
fn happy_party_train_waited_or_empty_stage_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hpt = hpt_in_live_zone(&mut game);

    // One member WAITED → not all active.
    let m1 = game.new_id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = m1;
    game.state.mods.add_orientation_modifier(m1, "wait");
    fire_trigger(&mut game, hpt, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(hpt, HeartColor::Heart00),
        0,
        "a waited member breaks すべて…アクティブ"
    );

    // Empty stage → condition cannot hold (count >= 1 required).
    game.state.player1.stage.stage[0] = -1;
    fire_trigger(&mut game, hpt, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(hpt, HeartColor::Heart00),
        0,
        "empty stage → すべてのメンバーがアクティブ is not met"
    );
}

#[test]
fn happy_party_train_mill_aqours_member_reduces_need_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hpt = hpt_in_live_zone(&mut game);

    // Bottom of deck (last pushed) = Aqours member.
    let filler = game.id(FILLER); // μ's, sits above the bottom
    let aqours_member = game.id("PL!S-sd1-001-SD");
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(aqours_member);

    // ab#1 is the deck-bottom mill (ab#0 is the all-active check).
    fire_trigger_nth(&mut game, hpt, AbilityTrigger::LiveStart, "ライブ開始時", 1);

    assert!(
        game.state.player1.waitroom.cards.contains(&aqours_member),
        "bottom card was milled to the waitroom"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(hpt, HeartColor::Heart00),
        -1,
        "milled card WAS 『Aqours』 member → need heart0 −1"
    );
}

#[test]
fn happy_party_train_mill_non_aqours_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hpt = hpt_in_live_zone(&mut game);

    let mus_member = game.id(FILLER); // μ's member on the bottom
    game.state.player1.main_deck.cards.push(mus_member);

    fire_trigger_nth(&mut game, hpt, AbilityTrigger::LiveStart, "ライブ開始時", 1);

    assert!(game.state.player1.waitroom.cards.contains(&mus_member));
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(hpt, HeartColor::Heart00),
        0,
        "μ's member milled → NO reduction"
    );
}

// ====================================================================
// B8 PL!-bp4-021-L ?←HEARTBEAT (score 6) — ライブ開始時 escalating:
// success zone score ≥6 → 必要ハート heart0 −1; ≥9 → さらにスコア+1。
// ====================================================================

#[test]
fn heartbeat_bp4021_escalating_thresholds() {
    let db = load_real_database();

    // Fresh game per tier — modify_required_hearts / score modifiers STACK
    // across repeated firings, so each threshold gets its own board.
    let run = |success_scores: &[i16]| -> (i32, i32) {
        let mut game = TestGame::new(db.clone());
        let hb = game.id("PL!-bp4-021-L");
        game.state.player1.live_card_zone.cards.push(hb);
        for (k, &score) in success_scores.iter().enumerate() {
            let (no, _) = match score {
                1 => ("PL!-sd1-019-SD", 1),          // START:DASH!!
                5 => ("PL!S-PR-024-PR", 5),          // 勇気はどこに?
                _ => ("PL!SP-bp1-027-L", 6),         // Sing！Shine！Smile！
            };
            let id = if k == 0 { game.id(no) } else { game.new_id(no) };
            game.state.player1.success_live_card_zone.add_card(id);
        }
        fire_trigger(&mut game, hb, AbilityTrigger::LiveStart, "ライブ開始時");
        (
            game.state
                .mods
                .get_need_heart_modifier(hb, HeartColor::Heart00),
            game.state.mods.get_score_modifier(hb),
        )
    };

    let (need, score) = run(&[1, 1, 1]); // total 3
    assert_eq!(need, 0, "total 3 < 6 → nothing");
    assert_eq!(score, 0);

    let (need, score) = run(&[6]); // total 6
    assert_eq!(need, -1, "total 6 ≥ 6 → 必要ハート heart0 −1");
    assert_eq!(score, 0, "total 6 < 9 → NO score bonus yet");

    let (need, score) = run(&[6, 5]); // total 11
    assert_eq!(need, -1);
    assert_eq!(score, 1, "total 11 ≥ 9 → スコア +1");
}

// ====================================================================
// B9 PL!N-bp4-028-L stars we chase (score 2) — ライブ開始時:
// 控え室にカード名の異なる『虹ヶ咲』ライブカード 4枚以上 → +1；
// 6枚以上 → 代わりに +2。Duplicates do NOT count toward distinct.
// ====================================================================

const NIJI_LIVES: [&str; 6] = [
    "PL!N-bp1-025-L",  // 虹色Passions！
    "PL!N-bp1-026-L",  // Poppin' Up!
    "PL!N-bp1-027-L",  // Solitude Rain
    "PL!N-bp1-028-L",  // Butterfly
    "PL!N-bp1-029-L",  // Eutopia
    "PL!N-sd1-025-SD", // Colorful Dreams! Colorful Smiles!
];

#[test]
fn stars_we_chase_four_distinct_lives_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let swc = game.id("PL!N-bp4-028-L");
    game.state.player1.live_card_zone.cards.push(swc);
    for no in NIJI_LIVES.iter().take(4) {
        game.state.player1.waitroom.cards.push(game.id(no));
    }
    fire_trigger(&mut game, swc, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state.mods.get_score_modifier(swc),
        1,
        "4 distinct-name 虹ヶ咲 lives → +1"
    );
}

#[test]
fn stars_we_chase_six_distinct_lives_instead_plus_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let swc = game.id("PL!N-bp4-028-L");
    game.state.player1.live_card_zone.cards.push(swc);
    for no in NIJI_LIVES {
        game.state.player1.waitroom.cards.push(game.id(no));
    }
    fire_trigger(&mut game, swc, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state.mods.get_score_modifier(swc),
        2,
        "6 distinct-name 虹ヶ咲 lives → 代わりに +2"
    );
}

#[test]
fn stars_we_chase_duplicates_and_three_distinct_do_not_qualify() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let swc = game.id("PL!N-bp4-028-L");
    game.state.player1.live_card_zone.cards.push(swc);

    // Six copies of ONE name = only 1 DISTINCT name → below 4 → nothing.
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id(NIJI_LIVES[0]));
    for _ in 1..6 {
        let copy = game.new_id(NIJI_LIVES[0]);
        game.state.player1.waitroom.cards.push(copy);
    }
    fire_trigger(&mut game, swc, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state.mods.get_score_modifier(swc),
        0,
        "6 same-name lives ≠ カード名の異なる4枚 → nothing"
    );

    // Three distinct → still below threshold.
    game.state.player1.waitroom.cards.clear();
    for no in NIJI_LIVES.iter().take(3) {
        game.state.player1.waitroom.cards.push(game.id(no));
    }
    fire_trigger(&mut game, swc, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state.mods.get_score_modifier(swc),
        0,
        "3 distinct < 4 → nothing"
    );
}

// ====================================================================
// B10 PL!SP-pb2-023-N 澁谷かのん — 常時 (sequential double-threshold):
// エネルギー6枚以上 → heart02；8枚以上 → さらに heart02。
// ====================================================================

#[test]
fn kanon_spbp2023_stacked_energy_threshold_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-pb2-023-N");
    game.add_to_stage(MemberArea::Center, kanon);

    // 5 → neither tier.
    game.give_energy(5);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(kanon, HeartColor::Heart02),
        0
    );

    // 7 → first tier only.
    game.give_energy(2);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(kanon, HeartColor::Heart02),
        1,
        "energy 7 ≥ 6 → one heart02"
    );

    // 8 → both tiers stack.
    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(kanon, HeartColor::Heart02),
        2,
        "energy 8 ≥ 8 → さらに one more heart02 (total 2)"
    );
}

// ====================================================================
// B11 PL!N-sd2-027-P 決意の光 (score 5) — ライブ開始時:
// 『虹ヶ咲』のメンバーを3人までウェイトにしてもよい：
// これによりウェイトにしたメンバー1人につき、このカードのスコアを＋１する。
// ====================================================================

#[test]
fn ketsumi_no_hikari_per_waited_member_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!N-sd2-027-P");
    game.state.player1.live_card_zone.cards.push(live);

    let niji_a = game.id("PL!N-PR-019-PR"); // 中須かすみ
    let niji_b = game.id("PL!N-PR-012-PR"); // 三船栞子
    let niji_c = game.id("PL!N-PR-014-PR"); // 鐘嵐珠
    game.state.player1.stage.stage[0] = niji_a;
    game.state.player1.stage.stage[1] = niji_b;
    game.state.player1.stage.stage[2] = niji_c;

    // Round 1: decline the optional cost → +0, nobody waited.
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice(), "optional wait-cost must be offered");
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(0), // decline
        Some("SelectCard") => game.select_indices(&[]),
        other => panic!("unexpected choice type {other:?}"),
    }
    assert!(
        !game.has_pending_choice(),
        "declining the optional cost ends the ability"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "waited nobody → +0"
    );

    // Round 2: wait ALL THREE → +3.
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice());
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(1), // accept
        Some("SelectCard") => game.select_indices(&[0, 1, 2]),
        other => panic!("unexpected choice type {other:?}"),
    }
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(niji_a),
        Some("wait"),
        "member 1 waited by the cost"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(niji_c),
        Some("wait"),
        "member 3 waited by the cost"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(live),
        3,
        "ウェイトにした1人につきスコア+1 → three waits = +3"
    );
}

// ====================================================================
// B12 PL!N-bp4-023-N ミア・テイラー — 登場:
// 『虹ヶ咲』のメンバー1人をウェイトにしてもよい：カードを1枚引き、手札を1枚控え室に置く。
// ====================================================================

#[test]
fn mia_bp4023_optional_wait_cost_gates_draw_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mia = game.id("PL!N-bp4-023-N");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.add_to_stage(MemberArea::Center, mia);
    let niji = game.id("PL!N-PR-019-PR");
    game.add_to_stage(MemberArea::LeftSide, niji);

    let keep = game.id("PL!N-PR-012-PR");
    game.add_to_hand(keep);

    // SKIP the optional cost → effect does not run at all.
    let hand_before = game.state.player1.hand.cards.len(); // 1
    fire_trigger(&mut game, mia, AbilityTrigger::Debut, "登場");
    if game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectTarget") => game.select_option(0), // decline
            Some("SelectCard") => game.select_indices(&[]),
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "declined cost → no draw/discard"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(niji),
        None,
        "member untouched when declined"
    );

    // ACCEPT: wait the Niji member → draw 1, discard 1.
    let sacrifice = game.new_id(FILLER);
    game.add_to_hand(sacrifice);
    fire_trigger(&mut game, mia, AbilityTrigger::Debut, "登場");
    assert!(game.has_pending_choice(), "cost offer expected");
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(1), // accept
        Some("SelectCard") => game.select_indices(&[0]),
        other => panic!("unexpected {other:?}"),
    }
    // The accepted cost asks WHICH stage member to wait, then the effect's
    // draw/discard runs — at most two more choices.
    for _ in 0..3 {
        if !game.has_pending_choice() {
            break;
        }
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. }
                if zone == "stage" =>
            {
                let idx = game
                    .state
                    .player1
                    .stage
                    .stage
                    .iter()
                    .position(|&c| c == niji)
                    .expect("Niji member still on stage");
                game.select_indices(&[idx]);
            }
            _ => {
                let idx = game
                    .state
                    .player1
                    .hand
                    .cards
                    .iter()
                    .position(|&c| c == sacrifice)
                    .unwrap_or(0);
                game.select_indices(&[idx]);
            }
        }
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(niji),
        Some("wait"),
        "accepting waited the Niji member"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&sacrifice),
        "the discard step put the chosen card in the waitroom"
    );
    assert!(game.state.player1.hand.cards.contains(&keep));
}

// ====================================================================
// B13 PL!HS-bp5-011-N 大沢瑠璃乃 — 登場: カードを1枚引く。(smoke)
// ====================================================================

#[test]
fn rurino_bp5011_debut_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp5-011-N");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.add_to_stage(MemberArea::Center, rurino);

    let hand_before = game.state.player1.hand.cards.len();
    fire_trigger(&mut game, rurino, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "登場 → draw 1"
    );
}
