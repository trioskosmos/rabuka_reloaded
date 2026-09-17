# -*- coding: utf-8 -*-
"""Count how often baton-touch plays occur and whether v7's [BATON] term can ever fire."""
import json

baton_plays = 0
member_plays = 0
main_decisions = 0
games_with_baton = set()
games = set()
for line in open(r"C:\Users\trios\AppData\Local\Temp\kilo\audit-v7-200.jsonl", encoding="utf-8"):
    try:
        rec = json.loads(line)
    except Exception:
        continue
    if rec.get("event") != "decision" or rec.get("phase") != "Main":
        continue
    ch = rec.get("chosen", {})
    if ch.get("action_type") != "PlayMemberToStage":
        continue
    games.add(rec.get("game"))
    member_plays += 1
    params = ch.get("parameters") or {}
    areas = params.get("available_areas") or []
    area = params.get("stage_area")
    is_bt = any(a.get("area") == area and a.get("is_baton_touch") for a in areas)
    ubt = params.get("use_baton_touch")
    if is_bt or ubt is True:
        baton_plays += 1
        games_with_baton.add(rec.get("game"))
    if ubt is True:
        print("use_baton_touch=True found: game", rec.get("game"))

print("games:", len(games))
print("member plays:", member_plays)
print("baton plays (per available_areas.is_baton_touch):", baton_plays)
print("games containing >=1 baton:", len(games_with_baton))
