#!/usr/bin/env python3
"""
Field-level usage analyzer: for each effect action, checks which fields from
abilities.json are actually read by the Rust handler function in effects.rs.

Outputs specific gaps like "field 'source' is never read in execute_appear".

Run: python scripts/field_usage_analyzer.py
"""

import json, re, os, sys
from collections import defaultdict

ABILITIES_PATH = os.path.join(os.path.dirname(__file__), '..', 'cards', 'abilities.json')
EFFECTS_PATH = os.path.join(os.path.dirname(__file__), '..', 'engine', 'src', 'ability', 'effects.rs')
COST_PATH = os.path.join(os.path.dirname(__file__), '..', 'engine', 'src', 'ability', 'cost.rs')
CONDITION_PATH = os.path.join(os.path.dirname(__file__), '..', 'engine', 'src', 'ability', 'condition.rs')

def load_json():
    with open(ABILITIES_PATH, encoding='utf-8') as f:
        return json.load(f)

def collect_actions(data):
    """Collect every action type and all field names used with it."""
    action_fields = defaultdict(set)
    action_examples = defaultdict(list)

    def walk_effect(eff, action_name):
        if not isinstance(eff, dict):
            return
        for k, v in eff.items():
            if k == "action":
                continue
            if k in ("actions", "options", "conditions"):
                continue
            if isinstance(v, (dict, list)):
                action_fields[action_name].add(f"{k}(obj)")
            else:
                action_fields[action_name].add(k)
        # sub-effects
        for sub in ("look_action", "select_action", "primary_effect", "alternative_effect",
                     "followup_action", "optional_action", "conditional_action", "opponent_action",
                     "trigger_condition", "alternative_condition"):
            if sub in eff and isinstance(eff[sub], dict):
                sub_a = eff[sub].get("action", action_name)
                walk_effect(eff[sub], sub_a)
        if "actions" in eff and isinstance(eff["actions"], list):
            for sa in eff["actions"]:
                if isinstance(sa, dict):
                    walk_effect(sa, sa.get("action", action_name))
        if "options" in eff and isinstance(eff["options"], list):
            for opt in eff["options"]:
                if isinstance(opt, dict):
                    walk_effect(opt, opt.get("action", action_name))

    for ability in data.get("unique_abilities", []):
        eff = ability.get("effect")
        if isinstance(eff, dict):
            act = eff.get("action", "")
            if act:
                action_fields[act]
                walk_effect(eff, act)
                if len(action_examples[act]) < 2:
                    action_examples[act].append(ability.get("cards", [None])[0])
    return action_fields, action_examples

def parse_handler_lines():
    """Return dict of handler_name -> set of referenced field names."""
    if not os.path.exists(EFFECTS_PATH):
        print(f"ERROR: {EFFECTS_PATH} not found")
        sys.exit(1)
    with open(EFFECTS_PATH, encoding='utf-8') as f:
        content = f.read()

    # Find all execute_ functions
    handlers = {}
    fn_pattern = re.compile(r'(?:pub\s+)?fn\s+(execute_\w+)\s*\([^)]*\)\s*->\s*Result')
    for m in fn_pattern.finditer(content):
        fn_name = m.group(1)
        # Get the function body
        start = m.start()
        brace_count = 0
        in_body = False
        body_start = 0
        for i in range(m.end(), len(content)):
            if content[i] == '{':
                brace_count += 1
                if not in_body:
                    body_start = i
                    in_body = True
            elif content[i] == '}':
                brace_count -= 1
                if in_body and brace_count == 0:
                    body = content[body_start:i+1]
                    # Find all effect.field references
                    fields = set()
                    # effect.field_name or effect."field_name"
                    field_refs = re.findall(r'(?:effect\.)(\w+)', body)
                    fields.update(field_refs)
                    # also effect.source_or(, effect.count_or(, etc.
                    # and cost.xxx in cost.rs handler
                    handlers[fn_name] = fields
                    break
    return handlers

