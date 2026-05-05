/// Batch 10 — Q36 timing definition cards (LiveSuccess timing)
/// All share the same QA: "when does LiveSuccess fire?"
/// Already covered by live timing tests. These just verify card existence.

mod helpers;
use helpers::*;

fn check_card_has_ability(card_no: &str) {
    let db = load_real_database();
    let id = db.get_card_id(card_no)
        .unwrap_or_else(|| panic!("Card {card_no} not found"));
    let card = db.get_card(id).unwrap();
    assert!(!card.abilities.is_empty(),
        "Card {} should have abilities", card_no);
}

#[test] fn pl_pb1_030_L() { check_card_has_ability("PL!-pb1-030-L"); }
#[test] fn pl_pb1_031_L() { check_card_has_ability("PL!-pb1-031-L"); }
#[test] fn pl_pb1_032_L() { check_card_has_ability("PL!-pb1-032-L"); }
#[test] fn plS_bp2_021_L() { check_card_has_ability("PL!S-bp2-021-L"); }
#[test] fn plS_bp2_022_L() { check_card_has_ability("PL!S-bp2-022-L"); }
#[test] fn plS_pb1_003_R() { check_card_has_ability("PL!S-pb1-003-R"); }
#[test] fn plS_pb1_007_R() { check_card_has_ability("PL!S-pb1-007-R"); }
#[test] fn plSP_bp1_024_L() { check_card_has_ability("PL!SP-bp1-024-L"); }
#[test] fn plSP_bp2_025_L() { check_card_has_ability("PL!SP-bp2-025-L"); }
#[test] fn plHS_bp1_021_L() { check_card_has_ability("PL!HS-bp1-021-L"); }
#[test] fn plHS_bp1_023_L() { check_card_has_ability("PL!HS-bp1-023-L"); }
#[test] fn plS_pb1_022_L() { check_card_has_ability("PL!S-pb1-022-L"); }
#[test] fn plS_pb1_024_L() { check_card_has_ability("PL!S-pb1-024-L"); }
#[test] fn plN_bp1_027_L() { check_card_has_ability("PL!N-bp1-027-L"); }
#[test] fn plN_bp3_031_L() { check_card_has_ability("PL!N-bp3-031-L"); }
