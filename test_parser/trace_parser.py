import sys
import os

# Add the root directory to sys.path
ROOT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.append(ROOT_DIR)

from test_parser.parser_v2 import AbilityParser


def main():
    parser = AbilityParser()

    # A single complex ability from the real data
    test_text = "{{kidou.png|起動}}このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。"

    print(f"--- TRACE START ---")
    print(f"INPUT: {test_text}\n")

    print(f"--- DECISION TREE ---")
    # We call parse_ability with debug=True
    parser.parse_ability(test_text, debug=True)

    print(f"\n--- TRACE END ---")


if __name__ == "__main__":
    main()
