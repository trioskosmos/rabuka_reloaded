#!/usr/bin/env python3
"""Analyze abilities.json to extract all action types, condition types, and their field values."""

import json
from collections import defaultdict, Counter
from typing import Dict, Set, Any, List
import sys

def extract_fields_recursive(obj: Any, path: str = "", results: Dict = None) -> Dict:
    """Recursively extract all fields and their values from a JSON object."""
    if results is None:
        results = {
            "fields": defaultdict(set),
            "field_values": defaultdict(Counter),
            "field_types": defaultdict(set),
            "action_types": set(),
            "condition_types": set(),
            "cost_types": set(),
        }
    
    if isinstance(obj, dict):
        # Check for action type
        if "action" in obj:
            action = obj["action"]
            results["action_types"].add(action)
            
            # Extract all fields for this action
            for key, value in obj.items():
                field_path = f"{path}.{key}" if path else key
                results["fields"][action].add(key)
                
                # Track value types
                if value is None:
                    results["field_types"][key].add("null")
                elif isinstance(value, bool):
                    results["field_types"][key].add("bool")
                    results["field_values"][key][str(value)] += 1
                elif isinstance(value, int):
                    results["field_types"][key].add("int")
                    results["field_values"][key][str(value)] += 1
                elif isinstance(value, str):
                    results["field_types"][key].add("str")
                    results["field_values"][key][value] += 1
                elif isinstance(value, list):
                    results["field_types"][key].add("list")
                    # Extract list item values
                    if len(value) > 0 and isinstance(value[0], str):
                        for item in value:
                            results["field_values"][f"{key}[]"][item] += 1
                elif isinstance(value, dict):
                    results["field_types"][key].add("dict")
                    # Recurse into nested dict
                    extract_fields_recursive(value, field_path, results)
        
        # Check for condition type
        if "type" in obj and "condition" in str(path).lower():
            cond_type = obj["type"]
            results["condition_types"].add(cond_type)
            for key, value in obj.items():
                results["fields"][f"condition:{cond_type}"].add(key)
        
        # Check for cost type
        if "type" in obj and "cost" in str(path).lower():
            cost_type = obj["type"]
            results["cost_types"].add(cost_type)
            for key, value in obj.items():
                results["fields"][f"cost:{cost_type}"].add(key)
        
        # Recurse into all dict values
        for key, value in obj.items():
            field_path = f"{path}.{key}" if path else key
            extract_fields_recursive(value, field_path, results)
    
    elif isinstance(obj, list):
        for item in obj:
            extract_fields_recursive(item, path, results)
    
    return results

def analyze_abilities(file_path: str):
    """Analyze abilities.json file."""
    with open(file_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    results = extract_fields_recursive(data)
    
    # Print summary
    print("=" * 80)
    print("ABILITY FIELD ANALYSIS")
    print("=" * 80)
    print()
    
    # Action types
    print(f"FOUND {len(results['action_types'])} UNIQUE ACTION TYPES:")
    print("-" * 80)
    for action in sorted(results['action_types']):
        print(f"  - {action}")
    print()
    
    # Condition types
    print(f"FOUND {len(results['condition_types'])} UNIQUE CONDITION TYPES:")
    print("-" * 80)
    for cond in sorted(results['condition_types']):
        print(f"  - {cond}")
    print()
    
    # Cost types
    print(f"FOUND {len(results['cost_types'])} UNIQUE COST TYPES:")
    print("-" * 80)
    for cost in sorted(results['cost_types']):
        print(f"  - {cost}")
    print()
    
    # Field values by field name
    print("FIELD VALUES (sorted by frequency):")
    print("=" * 80)
    for field in sorted(results['field_values'].keys()):
        values = results['field_values'][field]
        print(f"\n{field}:")
        for value, count in values.most_common():
            print(f"  - {value} ({count} occurrences)")
    
    # Fields by action type
    print("\n" + "=" * 80)
    print("FIELDS BY ACTION TYPE:")
    print("=" * 80)
    for action in sorted(results['action_types']):
        fields = results['fields'].get(action, set())
        print(f"\n{action}:")
        for field in sorted(fields):
            print(f"  - {field}")
    
    # Field types
    print("\n" + "=" * 80)
    print("FIELD TYPES:")
    print("=" * 80)
    for field in sorted(results['field_types'].keys()):
        types = results['field_types'][field]
        print(f"{field}: {', '.join(sorted(types))}")
    
    return results

def generate_field_mapping_report(results: Dict, output_file: str):
    """Generate a detailed field mapping report."""
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write("# Ability Field Mapping Report\n\n")
        f.write("This document describes all action types, condition types, and their fields found in abilities.json\n\n")
        
        # Action types
        f.write("## Action Types\n\n")
        for action in sorted(results['action_types']):
            f.write(f"### {action}\n\n")
            fields = results['fields'].get(action, set())
            if fields:
                f.write("**Fields:**\n\n")
                for field in sorted(fields):
                    f.write(f"- `{field}`\n")
                    
                    # Add value examples if available
                    value_key = field
                    if value_key in results['field_values']:
                        values = results['field_values'][value_key]
                        if values:
                            f.write(f"  - Example values: {', '.join([v for v, c in values.most_common(5)])}\n")
                f.write("\n")
        
        # Condition types
        f.write("## Condition Types\n\n")
        for cond in sorted(results['condition_types']):
            f.write(f"### {cond}\n\n")
            fields = results['fields'].get(f"condition:{cond}", set())
            if fields:
                f.write("**Fields:**\n\n")
                for field in sorted(fields):
                    f.write(f"- `{field}`\n")
            f.write("\n")
        
        # Cost types
        f.write("## Cost Types\n\n")
        for cost in sorted(results['cost_types']):
            f.write(f"### {cost}\n\n")
            fields = results['fields'].get(f"cost:{cost}", set())
            if fields:
                f.write("**Fields:**\n\n")
                for field in sorted(fields):
                    f.write(f"- `{field}`\n")
            f.write("\n")
        
        # Common field values
        f.write("## Common Field Values\n\n")
        f.write("### Source Locations\n\n")
        if "source" in results['field_values']:
            f.write("| Value | Count |\n")
            f.write("|-------|-------|\n")
            for value, count in results['field_values']['source'].most_common():
                f.write(f"| {value} | {count} |\n")
        
        f.write("\n### Destination Locations\n\n")
        if "destination" in results['field_values']:
            f.write("| Value | Count |\n")
            f.write("|-------|-------|\n")
            for value, count in results['field_values']['destination'].most_common():
                f.write(f"| {value} | {count} |\n")
        
        f.write("\n### Card Types\n\n")
        if "card_type" in results['field_values']:
            f.write("| Value | Count |\n")
            f.write("|-------|-------|\n")
            for value, count in results['field_values']['card_type'].most_common():
                f.write(f"| {value} | {count} |\n")
        
        f.write("\n### Target Players\n\n")
        if "target" in results['field_values']:
            f.write("| Value | Count |\n")
            f.write("|-------|-------|\n")
            for value, count in results['field_values']['target'].most_common():
                f.write(f"| {value} | {count} |\n")

if __name__ == "__main__":
    abilities_path = r"c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json"
    output_path = r"c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\ABILITY_FIELD_MAPPING.md"
    
    print(f"Analyzing {abilities_path}...")
    results = analyze_abilities(abilities_path)
    
    print(f"\nGenerating report to {output_path}...")
    generate_field_mapping_report(results, output_path)
    
    print("\nDone!")
