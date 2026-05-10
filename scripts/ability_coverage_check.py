#!/usr/bin/env python3
"""
Examines each action and all possible fields each action/condition/cost can have
in cards/abilities.json, checks engine code for implementation coverage,
and identifies gaps. Outputs a detailed report.
"""

import json
import re
import os
import glob as glob_mod
from collections import defaultdict

# ============================================================
# 1. Parse abilities.json for all action types, condition types, cost types, fields
# ============================================================

ABILITIES_PATH = os.path.join(os.path.dirname(__file__), '..', 'cards', 'abilities.json')
ENGINE_SRC_DIR = os.path.join(os.path.dirname(__file__), '..', 'engine', 'src')

def load_abilities():
    with open(ABILITIES_PATH, 'r', encoding='utf-8') as f:
        return json.load(f)

def collect_all_actions(data):
    """Walk all abilities and collect every action name + fields seen."""
    actions = defaultdict(lambda: defaultdict(set))  # action_type -> field_name -> set of values
    conditions = defaultdict(lambda: defaultdict(set))  # condition_type -> field_name -> set of values
    cost_types = defaultdict(lambda: defaultdict(set))  # cost_type -> field_name -> set of values
    
    def collect_effect_fields(eff, prefix="effect"):
        if not eff or not isinstance(eff, dict):
            return
        action = eff.get("action", "")
        if action:
            for k, v in eff.items():
                if k == "action":
                    continue
                val_str = str(v) if not isinstance(v, (list, dict)) else str(type(v).__name__)
                actions[action][k].add(val_str)
        
        # Recurse into compound sub-actions
        for sub_field in ["look_action", "select_action", "primary_effect", "alternative_effect",
                          "followup_action", "optional_action", "conditional_action", "opponent_action"]:
            if sub_field in eff and eff[sub_field]:
                collect_effect_fields(eff[sub_field] if isinstance(eff[sub_field], dict) else None, f"{prefix}.{sub_field}")
        
        if "actions" in eff and isinstance(eff["actions"], list):
            for i, sub_eff in enumerate(eff["actions"]):
                collect_effect_fields(sub_eff, f"{prefix}.actions[{i}]")
        
        if "options" in eff and isinstance(eff["options"], list):
            for i, opt in enumerate(eff["options"]):
                if isinstance(opt, dict):
                    collect_effect_fields(opt, f"{prefix}.options[{i}]")
        
        # Collect condition fields
        if "condition" in eff and isinstance(eff["condition"], dict):
            collect_condition_fields(eff["condition"], f"{prefix}.condition")
        if "result_condition" in eff and isinstance(eff["result_condition"], dict):
            collect_condition_fields(eff["result_condition"], f"{prefix}.result_condition")
        if "activation_condition_parsed" in eff and isinstance(eff["activation_condition_parsed"], dict):
            collect_condition_fields(eff["activation_condition_parsed"], f"{prefix}.activation_condition_parsed")
        if "alternative_condition" in eff and isinstance(eff["alternative_condition"], dict):
            collect_condition_fields(eff["alternative_condition"], f"{prefix}.alternative_condition")
        if "trigger_condition" in eff and isinstance(eff["trigger_condition"], dict):
            collect_condition_fields(eff["trigger_condition"], f"{prefix}.trigger_condition")
    
    def collect_condition_fields(cond, prefix="condition"):
        if not cond or not isinstance(cond, dict):
            return
        ct = cond.get("type", "")
        if ct:
            for k, v in cond.items():
                if k == "type":
                    continue
                val_str = str(v) if not isinstance(v, (list, dict)) else str(type(v).__name__)
                conditions[ct][k].add(val_str)
        # Recurse sub-conditions
        if "conditions" in cond and isinstance(cond["conditions"], list):
            for i, sub_cond in enumerate(cond["conditions"]):
                collect_condition_fields(sub_cond, f"{prefix}.conditions[{i}]")
        if "condition" in cond and isinstance(cond["condition"], dict):
            collect_condition_fields(cond["condition"], f"{prefix}.condition")
        if "cause" in cond and isinstance(cond["cause"], dict):
            collect_condition_fields(cond["cause"], f"{prefix}.cause")
        if "effect" in cond and isinstance(cond["effect"], dict):
            collect_effect_fields(cond["effect"], f"{prefix}.effect")
    
    for ability in data.get("unique_abilities", []):
        # Cost
        cost = ability.get("cost")
        if isinstance(cost, dict):
            ct = cost.get("type", "")
            if ct:
                for k, v in cost.items():
                    if k == "type":
                        continue
                    val_str = str(v) if not isinstance(v, (list, dict)) else str(type(v).__name__)
                    cost_types[ct][k].add(val_str)
                # Recurse sequential costs
                if "costs" in cost and isinstance(cost["costs"], list):
                    for sub_cost in cost["costs"]:
                        sub_ct = sub_cost.get("type", "")
                        if sub_ct:
                            for k, v in sub_cost.items():
                                if k == "type":
                                    continue
                                val_str = str(v) if not isinstance(v, (list, dict)) else str(type(v).__name__)
                                cost_types[sub_ct][k].add(val_str)
                if "options" in cost and isinstance(cost["options"], list):
                    for opt in cost["options"]:
                        opt_ct = opt.get("type", "")
                        if opt_ct:
                            for k, v in opt.items():
                                if k == "type":
                                    continue
                                val_str = str(v) if not isinstance(v, (list, dict)) else str(type(v).__name__)
                                cost_types[opt_ct][k].add(val_str)
        
        # Effect
        effect = ability.get("effect")
        if isinstance(effect, dict):
            collect_effect_fields(effect)
    
    return actions, conditions, cost_types


