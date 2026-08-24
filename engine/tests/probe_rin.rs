#[test]
fn probe_rin_condition() {
    let db = crate::helpers::load_real_database();
    let cid = crate::helpers::card_id(&db, "PL!-bp4-014-N");
    let card = db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("no LS ability");
    let cond = ab
        .effect
        .as_ref()
        .and_then(|e| e.condition.as_ref())
        .expect("no condition");
    eprintln!("PROBE debug:\n{:#?}", cond);
    eprintln!(
        "PROBE ability_filter={:?} triggers={:?} location={:?}",
        cond.get_ability_filter(),
        cond.get_ability_filter_triggers(),
        cond.get_location()
    );
}
