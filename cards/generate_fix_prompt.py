import json

ref = json.load(
    open(
        r"C:\Users\trios\Downloads\rabuka_reloaded-master (2)\rabuka_reloaded-master\cards\abilities.json",
        encoding="utf-8",
    )
)
gen = json.load(
    open(
        r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json",
        encoding="utf-8",
    )
)
rs = sorted(ref["unique_abilities"], key=lambda x: x.get("full_text", ""))
gs = sorted(gen["unique_abilities"], key=lambda x: x.get("full_text", ""))

out = []
out.append("=" * 80)
out.append("ABILITIES.JSON REGENERATION \u2014 REMAINING 26 DIFFS FIX PLAN")
out.append("=" * 80)
out.append("")
out.append("Primary file: cards/ability_extraction/parser.py")
out.append("  - process_abilities() starts at ~line 7242")
out.append("  - Additional fixes section starts at ~line 7637")
out.append("Secondary file: cards/ability_extraction/extract_card_abilities.py")
out.append("Target abilities.json: cards/abilities.json")
out.append(
    "Reference: C:\\Users\\trios\\Downloads\\rabuka_reloaded-master (2)\\rabuka_reloaded-master\\cards\\abilities.json"
)
out.append("Current state: 724/750 (96.5%)")
out.append("")

