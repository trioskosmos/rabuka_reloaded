import json
with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']

print("TOP 10 MOST COMMON ABILITIES (by card_count)")
sorted_by_count = sorted(abilities, key=lambda x: x['card_count'], reverse=True)
for i, a in enumerate(sorted_by_count[:10], 1):
    print(f'{i}. cards={a["card_count"]} trigger={a["triggers"]}')
    print(f'   full: {a["full_text"]}')
    cost = a.get("cost")
    if isinstance(cost, dict):
        print(f'   cost_type: {cost.get("type")}')
    else:
        print(f'   cost_type: none/null')
    eff = a["effect"]
    if isinstance(eff, dict):
        print(f'   eff_action: {eff["action"]}')
    elif isinstance(eff, list):
        print(f'   eff_actions: {[e["action"] for e in eff]}')
    print()

print("=== BREAKDOWN OF 起動 TRIGGER ===")
kido = [a for a in abilities if a.get("triggers") == "\u8d77\u52d5"]
print(f"Total 起動: {len(kido)}")
for a in kido[:5]:
    print(f'  [{a["triggers"]}] {a["triggerless_text"][:100]}')
print()

print("=== SEQUENTIAL EFFECT EXAMPLES ===")
seq = [a for a in abilities if isinstance(a.get("effect"), dict) and a["effect"].get("action") == "sequential"]
print(f"Total sequential: {len(seq)}")
for a in seq[:5]:
    print(f'  [{a["triggers"]}] {a["triggerless_text"][:120]}')
print()

print("=== GAIN_RESOURCE EXAMPLES ===")
gr = [a for a in abilities if isinstance(a.get("effect"), dict) and a["effect"].get("action") == "gain_resource"]
print(f"Total gain_resource: {len(gr)}")
for a in gr[:5]:
    rt = a["effect"].get("resource_type", "?")
    print(f'  [{a["triggers"]}] rt={rt}: {a["triggerless_text"][:100]}')
print()

print("=== LOOK_AND_SELECT EXAMPLES ===")
ls = [a for a in abilities if isinstance(a.get("effect"), dict) and a["effect"].get("action") == "look_and_select"]
print(f"Total look_and_select: {len(ls)}")
for a in ls[:3]:
    print(f'  [{a["triggers"]}] {a["triggerless_text"][:120]}')
print()

print("=== NO-TRIGGER & SPECIAL CASES ===")
for a in abilities:
    t = a.get("triggers")
    if not t or t == "":
        print(f'  trigger=None: {a["full_text"][:100]}')
# also check is_null
null_abilities = [a for a in abilities if a.get("is_null")]
print(f"is_null count: {len(null_abilities)}")
