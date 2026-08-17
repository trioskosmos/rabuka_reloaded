/// BP07: fill gaps in 自動 (Auto) coverage — abilities whose cards were not
/// referenced by any test (from cards/coverage_report.py). Drive the real engine
/// (real ability execution), not just injected events, so the cards work "as
/// written".
///
/// Untested 自動 batch:
///   area-move (moved) → gain resource:
///     PL!SP-bp7-014-N 嵐 千砂都 : area move → blade×2
///     PL!SP-sd2-012-SD2 澁谷かのん : area move → heart02 (対戦相手の効果でも発動)
///     PL!SP-sd2-022-SD2 鬼塚冬毬 : area move → heart03 (対戦相手の効果でも発動)
///   stage→discard → effect:
///     PL!N-bp7-014-N 中須かすみ : → add 虹ヶ咲 live from discard to hand
///     PL!HS-bp2-015-N 藤島 慈 : → draw 2, discard 1
///     PL!HS-bp6-019-N 大沢瑠璃乃 : → draw 2, discard 2
///   baton-touch debut → draw:
///     PL!N-PR-025-PR 優木せつ菜 : baton touch debut → draw 1 (or_condition)
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::game_modifiers::CardOrientation;
use rabuka_engine::zones::MemberArea;

/// きなこ: 起動 → swap with a chosen member (used as the real area-mover).
const MOVER: &str = "PL!SP-bp5-006-R";
const FILLER: &str = "PL!-sd1-010-SD";
const NIJI_LIVE: &str = "PL!N-bp1-026-L"; // 虹ヶ咲 live card (Poppin' Up!)
const LIELIA_LIVE: &str = "PL!SP-bp1-023-L"; // Liella live card, score 1 (≤3)
const NIJI_MEMBER: &str = "PL!N-bp7-001-R"; // 上原歩夢, 虹ヶ咲 member, cost 4
const NO_BLADE_HEART: &str = "PL!SP-bp1-021-N"; // ウィーン, member, no blade heart
const ENERGY: &str = "LL-E-001-SD";

use rabuka_engine::ability::types::Choice;

/// Drain auto-ability ordering + card-selection + option choices until nothing
/// is pending. Mirror the robust loop in auto_system_stress_test (no break;
/// select `count` cards; fall back to index 0). Accepts conditional optionals.
fn drain_auto(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { count, .. } => {
                if *count > 0 && *count < 10 {
                    game.select_indices(&(0..*count).collect::<Vec<_>>());
                } else {
                    game.select_indices(&[0]);
                }
            }
            Choice::SelectTarget { target, options, .. }
                if target == "conditional_optional" =>
            {
                // Accept the optional (do the follow-up action).
                game.select_choice_option(1);
            }
            Choice::SelectTarget { target, .. }
                if target == "position|destination" || target == "area_select" =>
            {
                let acts = game.generated_actions();
                if acts.is_empty() {
                    game.select_indices(&[]);
                } else {
                    game.select_generated(0);
                }
            }
            _ => game.select_indices(&[0]),
        }
    }
}

/// Baton-touch `arriver` onto `replaced`'s area, replacing it to the waitroom.
/// Returns the waitroom replacement card id.
fn baton_touch_off(game: &mut TestGame, replaced: i16, arriver: i16, area: MemberArea) {
    game.give_energy(30);
    game.state.player1.stage.set_area(area, replaced);
    game.state.player1.hand.cards.push(arriver);
    game.play_to_stage(arriver, area);
}

fn fill_deck(game: &mut TestGame) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
}

/// Activate the MOVER (きなこ) and swap it into `target_area`, moving whatever
/// occupies that area. Assumes the target is already on stage. Drains autos.
fn activate_swap(game: &mut TestGame, target_area: MemberArea) {
    let mover = game.id(MOVER);
    // The mover's 起動 cost mills 3 from deck top; ensure there's material.
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
    let mover_area = match target_area {
        MemberArea::LeftSide => MemberArea::RightSide,
        _ => MemberArea::LeftSide,
    };
    game.add_to_stage(mover_area, mover);
    game.give_energy(10);

    game.activate_ability(mover);
    game.drain_auto_ability_choices();
    let acts = game.generated_actions();
    let target_area_str = match target_area {
        MemberArea::LeftSide => "left",
        MemberArea::Center => "center",
        MemberArea::RightSide => "right",
    };
    let idx = acts
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some(target_area_str))
        .unwrap_or_else(|| panic!("no swap option to {}", target_area_str));
    game.select_generated(idx);
    game.drain_auto_ability_choices();
}

