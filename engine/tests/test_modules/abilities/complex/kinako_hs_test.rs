/// Tests for 桜小路きな子 (PL!HS-bp1-003-R＋) — Constant: stage has 桜坂 members on all areas.
///
/// Q81: Multi-name cards like "LL-bp1-001 園田海未&東條希&東雲好花" reference
/// individual character names, not the combined name.
use crate::helpers::*;

#[test]
fn kinako_hs_q81_multiname_has_individual_names_in_card_db() {
    let db = load_real_database();
    let game = TestGame::new(db.clone());

    // Verify the multi-name card exists in the database
    let multi = game.id("LL-bp1-001-R\u{ff0b}");
    let card = game.db.get_card(multi).expect("Multi-name card in DB");
    let name = &card.name;
    eprintln!("[KINAKO] multi card name: {}", name);
    // The name should contain '&' separating individual names
    assert!(
        name.contains('&'),
        "Multi-name card should have '&' in name"
    );
    let parts: Vec<&str> = name.split('&').collect();
    eprintln!("[KINAKO] individual names: {:?}", parts);
    assert!(parts.len() >= 3, "Should have 3+ individual names");
}
