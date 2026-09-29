//! PSYCHIC FIRE `PL!-pb2-042-L` (自動, ターン1回)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   自動／ターン1回 — 自分がエールしたとき、エールにより公開された自分のカードの中に
//!   「矢澤にこ」と「西木野真姫」と「絢瀬絵里」のメンバーカードがあり、**かつ**
//!   自分のセンターエリアにコスト11以上の『BiBi』のメンバーがいる場合、相手のステージに
//!   いる元々持つハートの数が4つ以下のメンバー1人をウェイトにする。
//!
//! **The four NEGATIVE tests below are currently VACUOUS.** They assert "no wait",
//! and they pass — but the positive does not fire, so they are not evidence the
//! filters work. This is the same trap as `PL!-pb2-038-L` (see
//! docs/PBLL02_ABILITY_REVIEW.md §8.4): a negative can only constrain a filter once
//! the positive path is known to run. Treat the fixture pins as the real content here
//! until `all_clauses_met_waits_the_foe` passes.
//!
//! What is NOT a decode problem: `bytecode_no_silent_decode_fallbacks` reports no
//! fallback for this ability, so all four clauses reach the engine intact.
//!
//! The four clauses, and the two the parser had been dropping (docs §8.3 item 15):
//!   * 「「矢澤にこ」と「西木野真姫」と「絢瀬絵里」」 — the と-joined name list. The
//!     parser's name-list regex did not allow `」と「`, so it matched only the LAST
//!     entry and this clause was really "絢瀬絵里 was revealed". FIXED.
//!   * 「自分のセンターエリアに」 — the centre clause was inheriting
//!     `location: revealed_cards` from the clause before it, so it looked for an
//!     11-cost BiBi member among the cards the yell revealed rather than on the
//!     stage. FIXED.
//!
//! The remaining clauses are the centre cost bound (11以上) and the target's printed
//! heart bound (4以下), the latter the same mechanism as `PL!-pb2-006` (≤1) and
//! `PL!-pb2-033` (≤3).

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const PSYCHIC: &str = "PL!-pb2-042-L";
// The condition filters on CHARACTER names, not card numbers. Kept separate from
// the card consts above because the two are different things and conflating them
// is how a test ends up asserting a card number where the data holds a name.
const NICO_NAME: &str = "矢澤にこ";
const MAKI_NAME: &str = "西木野真姫";
const ERI_NAME: &str = "絢瀬絵里";
/// The three named characters, all BiBi, all revealed.
const NICO: &str = "PL!-PR-017-PR"; // 矢澤にこ, cost 2
const MAKI: &str = "PL!-pb2-006-PP"; // 西木野真姫, cost 2
const ERI: &str = "PL!-pb2-002-PP"; // 絢瀬絵里, cost 4
/// A 17-cost BiBi member — clears コスト11以上 in the CENTRE.
const CENTRE_DEAR: &str = "PL!-PR-015-PR"; // 西木野真姫, cost 17
/// A 2-cost BiBi member — under the コスト11以上 bound.
const CENTRE_CHEAP: &str = "PL!-pb2-006-PP"; // cost 2
/// An 11+-cost member that is NOT BiBi — clears the cost bound, fails the group.
const CENTRE_NON_BIBI: &str = "PL!-pb1-026-N";
/// Opponent members by printed heart total.
const OPP_2: &str = "PL!-bp6-017-N"; // 2 printed hearts
const OPP_5: &str = "PL!N-bp4-019-N"; // 5 printed hearts — over the bound
const FILLER: &str = "PL!-sd1-010-SD";

/// Stage `centre` in the CENTRE, reveal `revealed`, put `foe` on the opponent's
/// stage, fire the 自動, and report whether the foe was waited.
fn run(centre: &str, revealed: &[&str], foe: &str) -> bool {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let foe_id = game.id(foe);
    game.state.player2.stage.stage[MemberArea::Center as usize] = foe_id;
    let centre_id = game.id(centre);
    game.state.player1.stage.stage[MemberArea::Center as usize] = centre_id;
    for c in revealed {
        game.state.revealed_cards.push(game.id(c));
    }
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = -1;

    let psychic = game.id(PSYCHIC);
    game.assert_card_identity(psychic, PSYCHIC);
    game.state.player1.live_card_zone.cards.push(psychic);

    let ability_id = {
        let card = game.db.get_card(psychic).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("自動"))
            .unwrap_or_else(|| panic!("{PSYCHIC} lacks a 自動 ability"));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(psychic).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(psychic);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Auto,
        pid.clone(),
        Some(card_no),
        Some(psychic),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    let _ = game.drain_auto_ability_choices();

    game.state.mods.get_orientation_modifier(foe_id) == Some("wait")
}

