import sys, re
sys.path.insert(0, 'cards/ability_extraction')

# Monkey-patch parse_action to trace dispatch matching
import parser as p
orig_parse_action = p.parse_action

def traced_parse_action(text):
    if 'コストを+4' not in text and '{{heart_05' not in text:
        return orig_parse_action(text)
    
    # Manual dispatch to find which rule matches
    from parser import R, _R
    # Re-play the dispatch table building by calling the parse_action internals...
    # Actually just add a print to the real function
    
    # Instead, let's do the dispatch manually within the function
    # by checking the .R list
    
    class FakeAction(dict):
        pass
    
    inner_action = {'text': text}
    from parser import extract_destination, extract_source, extract_count, _fill_defaults, _handle_cost_modification, extract_dynamic_count, extract_cost_limit, extract_state_change, extract_card_type, extract_target, extract_optional, extract_max, extract_quoted_text, categorize_quoted_text, extract_group_names, extract_position, strip_parenthetical, _strip_duration_prefix, POSITION_KEYWORDS
    from parser import _R as dispatch_rules
    
    inner_action['action'] = 'custom'
    for i, (cond, act, setter) in enumerate(dispatch_rules):
        try:
            if callable(cond):
                try:
                    match = cond(text, inner_action)
                except TypeError:
                    match = cond(text)
            else:
                match = cond in text
        except:
            match = False
        if match:
            print(f'  Rule {i} matches: {act}')
            if callable(cond):
                print(f'    cond: {cond.__code__.co_code[:20] if hasattr(cond, \"__code__\") else \"lambda\"}')
            else:
                print(f'    cond: {repr(cond)[:60]}')
            print(f'    setter: {setter}')
            break
    return orig_parse_action(text)

p.parse_action = traced_parse_action

text = 'このカードのコストを+4して{{heart_05.png|heart05}}を得る。この能力では下にあるメンバーカードは3枚までしか数えない。'
r = p.parse_action(text)
print()
print('Final action:', r.get('action'))
