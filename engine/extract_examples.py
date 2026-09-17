# -*- coding: utf-8 -*-
"""Extract per-decision rows for given (game, loser) pairs from audit JSONL."""
import json
import sys

GAMES = {40: "p2", 18: "p1", 1: "p2", 21: "p2"}
out = open(r"C:\Users\trios\AppData\Local\Temp\kilo\devgap-examples.txt", "w", encoding="utf-8")
for line in open(r"C:\Users\trios\AppData\Local\Temp\kilo\audit-v7-200.jsonl", encoding="utf-8"):
    try:
        rec = json.loads(line)
    except Exception:
        continue
    if rec.get("event") != "decision" or rec.get("game") not in GAMES:
        continue
    p = rec.get("policy_player")
    if p != GAMES[rec["game"]]:
        continue
    ph = rec.get("phase", "")
    ch = rec.get("chosen", {})
    own = rec.get("view", {}).get("own", {})
    en = own.get("energy", {}).get("active")
    hand = [(c or {}).get("card_no") for c in own.get("hand", [])]
    costs = [(c or {}).get("base_cost") for c in own.get("stage_left_center_right", [])]
    if ph == "Main":
        avail = [(a.get("parameters") or {}).get("final_cost")
                 for a in rec.get("available_actions", [])
                 if a.get("action_type") == "PlayMemberToStage"]
        out.write("g%s t%s MAIN p=%s en=%s chose=%s:%s stage=%s availDeploys=%s\n" % (
            rec["game"], rec["turn"], p, en, ch.get("action_type"),
            (ch.get("parameters") or {}).get("card_no"), costs, avail))
    elif ph.startswith("LiveCardSet") and rec.get("selection_operation") == "confirm":
        sel = rec.get("live_selected_hand_indices_before") or []
        out.write("g%s t%s SETCONF n=%d sel=%s\n" % (rec["game"], rec["turn"], len(sel), sel))
out.close()
print("done")
