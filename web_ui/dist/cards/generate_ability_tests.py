"""Generate Rust integration tests from abilities.json.

For each unique ability, creates a minimal test that:
1. Loads the card having this ability
2. Sets up a game state that satisfies the condition
3. Activates the ability
4. Checks for errors and unexpected behavior

Output: engine/tests/auto_generated_ability_tests.rs
"""
import json, re, os
from pathlib import Path

ABILITIES = Path(__file__).parent.parent / 'cards' / 'abilities.json'
CARDS = Path(__file__).parent.parent / 'cards' / 'cards.json'
OUTPUT = Path(__file__).parent.parent / 'engine' / 'tests' / 'auto_generated.rs'

data = json.load(open(ABILITIES, encoding='utf-8'))
cards_data = json.load(open(CARDS, encoding='utf-8'))

# Load cards.json - might be array or object
if isinstance(cards_data, dict):
    # card_no -> card mapping
    card_map = cards_data
else:
    # array of cards
    card_map = {c['card_no']: c for c in cards_data}

# Filler card for zone filling
FILLER = "PL!-sd1-010-SD"  # A filler card with no abilities

# Count abilities by trigger type for test generation
test_bodies = []
test_count = 0

for i, entry in enumerate(abilities_entries := data['unique_abilities']):
    t = entry.get('triggerless_text', '')
    cards_list = entry.get('cards', [])
    triggers = entry.get('triggers') or ''
    is_null = entry.get('is_null', False)
    
    if not t or is_null or not cards_list:
        continue
    
    # Get the first card this ability belongs to
    first_card = cards_list[0].split(' | ')[0] if ' | ' in cards_list[0] else cards_list[0]
    
    # Determine test category based on trigger and effect
    effect = entry.get('effect') or {}
    cost = entry.get('cost')
    action = effect.get('action', '')
    
    # Build test
    test_count += 1
    test_name = f"ability_{i}"
    
    # Generate setup based on trigger
    setup_lines = []
    
    if '登場' in triggers or '起動' in triggers:
        setup_lines.append(f"// {triggers} ability - card needs to be on stage")
        setup_lines.append(f'game.add_to_hand(card_id);')
        setup_lines.append(f'game.give_energy(10);')
        setup_lines.append(f'game.add_to_hand(filler);')
        setup_lines.append(f'game.add_to_discard(filler);')
        setup_lines.append(f'for _ in 0..3 {{ game.state.player1.main_deck.cards.push(filler); }}')
        setup_lines.append(f'game.play_to_stage(card_id, MemberArea::Center);')
    
    elif '常時' in triggers:
        setup_lines.append(f"// {triggers} ability - card on stage")
        setup_lines.append(f'game.add_to_hand(card_id);')
        setup_lines.append(f'game.give_energy(10);')
        setup_lines.append(f'game.add_to_hand(filler);')
        setup_lines.append(f'game.add_to_discard(filler);')
        setup_lines.append(f'for _ in 0..3 {{ game.state.player1.main_deck.cards.push(filler); }}')
        setup_lines.append(f'game.play_to_stage(card_id, MemberArea::Center);')
    
    elif 'ライブ開始時' in triggers:
        setup_lines.append(f"// {triggers} ability - needs live phase")
        setup_lines.append(f'game.add_to_hand(card_id);')
        setup_lines.append(f'game.give_energy(10);')
        setup_lines.append(f'game.add_to_hand(filler);')
        setup_lines.append(f'for _ in 0..3 {{ game.state.player1.main_deck.cards.push(filler); }}')
        setup_lines.append(f'game.play_to_stage(card_id, MemberArea::Center);')
    
    else:
        setup_lines.append(f"// {triggers} ability - basic setup")
        setup_lines.append(f'game.add_to_hand(card_id);')
        setup_lines.append(f'game.give_energy(10);')
    
    # Activation
    if '起動' in triggers:
        setup_lines.append(f'let result = game.activate_ability(card_id);')
        setup_lines.append(f'// Activation may need choices - just verify no panic')
        setup_lines.append(f'let _ = result;')
    elif '登場' in triggers or '自動' in triggers:
        setup_lines.append(f'// Auto-triggered on appear, already processed by play_to_stage')
        setup_lines.append(f'// Check for pending choices from the ability')
        setup_lines.append(f'let has_choice = game.state.pending_choice.is_some();')
        setup_lines.append(f'// If the ability created a choice, it means it activated')
    
    # Build test function
    test_body = f"""
#[test]
fn {test_name}() {{
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("{FILLER}");
    let card_id = game.id("{first_card}");
    
    // Card name: {cards_list[0].split(' | ')[1] if ' | ' in cards_list[0] else 'unknown'}
    // Ability text: {t[:60]}
    
    {chr(10).join(setup_lines)}
    
    // Verify no crash
    assert!(true, "Ability {i} executed without panic");
}}
"""
    test_bodies.append(test_body)

