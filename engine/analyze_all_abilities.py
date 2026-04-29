#!/usr/bin/env python3
"""
Comprehensive Ability Analysis Script
Analyzes all abilities.json to document every type, condition, action, and subsection.
"""

import json
import sys
from collections import defaultdict, Counter
from pathlib import Path

def analyze_abilities():
    """Analyze all abilities and generate comprehensive report"""
    
    # Load abilities
    abilities_file = Path("cards/abilities.json")
    if not abilities_file.exists():
        print("❌ abilities.json not found!")
        return
    
    with open(abilities_file, 'r', encoding='utf-8') as f:
        abilities_data = json.load(f)
    
    print("🔍 COMPREHENSIVE ABILITY ANALYSIS")
    print("=" * 60)
    
    # Track all types and their properties
    action_types = defaultdict(set)
    cost_types = defaultdict(set)
    condition_types = defaultdict(set)
    effect_types = defaultdict(set)
    trigger_types = defaultdict(set)
    duration_types = defaultdict(set)
    
    # Track all fields and their values
    all_fields = defaultdict(set)
    field_values = defaultdict(set)
    
    # Get actual abilities from the data structure
    if 'unique_abilities' in abilities_data:
        abilities_list = abilities_data['unique_abilities']
    else:
        abilities_list = abilities_data
    
    total_abilities = len(abilities_list)
    print(f"📊 Total Abilities: {total_abilities}")
    print()
    
    for i, ability in enumerate(abilities_list, 1):
        card_no = ability.get('card_count', 'unknown')
        print(f"📋 [{i:3d}] {card_no}")
        
        # Analyze triggers
        if 'triggers' in ability:
            triggers = ability['triggers']
            if isinstance(triggers, str):
                trigger_types[triggers].add(card_no)
                print(f"   🎯 Trigger: {triggers}")
            elif isinstance(triggers, list):
                for trigger in triggers:
                    trigger_types[trigger].add(card_no)
                    print(f"   🎯 Trigger: {trigger}")
        
        # Analyze cost
        if 'cost' in ability and ability['cost']:
            cost = ability['cost']
            if 'cost_type' in cost:
                cost_type = cost['cost_type']
                cost_types[cost_type].add(card_no)
                print(f"   💰 Cost Type: {cost_type}")
            
            # Analyze cost fields
            for field, value in cost.items():
                if field != 'cost_type':
                    all_fields[f"cost.{field}"].add(str(value))
                    if isinstance(value, str) and value not in field_values[f"cost.{field}"]:
                        field_values[f"cost.{field}"].add(value)
        
        # Analyze effect
        if 'effect' in ability and ability['effect']:
            effect = ability['effect']
            
            # Main action type
            if 'action' in effect:
                action = effect['action']
                action_types[action].add(card_no)
                print(f"   ⚡ Action: {action}")
            
            # Analyze effect fields
            for field, value in effect.items():
                if field != 'action':
                    all_fields[f"effect.{field}"].add(str(value))
                    if isinstance(value, str) and value not in field_values[f"effect.{field}"]:
                        field_values[f"effect.{field}"].add(value)
                    elif isinstance(value, list):
                        for item in value:
                            field_values[f"effect.{field}"].add(str(item))
            
            # Analyze nested effects (actions, look_action, select_action, etc.)
            analyze_nested_effects(effect, action_types, effect_types, all_fields, field_values, "   ")
            
            # Analyze conditions
            if 'condition' in effect:
                condition = effect['condition']
                if 'type' in condition:
                    cond_type = condition['type']
                    condition_types[cond_type].add(card_no)
                    print(f"   🔍 Condition Type: {cond_type}")
                
                # Analyze condition fields
                for field, value in condition.items():
                    if field != 'type':
                        all_fields[f"condition.{field}"].add(str(value))
                        if isinstance(value, str) and value not in field_values[f"condition.{field}"]:
                            field_values[f"condition.{field}"].add(value)
                    elif isinstance(value, list):
                        for item in value:
                            field_values[f"condition.{field}"].add(str(item))
            
            # Analyze duration
            if 'duration' in effect:
                duration = effect['duration']
                duration_types[duration].add(card_no)
                print(f"   ⏰ Duration: {duration}")
        
        print()
    
    # Generate comprehensive reports
    print_section_report("🎯 TRIGGER TYPES", trigger_types)
    print_section_report("💰 COST TYPES", cost_types)
    print_section_report("⚡ ACTION TYPES", action_types)
    print_section_report("🔍 CONDITION TYPES", condition_types)
    print_section_report("⏰ DURATION TYPES", duration_types)
    
    print_field_report("📋 ALL FIELDS", all_fields)
    print_value_report("💎 FIELD VALUES", field_values)
    
    # Generate summary statistics
    print_summary_statistics(trigger_types, cost_types, action_types, condition_types, duration_types, total_abilities)

