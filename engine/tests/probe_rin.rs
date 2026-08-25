#[test]
fn probe_sd2005_choice() {
    let db = crate::helpers::load_real_database();
    let cid = crate::helpers::card_id(&db, "PL!N-sd2-005-SD2");
    let card = db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("no LS ability");
    let eff = ab.effect.as_ref().unwrap();
    if let Some(actions) = eff.compound.actions.as_ref() {
        for (i, a) in actions.iter().enumerate() {
            eprintln!(
                "PROBE sd2005 step{} action={:?} choice_any={:?} heart_colors={:?}",
                i,
                a.action,
                a.choice_any(),
                a.heart_colors_any()
            );
        }
    } else {
        eprintln!("PROBE sd2005 NO compound actions — debug:\n{:#?}", eff);
    }
}
