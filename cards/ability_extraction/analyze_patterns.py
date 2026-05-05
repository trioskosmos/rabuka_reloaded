"""Analyze parser output for all 30 patterns in abilities.json."""
import json
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parser import parse_effect, parse_cost

with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'abilities.json'), 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

patterns = {
    1: ("no(mi) - activation restrictions", ['\u306e\u307f\u8d77\u52d5', '\u306e\u307f\u767a\u52d5']),
    2: ("mu(kou) - invalidate ability", ['\u7121\u52b9\u306b\u3059\u308b', '\u7121\u52b9\u306b']),
    3: ("tsui(ka) - additional/modify_score", ['\u8ffd\u52a0']),
    4: ("mou ichido - again/re-yell", ['\u3082\u3046\u4e00\u5ea6', '\u3082\u30461\u5ea6']),
    5: ("kawari(ni) - instead", ['\u4ee3\u308f\u308a\u306b']),
    6: ("toshite atsukau - treat as", ['\u3068\u3057\u3066\u6271\u3046']),
    7: ("subete no ryouiki - all zones", ['\u3059\u3079\u3066\u306e\u9818\u57df']),
    8: ("ta-n - turn-specific", ['\u30bf\u30fc\u30f3\u7d42\u4e86', '\u6b21\u306e\u30bf\u30fc\u30f3']),
    9: ("feizu - phase-specific", ['\u30d5\u30a7\u30a4\u30ba']),
    10: ("made - max modifier", ['\u679a\u307e\u3067']),
    11: ("restrictions (oku/appear/move)", ['\u7f6e\u304f\u3053\u3068\u304c\u3067\u304d\u306a\u3044', '\u767b\u5834\u3067\u304d\u306a\u3044', '\u79fb\u52d5\u3067\u304d\u306a\u3044']),
    12: ("cost modification", ['\u30b3\u30b9\u30c8\u306f', '\u30b3\u30b9\u30c8\u304c', '\u6e1b\u308b', '\u5897\u3048\u308b', '\u6e1b\u3089\u3059', '\u5897\u3084\u3059']),
    13: ("equal to", ['\u306b\u7b49\u3057\u3044']),
    14: ("total score", ['\u5408\u8a08\u30b9\u30b3\u30a2']),
    15: ("yell/cheer", ['\u30a8\u30fc\u30eb']),
    16: ("required heart", ['\u5fc5\u8981\u30cf\u30fc\u30c8']),
    17: ("surplus heart", ['\u4f59\u5270\u30cf\u30fc\u30c8']),
    18: ("reveal", ['\u516c\u958b']),
    19: ("select/choice", ['\u9078\u3076', '\u9078\u3093']),
    20: ("baton touch", ['\u30d0\u30c8\u30f3\u30bf\u30c3\u30c1']),
    21: ("make appear", ['\u767b\u5834\u3055\u305b\u308b']),
    22: ("add to hand", ['\u52a0\u3048\u308b']),
    23: ("wait state", ['\u30a6\u30a7\u30a4\u30c8']),
    24: ("active state", ['\u30a2\u30af\u30c6\u30a3\u30d6']),
    25: ("position change", ['\u30dd\u30b8\u30b7\u30e7\u30f3\u30c1\u30a7\u30f3\u30b8', '\u30d5\u30a9\u30fc\u30e1\u30fc\u30b7\u30e7\u30f3\u30c1\u30a7\u30f3\u30b8']),
    26: ("this card - self-targeting", ['\u3053\u306e\u30ab\u30fc\u30c9\u3092']),
    27: ("igai - exclusion", ['\u4ee5\u5916']),
    28: ("sorezore/zutsu - multiple targets", ['\u305d\u308c\u305e\u308c', '\u305a\u3064']),
    29: ("any number", ['\u597d\u304d\u306a\u679a\u6570']),
    30: ("any order", ['\u597d\u304d\u306a\u9806\u756a']),
}

