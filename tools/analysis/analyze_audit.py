# -*- coding: utf-8 -*-
"""Analyze bot_arena --audit JSONL: mulligan behavior + deployment curve.

Usage: python analyze_audit.py <audit.jsonl>
Prints:
  - mulligan discard-count distribution and keep composition
  - curve-chain detection (two members sum<=4 -> bridge -> finish)
  - per-turn stage cost / member count / blade count per player
  - live-set set-count distribution per phase
"""
import json
import sys
from collections import defaultdict


def card_cost(card):
    if card is None:
        return None
    return card.get("base_cost")


def card_no(card):
    if card is None:
        return None
    return card.get("card_no")


def main(path):
    mulligan_discards = {}      # (game, player) -> n_selected at confirm
    mulligan_hand = {}          # (game, player) -> hand card_nos at confirm
    stage_by_turn = {}          # (game, player, turn) -> (cost_sum, members, blades, energy)
    live_sets = []              # (game, player, turn, second, n_set, success_count)
    outcomes = {}               # game -> [succ_p1, succ_p2] last seen
    for line in open(sys.argv[1], encoding="utf-8"):
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            continue
        if rec.get("event") != "decision":
            continue
        phase = rec.get("phase", "")
        player = rec.get("policy_player")
        game = rec.get("game")
        chosen = rec.get("chosen", {})
        op = rec.get("selection_operation")
        view = rec.get("view", {})
        own = view.get("own", {})
        if phase.startswith("Mulligan") and op == "confirm":
            sel = rec.get("mulligan_selected_hand_indices_before", [])
            hand = [card_no(c) for c in own.get("hand", [])]
            mulligan_discards[(game, player)] = (len(sel), hand)
        elif phase == "Main":
            turn = rec.get("turn")
            stage = own.get("stage_left_center_right", [])
            costs = [c.get("base_cost") or 0 if c else 0 for c in stage]
            blades = [c.get("base_blades") or 0 if c else 0 for c in stage]
            members = sum(1 for c in stage if c)
            prev = stage_by_turn.get((game, player, turn))
            cur = (sum(costs), members, sum(blades), own.get("energy", {}).get("active", 0))
            if prev is None or sum(cur) > sum(prev):
                stage_by_turn[(game, player, turn)] = cur
        elif phase.startswith("LiveCardSet") and op == "confirm":
            sel = rec.get("live_selected_hand_indices_before", [])
            live_sets.append((game, player, rec.get("turn"),
                              phase.endswith("SecondAttacker"),
                              len(sel), own.get("success_count", 0)))
        succ = own.get("success_count")
        if succ is not None and player == "p1":
            outcomes[game] = succ
    n_confirms = len(mulligan_discards)
    dist = {}
    curve_keep = 0
    curve_discard = 0
    for (g, p), (n, hand) in sorted(mulligan_discards.items()):
        dist[n] = dist.get(n, 0) + 1
    print(f"mulligan confirms: {n_confirms}")
    print("discard-count distribution:", dict(sorted(dist.items())))
    print()
    turns = sorted({t for (_, _, t) in stage_by_turn})
    print("turn | snapshots | avg cost p1 | avg cost p2 | avg members | avg blades")
    for t in turns:
        snap = [(g, p) for (g, p, tt) in stage_by_turn if tt == t]
        if not snap:
            continue
        cost1 = [stage_by_turn[(g, p, t)][0] for (g, p) in snap if p == "p1"]
        cost2 = [stage_by_turn[(g, p, t)][0] for (g, p) in snap if p == "p2"]
        mem = [stage_by_turn[(g, p, t)][1] for (g, p) in snap]
        bl = [stage_by_turn[(g, p, t)][2] for (g, p) in snap]
        c1 = f"{sum(cost1)/len(cost1):5.1f}" if cost1 else "  n/a"
        c2 = f"{sum(cost2)/len(cost2):5.1f}" if cost2 else "  n/a"
        print(f"  {t:2d} | {len(snap):9d} | {c1} | {c2} | {sum(mem)/len(mem):5.2f} | {sum(bl)/len(bl):5.2f}")
    print()
    if live_sets:
        n_dist = {}
        for (_, _, _, _, n, _) in live_sets:
            n_dist[n] = n_dist.get(n, 0) + 1
        print("live-set size distribution:", dict(sorted(n_dist.items())))
        print(f"live-set decisions: {len(live_sets)}")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: python analyze_audit.py <audit.jsonl>")
        sys.exit(1)
    main(sys.argv[1])
