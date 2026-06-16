"""Orchestrator — runs the full extraction+parsing pipeline."""

import json
from pathlib import Path

from test_parser.extract import extract_all_abilities
from test_parser.normalize import process_abilities


def run_pipeline(cards_path: Path, output_path: Path = None) -> dict:
    """Run the full pipeline: extract → parse → normalize → output."""
    print(f"Extracting abilities from {cards_path}...")
    data = extract_all_abilities(cards_path)

    stats = data["statistics"]
    print(
        f"  Found {stats['total_abilities']} abilities across {stats['cards_with_abilities']} cards"
    )
    print(f"  Unique abilities: {stats['unique_abilities']}")

    print("Parsing costs and effects...")
    data = process_abilities(data)

    if output_path:
        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"Output written to {output_path}")

    return data
