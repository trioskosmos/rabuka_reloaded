use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Test to verify group matching is working
#[test]
fn test_yoshiko_group_matching() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R");
    let riko = game.id("PL!S-bp2-002-R");

    // Test group matching directly
    println!("Testing group matching for Aqours:");

    for (name, card_id) in [("Yoshiko", yoshiko), ("Chika", chika), ("Riko", riko)] {
        if let Some(card) = game.state.card_database.get_card(card_id) {
            let matches_unit = card.unit.as_deref() == Some("Aqours");
            let matches_group = card.group == "Aqours";
            let matches_series = card.series.contains("サンシャイン");

            println!(
                "{}: unit={:?}, group={}, series={}",
                name, card.unit, matches_group, matches_series
            );

            // Test the actual function used by abilities
            let matches = rabuka_engine::ability::util::card_matches_group_str(
                &game.state.card_database,
                card_id,
                Some("Aqours"),
            );
            println!("{} matches 'Aqours': {}", name, matches);
            assert!(matches, "{} should match group 'Aqours'", name);
        } else {
            panic!("Card not found in database for {}", name);
        }
    }

    // Verify all three cards pass the full filter_from_parts too
    for &card_id in &[yoshiko, chika, riko] {
        let filter = rabuka_engine::ability::util::filter_from_parts_full(
            Some("member_card"),
            Some("Aqours"),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(
            filter.matches(&game.state.card_database, card_id, true),
            "all Aqours cards should match member_card+Aqours filter"
        );
    }
}