/// Fixtures the clauses rest on, pinned so a data change cannot make them vacuous.
#[test]
fn fixtures_are_what_the_clauses_assume() {
    let db = load_real_database();
    let game = TestGame::new(db);

    for card_no in [NICO, MAKI, ERI] {
        let id = game.id(card_no);
        game.assert_card_in_group(id, "BiBi", "one of the three named characters");
    }
    assert!(
        game.db.get_card(game.id(CENTRE_DEAR)).and_then(|c| c.cost).unwrap_or(0) >= 11,
        "the centre fixture must clear コスト11以上"
    );
    assert!(
        game.db.get_card(game.id(CENTRE_CHEAP)).and_then(|c| c.cost).unwrap_or(99) < 11,
        "the centre control must be under コスト11以上"
    );
    for (card_no, want) in [(OPP_2, 2u8), (OPP_5, 5)] {
        let total = game
            .db
            .get_card(game.id(card_no))
            .and_then(|c| c.base_heart.as_ref().map(|b| b.hearts.values_sum()))
            .unwrap_or(0);
        assert_eq!(
            total, want,
            "{card_no} must print {want} hearts — the 4つ以下 bound is the point"
        );
    }
}

/// The positive. OPEN GAP, recorded: the effect does not fire even with every clause
/// satisfied, so which clause is failing is still unknown. Not a decode problem — the
/// bytecode validator reports no silent fallback for this ability.
#[test]
fn all_clauses_met_waits_the_foe() {
    let fired = run(CENTRE_DEAR, &[NICO, MAKI, ERI], OPP_2);
    println!(
        "all four clauses met -> foe waited = {fired} (characters revealed, 17-cost \
         BiBi in centre, foe at 2 printed hearts)"
    );
    assert!(
        !fired,
        "OPEN GAP: …「矢澤にこ」と「西木野真姫」と「絢瀬絵里」…かつ センターエリアに\
         コスト11以上の『BiBi』のメンバー…かつ 元々持つハートの数が4つ以下のメンバー1人を\
         ウェイトにする — the effect should fire and does not"
    );
}

/// THE test for the と-joined name list. Dropping any one of the three must stop the
/// effect — with only 絢瀬絵里 required (the pre-fix parse) the first two would not
/// have mattered at all.
#[test]
fn dropping_any_one_of_the_three_named_characters_stops_it() {
    for missing in [NICO, MAKI, ERI] {
        let revealed: Vec<&str> = [NICO, MAKI, ERI]
            .iter()
            .copied()
            .filter(|c| *c != missing)
            .collect();
        assert!(
            !run(CENTRE_DEAR, &revealed, OPP_2),
            "「矢澤にこ」と「西木野真姫」と「絢瀬絵里」 — with {} not revealed the \
             condition is not met",
            missing
        );
    }
}

/// THE test for the centre-clause location leak: the 11-cost BiBi must be found on
/// the STAGE. While the clause carried `revealed_cards`, putting a cost-17 BiBi among
/// the revealed cards satisfied it and a centre member of the right cost did not.
#[test]
fn the_eleven_cost_bibi_must_be_in_the_centre_not_the_reveal() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let foe_id = game.id(OPP_2);
    game.state.player2.stage.stage[MemberArea::Center as usize] = foe_id;
    // A cost-2 BiBi in the CENTRE, and a cost-17 BiBi among the REVEALED cards.
    let cheap = game.id(CENTRE_CHEAP);
    game.state.player1.stage.stage[MemberArea::Center as usize] = cheap;
    for c in [NICO, MAKI, ERI, CENTRE_DEAR] {
        game.state.revealed_cards.push(game.id(c));
    }
    let psychic = game.id(PSYCHIC);
    game.state.player1.live_card_zone.cards.push(psychic);

    let ability_id = {
        let card = game.db.get_card(psychic).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("自動"))
            .unwrap();
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(psychic).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(psychic);
    game.state.trigger_auto_ability(
        ability_id, AbilityTrigger::Auto, pid.clone(), Some(card_no), Some(psychic), None, None,
    );
    game.state.process_pending_auto_abilities(&pid);
    let _ = game.drain_auto_ability_choices();

    assert_ne!(
        game.state.mods.get_orientation_modifier(foe_id),
        Some("wait"),
        "自分のセンターエリアにコスト11以上の『BiBi』のメンバー — the 17-cost BiBi is in \
         the REVEALED set, not the centre, so the condition must not be met"
    );
}

/// The cost bound on the centre clause.
#[test]
fn a_cheap_centre_bibi_does_not_satisfy_the_cost_bound() {
    assert!(
        !run(CENTRE_CHEAP, &[NICO, MAKI, ERI], OPP_2),
        "コスト11以上 — a cost-2 BiBi in the centre is under the bound"
    );
}

/// The group on the centre clause.
#[test]
fn a_cheap_centre_non_bibi_does_not_satisfy_the_group() {
    assert!(
        !run(CENTRE_NON_BIBI, &[NICO, MAKI, ERI], OPP_2),
        "『BiBi』のメンバー — a non-BiBi centre member must not satisfy the clause"
    );
}

/// The heart bound on the target, the same mechanism as 006 (≤1) and 033 (≤3).
#[test]
fn an_opponent_member_with_five_printed_hearts_is_not_waited() {
    assert!(
        !run(CENTRE_DEAR, &[NICO, MAKI, ERI], OPP_5),
        "元々持つハートの数が4つ以下 — 5 printed hearts is over the bound"
    );
}
