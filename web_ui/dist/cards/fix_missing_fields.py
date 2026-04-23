#!/usr/bin/env python3
"""Fix missing fields in abilities.json by adding appropriate defaults based on context."""

import json
from typing import Dict, Any, Optional

def fix_move_cards_fields(obj: Any, parent_context: Dict[str, Any] = None) -> Any:
    """Recursively fix missing fields in move_cards actions."""
    if parent_context is None:
        parent_context = {}
    
    if isinstance(obj, dict):
        # Update parent context for nested actions
        new_context = dict(parent_context)
        if obj.get("action") == "look_at":
            new_context["source"] = obj.get("source")
            new_context["target"] = obj.get("target")
            new_context["look_count"] = obj.get("count")
        
        # Check if this is a move_cards action
        if obj.get("action") == "move_cards":
            # Required fields for move_cards
            # source: REQUIRED - where cards come from
            # destination: REQUIRED - where cards go
            # count: REQUIRED - how many cards
            # target: Can default to "self"
            # card_type: Can be null (any card type)
            
            # Inherit source from parent look_action if missing
            if "source" not in obj and parent_context.get("source"):
                obj["source"] = parent_context["source"]
            
            # If source is still missing, try to infer from text
            if "source" not in obj:
                text = obj.get("text", "")
                if "手札" in text or "hand" in text.lower():
                    obj["source"] = "hand"
                elif "デッキ" in text or "deck" in text.lower():
                    if "上から" in text or "top" in text.lower():
                        obj["source"] = "deck_top"
                    elif "下から" in text or "bottom" in text.lower():
                        obj["source"] = "deck_bottom"
                    else:
                        obj["source"] = "deck"
                elif "控え室" in text or "discard" in text.lower():
                    obj["source"] = "discard"
                elif "ステージ" in text or "stage" in text.lower():
                    obj["source"] = "stage"
                elif "エネルギー" in text or "energy" in text.lower():
                    obj["source"] = "energy_zone"
                else:
                    # Default to deck if can't determine
                    obj["source"] = "deck"
            
            # If destination is missing, try to infer from text
            if "destination" not in obj:
                text = obj.get("text", "")
                if "手札" in text or "hand" in text.lower() and ("加え" in text or "add" in text.lower() or "引く" in text or "draw" in text.lower()):
                    obj["destination"] = "hand"
                elif "控え室" in text or "discard" in text.lower() and ("置く" in text or "place" in text.lower()):
                    obj["destination"] = "discard"
                elif "ステージ" in text or "stage" in text.lower():
                    obj["destination"] = "stage"
                elif "デッキ" in text or "deck" in text.lower():
                    if "上に" in text or "top" in text.lower():
                        obj["destination"] = "deck_top"
                    elif "下に" in text or "bottom" in text.lower():
                        obj["destination"] = "deck_bottom"
                    else:
                        obj["destination"] = "deck"
                else:
                    # Can't determine, leave missing
                    pass
            
            # Inherit target from parent if missing, default to "self"
            if "target" not in obj:
                if parent_context.get("target"):
                    obj["target"] = parent_context["target"]
                else:
                    obj["target"] = "self"
            
            # Handle count - check text for "残り" (remaining)
            text = obj.get("text", "")
            if "count" not in obj:
                if "残り" in text or "rest" in text.lower() or "remaining" in text.lower():
                    # Use "remaining" for dynamic count
                    obj["count"] = "remaining"
                elif parent_context.get("look_count"):
                    # If we have a look count, this might be calculated
                    # For now, use "remaining" as a safe default
                    obj["count"] = "remaining"
                else:
                    # Try to extract count from text
                    import re
                    count_match = re.search(r'(\d+)枚', text)
                    if count_match:
                        obj["count"] = int(count_match.group(1))
                    else:
                        obj["count"] = 1  # Default to 1 if unknown
            
            # card_type can be null/omitted if any card type is acceptable
            # Only add if explicitly needed based on the text
            if "card_type" not in obj and "member" in obj.get("text", "").lower():
                obj["card_type"] = "member_card"
            elif "card_type" not in obj and "live" in obj.get("text", "").lower():
                obj["card_type"] = "live_card"
            elif "card_type" not in obj and "energy" in obj.get("text", "").lower():
                obj["card_type"] = "energy_card"
            # Otherwise, leave card_type as null (any card type)
        
        # For look_and_select, process look_action first to capture context, then select_action
        if obj.get("action") == "look_and_select":
            # Process look_action first to capture its context
            if "look_action" in obj and isinstance(obj["look_action"], dict):
                obj["look_action"] = fix_move_cards_fields(obj["look_action"], new_context)
                # Update context with look_action's info
                if obj["look_action"].get("action") == "look_at":
                    new_context["source"] = obj["look_action"].get("source")
                    new_context["target"] = obj["look_action"].get("target")
                    new_context["look_count"] = obj["look_action"].get("count")
            
            # Now process select_action with the updated context
            if "select_action" in obj and isinstance(obj["select_action"], dict):
                obj["select_action"] = fix_move_cards_fields(obj["select_action"], new_context)
        else:
            # Normal recursion for other structures
            if "look_action" in obj and isinstance(obj["look_action"], dict):
                obj["look_action"] = fix_move_cards_fields(obj["look_action"], new_context)
            
            if "select_action" in obj and isinstance(obj["select_action"], dict):
                obj["select_action"] = fix_move_cards_fields(obj["select_action"], new_context)
        
        if "actions" in obj and isinstance(obj["actions"], list):
            # Pass the same context to all actions in the list
            obj["actions"] = [fix_move_cards_fields(action, new_context) for action in obj["actions"]]
        
        if "costs" in obj and isinstance(obj["costs"], list):
            # Handle costs array in sequential_cost structures
            obj["costs"] = [fix_move_cards_fields(cost, new_context) for cost in obj["costs"]]
        
        if "cost" in obj and isinstance(obj["cost"], dict):
            obj["cost"] = fix_move_cards_fields(obj["cost"], new_context)
        
        if "effect" in obj and isinstance(obj["effect"], dict):
            obj["effect"] = fix_move_cards_fields(obj["effect"], new_context)
        
        # Recurse into all other dict values
        for key, value in obj.items():
            if key not in ["look_action", "select_action", "actions", "cost", "effect"]:
                if isinstance(value, (dict, list)):
                    obj[key] = fix_move_cards_fields(value, new_context)
    
    elif isinstance(obj, list):
        obj = [fix_move_cards_fields(item, parent_context) for item in obj]
    
    return obj

