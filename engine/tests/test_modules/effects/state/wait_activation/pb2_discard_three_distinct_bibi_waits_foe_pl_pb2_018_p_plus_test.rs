//! 絢瀬絵里 `PL!-pb2-018-P＋` (登場／ライブ開始時)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   登場／ライブ開始時 — 手札の名前の異なる『BiBi』のメンバーカードを3枚控え室に置いても
//!   よい：自分のステージにいるメンバーが『BiBi』のみの場合、相手のステージにいるメンバー
//!   1人をウェイトにする。
//!
//! In English: you may discard 3 differently-named BiBi member cards from your hand;
//! if every member on your stage is BiBi, put one opponent member into wait.
//!
//! # Parser work done for this card
//!
//! The cost was losing `distinct`: 「手札の**名前の異なる**『BiBi』のメンバーカードを3枚」
//! uses the short form, and only the explicit 「カード名の異なる」 was recognised, so two
//! prints of one character satisfied a three-card cost. Fixed in
//! `_post_fix_cost_and_select`, scoped to COSTS and select/look actions only — applying
//! it to the effect path as well changed `PL!SP-bp2-005-P＋` (Mia) and the Q118 pair,
//! where the same short phrase carries different intent.
//!
//! # The positive case now fires, and never fired because of THIS FILE
//!
//! This file used to carry an OPEN GAP claiming the positive case did not fire and that
//! "the optional 「3枚控え室に置いてもよい」 cost is never raised afterwards". That
//! diagnosis was wrong, and it was wrong in the most embarrassing way: the fixture
//! itself could not satisfy the card.
//!
//! `BIBI_2` was `PL!-pb2-003-PP`, annotated 矢澤にこ — but that card is **南ことり**, a
//! Printemps member. Only one of the three "BiBi" discards was actually BiBi, so
//! `offer_hand_cost_choice` correctly found fewer than three eligible cards and
//! auto-skipped the optional cost, which then gated the effect off. The engine was
//! right; the test was not.
//!
//! With three genuinely distinct BiBi members the whole chain runs: the auto-ability
//! order prompt, then 3 + 2 + 1 hand selections, then the wait.
//!
//! **The lesson, kept in the assertions below:** every fixture constant in this file
//! now carries an explicit `assert_all_in_group(..., "BiBi", ...)` / `assert_card_type`.
//! A mistyped or mis-annotated card number is silent — the card exists, it is just the
//! wrong card — and it produced a confident, wrong, multi-paragraph diagnosis here and
//! in `PL!-pb2-016-P＋`.
//!
//! The 3-slot trap applies here as it does to `PL!-pb2-029-N`: ERI takes the centre, so
//! only ONE ally can be staged beside her without baton-touching the other.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-pb2-018-P＋";
/// BiBi members with THREE DIFFERENT names — the cost is distinct by name.
/// Verified: BiBi has exactly three members (絢瀬絵里 / 西木野真姫 / 矢澤にこ), so these
/// are not a sample but the complete set the cost can be paid with.
const BIBI_1: &str = "PL!-pb2-002-PP"; // 絢瀬絵里
const BIBI_2: &str = "PL!-pb2-006-PP"; // 西木野真姫
const BIBI_3: &str = "PL!-PR-009-PR"; // 矢澤にこ
/// Not BiBi.
const NON_BIBI: &str = "PL!S-bp5-012-N"; // 松浦果南, AZALEA
const FILLER: &str = "PL!-sd1-010-SD";

/// Check the fixture constants really are what the test claims. Called by every test
/// that uses the BiBi trio, because a wrong constant here silently disables the cost.
fn assert_fixture(db: std::sync::Arc<rabuka_engine::core::card::CardDatabase>) {
    let game = TestGame::new(db);
    game.assert_all_in_group(
        &[game.id(BIBI_1), game.id(BIBI_2), game.id(BIBI_3)],
        "BiBi",
        "the three discard cost cards must all be BiBi — an earlier revision used \
         PL!-pb2-003-PP, which is 南ことり (Printemps), and the cost could never be paid",
    );
    game.assert_card_in_group(
        game.id(NON_BIBI),
        "AZALEA",
        "the non-BiBi control",
    );
}

/// Debut ERI with `ally` on the left, `hand` seeded, `foe` on the opponent's stage, and
/// resolve every prompt: pick the second auto-ability (the one carrying the cost) and
/// pay the optional discard. Returns whether the foe ended up waited.
fn run(ally: &str, hand: &[&str], foe: &str) -> bool {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.clear();
    for c in hand {
        game.state.player1.hand.cards.push(game.id(c));
    }
    let foe_id = game.id(foe);
    game.state.player2.stage.stage[MemberArea::Center as usize] = foe_id;

    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    game.add_to_hand(eri);
    game.give_energy(30);
    game.state.player1.stage.stage[0] = game.id(ally);
    game.play_to_stage(eri, MemberArea::Center);

    // The debut offers BOTH of ERI's 登場 abilities, so the order prompt appears first.
    // Choosing nothing — which the old helper did — declines the whole thing, which is
    // why the old file measured "the cost is never raised" and drew a conclusion from
    // it. Choose the ability that carries the cost, then pay it.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        let prompt = format!("{:?}", game.get_pending_choice());
        if prompt.starts_with("SelectAutoAbility") {
            game.select_indices(&[1]);
        } else {
            game.select_indices(&[0]);
        }
    }
    game.state.mods.get_orientation_modifier(foe_id) == Some("wait")
}

