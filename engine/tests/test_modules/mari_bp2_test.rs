use crate::helpers::load_real_database;
use rabuka_engine::card::AbilityEffect;

fn card_id(db: &rabuka_engine::card::CardDatabase, card_no: &str) -> i16 {
    crate::helpers::card_id(db, card_no)
}

/// Verify Mari card is parsed as gain_ability with proper structure.
#[test]
fn mari_ability_parsed_as_gain_ability() {
    let db = load_real_database();
    let mari_id = card_id(&db, "PL!S-bp2-008-R\u{ff0b}");
    let card = db.get_card(mari_id).expect("Mari card should exist");

    // Find the constant ability (ab#1)
    let gain_ab = card.abilities.iter().find(|a| {
        a.triggers.as_ref().is_some_and(|t| t == "常時")
            && a.effect
                .as_ref()
                .is_some_and(|e| e.action == "gain_ability")
    });
    assert!(
        gain_ab.is_some(),
        "Mari should have a constant gain_ability ability"
    );
    let effect = gain_ab.unwrap().effect.as_ref().unwrap();
    assert_eq!(
        effect.action, "gain_ability",
        "Action should be gain_ability"
    );
    assert!(
        effect.ability_gain.is_some(),
        "ability_gain should be present"
    );
    assert!(
        effect.gained_effect.is_some(),
        "gained_effect should be present"
    );
    let gained = effect.gained_effect.as_ref().unwrap();
    assert_eq!(
        gained.action, "conditional_alternative",
        "Gained effect should be conditional_alternative"
    );
}
