"""Entry point: python -m test_parser"""

import sys
from pathlib import Path

from test_parser.main import run_pipeline

if __name__ == "__main__":
    cards_path = Path(__file__).parent.parent / "cards" / "cards.json"
    output_path = Path(__file__).parent.parent / "cards" / "abilities.json"

    if len(sys.argv) > 1:
        cards_path = Path(sys.argv[1])
    if len(sys.argv) > 2:
        output_path = Path(sys.argv[2])

    run_pipeline(cards_path, output_path)