/// Place `target` on stage and area-move it via a real swap with MOVER.
fn area_move_by_swap(game: &mut TestGame, target: i16, target_area: MemberArea) {
    game.add_to_stage(target_area, target);
    activate_swap(game, target_area);
}

fn heart_mod(game: &TestGame, cid: i16, hc: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(cid, hc)
}

fn blade(game: &TestGame, cid: i16) -> i32 {
    game.state.mods.get_blade_modifier(cid)
}

// ===================================================================
// area-move → gain resource
// ===================================================================

/// 嵐千砂都: このメンバーがエリアを移動したとき…ブレード×2を得る。
#[test]
fn chisato_area_move_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id("PL!SP-bp7-014-N");
    area_move_by_swap(&mut game, target, MemberArea::LeftSide);
    assert_eq!(blade(&game, target), 2, "嵐千砂都 should gain 2 blades on area move");
}

/// 澁谷かのん: エリア移動でheart02を得る(相手の効果でも発動)。
#[test]
fn kanon_area_move_gains_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id("PL!SP-sd2-012-SD2");
    area_move_by_swap(&mut game, target, MemberArea::Center);
    assert_eq!(heart_mod(&game, target, HeartColor::Heart02), 1, "澁谷かのん should gain heart02 on area move");
}

/// 鬼塚冬毬: エリア移動でheart03を得る(相手の効果でも発動)。
#[test]
fn fuyumari_area_move_gains_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id("PL!SP-sd2-022-SD2");
    area_move_by_swap(&mut game, target, MemberArea::Center);
    assert_eq!(heart_mod(&game, target, HeartColor::Heart03), 1, "鬼塚冬毬 should gain heart03 on area move");
}

// ===================================================================
// stage→discard (real baton touch)
// ===================================================================

/// 中須かすみ: このメンバーがステージから控え室に置かれたとき、
/// 自分の控え室にある『虹ヶ咲』のライブカードを1枚手札に加える。
#[test]
fn kasumi_stage_to_discard_adds_niji_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp7-014-N");
    let niji = game.id(NIJI_LIVE);
    let arriver = game.id("PL!-sd1-002-SD");
    game.state.player1.waitroom.cards.push(niji);

    baton_touch_off(&mut game, kasumi, arriver, MemberArea::Center);
    drain_auto(&mut game);

    assert!(
        game.state.player1.waitroom.cards.contains(&kasumi),
        "かすみ should be in the waitroom after baton touch"
    );
    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "かすみ ab#0 should add a 虹ヶ咲 live card from the discard to hand"
    );
}

/// 藤島慈: このメンバーがステージから控え室に置かれたとき、カードを2枚引き、
/// 手札を1枚控え室に置く。(net hand +1)
#[test]
fn toko_stage_to_discard_draws_2_discards_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let toko = game.id("PL!HS-bp2-015-N");
    let arriver = game.id("PL!-sd1-002-SD");
    fill_deck(&mut game);

    baton_touch_off(&mut game, toko, arriver, MemberArea::Center);
    drain_auto(&mut game);

    // hand empty after playing arriver; auto draws 2, discards 1 → final 1.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "藤島慈: draw 2, discard 1 → net hand +1 (from 0 after playing arriver)"
    );
}

/// 大沢瑠璃乃: このメンバーがステージから控え室に置かれたとき、カードを2枚引き、
/// 手札を2枚控え室に置く。(net hand 0)
#[test]
fn rurino_stage_to_discard_draws_2_discards_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rurino = game.id("PL!HS-bp6-019-N");
    let arriver = game.id("PL!-sd1-002-SD");
    fill_deck(&mut game);

    baton_touch_off(&mut game, rurino, arriver, MemberArea::Center);
    drain_auto(&mut game);

    // hand empty after playing arriver; auto draws 2, discards 2 → final 0.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "大沢瑠璃乃: draw 2, discard 2 → net hand 0"
    );
}

// ===================================================================
// baton-touch debut → draw (or_condition)
// ===================================================================