def analyze_nested_effects(effect, action_types, effect_types, all_fields, field_values, indent=""):
    """Analyze nested effect structures"""
    
    # Check for nested structures
    nested_keys = ['actions', 'look_action', 'select_action', 'primary_effect', 'alternative_effect', 
                   'followup_action', 'optional_action', 'conditional_action', 'gained_ability']
    
    for key in nested_keys:
        if key in effect:
            nested = effect[key]
            if isinstance(nested, dict):
                if 'action' in nested:
                    action = nested['action']
                    action_types[action].add(f"nested.{key}")
                    print(f"{indent}   📦 Nested {key}: {action}")
                
                # Analyze nested fields
                for field, value in nested.items():
                    if field != 'action':
                        all_fields[f"{key}.{field}"].add(str(value))
                        if isinstance(value, str) and value not in field_values[f"{key}.{field}"]:
                            field_values[f"{key}.{field}"].add(value)
            elif isinstance(nested, list):
                for i, item in enumerate(nested):
                    if isinstance(item, dict) and 'action' in item:
                        action = item['action']
                        action_types[action].add(f"nested.{key}[{i}]")
                        print(f"{indent}   📦 Nested {key}[{i}]: {action}")

def print_section_report(title, data_dict):
    """Print a section report with counts"""
    print(f"\n{title}")
    print("=" * len(title))
    
    sorted_items = sorted(data_dict.items(), key=lambda x: len(x[1]), reverse=True)
    for item, cards in sorted_items:
        count = len(cards)
        print(f"  {item}: {count} cards")
    
    print(f"  📊 Total types: {len(data_dict)}")

def print_field_report(title, fields_dict):
    """Print all discovered fields"""
    print(f"\n{title}")
    print("=" * len(title))
    
    sorted_fields = sorted(fields_dict.keys())
    for field in sorted_fields:
        values = fields_dict[field]
        print(f"  {field}: {len(values)} unique values")
    
    print(f"  📊 Total fields: {len(fields_dict)}")

def print_value_report(title, values_dict):
    """Print field value statistics"""
    print(f"\n{title} - TOP VALUES")
    print("=" * (len(title) + 11))
    
    # Show top 10 values for each field with many values
    sorted_fields = sorted(values_dict.items(), key=lambda x: len(x[1]), reverse=True)[:10]
    for field, values in sorted_fields:
        if len(values) > 1:  # Only show fields with multiple values
            print(f"  {field}: {len(values)} values")
            # Show sample values
            sample_values = sorted(list(values))[:5]
            print(f"    Samples: {', '.join(str(v) for v in sample_values)}{'...' if len(values) > 5 else ''}")

def print_summary_statistics(trigger_types, cost_types, action_types, condition_types, duration_types, total_abilities):
    """Print summary statistics"""
    print(f"\n📈 SUMMARY STATISTICS")
    print("=" * 20)
    
    print(f"  📊 Total Abilities Analyzed: {total_abilities}")
    print(f"  🎯 Trigger Types: {len(trigger_types)}")
    print(f"  💰 Cost Types: {len(cost_types)}")
    print(f"  ⚡ Action Types: {len(action_types)}")
    print(f"  🔍 Condition Types: {len(condition_types)}")
    print(f"  ⏰ Duration Types: {len(duration_types)}")
    
    # Calculate coverage
    abilities_with_triggers = sum(len(cards) for cards in trigger_types.values())
    abilities_with_costs = sum(len(cards) for cards in cost_types.values())
    abilities_with_effects = sum(len(cards) for cards in action_types.values())
    abilities_with_conditions = sum(len(cards) for cards in condition_types.values())
    abilities_with_duration = sum(len(cards) for cards in duration_types.values())
    
    print(f"\n📈 COVERAGE ANALYSIS")
    print(f"  🎯 Abilities with Triggers: {abilities_with_triggers}/{total_abilities} ({abilities_with_triggers*100/total_abilities:.1f}%)")
    print(f"  💰 Abilities with Costs: {abilities_with_costs}/{total_abilities} ({abilities_with_costs*100/total_abilities:.1f}%)")
    print(f"  ⚡ Abilities with Actions: {abilities_with_effects}/{total_abilities} ({abilities_with_effects*100/total_abilities:.1f}%)")
    print(f"  🔍 Abilities with Conditions: {abilities_with_conditions}/{total_abilities} ({abilities_with_conditions*100/total_abilities:.1f}%)")
    print(f"  ⏰ Abilities with Duration: {abilities_with_duration}/{total_abilities} ({abilities_with_duration*100/total_abilities:.1f}%)")

if __name__ == "__main__":
    analyze_abilities()
