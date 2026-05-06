/// Tests verifying correct parser output for batch pb1 cards.
/// Full gameplay tests require complex condition evaluation.
mod helpers;
use helpers::*;

/// Q205: Setsuna ab#0 — temporal_condition parsed correctly.
#[test]
fn setsuna_q205_parser_output_correct() {
    let db = load_real_database();
    let card = db.get_card_id("PL!N-pb1-007-R").expect("Card exists");
    let card_data = db.get_card(card).unwrap();
    let ability = card_data.abilities.iter().find(|a| {
        a.triggers.as_deref() == Some("常時")
    }).expect("Has 常時 ability");
    let cond = ability.effect.as_ref().and_then(|e| e.condition.as_ref()).expect("Has condition");
    assert_eq!(cond.condition_type.as_deref(), Some("temporal_condition"),
        "Temporal condition type");
}

/// Q206: Emma ab#0 — modify_cost parsed with type, group, value.
#[test]
fn emma_q206_parser_output_correct() {
    let db = load_real_database();
    let card = db.get_card_id("PL!N-pb1-008-R").expect("Card exists");
    let card_data = db.get_card(card).unwrap();
    let ability = card_data.abilities.iter().find(|a| {
        a.effect.as_ref().map_or(false, |e| e.action == "modify_cost")
    }).expect("Has modify_cost ability");
    let eff = ability.effect.as_ref().unwrap();
    assert_eq!(eff.operation.as_deref(), Some("subtract"),
        "Cost reduction");
    assert_eq!(eff.value, Some(2), "Reduce by 2");
}

/// Q198: Shion ab#0 — appearance_condition parsed with exclude_self and cost_limit.
#[test]
fn shion_q198_parser_output_correct() {
    let db = load_real_database();
    let card = db.get_card_id("PL!N-pb1-012-R").expect("Card exists");
    let card_data = db.get_card(card).unwrap();
    let ability = card_data.abilities.iter().find(|a| {
        a.triggers.as_ref().map_or(false, |t| t == "自動")
    }).expect("Has 自動 ability");
    assert_eq!(ability.use_limit, Some(1), "Turn 1 limit");
    let eff = ability.effect.as_ref().unwrap();
    let cond = eff.condition.as_ref().expect("Has condition");
    assert_eq!(cond.condition_type.as_deref(), Some("appearance_condition"),
        "Appearance condition");
}