/// 優木せつ菜: 自分のステージに、このメンバーか、ほかのメンバーがバトンタッチして
/// 登場したとき、カードを1枚引く。  (or_condition, ターン2回)
#[test]
fn setsuna_baton_touch_debut_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = game.id("PL!N-PR-025-PR");
    let filler = game.id(FILLER);
    let arriver = game.id("PL!-sd1-002-SD");
    fill_deck(&mut game);

    // setsuna on left; a filler on center. Baton-touch arriver onto center
    // → arriver debuts via baton touch → setsuna (on stage) draws 1.
    game.state.player1.stage.set_area(MemberArea::LeftSide, setsuna);
    baton_touch_off(&mut game, filler, arriver, MemberArea::Center);
    drain_auto(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "せつ菜 ab#0 should draw 1 when another member debuts via baton touch"
    );
}

// ===================================================================
// Complex / edge cases — the remaining untested 自動 abilities
// ===================================================================

/// 米女メイ: このメンバーがエリアを移動したとき、自分の控え室から、スコア3以下の
/// 『Liella!』のライブカードを1枚手札に加える。
#[test]
fn may_area_move_adds_liella_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let may = game.id("PL!SP-bp4-007-R");
    let liella = game.id(LIELIA_LIVE); // score 1, Liella, ≤3
    game.state.player1.waitroom.cards.push(liella);

    area_move_by_swap(&mut game, may, MemberArea::Center);

    assert!(
        game.state.player1.hand.cards.contains(&liella),
        "米女メイ ab#0 should add a score≤3 Liella live card from the discard to hand"
    );
}

/// 若菜四季 two-ability interaction: ab#0 起動 waits self + draws 1; ab#1 自動
/// activates her when a WAIT-state member area-moves.
#[test]
fn shiki_wait_then_area_move_reactivates() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shiki = game.id("PL!SP-bp7-008-R");
    fill_deck(&mut game);
    game.add_to_stage(MemberArea::Center, shiki);
    game.give_energy(10);

    // ab#0 起動: wait self → draw 1.
    game.activate_ability(shiki);
    game.drain_auto_ability_choices();
    assert_eq!(
        game.state.mods.orientation_modifiers.get(&shiki),
        Some(&CardOrientation::Wait),
        "若菜四季 ab#0 should wait herself"
    );

    // ab#1 自動: wait member area-moves → activate her.
    activate_swap(&mut game, MemberArea::Center);
    assert_ne!(
        game.state.mods.orientation_modifiers.get(&shiki),
        Some(&CardOrientation::Wait),
        "若菜四季 ab#1 should reactivate a wait member on area move"
    );
}

/// 夕霧綴理: このメンバーがステージから控え室に置かれたとき、デッキの上から5枚を見て、
/// その中のライブカードを1枚手札に加えてもよい。
#[test]
fn yuiguri_stage_to_discard_adds_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yuiguri = game.id("PL!HS-bp2-013-N");
    let arriver = game.id("PL!-sd1-002-SD");
    let niji = game.id(NIJI_LIVE);
    // Put the live card on deck top so it's among the looked-at 5.
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(niji);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }

    baton_touch_off(&mut game, yuiguri, arriver, MemberArea::Center);
    drain_auto(&mut game);

    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "夕霧綴理 ab#0 should reveal a live card from the looked-at 5 into hand"
    );
}

/// 村野さやか: このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に
/// 置いてもよい。そうした場合、ステージのメンバー1人はheart05＋ブレードを得る。
#[test]
fn sayaka_stage_to_discard_buffs_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp6-018-N");
    let arriver = game.id("PL!-sd1-002-SD");
    let target = game.id("PL!S-sd1-001-SD"); // a stage member to buff
    // A card in hand for the optional discard.
    game.state.player1.hand.cards.push(game.id(FILLER));
    game.state.player1.stage.set_area(MemberArea::LeftSide, target);

    baton_touch_off(&mut game, sayaka, arriver, MemberArea::Center);
    drain_auto(&mut game); // accepts conditional_optional, picks a member

    let h05 = game.state.mods.get_heart_modifier(target, HeartColor::Heart05);
    let bl = game.state.mods.get_blade_modifier(target);
    assert!(
        h05 >= 1 && bl >= 1,
        "村野さやか ab#0 should grant heart05+blade to a stage member (got h05={} blade={})",
        h05,
        bl
    );
}

