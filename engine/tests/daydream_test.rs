/// Daydream Mermaid (PL!N-bp4-030-L) — Q191: LiveSuccess effect with multiple options
/// can only select 1, not 2. Verify the parser output enforces single choice.

mod helpers;
use helpers::*;

#[test]
fn daydream_mermaid_q191_single_choice_only() {
    let db = load_real_database();
    let card = db.get_card_by_no("PL!N-bp4-030-L").expect("card exists");
    let ab = card.abilities.iter()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .expect("LiveSuccess ability");
    let effect = ab.effect.as_ref().expect("effect exists");
    // The effect is sequential. First action is select with count=1 (single choice)
    if let Some(ref actions) = effect.actions {
        if let Some(first) = actions.first() {
            assert_eq!(first.action, "select");
            assert_eq!(first.count, Some(1));
            return;
        }
    }
    panic!("Expected sequential with select count=1");
}