def print_effect(e, indent=0, out=None):
    prefix = "  " * indent
    if isinstance(e, dict):
        for k, v in e.items():
            if k == 'text':
                continue
            if isinstance(v, dict):
                out.write(f"{prefix}  {k}:\n")
                print_effect(v, indent + 2, out)
            elif isinstance(v, list):
                out.write(f"{prefix}  {k}: [{len(v)} items]\n")
                for i, item in enumerate(v[:2]):
                    if isinstance(item, dict):
                        out.write(f"{prefix}    [{i}]:\n")
                        print_effect(item, indent + 3, out)
                    else:
                        out.write(f"{prefix}    [{i}]: {item}\n")
                if len(v) > 2:
                    out.write(f"{prefix}    ... +{len(v)-2} more\n")
            else:
                out.write(f"{prefix}  {k}: {v}\n")
    else:
        out.write(f"{prefix}  {e}\n")

def find_issues(ability, effect):
    issues = []
    if effect.get('action') == 'custom' and not effect.get('actions'):
        issues.append("action=custom (unresolved)")
    ft = ability.get('full_text', '')
    eff_text = effect.get('text', '')
    # Check for specific pattern mismatches
    for kw, check_name in [
        ('\u30a8\u30fc\u30eb\u3092\u884c\u3046', 'yell_act'),
        ('\u7121\u52b9', 'invalidate'),
        ('\u8ffd\u52a0', 'additional'),
        ('\u4ee3\u308f\u308a\u306b', 'instead'),
        ('\u767b\u5834\u3055\u305b\u308b', 'make_appear'),
        ('\u52a0\u3048\u308b', 'add_to_hand'),
        ('\u516c\u958b', 'reveal'),
        ('\u9078\u3076', 'select'),
    ]:
        if kw in ft and kw not in str(effect):
            issues.append(f"{check_name}({kw}) in text but not parsed")
    return issues

with open('pattern_analysis.txt', 'w', encoding='utf-8') as out:
    out.write(f"Total unique abilities: {len(abilities)}\n")
    
    for pnum, (desc, keywords) in patterns.items():
        out.write("\n")
        out.write("=" * 70 + "\n")
        out.write(f"PATTERN {pnum}: {desc}\n")
        out.write("=" * 70 + "\n")
        
        matching = []
        for a in abilities:
            ft = a.get('full_text', '')
            if any(kw in ft for kw in keywords):
                matching.append(a)
        
        out.write(f"Found {len(matching)} matching abilities\n")
        
        for idx, a in enumerate(matching[:5]):
            ft = a.get('full_text', '')
            cc = a.get('card_count', 0)
            effect = a.get('effect')
            if effect is None:
                out.write(f"\n--- [{idx+1}] card_count={cc} (NO EFFECT) ---\n")
                out.write(f"  raw: {ft[:150]}\n")
                continue
            eff_action = effect.get('action', 'MISSING')
            
            out.write(f"\n--- [{idx+1}] card_count={cc} ---\n")
            out.write(f"  raw: {ft[:150]}\n")
            out.write(f"  action: {eff_action}\n")
            
            issues = find_issues(a, effect)
            if issues:
                out.write(f"  PROBLEM: {'; '.join(issues)}\n")
            
            # Show key fields
            key_fields = ['source', 'destination', 'count', 'card_type', 'state_change', 
                         'duration', 'resource', 'target', 'condition', 'self_target',
                         'activation_condition', 'optional', 'max', 'multiple_targets',
                         'placement_order', 'any_number', 'action']
            for k in key_fields:
                if k in effect:
                    v = effect[k]
                    if isinstance(v, dict):
                        v_type = v.get('type', '')
                        out.write(f"  {k}: ({v_type})\n")
                    else:
                        out.write(f"  {k}: {v}\n")
            
            # Check for missing important fields
            if '\u30a8\u30fc\u30eb' in ft and 'yell' not in ft.lower():
                if effect.get('action') not in ('re_yell', 'modify_yell_count', 'move_cards', 'gain_resource', 'look_and_select'):
                    out.write(f"  WARNING: yell in text but action={eff_action}\n")
        
        if len(matching) > 5:
            out.write(f"  (+{len(matching)-5} more)\n")
    
    out.write("\n\nDONE\n")

print("Analysis written to pattern_analysis.txt")