/// 優木せつ菜: このメンバーがステージから控え室に置かれたとき、『虹ヶ咲』のメンバーと
/// バトンタッチしていた場合、エネルギーデッキからエネルギー1枚を登場したメンバーの下に置く。
#[test]
fn setsuna_bp7_baton_touch_places_energy_under_arriver() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = game.id("PL!N-bp7-019-N"); // 虹ヶ咲 member herself
    let arriver = game.id(NIJI_MEMBER); // 上原歩夢, 虹ヶ咲 member
    let energy = game.id(ENERGY);
    game.state.player1.energy_deck.cards.push(energy);

    // Baton-touch 歩夢 over せつ菜 (虹ヶ咲 → 虹ヶ咲) on center (slot 1).
    baton_touch_off(&mut game, setsuna, arriver, MemberArea::Center);
    drain_auto(&mut game);

    assert!(
        game.state.player1.waitroom.cards.contains(&setsuna),
        "せつ菜 should be in the waitroom after baton touch"
    );
    assert_eq!(
        game.state.player1.stage.under_cards[1].len(),
        1,
        "せつ菜 ab#0 should place 1 energy under the arriving 虹ヶ咲 member"
    );
    assert_eq!(
        game.state.player1.stage.under_cards[1][0],
        energy,
        "the card placed under the arriving member is the energy card"
    );
}

/// Reproduce the real yell flow for p1: draw `count` into the resolution zone,
/// record each as a revealed card, set yell_occurred, then run the auto scan.
fn yell_reveal_and_scan(game: &mut TestGame, count: u8) -> Vec<i16> {
    let pid = game.state.player1.id.clone();
    game.state.perform_cheer_check(&pid, count).unwrap();
    let revealed: Vec<i16> = game.state.resolution_zone.cards.iter().copied().collect();
    for &cid in &revealed {
        game.state.push_revealed_card(cid, None, false, Some(0), "yell");
    }
    game.state.yell_occurred = !revealed.is_empty();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);
    game.state.yell_occurred = false;
    game.drain_auto_ability_choices();
    revealed
}

/// 鬼塚夏美: エールにより公開された自分のカードの中にブレードハートを持つカードが
/// ないとき、ライブ終了時まで、heart02を得る。
#[test]
fn natsumi_yell_no_blade_heart_gains_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-bp2-020-N");
    game.state.player1.stage.set_area(MemberArea::Center, natsumi);

    // Deck of NON-blade-heart members so the yell reveals none with a blade heart.
    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(game.id(NO_BLADE_HEART));
    }

    yell_reveal_and_scan(&mut game, 3);

    assert_eq!(
        heart_mod(&game, natsumi, HeartColor::Heart02),
        1,
        "鬼塚夏美 ab#0 should gain heart02 when a yell reveals no blade-heart card"
    );
}

/// 三船栞子: ライブフェイズの間、自分のステージの『虹ヶ咲』のメンバー1人がウェイト
/// 状態になったとき、手札を1枚控え室に置いてもよい。そうしたとき、そのメンバーを
/// アクティブにする。
#[test]
fn shioriko_live_phase_wait_discard_activates() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shioriko = game.id("PL!N-bp7-022-N"); // 虹ヶ咲 member
    let waited = game.id(NIJI_MEMBER); // 上原歩夢, 虹ヶ咲 member
    game.state.player1.hand.cards.push(game.id(FILLER)); // optional discard
    game.state.player1.stage.set_area(MemberArea::Center, waited);
    game.state.player1.stage.set_area(MemberArea::LeftSide, shioriko);

    // Enter the live performance phase and record the wait state change.
    use rabuka_engine::core::types::Phase;
    game.state.current_phase = Phase::FirstAttackerPerformance;
    game.state
        .recently_state_changed
        .push((waited, "active".to_string(), "wait".to_string()));
    game.state.mods.orientation_modifiers.insert(waited, CardOrientation::Wait);

    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);

    // The auto offers the optional discard → accept → the waited member activates.
    drain_auto(&mut game);

    assert_ne!(
        game.state.mods.orientation_modifiers.get(&waited),
        Some(&CardOrientation::Wait),
        "三船栞子 ab#0 should activate a waited 虹ヶ咲 member during the live phase"
    );
}
