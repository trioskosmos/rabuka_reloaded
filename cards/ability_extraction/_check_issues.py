import json

with open("cards/abilities.json", encoding="utf-8") as f:
    data = json.load(f)
abilities = data["unique_abilities"]

icon_all_issues = 0
for a in abilities:
    t = a.get("triggerless_text", "") + a.get("full_text", "")
    eff = a.get("effect", {}) or {}
    if "{{icon_all.png" not in t:
        continue

    def find_ht(d):
        if isinstance(d, dict):
            if d.get("heart_type") == "all":
                return True
            for v in d.values():
                if find_ht(v):
                    return True
        elif isinstance(d, list):
            for item in d:
                if find_ht(item):
                    return True
        return False

    if not find_ht(eff):
        icon_all_issues += 1

af_issues = 0
for a in abilities:
    t = a.get("triggerless_text", "") + a.get("full_text", "")
    eff = a.get("effect", {}) or {}
    if "能力を持たない" not in t:
        continue

    def scan_af(d, found=[False]):
        if isinstance(d, dict):
            if d.get("ability_filter") or d.get("or_ability_filters"):
                found[0] = True
            for v in d.values():
                scan_af(v, found)
        elif isinstance(d, list):
            for item in d:
                scan_af(item, found)
        return found[0]

    if not scan_af(eff):
        af_issues += 1

cf_issues = 0
for a in abilities:
    t = a.get("triggerless_text", "") + a.get("full_text", "")
    eff = a.get("effect", {}) or {}
    if "そうした場合" not in t:
        continue
    has = (
        eff.get("followup_action")
        or eff.get("optional_action")
        or eff.get("conditional_action")
        or eff.get("alternative_condition")
        or (eff.get("action") == "conditional_on_optional")
        or (eff.get("action") == "sequential" and eff.get("conditional") is True)
    )
    if not has:
        cf_issues += 1

dur_issues = 0
for a in abilities:
    t = a.get("triggerless_text", "") + a.get("full_text", "")
    eff = a.get("effect", {}) or {}
    if "ライブ終了時まで" not in t or "：" in t:
        continue

    def find_dur(d, found=[False]):
        if isinstance(d, dict):
            if d.get("duration") == "live_end":
                found[0] = True
            for v in d.values():
                find_dur(v, found)
        elif isinstance(d, list):
            for item in d:
                find_dur(item, found)
        return found[0]

    if not find_dur(eff):
        dur_issues += 1

print(
    f"ICON_ALL: {icon_all_issues}, ABILITY_FILTER: {af_issues}, COND_FOLLOWUP: {cf_issues}, DURATION: {dur_issues}"
)
