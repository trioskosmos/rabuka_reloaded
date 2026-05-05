/// Batch QA coverage test — verifies all cards with QA entries parse correctly
/// and have valid abilities. Catches parser regressions and missing abilities.

mod helpers;
use helpers::*;

const QA_CARD_NOS: &[&str] = &[
    // #5 — 中須かすみ ab#1
    "PL!N-bp1-002-R\u{ff0b}",
    // #19 — 南ことり ab#1 (joint card name rules)
    "PL!-bp5-003-R\u{ff0b}",
    // #21 — 高坂穂乃果 ab#0
    "PL!-pb1-001-R",
    // #26 — AWOKE ab#0
    "PL!HS-bp1-022-L",
    // #31 — TOKIMEKI Runners ab#1
    "PL!N-bp5-026-L",
    // #32 — ミラクル STAY TUNE! ab#0
    "PL!N-bp5-027-L",
    // #33 — 繚乱！ビクトリーロード ab#1
    "PL!N-bp5-030-L",
    // #38 — MIRACLE WAVE ab#0
    "PL!S-bp3-019-L",
    // #39 — 桜内梨子 ab#0
    "PL!S-pb1-002-R",
    // #41 — 澁谷かのん ab#0
    "PL!SP-bp2-001-R\u{ff0b}",
    // #47 — 平安名すみれ ab#1
    "PL!SP-bp4-004-R\u{ff0b}",
    // #48 — Dazzling Game ab#1
    "PL!SP-bp4-023-L",
    // #49 — 葉月恋 ab#1
    "PL!SP-bp5-005-R\u{ff0b}",
    // 1-QA cards with interesting mechanics
    "PL!-bp3-002-R", "PL!-bp3-003-R", "PL!-bp3-004-R\u{ff0b}",
    "PL!-bp3-008-R\u{ff0b}", "PL!-bp3-019-L", "PL!-bp3-023-L",
    "PL!-bp4-009-R", "PL!-bp5-004-R\u{ff0b}", "PL!-bp5-007-R",
    "PL!-bp5-009-R", "PL!-pb1-008-R", "PL!-pb1-009-R",
    "PL!-pb1-013-R", "PL!-pb1-015-R", "PL!-pb1-030-L",
    "PL!-pb1-031-L", "PL!-pb1-032-L",
    "PL!N-bp5-003-R", "PL!N-bp5-007-R\u{ff0b}", "PL!N-bp5-008-R",
    "PL!N-bp5-010-R",
    // SP-bp2 nonsense cards
    "PL!SP-bp2-011-R", "PL!SP-bp2-025-L",
];

#[test]
fn qa_cards_parse_with_valid_abilities() {
    let db = load_real_database();
    let mut errors = Vec::new();

    for &card_no in QA_CARD_NOS {
        match db.get_card_id(card_no) {
            Some(id) => {
                let card = db.get_card(id).unwrap();
                if card.abilities.is_empty() {
                    errors.push(format!("{}: no abilities", card_no));
                }
            }
            None => {
                errors.push(format!("{}: not found in database", card_no));
            }
        }
    }

    if !errors.is_empty() {
        panic!("QA card validation errors:\n{}", errors.join("\n"));
    }
}
