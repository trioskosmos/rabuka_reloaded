/// Fast coverage: remaining parser/engine QAs.
mod helpers;
use helpers::*;

/// Solitude Rain (PL!N-bp1-027-L) — Q67: LiveStart condition about having another
/// live card of the same series. Parser check only.
#[test]
fn solitude_rain_q67_live_start_condition() {
    let db = load_real_database();
    let card = db.get_card_by_no("PL!N-bp1-027-L").expect("card exists");
    let has_live_start = card.abilities.iter().any(|a| {
        a.triggers.as_deref() == Some("ライブ開始時")
    });
    assert!(has_live_start, "Parsed LiveStart trigger");
}

/// Daydream Mermaid (PL!N-bp4-030-L) — Q191: data-level parser check.
#[test]
fn daydream_mermaid_q191_parsed() {
    let db = load_real_database();
    let card = db.get_card_by_no("PL!N-bp4-030-L").expect("card exists");
    let has_live_success = card.abilities.iter().any(|a| {
        a.triggers.as_deref() == Some("ライブ成功時")
    });
    assert!(has_live_success, "Parsed LiveSuccess trigger");
}

/// Dream with You (PL!N-sd1-028-SD) — Q116: basic live card existence.
#[test]
fn dream_with_you_q116_parsed() {
    let db = load_real_database();
    let card = db.get_card_by_no("PL!N-sd1-028-SD").expect("card exists");
    assert!(card.is_live(), "Is a live card");
}
