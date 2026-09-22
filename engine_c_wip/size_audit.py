#!/usr/bin/env python3
"""Size audit: compare each C source file in engine_c/src against its Rust
twin in engine/src, by line count. Run from engine_c/:  python size_audit.py

Proves how much of the port is still missing (functions with no C equivalent)."""
import os, glob

ROOT = os.path.dirname(os.path.abspath(__file__))
C_ROOT = os.path.join(ROOT, "src")
R_ROOT = os.path.join(ROOT, "..", "engine", "src")

# C file -> list of Rust twin files (relative to engine/src)
PAIRS = [
    ("ability/vm.c", ["ability/vm.rs"]),
    ("ability/condition.c", ["ability/condition/card.rs", "ability/condition/compound.rs", "ability/condition/state.rs"]),
    ("ability/choice.c", ["ability/choice.rs"]),
    ("ability/compound.c", ["ability/compound.rs"]),
    ("ability/ability_queue.c", ["ability_queue.rs", "triggers.rs"]),
    ("ability/dynamic_count.c", ["ability/dynamic_count.rs"]),
    ("ability/util.c", ["ability/util.rs"]),
    ("ability/cost.c", ["ability/cost.rs"]),
    ("ability/resolver.c", ["ability/resolver.rs"]),
    ("ability/effects/move.c", ["ability/move_cards.rs"]),
    ("ability/effects/look.c", ["ability/look.rs"]),
    ("ability/effects/draw.c", ["ability/effects/draw.rs"]),
    ("ability/effects/misc.c", ["ability/effects/misc.rs"]),
    ("ability/effects/ability.c", ["ability/effects/ability.rs", "ability/ability.rs"]),
    ("ability/effects/state.c", ["ability/effects/state.rs"]),
    ("core/card.c", ["core/card.rs"]),
    ("core/data.c", ["core/data.rs"]),
    ("core/alloc.c", ["core/alloc.rs"]),
    ("core/modifiers.c", ["core/game_state/modifiers.rs"]),
    ("core/stats_pipeline.c", ["core/stats_pipeline.rs"]),
    ("core/game_state_abilities.c", ["core/game_state/abilities.rs"]),
    ("core/tracking.c", ["core/game_state/tracking.rs"]),
    ("core/zones.c", ["core/zones.rs"]),
    ("turn/phase.c", ["turn/phases.rs"]),
    ("turn/live.c", ["turn/live.rs"]),
    ("turn/triggers.c", ["triggers.rs"]),
    ("engine.c", ["engine.rs"]),
]

def lines(path):
    if not os.path.exists(path):
        return 0
    with open(path, encoding="utf-8", errors="ignore") as f:
        return sum(1 for _ in f)

tot_c = tot_r = 0
print(f"{'C file':<42}{'C':>6}{'Rust':>8}{'%':>6}")
print("-" * 62)
for cf, rs in PAIRS:
    cl = lines(os.path.join(C_ROOT, cf))
    rl = sum(lines(os.path.join(R_ROOT, r)) for r in rs)
    tot_c += cl; tot_r += rl
    pct = f"{round(100*cl/rl)}%" if rl else "?"
    print(f"{cf:<42}{cl:>6}{rl:>8}{pct:>6}")
print("-" * 62)
print(f"{'TOTAL (mapped)':<42}{tot_c:>6}{tot_r:>8}{round(100*tot_c/tot_r)}%")
