"""Add debug print to execute_draw."""
with open('engine/src/ability/effects.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old = 'fn execute_draw(&mut self, count: u32, target: &str, source: &str, destination: &str, card_type: Option<&str>, per_unit: bool, per_unit_count: u32, per_unit_type: Option<&str>) -> Result<(), String> {'
new = old + '\n        eprintln!("DEBUG execute_draw: count={} target={} source={} dest={}", count, target, source, destination);'
content = content.replace(old, new, 1)

with open('engine/src/ability/effects.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print('Added debug')