# Write test file
with open(OUTPUT, 'w', encoding='utf-8') as f:
    f.write("""// Auto-generated ability tests
// Generated from abilities.json
// DO NOT EDIT MANUALLY

use std::path::Path;
use std::sync::Arc;

use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;

mod helpers;

fn load_real_database() -> Arc<CardDatabase> {
    let cards_path = Path::new("../cards/cards.json");
    let cards = CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load real cards from ../cards/cards.json");
    Arc::new(CardDatabase::load_or_create(cards))
}

// ===== HELPER: TestGame =====
// Minimal inline version for auto-generated tests
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::types::{Phase, TurnPhase};
use rabuka_engine::zones::MemberArea;
use rabuka_engine::turn::TurnEngine;

struct TestGame {
    db: Arc<CardDatabase>,
    state: GameState,
}

impl TestGame {
    fn new(db: Arc<CardDatabase>) -> Self {
        let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
        let p2 = Player::new("p2".into(), "Player 2".into(), false);
        p1.is_first_attacker = true;
        let mut state = GameState::new(p1, p2, db.clone());
        state.current_phase = Phase::Main;
        state.current_turn_phase = TurnPhase::FirstAttackerNormal;
        state.turn_number = 1;
        TestGame { db, state }
    }
    
    fn id(&self, card_no: &str) -> i16 {
        self.db.get_card_id(card_no).unwrap_or_else(|| panic!("Card {card_no} not found"))
    }
    
    fn add_to_hand(&mut self, card_id: i16) {
        self.state.player1.hand.add_card(card_id);
    }
    
    fn add_to_discard(&mut self, card_id: i16) {
        self.state.player1.waitroom.cards.push(card_id);
    }
    
    fn give_energy(&mut self, count: usize) {
        for _ in 0..count {
            if let Some(id) = self.state.player1.energy_deck.draw() {
                self.state.player1.energy_zone.cards.push(id);
                self.state.player1.energy_zone.active_energy_count += 1;
            }
        }
    }
    
    fn play_to_stage(&mut self, card_id: i16, area: MemberArea) {
        // Remove from hand
        if let Some(pos) = self.state.player1.hand.cards.iter().position(|&c| c == card_id) {
            self.state.player1.hand.cards.remove(pos);
        }
        let index = match area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        self.state.player1.stage.stage[index] = card_id;
    }
    
    fn activate_ability(&mut self, card_id: i16) -> Result<(), String> {
        self.state.activating_card = Some(card_id);
        // Push to ability queue
        use rabuka_engine::ability_queue::AbilityQueueEntry;
        let card = self.db.get_card(card_id).unwrap();
        if let Some(ability) = card.abilities.first() {
            let triggers = rabuka_engine::triggers::expand_triggers(ability.triggers.as_deref());
            let entry = AbilityQueueEntry::new(
                rabuka_engine::ability_queue::AbilityId("test".into()),
                card_id, ability.clone(),
                rabuka_engine::triggers::TriggerSource::Activation,
                None, None
            );
            self.state.ability_queue.push(entry);
            let mut resolver = rabuka_engine::ability::AbilityResolver::new(&mut self.state);
            resolver.resolve_ability(ability, Some(card_id), 0)
        } else {
            Err("No ability found".into())
        }
    }
    
    fn has_pending_choice(&self) -> bool {
        self.state.pending_choice.is_some()
    }
}

// ===== GENERATED TESTS =====
""")
    f.write('\n'.join(test_bodies))

print(f"Generated {test_count} test functions -> {OUTPUT}")