def parse_dispatch_lines():
    """Map EffectAction::Xxx -> the handler function name called."""
    if not os.path.exists(EFFECTS_PATH):
        return {}
    with open(EFFECTS_PATH, encoding='utf-8') as f:
        content = f.read()

    dispatch_map = {}
    # Pattern: EffectAction::Foo => self.execute_foo(...)
    pattern = re.compile(r'EffectAction::(\w+)\s*=>\s*self\.(execute_\w+)\(')
    for m in pattern.finditer(content):
        dispatch_map[m.group(1)] = m.group(2)
    return dispatch_map

def compute_action_to_handler(dispatch_map):
    """Map JSON action names to handler function names via EffectAction enum."""
    # The EffectAction enum uses PascalCase versions of the action names
    action_to_handler = {}
    for json_action, handler_fn in dispatch_map.items():
        action_to_handler[json_action] = handler_fn
    return action_to_handler

def main():
    data = load_json()
    action_fields, action_examples = collect_actions(data)
    handlers = parse_handler_lines()
    dispatch_map = parse_dispatch_lines()

    print("=" * 100)
    print("FIELD USAGE GAP ANALYSIS")
    print(f"Unique actions in JSON: {len(action_fields)}")
    print(f"Handler functions found: {len(handlers)}")
    print("=" * 100)

    # Convert dispatch_map keys to lowercase for JSON matching
    # EffectAction::Appear -> handler: execute_appear, JSON action: "appear"
    def action_to_lower(name):
        return re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower()
    action_map = {}
    for da_name, handler_fn in dispatch_map.items():
        json_name = action_to_lower(da_name)
        action_map[json_name] = handler_fn

    # Build reverse map: handler_name -> list of action types that use it
    handler_to_actions = defaultdict(list)
    for json_act, handler_fn in action_map.items():
        handler_to_actions[handler_fn].append(json_act)

    # Debug: what handlers did we map?
    for h in sorted(handler_to_actions):
        print(f"  [DEBUG] {h} <- {handler_to_actions[h]}")

    total_gaps = 0
    for handler_fn, json_actions in sorted(handler_to_actions.items()):
        handler_uses = handlers.get(handler_fn, set())
        for json_act in json_actions:
            fields_needed = action_fields.get(json_act, set())
            if not fields_needed:
                continue
            # Fields that JSON says are needed but handler never reads
            missed = set()
            for f in fields_needed:
                fname = f.replace("(obj)", "")
                if fname not in handler_uses:
                    missed.add(f)
            if missed:
                example = action_examples.get(json_act, ["?"])[0] or ""
                example_clean = example.encode('ascii', 'replace').decode()
                print(f"\n[GAP] {json_act} ({handler_fn})")
                print(f"   Example card: {example_clean}")
                print(f"   Fields in JSON but NOT read by handler:")
                for m in sorted(missed):
                    total_gaps += 1
                    print(f"     - {m}")
                print(f"   Handler reads: {sorted(handler_uses)}")
            else:
                print(f"\n[OK] {json_act} ({handler_fn}) -- all {len(fields_needed)} fields used")

    # Also check cost handler
    print("\n" + "=" * 100)
    print("COST HANDLER ANALYSIS")
    print("=" * 100)
    if os.path.exists(COST_PATH):
        with open(COST_PATH, encoding='utf-8') as f:
            cost_content = f.read()
        # Find cost type handlers in cost.rs
        cost_types_json = defaultdict(set)
        for ability in data.get("unique_abilities", []):
            cost = ability.get("cost")
            if isinstance(cost, dict):
                ct = cost.get("type", "")
                if ct:
                    for k in cost:
                        if k != "type":
                            cost_types_json[ct].add(k)

        for ct, fields in sorted(cost_types_json.items()):
            # Check if this cost type is handled
            if f'Some("{ct}")' in cost_content or f'"{ct}"' in cost_content:
                print(f"  [OK] {ct} -- handled in cost.rs")
            else:
                print(f"  [MISSING] {ct} -- NOT handled in cost.rs (fields: {fields})")
                total_gaps += 1

    print(f"\n{'=' * 100}")
    if total_gaps:
        print(f"[SUMMARY] {total_gaps} gap(s) found. Fix the handlers above to read all fields.")
    else:
        print("[SUMMARY] No gaps! All JSON fields are read by their handlers.")

if __name__ == '__main__':
    main()
