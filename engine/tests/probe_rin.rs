#[test]
fn probe_sd2005_choice() {
    let db = crate::helpers::load_real_database();
    let cid = crate::helpers::card_id(&db, "PL!HS-cl1-012-CL");
    let card = db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .expect("no ライブ成功時 ability");
    let eff = ab.effect.as_ref().expect("no effect");
    eprintln!(
        "PROBE cl1012 action={:?} has_condition={:?} cond_debug={:#?}",
        eff.action,
        eff.condition.is_some(),
        eff.condition
    );
}
