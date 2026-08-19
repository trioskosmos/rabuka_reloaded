#!/usr/bin/env python3
"""DEPRECATED — merged into `python cards/test_inventory.py`.

Old script: listed untested abilities / generated untested_abilities_report.txt.
New canonical output:
  * engine/tests/TEST_COVERAGE.md  (gap tables)
  * engine/tests/TEST_INVENTORY.json

Just run `python cards/test_inventory.py`.
"""
import sys
from pathlib import Path
import importlib.util

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("test_inventory", ROOT / "cards" / "test_inventory.py")
mod = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(mod)

if __name__ == "__main__":
    sys.stderr.write("[DEPRECATED] ability_docs_scripts/find_untested_abilities.py -> use `python cards/test_inventory.py`\n")
    sys.exit(mod.main())
