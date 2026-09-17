# -*- coding: utf-8 -*-
"""Find Main-phase Pass decisions where affordable nonzero-cost deploys existed."""
import json

passes_with_deploys = 0
junk_only_sets = 0
total_sets = 0
total_passes = 0
en_hist = {}
for line in open(r"C:\Users\trios\AppData\Local\Temp\kilo\audit-v7-200.jsonl", encoding="utf-8"):
    try:
        rec = json.loads(line)
    except Exception:
        continue
    if rec.get("event") != "decision" or rec.get("phase") != "Main":
        continue
    ch = rec.get("chosen", {})
    if ch.get("action_type") != "Pass":
        continue
    total_passes += 1
    avail = [(a.get("parameters") or {}).get("final_cost", 0)
             for a in rec.get("available_actions", [])
             if a.get("action_type") == "PlayMemberToStage"]
    real = [c for c in avail if c and c > 0]
    en = (rec.get("view", {}).get("own", {}).get("energy", {}) or {}).get("active", 0)
    if real:
        passes_with_deploys += 1
        en_hist[en] = en_hist.get(en, 0) + 1
        if passes_with_deploys <= 12:
            print("g%s t%s en=%s realDeploys=%s" % (
                rec["game"], rec["turn"], en, sorted(set(real))))
    elif rec.get("phase", "").startswith("LiveCardSet") and rec.get("selection_operation") == "confirm":
        total_sets += 1
        sel = rec.get("live_selected_hand_indices_before") or []
        hand = rec.get("view", {}).get("own", {}).get("hand", [])
        n_live = sum(1 for i in sel if i < len(hand)
                     and hand[i] and hand[i].get("card_type") == "live_card")
        if sel and n_live == 0:
            junk_only_sets += 1
print("total Main Passes:", total_passes)
print("Passes with affordable nonzero-cost deploys:", passes_with_deploys)
print("energy distribution at those passes:", dict(sorted(en_hist.items())))
print("live-set confirms:", total_sets, "| junk-only sets:", junk_only_sets)
