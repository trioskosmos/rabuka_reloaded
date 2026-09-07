import re

with open('engine/src/turn/triggers.rs', 'r') as f:
    content = f.read()

# Fix 1: SuppressAbilityTrigger collapsible_if
pattern1 = r'if effect\.action\s*== crate::ability::enums::ActionType::SuppressAbilityTrigger\s*{\s*if effect\.suppressed_trigger_any\(\) == Some\(trigger_name\) {'
replacement1 = 'if effect.action\n                            == crate::ability::enums::ActionType::SuppressAbilityTrigger\n                            && effect.suppressed_trigger_any() == Some(trigger_name) {'
content = re.sub(pattern1, replacement1, content)

# Fix 2: First LiveStart collapsible_if
pattern2 = r'if ability\.has_trigger\(crate::triggers::TriggerKind::LiveStart\) {\s*if seen\.insert\(\(\*card_id, aidx\)\) {'
replacement2 = 'if ability.has_trigger(crate::triggers::TriggerKind::LiveStart)\n                            && seen.insert((*card_id, aidx)) {'
content = re.sub(pattern2, replacement2, content)

# Fix 3: Second LiveStart collapsible_if
pattern3 = r'if ability\.has_trigger\(crate::triggers::TriggerKind::LiveStart\) {\s*if seen\.insert\(\(card_id, aidx\)\) {'
replacement3 = 'if ability.has_trigger(crate::triggers::TriggerKind::LiveStart)\n                                && seen.insert((card_id, aidx)) {'
content = re.sub(pattern3, replacement3, content)

with open('engine/src/turn/triggers.rs', 'w') as f:
    f.write(content)

print("Done")