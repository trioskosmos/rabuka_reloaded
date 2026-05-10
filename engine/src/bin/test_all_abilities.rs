use std::path::Path;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::types::{Phase, TurnPhase};
use rabuka_engine::ability_resolver::AbilityResolver;

const FILLER: &str = "PL!-sd1-010-SD";
const ENERGY_CARD: &str = "LL-E-001-SD";
const LIVE_CARD: &str = "PL!-sd1-019-SD";

#[derive(Deserialize)]
#[allow(dead_code)]
struct Scenario {
    index: usize,
    card_no: String,
    triggers: String,
    action: String,
    text: String,
    setup: Setup,
    expected: Expected,
    checks: Checks,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Setup {
    stage: bool,
    live_phase: bool,
    hand_cards: usize,
    energy: usize,
    discard_cards: usize,
    deck_cards: usize,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Expected {
    action: String,
    source: Option<String>,
    destination: Option<String>,
    card_type: Option<String>,
    count: Option<u32>,
    target: String,
    has_selection: bool,
    optional: bool,
    conditional_on_optional: bool,
    cost_type: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Checks {
    has_energy_in_text: bool,
    has_member_in_text: bool,
    has_cost_limit: bool,
    has_color_filter: bool,
}

#[derive(Serialize)]
struct TestResult {
    index: usize,
    status: String,
    error: Option<String>,
    actual_action: Option<String>,
    actual_source: Option<String>,
    actual_dest: Option<String>,
    actual_card_type: Option<String>,
    actual_count: Option<u32>,
    pending_choice: bool,
    hand_size_before: usize,
    hand_size_after: usize,
    discard_size_before: usize,
    discard_size_after: usize,
    energy_count_before: usize,
    energy_count_after: usize,
}

fn load_db() -> Arc<CardDatabase> {
    let cards_path = Path::new("../cards/cards.json");
    let cards = CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    Arc::new(CardDatabase::load_or_create(cards))
}

fn run_scenario(db: &Arc<CardDatabase>, scenario: &Scenario) -> TestResult {
    let filler = db.get_card_id(FILLER).unwrap_or(-1);
    let energy_card = db.get_card_id(ENERGY_CARD).unwrap_or(filler);
    let live_card = db.get_card_id(LIVE_CARD).unwrap_or(filler);
    let card_id = db.get_card_id(&scenario.card_no).unwrap_or(-1);

    let mut p1 = Player::new("p1".into(), "P1".into(), true);
    let p2 = Player::new("p2".into(), "P2".into(), false);
    p1.is_first_attacker = true;
    let mut state = GameState::new(p1, p2, db.clone());

    // Populate energy deck with real energy cards
    for _ in 0..30 {
        state.player1.energy_deck.cards.push(energy_card);
    }
    // Populate main deck
    for _ in 0..scenario.setup.deck_cards.max(20) {
        state.player1.main_deck.cards.push(filler);
    }
    state.player1.hand.add_card(card_id);
    for _ in 0..scenario.setup.hand_cards.max(10) {
        state.player1.hand.add_card(filler);
    }
    // Add live cards to hand (for reveal costs requiring live_card type)
    for _ in 0..5 {
        state.player1.hand.add_card(live_card);
    }
    // Add card_id copies to hand (for character-name filter matching)
    for _ in 0..3 {
        state.player1.hand.add_card(card_id);
    }
    for _ in 0..scenario.setup.discard_cards.max(10) {
        state.player1.waitroom.cards.push(filler);
    }
    // Add card_id copies to discard (for character-name filter costs)
    for _ in 0..6 {
        state.player1.waitroom.cards.push(card_id);
    }
    // Energy: add active energy cards
    for _ in 0..10 {
        if let Some(id) = state.player1.energy_deck.draw() {
            state.player1.energy_zone.cards.push(id);
            state.player1.energy_zone.active_energy_count += 1;
        }
    }
    // Add wait-state energy (for deactivation abilities)
    for _ in 0..10 {
        if let Some(id) = state.player1.energy_deck.draw() {
            state.player1.energy_zone.cards.push(id);
            // Don't increment active count — these are wait cards
        }
    }
    // Find a second member card of the same series as card_id for stage
    // (needed when exclude_self=true removes card_id and filler doesn't match group)
    let same_series_member = if card_id >= 0 {
        db.get_card(card_id).and_then(|c| {
            let _series = &c.series;
            db.cards.iter().find(|(id, other)| {
                **id != card_id && other.is_member() && &other.series == _series
            }).map(|(id, _)| *id)
        }).unwrap_or(filler)
    } else { filler };
    // Find a multi-purpose member card for stage[0]:
    //   - cost <= 2 for cost_limit filters
    //   - unit=Printemps for group-based filters like index 493
    //   - series containing common group keywords
    let stage0_card = if card_id >= 0 {
        db.get_card(card_id).and_then(|c| {
            let _series = &c.series;
            db.cards.iter().find(|(id, other)| {
                **id != card_id && other.is_member() && other.cost.map_or(false, |c| c <= 2)
                    && other.unit.as_deref() == Some("Printemps")
            }).map(|(id, _)| *id)
        }).or_else(|| {
            db.cards.iter().find(|(_, c)| {
                c.is_member() && c.cost.map_or(false, |cost| cost <= 2)
            }).map(|(id, _)| *id)
        }).unwrap_or(filler)
    } else { filler };

    // Find a same-series member for exclude_self scenarios (indices 61, 62)
    let _same_series_member = if card_id >= 0 {
        db.get_card(card_id).and_then(|c| {
            let _series = &c.series;
            db.cards.iter().find(|(id, other)| {
                **id != card_id && other.is_member() && &other.series == _series
            }).map(|(id, _)| *id)
        }).unwrap_or(filler)
    } else { filler };

    // Populate both players' stages for change_state/targeting abilities
    state.player1.stage.stage[0] = stage0_card;
    state.player1.stage.stage[1] = card_id;
    state.player1.stage.stage[2] = same_series_member;
    // Same for opponent (for opponent-targeted change_state effects)
    state.player2.stage.stage[0] = stage0_card;
    state.player2.stage.stage[1] = card_id;
    state.player2.stage.stage[2] = same_series_member;
    // Remove card_id copies from hand (they were placed on stage)
    if card_id >= 0 {
        state.player1.hand.cards.retain(|c| *c != card_id);
        // Re-add one copy to hand
        state.player1.hand.add_card(card_id);
    }
    // Cards in success live zone
    for _ in 0..scenario.setup.discard_cards.max(5) {
        state.player1.success_live_card_zone.cards.push(filler);
    }
    // Revealed cards (for yell/reveal-based abilities)
    for _ in 0..8 {
        state.revealed_cards.push(filler);
    }
    // Additional hand cards for reveal/discard costs
    for _ in 0..10 {
        state.player1.hand.add_card(filler);
    }
    // Additional discard pile
    for _ in 0..10 {
        state.player1.waitroom.cards.push(filler);
    }

    // Set phase
    if scenario.setup.live_phase {
        state.current_phase = Phase::LiveCardSetP1Turn;
        state.current_turn_phase = TurnPhase::Live;
    } else {
        state.current_phase = Phase::Main;
        state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    }
    state.turn_number = 1;

    let hand_before = state.player1.hand.cards.len();
    let discard_before = state.player1.waitroom.cards.len();
    let energy_before = state.player1.energy_zone.cards.len();

    let mut result = TestResult {
        index: scenario.index,
        status: "ok".into(),
        error: None,
        actual_action: None,
        actual_source: None,
        actual_dest: None,
        actual_card_type: None,
        actual_count: None,
        pending_choice: false,
        hand_size_before: hand_before,
        hand_size_after: hand_before,
        discard_size_before: discard_before,
        discard_size_after: discard_before,
        energy_count_before: energy_before,
        energy_count_after: energy_before,
    };

    if card_id < 0 {
        result.status = "skip".into();
        result.error = Some("Card not found".into());
        return result;
    }

    let card = db.get_card(card_id);
    if card.is_none() {
        result.status = "skip".into();
        result.error = Some("Card not in DB".into());
        return result;
    }
    let card = card.unwrap();
    if card.abilities.is_empty() {
        result.status = "skip".into();
        result.error = Some("No abilities".into());
        return result;
    }
    let ability = card.abilities[0].clone();

    // Record expected fields
    if let Some(ref effect) = ability.effect {
        result.actual_action = Some(effect.action.clone());
        result.actual_source = effect.source.clone();
        result.actual_dest = effect.destination.clone();
        result.actual_card_type = effect.card_type.clone();
        result.actual_count = effect.count;
    }

    // Resolve the ability
    let mut resolver = AbilityResolver::new(&mut state);
    match resolver.resolve_ability(&ability, Some(card_id), 0) {
        Ok(()) => {
            result.status = "ok".into();
            result.pending_choice = state.pending_choice.is_some();
            result.hand_size_after = state.player1.hand.cards.len();
            result.discard_size_after = state.player1.waitroom.cards.len();
            result.energy_count_after = state.player1.energy_zone.cards.len();
        }
        Err(e) => {
            result.status = "error".into();
            result.error = Some(e);
        }
    }

    result
}

fn main() {
    let scenarios_path = Path::new("../cards/scenarios.json");
    let data = std::fs::read_to_string(scenarios_path)
        .expect("Failed to read scenarios.json");
    let scenarios: Vec<Scenario> = serde_json::from_str(&data)
        .expect("Failed to parse scenarios.json");

    let db = load_db();
    eprintln!("Loaded {} cards, testing {} scenarios", db.cards.len(), scenarios.len());

    let mut results = Vec::new();
    for s in &scenarios {
        results.push(run_scenario(&db, s));
    }

    let output = serde_json::json!({
        "total": results.len(),
        "ok": results.iter().filter(|r| r.status == "ok").count(),
        "errors": results.iter().filter(|r| r.status == "error").count(),
        "skipped": results.iter().filter(|r| r.status == "skip").count(),
        "results": results,
    });
    let output_str = serde_json::to_string_pretty(&output).unwrap();
    std::fs::write("test_results.json", &output_str).expect("Failed to write results");
    eprintln!("Results written to test_results.json");
    println!("{}", output_str);
}
