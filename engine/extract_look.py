import re

with open('../effects_head.txt', 'r', encoding='utf-16') as f:
    content = f.read()

func_names = [
    'execute_look_and_select',
    'execute_reveal', 
    'execute_select',
    'execute_look_at',
    'execute_reveal_per_group'
]

def extract_function(text, func_name):
    pattern = r'( {4,12}(?:pub\s+)?fn\s+' + func_name + r'\b.*?\{)'
    match = re.search(pattern, text, re.DOTALL)
    if not match:
        print(f'Could not find {func_name}')
        return None
    start = match.start()
    brace_start = match.end() - 1
    depth = 1
    pos = brace_start + 1
    while pos < len(text) and depth > 0:
        if text[pos] == '{':
            depth += 1
        elif text[pos] == '}':
            depth -= 1
        pos += 1
    return text[start:pos]

extracted = []
for name in func_names:
    func = extract_function(content, name)
    if func:
        extracted.append(func)
        print(f'Extracted {name}: {len(func)} chars')
    else:
        print(f'FAILED to extract {name}')

with open('src/ability/look.rs', 'w', encoding='utf-8') as f:
    f.write('use crate::card::AbilityEffect;\n')
    f.write('use crate::zones;\n')
    f.write('use super::types::{Choice, ExecutionContext, LookAndSelectStep};\n')
    f.write('use super::resolver::AbilityResolver;\n')
    f.write('use super::util;\n')
    f.write('\n')
    f.write("impl<'a> AbilityResolver<'a> {\n")
    for func in extracted:
        func = re.sub(r'^(\s*)fn\s+', r'\1pub fn ', func, count=1)
        f.write(func)
        f.write('\n')
    f.write('}\n')

print(f'Done - wrote {len(extracted)} functions to look.rs')
