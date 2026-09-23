# -*- coding: utf-8 -*-
"""Hunt concrete bad plays in bot_arena --audit JSONL.

Categories:
  A. Main Pass while affordable nonzero-cost deploys exist (and energy ok)
  B. Main Pass while energy >= 5 and any deploy cost <= energy (late hoard)
  C. Live-set confirm with 0 lives selected but lives in hand
  D. Live-set confirm pure-junk when a floor-clearing single existed in hand
  E. Deploy chosen that DROPS blades and does not raise cost (sideways waste)
  F. Chose Pass when a deploy raises stage cost (empty main with free growth)

Usage: python hunt_bad_plays.py <audit.jsonl> [--examples N]
"""
import json
import sys
from collections import Counter, defaultdict


def card_no(c):
    if not c:
        return None
    return c.get("card_no")


def card_cost(c):
    if not c:
        return 0
    return c.get("base_cost") or 0


def card_type(c):
    if not c:
        return None
    return c.get("card_type")


def main():
    path = sys.argv[1]
    n_ex = 12
    if "--examples" in sys.argv:
        n_ex = int(sys.argv[sys.argv.index("--examples") + 1])

    counts = Counter()
    examples = defaultdict(list)
    total_main = 0
    total_pass = 0
    total_setconf = 0
    total_set_with_lives = 0
    phases = Counter()
    chosen_main = Counter()

    with open(path, encoding="utf-8") as f:
        for line in f:
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            if rec.get("event") != "decision":
                continue
            phase = rec.get("phase", "")
            phases[phase] += 1
            chosen = rec.get("chosen") or {}
            view = rec.get("view") or {}
            own = view.get("own") or {}
            hand = own.get("hand") or []
            stage = own.get("stage_left_center_right") or []
            en = (own.get("energy") or {}).get("active", 0)
            succ = own.get("success_count", 0)
            opp = view.get("opponent") or {}
            opp_succ = opp.get("success_count", 0)
            g, p, t = rec.get("game"), rec.get("policy_player"), rec.get("turn")
            avail = rec.get("available_actions") or []

            if phase == "Main":
                total_main += 1
                at = chosen.get("action_type", "?")
                chosen_main[at] += 1

                plays = [a for a in avail if a.get("action_type") == "PlayMemberToStage"]
                # cost of each available play (final_cost if present else card base)
                costs = []
                for a in plays:
                    pr = a.get("parameters") or {}
                    fc = pr.get("final_cost")
                    if fc is None:
                        cid = pr.get("card_no") or pr.get("card_id")
                        # fall back: find matching hand card by index-like fields
                        fc = None
                        hi = pr.get("hand_index")
                        if hi is not None and hi < len(hand):
                            fc = card_cost(hand[hi])
                    if fc is None:
                        # try any hand cost from parameters card
                        fc = 0
                    costs.append(fc)
                stage_cost = sum(card_cost(c) for c in stage)
                hand_costs = [card_cost(c) for c in hand]
                max_hand_cost = max(hand_costs) if hand_costs else 0

                if at == "Pass":
                    total_pass += 1
                    # A: any affordable nonzero deploy
                    affordable = [c for c in costs if c and c > 0 and c <= en]
                    # also consider hand costs if final_cost missing
                    affordable_hand = [c for c in hand_costs if c > 0 and c <= en and c <= stage_cost]
                    # F: any hand card cost > stage_cost growth potential?
                    growth = [c for c in hand_costs if c > stage_cost and c <= en]

                    if affordable or (not costs and affordable_hand):
                        counts["A_pass_with_affordable_deploy"] += 1
                        if len(examples["A"]) < n_ex:
                            examples["A"].append(
                                "g%s t%s en=%s stage_cost=%s hand_costs=%s avail_costs=%s"
                                % (g, t, en, stage_cost, sorted(hand_costs), sorted(costs))
                            )
                    elif growth:
                        counts["F_pass_no_cost_growth"] += 1
                        if len(examples["F"]) < n_ex:
                            examples["F"].append(
                                "g%s t%s en=%s stage_cost=%s hand_costs=%s (no growth possible)"
                                % (g, t, en, stage_cost, sorted(hand_costs))
                            )
                    # B: energy hoard — en high but passed
                    if en >= 6 and any(c > 0 for c in hand_costs):
                        counts["B_pass_energy_ge6"] += 1
                        if len(examples["B"]) < n_ex:
                            examples["B"].append(
                                "g%s t%s en=%s stage_cost=%s hand_costs=%s"
                                % (g, t, en, stage_cost, sorted(hand_costs))
                            )

                elif at == "PlayMemberToStage":
                    pr = chosen.get("parameters") or {}
                    # E: chosen play targets occupied slot without card_indices? hard;
                    # instead: after-play stage cost should rise for non-baton.
                    # Compare chosen cost vs stage: if final_cost < stage max and
                    # no baton flag, might be OK. Skip if baton detection unclear.
                    pass

            elif phase.startswith("LiveCardSet") and rec.get("selection_operation") == "confirm":
                total_setconf += 1
                sel = rec.get("live_selected_hand_indices_before") or []
                # indices may be hand indices before selection
                lives_in_hand = [
                    i for i, c in enumerate(hand) if card_type(c) == "live_card"
                ]
                n_sel_lives = sum(
                    1 for i in sel if i < len(hand) and card_type(hand[i]) == "live_card"
                )
                n_sel = len(sel)
                if n_sel > 0 and n_sel_lives == 0:
                    counts["C_set_pure_junk"] += 1
                    if len(examples["C"]) < n_ex:
                        examples["C"].append(
                            "g%s t%s succ=%s/%s sel=%s lives_in_hand=%s hand_types=%s"
                            % (
                                g, t, succ, opp_succ, sel, lives_in_hand,
                                [card_type(c) for c in hand],
                            )
                        )
                if lives_in_hand:
                    total_set_with_lives += 1
                    if n_sel == 0:
                        counts["D_fold_with_lives"] += 1
                        if len(examples["D"]) < n_ex:
                            examples["D"].append(
                                "g%s t%s succ=%s/%s lives_in_hand=%s hand=%s"
                                % (g, t, succ, opp_succ, lives_in_hand,
                                   [card_no(c) for c in hand])
                            )
                    if n_sel_lives == 0 and n_sel > 0:
                        pass  # already C

    print("phases:", dict(phases))
    print("main chosen:", dict(chosen_main))
    print("main decisions:", total_main, "passes:", total_pass)
    print("live confirms:", total_setconf, "confirms with lives in hand:", total_set_with_lives)
    print("counts:")
    for k in sorted(counts):
        print("  %-32s %6d" % (k, counts[k]))
    print("\nexamples:")
    for k in sorted(examples):
        print("--- %s ---" % k)
        for line in examples[k]:
            print(" ", line)


if __name__ == "__main__":
    main()