/// THE case: all-BiBi stage, three distinct BiBi discards paid, foe waited.
#[test]
fn the_positive_case_fires() {
    assert_fixture(load_real_database());
    assert!(
        run(BIBI_1, &[BIBI_1, BIBI_2, BIBI_3], NON_BIBI),
        "自分のステージにいるメンバーが『BiBi』のみの場合、相手…1人をウェイトにする — \
         the stage is all-BiBi and the optional three-card discard was paid with three \
         differently-named BiBi members, so the foe must be waited"
    );
}

/// The cost really is paid: three cards reach the 控え室.
#[test]
fn the_optional_cost_moves_three_distinct_bibi_to_the_waitroom() {
    assert_fixture(load_real_database());
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.clear();
    for c in [BIBI_1, BIBI_2, BIBI_3] {
        game.state.player1.hand.cards.push(game.id(c));
    }
    let eri = game.id(ERI);
    game.add_to_hand(eri);
    game.give_energy(30);
    game.state.player1.stage.stage[0] = game.id(BIBI_1);
    let waitroom_before = game.state.player1.waitroom.cards.len();

    game.play_to_stage(eri, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        let prompt = format!("{:?}", game.get_pending_choice());
        if prompt.starts_with("SelectAutoAbility") {
            game.select_indices(&[1]);
        } else {
            game.select_indices(&[0]);
        }
    }

    assert_eq!(
        game.state.player1.waitroom.cards.len() - waitroom_before,
        3,
        "手札の名前の異なる『BiBi』のメンバーカードを3枚控え室に置いてもよい — paying the \
         optional cost moves exactly three cards to the 控え室"
    );
}

/// Skipping the optional cost means the effect does not fire — 「…よい」 really is
/// optional, and the cost gates the effect rather than merely accompanying it.
#[test]
fn skipping_the_optional_cost_means_no_wait() {
    assert_fixture(load_real_database());
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.clear();
    for c in [BIBI_1, BIBI_2, BIBI_3] {
        game.state.player1.hand.cards.push(game.id(c));
    }
    let foe_id = game.id(NON_BIBI);
    game.state.player2.stage.stage[MemberArea::Center as usize] = foe_id;
    let eri = game.id(ERI);
    game.add_to_hand(eri);
    game.give_energy(30);
    game.state.player1.stage.stage[0] = game.id(BIBI_1);

    game.play_to_stage(eri, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        let prompt = format!("{:?}", game.get_pending_choice());
        if prompt.starts_with("SelectAutoAbility") {
            game.select_indices(&[1]);
        } else if prompt.contains("skip") || prompt.contains("スキップ") {
            // The pay/skip gate: answer "skip".
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    assert_ne!(
        game.state.mods.get_orientation_modifier(foe_id),
        Some("wait"),
        "…置いてもよい — declining the optional cost must leave the gated effect unfired"
    );
}

/// The stage-exclusivity gate, breaking the positive on its own.
#[test]
fn a_non_bibi_ally_blocks_the_wait() {
    assert_fixture(load_real_database());
    assert!(
        !run(NON_BIBI, &[BIBI_1, BIBI_2, BIBI_3], NON_BIBI),
        "自分のステージにいるメンバーが『BiBi』のみの場合 — an AZALEA ally breaks \
         exclusivity, so the foe is not waited even with the cost paid"
    );
}

/// A hand with no BiBi at all has nothing the cost can pay with.
#[test]
fn a_hand_without_bibi_produces_no_wait() {
    assert_fixture(load_real_database());
    assert!(
        !run(BIBI_1, &[NON_BIBI, NON_BIBI, NON_BIBI], NON_BIBI),
        "『BiBi』のメンバーカードを3枚 — with no BiBi in hand the optional cost has \
         nothing it can pay with, so it auto-skips and the effect does not fire"
    );
}

/// Two prints of one character are not two distinct names.
#[test]
fn two_prints_are_one_name() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let a = game.id(BIBI_1);
    let b = game.new_id(BIBI_1);
    game.assert_same_card_name(a, b, "duplicate fixture");
}

/// The cost carries `distinct` — pinned so it cannot silently regress to the
/// vacuous three-of-anything form.
#[test]
fn the_cost_requires_three_distinct_names() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let id = game.id(ERI);
    let cost = {
        let owned = game
            .db
            .get_card(id)
            .unwrap()
            .resolved_abilities()
            .find(|a| {
                a.triggers
                    .as_deref()
                    .is_some_and(|t| t.contains("登場") && t.contains("ライブ開始時"))
            });
        owned.and_then(|a| a.cost.clone())
    }
    .expect("the 登場／ライブ開始時 ability has a cost");
    let json = serde_json::to_value(&cost).unwrap();
    assert_eq!(
        cost.distinct_any(),
        Some(rabuka_engine::card::DistinctType::CardName),
        "名前の異なる『BiBi』のメンバーカードを3枚 — the cost is distinct by name"
    );
    assert_eq!(
        json.get("count").and_then(|v| v.as_i64()),
        Some(3),
        "…を3枚控え室に置く"
    );
    assert_eq!(
        json.get("optional").and_then(|v| v.as_bool()),
        Some(true),
        "…置いてもよい — the cost is optional"
    );
}