# ============================================================
# 2. Parse Rust engine code for known types/fields
# ============================================================

# Known effect actions from EffectAction enum in effects.rs
ENGINE_EFFECT_ACTIONS = {
    "sequential", "conditional_alternative", "look_and_select",
    "draw", "draw_card", "draw_until_count", "discard_card", "move_cards",
    "gain_resource", "change_state", "modify_score", "modify_required_hearts",
    "set_cost", "set_blade_type", "set_heart_type", "activate_ability",
    "invalidate_ability", "gain_ability", "play_baton_touch", "reveal",
    "select", "look_at", "modify_required_hearts_global", "modify_yell_count",
    "place_energy_under_member", "activation_cost", "position_change", "formation_change", "appear",
    "choice", "pay_energy", "set_card_identity", "repeat_procedure",
    "discard_until_count", "restriction", "re_yell", "activation_restriction",
    "choose_required_hearts", "modify_limit", "set_blade_count", "do_nothing",
    "set_required_hearts", "set_score", "specify_heart_color",
    "modify_required_hearts_success", "set_cost_to_use", "all_blade_timing",
    "set_card_identity_all_regions", "shuffle", "reveal_per_group",
    "conditional_on_result", "conditional_on_optional", "modify_cost",
    "reveal_until_live_card", "custom",
}

# Known condition types from condition.rs
ENGINE_CONDITION_TYPES = {
    "compound", "comparison_condition", "location_condition",
    "card_count_condition", "group_condition", "position_condition",
    "appearance_condition", "temporal_condition", "state_condition",
    "energy_state_condition", "movement_condition",
    "ability_negation_condition", "or_condition", "any_of_condition",
    "score_threshold_condition", "choice_condition",
    "position_change_condition", "state_change_condition",
    "opponent_choice_condition", "opponent_live_success",
    "complex_condition", "no_excess_heart", "otherwise_condition",
}

# Known cost types from cost.rs
ENGINE_COST_TYPES = {
    "sequential_cost", "choice_condition", "move_cards",
    "change_state", "pay_energy", "reveal", "place_energy_under_member",
    "energy_condition",
}

