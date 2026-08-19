#!/usr/bin/env python3
"""DEPRECATED — use `python cards/test_inventory.py` instead.

This file is kept as a thin alias so existing docs/CI that call
`python cards/coverage_report.py` keep working. It simply delegates to
`cards/test_inventory.py` which now generates:

  * engine/tests/TEST_COVERAGE.md
  * docs/ABILITY_MATRIX.md
  * engine/tests/TEST_INVENTORY.json + .md

Run the canonical command directly:

    python cards/test_inventory.py          # regenerate all
    python cards/test_inventory.py --check  # CI: fail if stale
"""
import sys
from pathlib import Path

# delegate to test_inventory
import importlib.util

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("test_inventory", ROOT / "cards" / "test_inventory.py")
mod = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(mod)

if __name__ == "__main__":
    sys.stderr.write("[DEPRECATED] cards/coverage_report.py -> use `python cards/test_inventory.py`\n")
    sys.exit(mod.main())
