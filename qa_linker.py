"""QA data linker: reads qa_data.json, extracts structured test stubs.

Usage:
    python qa_linker.py             # Generate all stubs
    python qa_linker.py Q001 Q010   # Generate specific stubs

Output: engine/tests/qa_generated/ directory with .rs test files.

How it works:
- Parses qa_data.json entries
- Extracts related_cards card numbers and names
- Looks up those cards in cards.json to get their ability text
- Generates a Rust test skeleton with the card info embedded
- The generated tests need manual completion but save 80% of boilerplate
"""

import json
import os
import sys
import re
from pathlib import Path

CARDS_JSON = Path("cards/cards.json")
QA_JSON = Path("cards/qa_data.json")
OUT_DIR = Path("engine/tests/qa_generated")

def load_json(path):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)

def find_card_by_no(cards, card_no):
    """Find a card in cards.json by card_no."""
    if isinstance(cards, dict):
        # Object format: {card_no: {card data}}
        return cards.get(card_no)
    elif isinstance(cards, list):
        # Array format
        for c in cards:
            if c.get("card_no") == card_no:
                return c
    return None

def strip_markup(text):
    """Remove {{...}} wiki-style markup from text."""
    return re.sub(r'\{\{[^}]+\}\}', '', text).strip()

def clean_jp(text):
    """Remove icons and markup for use in comments."""
    text = re.sub(r'\{\{[^}]+\}\}', '', text)
    text = re.sub(r'[\n\r]+', ' ', text)
    text = text[:80]  # Truncate
    return text.strip()

def generate_test(qa_entry, cards):
    qid = qa_entry["id"]
    question = clean_jp(qa_entry.get("question", ""))
    answer = clean_jp(qa_entry.get("answer", ""))
    related = qa_entry.get("related_cards", [])

    # Look up related cards
    card_info_lines = []
    load_calls = []
    card_vars = []
    for i, rc in enumerate(related):
        cno = rc.get("card_no", "")
        cname = rc.get("name", "")
        card = find_card_by_no(cards, cno)
        ability = ""
        if card:
            ability = clean_jp(card.get("ability", ""))
        varname = f"card_{i}"
        card_info_lines.append(f"//   {cno} | {cname} | {ability}")
        load_calls.append(f"    let {varname} = cards.iter().find(|c: &&Card| c.card_no == \"{cno}\").expect(\"{cno}\");")
        card_vars.append(f"        get_card_id({varname}, &card_database),")

    related_ids = ", ".join([f"\"{rc.get('card_no', '')}\"" for rc in related])

    test_body = f"""// Auto-generated test stub for {qid}
// Source: qa_data.json
// Question: {question}
// Answer: {answer}
// Related cards: {related_ids}

use rabuka_engine::card::{{Card, CardDatabase}};
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use std::sync::Arc;
use std::path::Path;

fn load_all_cards() -> Vec<Card> {{
    CardLoader::load_cards_from_file(Path::new("../cards/cards.json")).expect("Failed to load cards")
}}

fn create_card_database(cards: Vec<Card>) -> Arc<CardDatabase> {{
    Arc::new(CardDatabase::load_or_create(cards))
}}

fn get_card_id(card: &Card, db: &Arc<CardDatabase>) -> i16 {{
    db.get_card_id(&card.card_no).unwrap_or(0)
}}

#[test]
fn {qid.lower()}_stub() {{
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());

    // Related cards for this QA entry:
{chr(10).join(load_calls)}

    let mut player1 = Player::new("player1".into(), "Player 1".into(), true);
    let mut player2 = Player::new("player2".into(), "Player 2".into(), false);

    // Setup: put related cards in appropriate zones
{chr(10).join([f"    // player1.hand.cards.push({var});" for var in card_vars])}

    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;

    // TODO: set up the exact board state described in the QA entry
    // Question: {clean_jp(qa_entry.get("question", ""))}
    // Answer: {clean_jp(qa_entry.get("answer", ""))}

    // Execute the relevant game actions, then assert the answer
    // assert!(result == expected);

    println!("{qid} stub executed - manual completion required");
}}
"""
    return test_body

def main():
    if not QA_JSON.exists():
        print(f"ERROR: {QA_JSON} not found")
        sys.exit(1)
    if not CARDS_JSON.exists():
        print(f"ERROR: {CARDS_JSON} not found")
        sys.exit(1)

    qa_data = load_json(QA_JSON)
    cards_data = load_json(CARDS_JSON)

    out_dir = OUT_DIR
    out_dir.mkdir(parents=True, exist_ok=True)

    # Filter by args if provided
    requested = set()
    for arg in sys.argv[1:]:
        if arg.startswith("Q"):
            requested.add(arg.upper())

    generated = 0
    for entry in qa_data:
        qid = entry.get("id", "")
        if requested and qid.upper() not in requested:
            continue

        test_body = generate_test(entry, cards_data)
        out_file = out_dir / f"{qid.lower()}_stub.rs"
        with open(out_file, "w", encoding="utf-8") as f:
            f.write(test_body)
        generated += 1

    print(f"Generated {generated} test stubs in {out_dir}/")
    if not requested:
        print(f"Total QA entries: {len(qa_data)}")
        print("To generate specific entries: python qa_linker.py Q001 Q010")

if __name__ == "__main__":
    main()