def validate_move_cards(obj: Any, path: str = "") -> list:
    """Validate move_cards actions and report missing required fields."""
    issues = []
    
    if isinstance(obj, dict):
        if obj.get("action") == "move_cards":
            required_fields = ["source", "destination"]
            for field in required_fields:
                if field not in obj or obj[field] is None:
                    issues.append(f"{path}.{field}: Missing required field '{field}' - Text: {obj.get('text', 'N/A')}")
            
            # count is also required (can be "variable", "remaining", or a number)
            if "count" not in obj:
                issues.append(f"{path}.count: Missing required field 'count' - Text: {obj.get('text', 'N/A')}")
            
            # target should be present (can default to self but should be explicit)
            if "target" not in obj:
                issues.append(f"{path}.target: Missing field 'target' (should default to 'self') - Text: {obj.get('text', 'N/A')}")
        
        # Recurse
        for key, value in obj.items():
            new_path = f"{path}.{key}" if path else key
            issues.extend(validate_move_cards(value, new_path))
    
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            new_path = f"{path}[{i}]"
            issues.extend(validate_move_cards(item, new_path))
    
    return issues

def main():
    abilities_path = r"c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json"
    backup_path = r"c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json.backup_before_fix"
    
    print(f"Loading {abilities_path}...")
    with open(abilities_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    print("Validating current state...")
    issues = validate_move_cards(data)
    print(f"Found {len(issues)} issues with move_cards actions")
    
    if issues:
        print("\nFirst 20 issues:")
        for issue in issues[:20]:
            print(f"  - {issue}")
        if len(issues) > 20:
            print(f"  ... and {len(issues) - 20} more")
    
    print("\nFixing missing fields...")
    fixed_data = fix_move_cards_fields(data)
    
    print("Validating after fix...")
    issues_after = validate_move_cards(fixed_data)
    print(f"Found {len(issues_after)} issues after fix")
    
    if issues_after:
        print("\nRemaining issues:")
        for issue in issues_after[:20]:
            print(f"  - {issue}")
        if len(issues_after) > 20:
            print(f"  ... and {len(issues_after) - 20} more")
    
    # Backup original
    print(f"\nCreating backup at {backup_path}...")
    with open(backup_path, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    
    # Write fixed version
    print(f"Writing fixed version to {abilities_path}...")
    with open(abilities_path, 'w', encoding='utf-8') as f:
        json.dump(fixed_data, f, ensure_ascii=False, indent=2)
    
    print("\nDone!")

if __name__ == "__main__":
    main()