# Known effect fields from AbilityEffect struct in card.rs
KNOWN_EFFECT_FIELDS = {
    "text", "action", "source", "destination", "count", "target_count",
    "card_type", "target", "duration", "resource", "position",
    "state_change", "optional", "max", "effect_constraint",
    "resource_icon_count", "ability_gain", "quoted_text", "per_unit",
    "condition", "look_action", "select_action", "actions",
    "primary_effect", "alternative_effect", "result_condition",
    "followup_action", "optional_action", "conditional_action",
    "conditional_negation", "operation", "value", "heart_colors",
    "blade_type", "energy_count", "target_member", "choice_options",
    "options", "per_unit_count", "per_unit_type", "repeat_limit",
    "is_further", "restriction_type", "restricted_destination",
    "dynamic_count", "placement_order", "cost_limit",
    "cost_limit_operator", "any_number", "distinct", "name_constraint",
    "name_constraint_source", "activation_condition_parsed",
    "ability_text", "use_limit", "triggers", "self_cost",
    "exclude_self", "exclude_selected", "effect_type", "choice",
    "timing", "treat_as", "replaces_event", "choice_based",
    "identities", "action_by", "opponent_action", "lose_blade_hearts",
    "conditional", "choice_type", "heart_type", "or_card_types",
    "activation_position", "source_position", "exclude_position",
    "all_regions", "character_effects", "group_names",
    "heart_selection", "location", "multiple_targets", "question",
    "answers", "choice_maker", "state", "target_trigger",
    "timing_condition", "self_target", "trigger_type", "sign",
    "phase", "all", "original_value", "group_reference",
    "trigger_condition", "alternative_condition",
    "parenthetical", "characters", "source_card", "energy",
    "heart_color",  # handled by serde(alias) on heart_type
    "max_repeats",  # handled by serde(alias) on repeat_limit
}

# Known condition fields from Condition struct in card.rs
KNOWN_CONDITION_FIELDS = {
    "text", "type", "location", "locations", "count", "operator",
    "card_type", "target", "group_names", "characters", "state",
    "position", "temporal_scope", "distinct", "exclude_self",
    "any_of", "cost_limit", "negation", "baton_touch_trigger",
    "baton_touch_source", "movement_state", "energy_state",
    "comparison_target", "movement", "temporal", "phase",
    "comparison_type", "appearance", "conditions", "options",
    "condition", "card_property", "all_areas", "no_excess_heart",
    "resource_type", "all", "unit", "values", "cause", "effect",
    "from_state", "heart_type", "to_state", "aggregate",
    "heart_colors", "ability_negation", "original_value",
    "all_members", "scope",
}

# Known cost fields from AbilityCost struct in card.rs
KNOWN_COST_FIELDS = {
    "text", "type", "source", "destination", "count", "card_type",
    "target", "optional", "energy", "state_change", "position",
    "options", "self_cost", "exclude_self", "same_unit_name",
    "costs", "cost_limit", "cost_limit_operator", "characters",
    "group_names", "target_member", "placement_order", "shuffle",
}


def scan_engine_files():
    """Read all engine Rust files and return their content for searching."""
    engine_files = {}
    for root, dirs, files in os.walk(ENGINE_SRC_DIR):
        for f in files:
            if f.endswith('.rs'):
                path = os.path.join(root, f)
                with open(path, 'r', encoding='utf-8') as fh:
                    engine_files[os.path.relpath(path, ENGINE_SRC_DIR)] = fh.read()
    return engine_files

def check_action_coverage(action_name, fields, engine_files, effects_actions):
    """Check if an action has engine coverage for all its fields."""
    issues = []
    
    # Check if action is known
    if action_name not in effects_actions:
        issues.append(f"ACTION '{action_name}' NOT FOUND in engine EffectAction enum")
        return issues
    
    # Check each field used with this action
    for field_name in fields:
        if field_name in KNOWN_EFFECT_FIELDS:
            continue  # Field struct exists
        issues.append(f"  Field '{field_name}' NOT in KnownEffectFields struct")
    
    return issues


# ============================================================
# 3. Analyzer
# ============================================================

