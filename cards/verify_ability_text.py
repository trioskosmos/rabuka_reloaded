import json
import re

# Load abilities.json
with open(r'c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

# Load original card data for comparison
try:
    with open(r'c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\all_cards.json', 'r', encoding='utf-8') as f:
        cards_data = json.load(f)
    cards_dict = {card['card_id']: card for card in cards_data}
except FileNotFoundError:
    print("Warning: all_cards.json not found, skipping text comparison")
    cards_dict = {}

def normalize_text(text):
    """Normalize text for comparison"""
    if not text:
        return ""
    # Remove extra whitespace, normalize line breaks
    text = re.sub(r'\s+', ' ', text.strip())
    # Remove common formatting differences
    text = text.replace('（', '(').replace('）', ')')
    text = text.replace('「', '"').replace('」', '"')
    text = text.replace('『', '[').replace('』', ']')
    return text

def check_ability_text_matching():
    """Check if parsed ability text matches original card text"""
    mismatches = []
    matches = 0
    total = 0
    
    for ability in abilities:
        ability_id = ability.get('id')
        if not ability_id:
            continue
            
        total += 1
        
        # Get parsed ability text
        parsed_text = ability.get('full_text') or ability.get('text', '')
        parsed_text = normalize_text(parsed_text)
        
        # Get original card text if available
        original_text = ""
        if cards_dict:
            # Find cards that use this ability
            for card_id, card in cards_dict.items():
                if card.get('ability_id') == ability_id:
                    original_text = card.get('text', '')
                    break
        
        original_text = normalize_text(original_text)
        
        # Compare texts
        if original_text and parsed_text:
            if original_text == parsed_text:
                matches += 1
            else:
                mismatches.append({
                    'ability_id': ability_id,
                    'original': original_text,
                    'parsed': parsed_text,
                    'difference': len(original_text) - len(parsed_text)
                })
        elif not original_text:
            # No original text to compare
            matches += 1
    
    print(f"Text Matching Results:")
    print(f"  Total abilities: {total}")
    print(f"  Perfect matches: {matches}")
    print(f"  Mismatches: {len(mismatches)}")
    if total > 0:
        print(f"  Match rate: {(matches/total*100):.1f}%")
    else:
        print(f"  Match rate: N/A (no abilities found)")
    
    if mismatches:
        print(f"\nTop 10 Mismatches:")
        for i, mismatch in enumerate(mismatches[:10]):
            print(f"  {i+1}. Ability {mismatch['ability_id']} (diff: {mismatch['difference']})")
            print(f"     Original: {mismatch['original'][:50]}...")
            print(f"     Parsed:   {mismatch['parsed'][:50]}...")
            print()
    
    return matches, len(mismatches)

def check_common_parsing_issues():
    """Check for common parsing issues"""
    issues = {
        'missing_triggers': 0,
        'missing_costs': 0,
        'empty_effects': 0,
        'missing_conditions': 0,
        'invalid_actions': 0,
    }
    
    for ability in abilities:
        # Check for missing triggers
        if not ability.get('triggers') and ability.get('text'):
            issues['missing_triggers'] += 1
        
        # Check for missing costs when text suggests cost exists
        text = ability.get('text', '').lower()
        if 'コスト' in text or 'cost' in text:
            if not ability.get('cost'):
                issues['missing_costs'] += 1
        
        # Check for empty effects
        if not ability.get('effect'):
            issues['empty_effects'] += 1
        
        # Check for missing conditions
        if '：' in text and not ability.get('condition'):
            issues['missing_conditions'] += 1
        
        # Check for invalid action types
        effect = ability.get('effect', {})
        if isinstance(effect, dict):
            action = effect.get('action', '')
            if action and action not in [
                'move_cards', 'draw', 'draw_card', 'gain_resource', 'modify_score',
                'look_and_select', 'sequential', 'choice', 'appear', 'modify_cost',
                'change_state', 'choose_required_hearts', 'pay_energy', 'play_baton_touch'
            ]:
                issues['invalid_actions'] += 1
    
    print(f"\nCommon Parsing Issues:")
    for issue_type, count in issues.items():
        print(f"  {issue_type}: {count}")
    
    return issues

def check_card_count_consistency():
    """Check if card_count matches actual card usage"""
    card_count_issues = []
    
    for ability in abilities:
        ability_id = ability.get('id')
        card_count = ability.get('card_count', 0)
        
        if cards_dict:
            # Count actual cards using this ability
            actual_count = sum(1 for card in cards_dict.values() 
                            if card.get('ability_id') == ability_id)
            
            if card_count != actual_count:
                card_count_issues.append({
                    'ability_id': ability_id,
                    'declared': card_count,
                    'actual': actual_count
                })
    
    if card_count_issues:
        print(f"\nCard Count Issues ({len(card_count_issues)}):")
        for issue in card_count_issues[:10]:
            print(f"  Ability {issue['ability_id']}: declared {issue['declared']}, actual {issue['actual']}")
    
    return len(card_count_issues)

# Run all checks
print("="*60)
print("ABILITY TEXT VERIFICATION")
print("="*60)

# Check text matching
matches, mismatches = check_ability_text_matching()

# Check parsing issues
issues = check_common_parsing_issues()

# Check card count consistency
count_issues = check_card_count_consistency()

# Summary
print(f"\n" + "="*60)
print("VERIFICATION SUMMARY")
print("="*60)
if matches + mismatches > 0:
    print(f"Text matching: {(matches/(matches+mismatches)*100):.1f}% correct")
else:
    print(f"Text matching: N/A (no abilities to compare)")
print(f"Total issues found: {sum(issues.values()) + count_issues}")
print(f"Critical issues (missing costs/effects): {issues['missing_costs'] + issues['empty_effects']}")

if mismatches > 0 or sum(issues.values()) > 0 or count_issues > 0:
    print(f"\n⚠️  Issues found - parser may need refinement")
else:
    print(f"\n✅ All checks passed")
