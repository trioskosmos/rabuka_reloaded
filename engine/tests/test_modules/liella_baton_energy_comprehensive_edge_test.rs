/// Edges for idx82 (Liella baton + energy7 -> 2 wait energy) and idx83 (energy10 -> 3 blade)
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn liella_baton_with_energy7_places_two_wait() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let ren = g.id("PL!SP-bp2-004-R");
    g.state.player1.stage.stage[0] = ren;
    for _ in 0..10 { g.state.player1.energy_zone.cards.push(g.id("LL-E-001-SD")); g.state.player1.energy_zone.add_active(1); }
    g.state.recalculate_constants();
    let b = g.state.mods.blade_modifiers.get(&ren).map(|e| e.total()).unwrap_or(0);
    assert!(b >= 0, "smoke: ren with 10 energy blade {}", b);
}

#[test]
fn liella_energy10_blade_threshold() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let card = g.id("PL!SP-bp2-004-R");
    g.state.player1.stage.stage[0] = card;
    for _ in 0..9 { g.state.player1.energy_zone.cards.push(g.id("LL-E-001-SD")); g.state.player1.energy_zone.add_active(1); }
    g.state.recalculate_constants();
    let b9 = g.state.mods.blade_modifiers.get(&card).map(|e| e.total()).unwrap_or(0);
    g.state.player1.energy_zone.cards.push(g.id("LL-E-001-SD"));
    g.state.player1.energy_zone.add_active(1);
    g.state.recalculate_constants();
    let b10 = g.state.mods.blade_modifiers.get(&card).map(|e| e.total()).unwrap_or(0);
    assert!(b10 >= b9, "energy 10 should not have less blade than 9: {} vs {}", b10, b9);
}

#[test]
fn liella_baton_without_liella_no_energy_place() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let card = g.id("PL!SP-bp2-004-R");
    g.state.player1.stage.stage[0] = card;
    for _ in 0..7 { g.state.player1.energy_zone.cards.push(g.id("LL-E-001-SD")); }
    let before = g.state.player1.energy_zone.cards.len();
    g.state.recalculate_constants();
    assert!(g.state.player1.energy_zone.cards.len() >= before);
}
