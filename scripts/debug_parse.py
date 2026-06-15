import sys
import os
from pathlib import Path

ROOT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "."))
sys.path.append(ROOT_DIR)
sys.path.append(os.path.join(ROOT_DIR, 'test_parser'))

from test_parser.parser_v2 import AbilityParser

parser = AbilityParser()
text = "手札を1枚控え室に置いてもよい："
print(f"Parsing: {text}")
res = parser.parse_ability(text, debug=True)
print(res.model_dump_json(indent=2, ensure_ascii=False))
