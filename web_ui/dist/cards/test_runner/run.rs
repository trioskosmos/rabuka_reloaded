use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::types::{Phase, TurnPhase};
use rabuka_engine::zones::MemberArea;
use rabuka_engine::ability::AbilityResolver;

#[derive(Deserialize)]
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
struct Setup {
    stage: bool,
    live_phase: bool,
    hand_cards: usize,
    energy: usize,
    discard_cards: usize,
    deck_cards: usize,
}

#[derive(Deserialize)]
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
    stage_cards_after: Vec<i16>,
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
    let card_id = db.get_card_id(&scenario.card_no).unwrap_or(-1);

    // Setup game state
    let mut p1 = Player::new("p1".into(), "P1".into(), true);
    let p2 = Player::new("p2".into(), "P2".into(), false);
    p1.is_first_attacker = true;
    let mut state = GameState::new(p1, p2, db.clone());

    let hand_before;
    let discard_before;
    let energy_before;

    // Setup zones
    {
        let p = &mut state.player1;

        // Add filler cards to deck
        for _ in 0..scenario.setup.deck_cards {
            p.main_deck.cards.push(filler);
        }

        // Add hand cards
        p.hand.add_card(card_id);
        for _ in 0..scenario.setup.hand_cards.saturating_sub(1) {
            p.hand.add_card(filler);
        }

        // Add discard cards
        for _ in 0..scenario.setup.discard_cards {
            p.waitroom.cards.push(filler);
        }

        // Add energy
        for _ in 0..scenario.setup.energy {
            if let Some(id) = p.energy_deck.draw() {
                p.energy_zone.cards.push(id);
                p.energy_zone.active_energy_count += 1;
            }
        }

        hand_before = p.hand.cards.len();
        discard_before = p.waitroom.cards.len();
        energy_before = p.energy_zone.cards.len();
    }

    // Place on stage if needed
    if scenario.setup.stage {
        let p = &mut state.player1;
        if let Some(pos) = p.hand.cards.iter().position(|&c| c == card_id) {
            p.hand.cards.remove(pos);
        }
        state.player1.stage.stage[1] = card_id; // Center
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

    // Activate ability
    state.activating_card = Some(card_id);
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
        stage_cards_after: state.player1.stage.stage.to_vec(),
        hand_size_before,
        hand_size_after: state.player1.hand.cards.len(),
        discard_size_before,
        discard_size_after: state.player1.waitroom.cards.len(),
        energy_count_before: energy_before,
        energy_count_after: state.player1.energy_zone.cards.len(),
    };

    let card = db.get_card(card_id);
    if card.is_none() {
        result.status = "skip".into();
        result.error = Some("Card not found".into());
        return result;
    }

    let card = card.unwrap();
    let ability = card.abilities.first().cloned();
    if ability.is_none() {
        result.status = "skip".into();
        result.error = Some("No abilities".into());
        return result;
    }

    let ability = ability.unwrap();

    // Store expected fields from the ability's effect
    if let Some(ref effect) = ability.effect {
        result.actual_action = Some(effect.action.clone());
        result.actual_source = effect.source.clone();
        result.actual_dest = effect.destination.clone();
        result.actual_card_type = effect.card_type.clone();
        result.actual_count = effect.count;
    }

    // Try to resolve the ability
    let mut resolver = AbilityResolver::new(&mut state);
    match resolver.resolve_ability(&ability, Some(card_id), 0) {
        Ok(()) => {
            result.status = "ok".into();
            result.pending_choice = state.pending_choice.is_some();
            result.hand_size_after = state.player1.hand.cards.len();
            result.discard_size_after = state.player1.waitroom.cards.len();
            result.energy_count_after = state.player1.energy_zone.cards.len();
            result.stage_cards_after = state.player1.stage.stage.to_vec();
        }
        Err(e) => {
            result.status = "error".into();
            result.error = Some(e);
        }
    }

    result
}

const FILLER: &str = "PL!-sd1-010-SD";

fn main() {
    let scenarios_path = Path::new("../test_runner/scenarios.json");
    let scenarios_data = std::fs::read_to_string(scenarios_path)
        .expect("Failed to read scenarios.json");
    let scenarios: Vec<Scenario> = serde_json::from_str(&scenarios_data)
        .expect("Failed to parse scenarios.json");

    let db = load_db();
    println!("Loaded {} cards", db.cards.len());
    println!("Testing {} scenarios", scenarios.len());

    let mut results = Vec::new();
    for scenario in &scenarios {
        let result = run_scenario(&db, scenario);
        results.push(result);
    }

    // Output results as JSON
    let output = serde_json::to_string_pretty(&serde_json::json!({
        "total": results.len(),
        "ok": results.iter().filter(|r| r.status == "ok").count(),
        "errors": results.iter().filter(|r| r.status == "error").count(),
        "skipped": results.iter().filter(|r| r.status == "skip").count(),
        "results": results,
    })).unwrap();
    println!("{}", output);
}