def analyze():
    data = load_abilities()
    actions, conditions, cost_types = collect_all_actions(data)
    engine_files = scan_engine_files()
    
    report_lines = []
    report_lines.append("=" * 100)
    report_lines.append("ABILITY COVERAGE REPORT")
    report_lines.append(f"Total unique abilities: {len(data['unique_abilities'])}")
    report_lines.append(f"Total cards with abilities: {data['statistics']['cards_with_abilities']}")
    report_lines.append(f"Engine Rust files scanned: {len(engine_files)}")
    report_lines.append("=" * 100)
    
    # ---- SECTION 1: Effect Actions ----
    report_lines.append("\n\n## SECTION 1: EFFECT ACTIONS\n")
    
    json_actions = set(actions.keys())
    engine_actions = set(ENGINE_EFFECT_ACTIONS)
    
    missing_in_engine = json_actions - engine_actions
    extra_in_engine = engine_actions - json_actions
    covered = json_actions & engine_actions
    
    report_lines.append(f"Actions in JSON: {len(json_actions)}")
    report_lines.append(f"Actions in Engine: {len(engine_actions)}")
    report_lines.append(f"Covered: {len(covered)}")
    report_lines.append(f"Missing in Engine: {len(missing_in_engine)}")
    report_lines.append(f"Extra in Engine (not in JSON): {len(extra_in_engine)}")
    
    if missing_in_engine:
        report_lines.append("\n--- Missing Actions (in JSON but not Engine) ---")
        for a in sorted(missing_in_engine):
            report_lines.append(f"  ❌ {a}")
            report_lines.append(f"     Fields: {dict(actions[a])}")
    
    if extra_in_engine:
        report_lines.append("\n--- Extra Actions (in Engine but not JSON) ---")
        for a in sorted(extra_in_engine):
            report_lines.append(f"  ⚠️  {a}")
    
    report_lines.append("\n--- Covered Actions Detail ---")
    for a in sorted(covered):
        report_lines.append(f"\n  ✅ {a}")
        fields = actions[a]
        for fname, fvals in sorted(fields.items()):
            if fname not in KNOWN_EFFECT_FIELDS:
                report_lines.append(f"     ⚠️  Field '{fname}' not in struct: values={fvals}")
    
    # ---- SECTION 2: Condition Types ----
    report_lines.append("\n\n## SECTION 2: CONDITION TYPES\n")
    
    json_conditions = set(conditions.keys())
    engine_conditions = set(ENGINE_CONDITION_TYPES)
    
    missing_conds = json_conditions - engine_conditions
    extra_conds = engine_conditions - json_conditions
    
    report_lines.append(f"Conditions in JSON: {len(json_conditions)}")
    report_lines.append(f"Conditions in Engine: {len(engine_conditions)}")
    report_lines.append(f"Covered: {len(json_conditions & engine_conditions)}")
    
    if missing_conds:
        report_lines.append("\n--- Missing Conditions (in JSON but not Engine) ---")
        for c in sorted(missing_conds):
            report_lines.append(f"  ❌ {c}")
            report_lines.append(f"     Fields: {dict(conditions[c])}")
    
    if extra_conds:
        report_lines.append("\n--- Extra Conditions (in Engine but not JSON) ---")
        for c in sorted(extra_conds):
            report_lines.append(f"  ⚠️  {c}")
    
    # Check condition field coverage
    report_lines.append("\n--- Condition Field Coverage by Type ---")
    for ct in sorted(conditions.keys()):
        fields = conditions[ct]
        for fname, fvals in sorted(fields.items()):
            if fname not in KNOWN_CONDITION_FIELDS:
                report_lines.append(f"  ⚠️  {ct} field '{fname}' not in Condition struct: values={fvals}")
    
    # ---- SECTION 3: Cost Types ----
    report_lines.append("\n\n## SECTION 3: COST TYPES\n")
    
    json_costs = set(cost_types.keys())
    engine_costs = set(ENGINE_COST_TYPES)
    
    missing_costs = json_costs - engine_costs
    extra_costs = engine_costs - json_costs
    
    report_lines.append(f"Cost types in JSON: {len(json_costs)}")
    report_lines.append(f"Cost types in Engine: {len(engine_costs)}")
    
    if missing_costs:
        report_lines.append("\n--- Missing Cost Types (in JSON but not Engine) ---")
        for c in sorted(missing_costs):
            report_lines.append(f"  ❌ {c}")
            report_lines.append(f"     Fields: {dict(cost_types[c])}")
    
    if extra_costs:
        report_lines.append("\n--- Extra Cost Types (in Engine but not JSON) ---")
        for c in sorted(extra_costs):
            report_lines.append(f"  ⚠️  {c}")
    
    # Check cost field coverage
    report_lines.append("\n--- Cost Field Coverage by Type ---")
    for ct in sorted(cost_types.keys()):
        fields = cost_types[ct]
        for fname, fvals in sorted(fields.items()):
            if fname not in KNOWN_COST_FIELDS:
                report_lines.append(f"  ⚠️  {ct} field '{fname}' not in AbilityCost struct: values={fvals}")
    
    # ---- SECTION 4: Utility Function Usage ----
    report_lines.append("\n\n## SECTION 4: UTILITY FUNCTION ANALYSIS\n")
    
    util_file = engine_files.get('ability/util.rs', '')
    if util_file:
        util_functions = re.findall(r'pub(?:\(crate\))?\s+fn\s+(\w+)', util_file)
        report_lines.append(f"Utility functions in ability/util.rs: {len(util_functions)}")
        for fn in sorted(util_functions):
            report_lines.append(f"  - {fn}")
    
    # Check how many effect/condition/cost files import from util
    for fname, content in engine_files.items():
        if fname.endswith('.rs') and 'util' in fname:
            continue
        if 'use super::util' in content or 'use crate::ability::util' in content:
            uses = re.findall(r'util::(\w+)', content)
            if uses:
                report_lines.append(f"\n  {fname} uses util: {set(uses)}")
    
    # ---- SECTION 5: Gaps Summary ----
    report_lines.append("\n\n## SECTION 5: GAPS SUMMARY\n")
    
    total_issues = 0
    if missing_in_engine:
        report_lines.append(f"\n❌ MISSING ACTIONS (need engine implementation):")
        for a in sorted(missing_in_engine):
            report_lines.append(f"   - {a}")
            total_issues += 1
    
    if missing_conds:
        report_lines.append(f"\n❌ MISSING CONDITION TYPES:")
        for c in sorted(missing_conds):
            report_lines.append(f"   - {c}")
            total_issues += 1
    
    if missing_costs:
        report_lines.append(f"\n❌ MISSING COST TYPES:")
        for c in sorted(missing_costs):
            report_lines.append(f"   - {c}")
            total_issues += 1
    
    # Check for fields in JSON that are NOT in the Rust struct definitions
    report_lines.append("\n--- Fields in JSON lacking struct definition in Rust ---")
    all_json_effect_fields = set()
    for action_name, fields in actions.items():
        all_json_effect_fields.update(fields.keys())
    missing_effect_fields = all_json_effect_fields - KNOWN_EFFECT_FIELDS - {'action'}
    if missing_effect_fields:
        for f in sorted(missing_effect_fields):
            report_lines.append(f"  ⚠️  Effect field '{f}' missing from AbilityEffect struct")
            total_issues += 1
    
    all_json_cond_fields = set()
    for ct, fields in conditions.items():
        all_json_cond_fields.update(fields.keys())
    missing_cond_fields = all_json_cond_fields - KNOWN_CONDITION_FIELDS - {'type'}
    if missing_cond_fields:
        for f in sorted(missing_cond_fields):
            report_lines.append(f"  ⚠️  Condition field '{f}' missing from Condition struct")
            total_issues += 1
    
    all_json_cost_fields = set()
    for ct, fields in cost_types.items():
        all_json_cost_fields.update(fields.keys())
    missing_cost_fields = all_json_cost_fields - KNOWN_COST_FIELDS - {'type'}
    if missing_cost_fields:
        for f in sorted(missing_cost_fields):
            report_lines.append(f"  ⚠️  Cost field '{f}' missing from AbilityCost struct")
            total_issues += 1
    
    if total_issues == 0:
        report_lines.append("\n✅ No gaps found! All actions, conditions, costs and fields are covered.")
    else:
        report_lines.append(f"\n{total_issues} issue(s) found above.")
    
    report_lines.append("\n" + "=" * 100)
    
    return "\n".join(report_lines)


if __name__ == '__main__':
    report = analyze()
    # Write to file first (handles all unicode)
    output_path = os.path.join(os.path.dirname(__file__), 'ability_coverage_report.txt')
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write(report)
    # Print a summary to stdout (avoid encoding issues with emoji/symbols)
    print(f"Report written to: {output_path}")
    # Show key stats quickly
    lines = report.split('\n')
    for line in lines:
        if 'Missing' in line or 'Covered' in line or 'Total' in line or 'issues' in line:
            print(line.strip())
