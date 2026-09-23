# -*- coding: utf-8 -*-
"""Loss-cluster analysis over bot_arena --audit JSONL.

Usage: python analyze_losses.py <audit.jsonl> [--examples N]

Per game, attributes the loser's failure to measurable behaviors:
  - empty Main phases (deployed nothing)
  - failed live checks (set >=1 card, success count did not rise)
  - live-set folds (confirmed empty)
  - mulligan discards
  - stage cost at loss (development gap vs winner)
Prints aggregate clusters plus N example games with per-turn detail.
"""
import json
import sys
from collections import defaultdict


def main():
    path = sys.argv[1]
    n_examples = 3
    if "--examples" in sys.argv:
        n_examples = int(sys.argv[sys.argv.index("--examples") + 1])

    # per (game, player)
    feats = defaultdict(lambda: {
        "empty_mains": 0, "mains": 0, "deployed": False,
        "sets": {},          # turn -> set size at confirm
        "succ_at_set": {},   # turn -> own success at confirm
        "succ_seen": {},     # turn -> max success seen that turn
        "mull_discards": 0,
        "max_stage_cost": 0,
        "max_blades": 0,
    })
    game_final = {}  # game -> (z1, z2)
    game_turns = {}

    with open(path, encoding="utf-8") as f:
        for line in f:
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            if rec.get("event") == "game_end":
                z1, z2 = rec["success_counts"]
                game_final[rec["game"]] = (z1, z2, rec.get("turn", 0))
                continue
            if rec.get("event") != "decision":
                continue
            g, p, turn = rec["game"], rec["policy_player"], rec.get("turn", 0)
            f = feats[(g, p)]
            phase = rec.get("phase", "")
            op = rec.get("selection_operation")
            chosen = rec.get("chosen", {})
            own = rec.get("view", {}).get("own", {})
            if phase.startswith("Mulligan") and op == "confirm":
                f["mull_discards"] = len(rec.get("mulligan_selected_hand_indices_before", []))
            elif phase == "Main":
                at = chosen.get("action_type")
                if at == "Pass":
                    f["mains"] += 1
                elif at in ("PlayMemberToStage",) or (at == "UseAbility" and
                        (chosen.get("parameters") or {}).get("use_baton_touch") is True):
                    f["deployed"] = True
                sc = own.get("success_count")
                if sc is not None:
                    f["succ_seen"][turn] = max(f["succ_seen"].get(turn, 0), sc)
                stage = [c for c in own.get("stage_left_center_right", []) if c]
                f["max_stage_cost"] = max(f["max_stage_cost"],
                    sum(c.get("base_cost") or 0 for c in stage))
                f["max_blades"] = max(f["max_blades"],
                    sum(c.get("base_blades") or 0 for c in stage))
            elif phase.startswith("LiveCardSet") and op == "confirm":
                sel = rec.get("live_selected_hand_indices_before", [])
                hand = own.get("hand", [])
                n_live = sum(1 for i in sel if i < len(hand)
                             and hand[i] and hand[i].get("card_type") == "live_card")
                n_junk = len(sel) - n_live
                f["sets"][turn] = (len(sel), own.get("success_count", 0),
                                   phase.endswith("SecondAttacker"), n_live, n_junk)

    # failed checks: a confirmed set (>=1 card) at turn T where success never
    # rose above the confirm-time value at any later row of that game.
    # success attribution: p1's final = z1
    fails = defaultdict(int)
    attempts = defaultdict(int)
    for (g, p), f in feats.items():
        if not game_final.get(g):
            continue
        z1, z2, _ = game_final[g]
        final_succ = z1 if p == "p1" else z2
        last_succ_seen = max(f["succ_seen"].values()) if f["succ_seen"] else 0
        for turn, (size, succ_at, _, _, _) in sorted(f["sets"].items()):
            if size == 0:
                continue
            attempts[p] += 1
            # a set "converted" if the player's final successes exceed the
            # success count at the last set of the game
            if final_succ <= f["succ_seen"].get(turn, succ_at):
                fails[p] += 1

    # classify games
    clusters = defaultdict(int)
    examples = defaultdict(list)
    for g, (z1, z2, turns) in sorted(game_final.items()):
        if z1 >= 3 and z2 <= 2:
            loser = "p2"
        elif z2 >= 3 and z1 <= 2:
            loser = "p1"
        else:
            continue
        f = feats[(g, loser)]
        w = feats[(g, "p1" if loser == "p2" else "p2")]
        tags = []
        if f["mains"] and f.get("deployed") is False:
            tags.append("all-empty-mains")
        n_empty = 0
        failed = sum(1 for turn, (size, _, _, _, _) in f["sets"].items()
                     if size > 0 and final_succ_of(game_final, g, loser) <= f["succ_seen"].get(turn, 0))
        if failed:
            tags.append(f"failed_checks:{failed}")
        if any(size == 0 for size, _, _, _, _ in f["sets"].values()) and not f["sets"]:
            tags.append("folded_all")
        gap = w["max_stage_cost"] - f["max_stage_cost"]
        if gap >= 6:
            tags.append(f"dev_gap:{gap}")
        key = tags[0] if tags else "no_obvious_tag"
        clusters[key] += 1
        if len(examples[key]) < n_examples:
            examples[key].append((g, loser, turns, tags, f, w))

    total_losses = sum(clusters.values())
    print(f"decided games: {total_losses}")
    print("loss clusters (loser-side features):")
    for k, c in sorted(clusters.items(), key=lambda kv: -kv[1]):
        print(f"  {k:24s} {c} ({100*c/total_losses:.0f}%)")

    def dump(f, w):
        sets = " ".join(f"t{t}:{s[0]}({s[3]}L+{s[4]}j){'S' if s[1] else 'F'}"
                        for t, s in sorted(f["sets"].items()))
        return (f"    mull={f['mull_discards']} mains={f['mains']} "
                f"sets=[{sets}] maxCost={f['max_stage_cost']} maxBlades={f['max_blades']} | "
                f"winner: mull={w['mull_discards']} mains={w['mains']} maxCost={w['max_stage_cost']}")

    print("\nexample games per cluster:")
    for k, exs in examples.items():
        for (g, loser, turns, tags, f, w) in exs:
            print(f"  game {g} loser={loser} turns={turns} tags={tags}")
            print(f"    {dump(f, w)}")


def final_succ_of(game_final, g, p):
    z1, z2, _ = game_final[g]
    return z1 if p == "p1" else z2


if __name__ == "__main__":
    main()