count = 0
for r, g in zip(rs, gs):
    rd = {
        k: v for k, v in r.items() if k not in ("cards", "card_count", "generated_at")
    }
    gd = {
        k: v for k, v in g.items() if k not in ("cards", "card_count", "generated_at")
    }
    if rd == gd:
        continue
    count += 1
    eff_r = r.get("effect", {}) or {}
    eff_g = g.get("effect", {}) or {}

    out.append(f"--- DIFF #{count} ---")
    out.append(f"Full text: {r['full_text']}")
    out.append("")

    # Root cause
    if eff_r.get("action") != eff_g.get("action"):
        out.append(
            f"1. ROOT CAUSE: action differs: R={eff_r.get('action')} G={eff_g.get('action')}"
        )

    if eff_r.get("condition") != eff_g.get("condition"):
        cr = eff_r.get("condition", {}) or {}
        cg = eff_g.get("condition", {}) or {}
        out.append(f"1. ROOT CAUSE: condition differs")
        for k in sorted(set(list(cr.keys()) + list(cg.keys()))):
            if cr.get(k) != cg.get(k):
                out.append(f"   {k}: R={repr(cr.get(k))[:60]} G={repr(cg.get(k))[:60]}")

    if eff_r.get("primary_effect") != eff_g.get("primary_effect"):
        out.append(f"1. ROOT CAUSE: primary_effect differs")
        pe_r = eff_r.get("primary_effect", {}) or {}
        pe_g = eff_g.get("primary_effect", {}) or {}
        for k in sorted(set(list(pe_r.keys()) + list(pe_g.keys()))):
            rv, gv = pe_r.get(k), pe_g.get(k)
            if rv != gv:
                if isinstance(rv, str):
                    rv = rv[:60]
                if isinstance(gv, str):
                    gv = gv[:60]
                out.append(f"   {k}: R={repr(rv)} G={repr(gv)}")

    if eff_r.get("actions") != eff_g.get("actions"):
        out.append(f"1. ROOT CAUSE: actions array differs")

    if eff_r.get("select_action") != eff_g.get("select_action"):
        sa_r = eff_r.get("select_action", {}) or {}
        sa_g = eff_g.get("select_action", {}) or {}
        out.append(f"1. ROOT CAUSE: select_action differs")
        for k in sorted(set(list(sa_r.keys()) + list(sa_g.keys()))):
            if sa_r.get(k) != sa_g.get(k):
                out.append(
                    f"   {k}: R={repr(sa_r.get(k))[:60]} G={repr(sa_g.get(k))[:60]}"
                )

    if eff_r.get("followup_action") != eff_g.get("followup_action"):
        out.append(f"1. ROOT CAUSE: followup_action differs")

    out.append("")
    out.append("2. REQUIRED CODE CHANGE:")

    ft = r["full_text"]

    if "かのん" in ft and "可可" in ft:
        out.append("   File: cards/ability_extraction/parser.py")
        out.append("   Handler: _try_per_character_resource (~line 5220)")
        out.append("   Problem: flat sequential[blade,heart,blade,heart] instead of")
        out.append(
            "            sequential[sequential[blade,heart], sequential[blade,heart]]"
        )
        out.append("   Fix: wrap each character blade+heart in nested sequential")
    elif "SunnyPassion" in ft:
        out.append("   File: cards/ability_extraction/parser.py")
        out.append("   Handler: _try_look_and_select (~line 5570)")
        out.append("   Problem: OR groups flattened to AND (flat group_names)")
        out.append("   Fix: detect が/または OR patterns, produce options array")
    elif "能力を持たない" in ft and r.get("triggerless_text", "").startswith(
        "自分のライブカード置き場"
    ):
        out.append("   File: cards/ability_extraction/parser.py")
        out.append("   Section: Additional fixes, add between E and F")
        out.append(
            '   Problem: select_action missing ability_filter for "X能力を持たない"'
        )
        out.append(
            '   Fix: regex r"[「（](.+?)[」）]能力を持たない" on triggerless_text'
        )
    elif "のうち、1つを選ぶ" in ft:
        out.append("   File: cards/ability_extraction/parser.py")
        out.append("   Section: Additional fixes D2 area")
        out.append(
            "   Problem: second action missing timing_condition and moved_this_turn"
        )
        out.append("   Fix: propagate timing_condition from select to gain_resource")
    elif "エネルギーが7枚" in ft or "エネルギー" in ft and "Liella!" in ft:
        out.append("   File: cards/ability_extraction/parser.py")
        out.append("   Section: Additional fixes, after E0")
        out.append("   Problem: sub-actions missing duration and target")
        out.append(
            "   Fix: propagate duration from parent sequential to sub gain_resource actions"
        )
    elif "Aqours" in ft and "SaintSnow" in ft:
        out.append("   File: cards/ability_extraction/parser.py")
        out.append("   Section: Additional fixes, add entry")
        out.append("   Problem: comparison_condition sub-condition missing location")
        out.append("   Fix: propagate location from compound parent to sub-conditions")
    elif "デッキの一番上" in ft:
        out.append("   File: cards/ability_extraction/parser.py")
        out.append("   Section: Additional fixes, after E")
        out.append(
            "   Problem: R=sequential, G=conditional_on_result (reverse overmatch)"
        )
        out.append(
            "   Fix: if conditional_on_result and primary_effect has simple move, revert"
        )
    elif "DOLLCHESTRA" in ft:
        out.append("   File: cards/ability_extraction/parser.py")
        out.append("   Section: Additional fixes, add entry")
        out.append(
            "   Problem: primary_effect is select instead of sequential[select,modify_cost]"
        )
        out.append(
            "   Fix: reconstruct when pe.action=select and parent has group_names+original_value"
        )
    elif "シャッフル" in ft or "shuffle" in r.get("triggerless_text", ""):
        out.append("   File: cards/ability_extraction/parser.py")
        out.append("   Section: Additional fixes, add entry")
        out.append(
            "   Problem: primary_effect should be sequential[shuffle, move_cards] with multiple_targets"
        )
        out.append("   Fix: when source=discard and shuffle=true, wrap in sequential")
    elif "ウェイト" in ft and "カードを3枚" in ft:
        out.append("   File: cards/ability_extraction/parser.py")
        out.append(
            "   Section C (already modified): need card_property on followup sub-condition"
        )
        out.append(
            "   Add to C-extend: for remaining followup actions, also check/copy card_property"
        )
    elif (
        "成功" in ft
        and r.get("effect", {}).get("condition", {}).get("type") == "temporal_condition"
    ):
        out.append("   File: cards/ability_extraction/parser.py")
        out.append("   Section B (already fixed): verify success temporal conditions")
    else:
        out.append("   File: cards/ability_extraction/parser.py")
        out.append("   Section: Additional fixes (post-processing)")
        out.append("   See diff details above for exact field changes needed")

    out.append("")
    out.append("3. HOW TO VERIFY THIS DIFF FIXED:")
    out.append('   grep -c "TERM" cards/abilities.json # check before')
    out.append('   python -c "import json; ... # run comparison"  ')
    out.append("")

out.append("=" * 80)
out.append("SUMMARY")
out.append("=" * 80)
out.append(f"Total: {count} diffs remaining after 26 fixed")
out.append("")
out.append("Quick-win fixes (in post-processing section of parser.py):")
out.append("  - compound condition sub-field propagation (4 entries)")
out.append("  - duration propagation from parent sequential (1 entry)")
out.append("  - move_cards conditional_on_result overmatch revert (1 entry)")
out.append("  - DOLLCHESTRA primary_effect reconstruct (1 entry)")
out.append("  - shuffle action structure (1 entry)")
out.append("")
out.append("Parser handler changes needed:")
out.append("  - _try_per_character_resource: character resource nesting")
out.append("  - _try_look_and_select: OR options detection")
out.append("  - _try_select: or_ability_filters extraction")

with open("cards/remaining_26_FIX_PROMPT.md", "w", encoding="utf-8") as f:
    f.write("\n".join(out))

print(f"Written {len(out)} lines to cards/remaining_26_FIX_PROMPT.md")
