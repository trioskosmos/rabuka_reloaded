import json
from collections import Counter

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']
total = len(abilities)

def safe_str(s):
    return str(s) if s is not None else '?'

def get_template_no_trigger(a):
    """Same as before but without trigger prefix."""
    text = a["triggerless_text"]
    
    if "\uff1a" in text:
        parts = ["COLON"]
        cost = a.get("cost")
        if isinstance(cost, dict):
            parts.append("COST:" + cost.get("type", "?"))
        elif isinstance(cost, list):
            parts.append("COST:[" + "+".join(c.get("type", "?") for c in cost) + "]")
        else:
            parts.append("COST:?")
        
        eff = a.get("effect")
        if isinstance(eff, dict):
            ea = eff.get("action", "?")
            if ea == "move_cards":
                parts.append("EFF:" + ea + "(" + safe_str(eff.get("source")) + "->" + safe_str(eff.get("destination")) + ")")
            elif ea == "gain_resource":
                parts.append("EFF:" + ea + "(" + safe_str(eff.get("resource_type")) + ")")
            else:
                parts.append("EFF:" + ea)
        elif isinstance(eff, list):
            actions = [e.get("action", "?") for e in eff]
            parts.append("EFF:[" + "+".join(actions) + "]")
        else:
            parts.append("EFF:?")
    else:
        parts = ["NOCOLON"]
        structs = []
        if "\u5834\u5408" in text: structs.append("BAAI")
        if "\u3068\u304d" in text: structs.append("TOKI")
        if "\u306a\u3089" in text: structs.append("NARA")
        if "\u305d\u306e\u5f8c" in text: structs.append("SONOGO")
        if "\u3055\u3089\u306b" in text: structs.append("SARANI")
        if "\u304b\u304e\u308a" in text: structs.append("KAGIRI")
        if "\u306b\u3064\u304d" in text: structs.append("NITSUKI")
        if "\u305d\u306e\u4e2d\u304b\u3089" in text: structs.append("SONONAKA")
        if "\u4ee5\u4e0b\u304b\u30891\u3064\u3092\u9078\u3076" in text or "\u4ee5\u4e0b\u304b\u3089\u3072\u3068\u3064" in text:
            structs.append("CHOICE")
        if structs:
            parts.append("STRUCT:" + "+".join(structs[:3]))
        
        eff = a.get("effect")
        if isinstance(eff, dict):
            ea = eff.get("action", "?")
            if ea == "move_cards":
                parts.append("EFF:" + ea + "(" + safe_str(eff.get("source")) + "->" + safe_str(eff.get("destination")) + ")")
            elif ea == "gain_resource":
                parts.append("EFF:" + ea + "(" + safe_str(eff.get("resource_type")) + ")")
            else:
                parts.append("EFF:" + ea)
        elif isinstance(eff, list):
            actions = [e.get("action", "?") for e in eff]
            parts.append("EFF:[" + "+".join(actions) + "]")
        else:
            parts.append("EFF:?")
    
    return " | ".join(parts)

templates_nt = Counter()
for a in abilities:
    templates_nt[get_template_no_trigger(a)] += 1

print("=== TRIGGER-AGNOSTIC TEMPLATES ===")
print(f"Total: {len(templates_nt)}")
print()

# Top 15
sorted_nt = templates_nt.most_common()
for i, (tmpl, cnt) in enumerate(sorted_nt[:15], 1):
    example = ""
    for a in abilities:
        if get_template_no_trigger(a) == tmpl:
            example = a["triggerless_text"][:110]
            break
    print(f"{i:2d}. [{cnt:3d}] {tmpl}")
    print(f"     eg: {example}")
    print()

print("=== COVERAGE (trigger-agnostic) ===")
total_nt = len(templates_nt)
cum = 0
for i, (tmpl, cnt) in enumerate(sorted_nt, 1):
    cum += cnt
    if i in [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]:
        print(f"  Top {i:3d} templates: {cum:3d}/{total} = {cum/total*100:5.1f}%")

print()
t80 = next(i for i, (_, c) in enumerate(sorted_nt, 1) if sum(x[1] for x in sorted_nt[:i]) >= total*0.8)
t90 = next(i for i, (_, c) in enumerate(sorted_nt, 1) if sum(x[1] for x in sorted_nt[:i]) >= total*0.9)
print(f"80% coverage: ~{t80} templates")
print(f"90% coverage: ~{t90} templates")
print(f"Total distinct: {total_nt}")
print(f"Singletons: {sum(1 for _,c in sorted_nt if c==1)}")

# Show the collapse ratio
print()
print("=== TEMPLATE REDUCTION WHEN IGNORING TRIGGER ===")
print(f"  With trigger: 219 templates")
print(f"  Without trigger: {total_nt} templates")
print(f"  Reduction: {219 - total_nt} fewer templates ({(219-total_nt)/219*100:.1f}% reduction)")
