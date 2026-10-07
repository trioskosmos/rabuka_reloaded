import os
import subprocess

base = os.path.normpath('engine/src/ability/condition')
src = os.path.join(base, 'card.rs')
state_src = os.path.join(base, 'state.rs')

# Always load the original card.rs and state.rs from git HEAD so splitting is idempotent
try:
    cmd = ['git', 'show', 'HEAD:engine/src/ability/condition/card.rs']
    orig_text = subprocess.check_output(cmd, encoding='utf-8')
    all_lines = orig_text.splitlines(keepends=True)
except Exception:
    with open(src, encoding='utf-8') as f:
        all_lines = f.readlines()

try:
    cmd = ['git', 'show', 'HEAD:engine/src/ability/condition/state.rs']
    orig_state_text = subprocess.check_output(cmd, encoding='utf-8')
    all_state_lines = orig_state_text.splitlines(keepends=True)
except Exception:
    with open(state_src, encoding='utf-8') as f:
        all_state_lines = f.readlines()

def block(start, end):
    """Extract lines[start-1:end] (1-indexed, end inclusive)."""
    return ''.join(all_lines[start - 1:end])

def state_block(start, end):
    """Extract lines[start-1:end] from state.rs (1-indexed, end inclusive)."""
    return ''.join(all_state_lines[start - 1:end])

def footer():
    return "}\n"

headers = {
    'location': (
        "use super::ConditionContext;\n"
        "use super::predicates;\n"
        "use crate::ability::enums::Zone;\n"
        "use crate::ability::util::{self, compare_counts};\n"
        "use crate::card::{AbilityFilter, BladeColor, CardProperty, Condition};\n"
        "use crate::core::constants::CountCast;\n"
        "use crate::HashSet;\n"
        "#[cfg(feature = \"no_std\")]\n"
        "use alloc::{\n"
        "    borrow::ToOwned,\n"
        "    string::{String, ToString},\n"
        "    vec::Vec,\n"
        "};\n"
        "\n"
        "impl<'a> ConditionContext<'a> {\n"
    ),
    'group': (
        "use super::ConditionContext;\n"
        "use crate::ability::enums::Zone;\n"
        "use crate::ability::util::{self, compare_counts};\n"
        "use crate::card::Condition;\n"
        "#[cfg(feature = \"no_std\")]\n"
        "use alloc::{\n"
        "    string::{String, ToString},\n"
        "    vec::Vec,\n"
        "};\n"
        "\n"
        "impl<'a> ConditionContext<'a> {\n"
    ),
    'appearance': (
        "use super::ConditionContext;\n"
        "use crate::ability::enums::Zone;\n"
        "use crate::ability::util;\n"
        "use crate::card::{CardProperty, Condition};\n"
        "#[cfg(feature = \"no_std\")]\n"
        "use alloc::{\n"
        "    string::{String, ToString},\n"
        "    vec::Vec,\n"
        "};\n"
        "\n"
        "impl<'a> ConditionContext<'a> {\n"
    ),
    'card_count': (
        "use super::ConditionContext;\n"
        "use super::predicates;\n"
        "use crate::ability::debug::AbDebug;\n"
        "use crate::ability::enums::Zone;\n"
        "use crate::ability::util::{self, compare_counts};\n"
        "use crate::card::{BladeColor, CardProperty, ComparisonTarget, Condition};\n"
        "use crate::core::constants::CountCast;\n"
        "use crate::HashMap;\n"
        "use smallvec::SmallVec;\n"
        "#[cfg(feature = \"no_std\")]\n"
        "use alloc::{\n"
        "    borrow::ToOwned,\n"
        "    string::{String, ToString},\n"
        "    vec::Vec,\n"
        "};\n"
        "\n"
        "impl<'a> ConditionContext<'a> {\n"
    ),
    'ability_filter': (
        "use super::ConditionContext;\n"
        "use crate::ability::enums::Zone;\n"
        "use crate::card::{AbilityFilter, Condition};\n"
        "use crate::core::constants::CountCast;\n"
        "#[cfg(feature = \"no_std\")]\n"
        "use alloc::vec::Vec;\n"
        "\n"
        "impl<'a> ConditionContext<'a> {\n"
    ),
    'movement': (
        "use super::ConditionContext;\n"
        "use crate::ability::enums::Zone;\n"
        "use crate::ability::util::{self, compare_counts};\n"
        "use crate::card::{CardProperty, Condition, HeartColor, TriggerEvent};\n"
        "use crate::core::constants::CountCast;\n"
        "use crate::HashSet;\n"
        "use smallvec::SmallVec;\n"
        "#[cfg(feature = \"no_std\")]\n"
        "use alloc::{\n"
        "    string::{String, ToString},\n"
        "    vec::Vec,\n"
        "};\n"
        "\n"
        "impl<'a> ConditionContext<'a> {\n"
    ),
}

blocks = {
    'location': [
        (84, 372),
        (400, 725),
        (772, 1344),
    ],
    'group': [
        (1345, 1499),
        (3603, 3625),
    ],
    'movement': [
        (1500, 1800),
    ],
    'card_count': [
        (1881, 2551),
        (3403, 3602),
        (3626, 3788),
        (3808, 3846),
        (3886, 4447),
    ],
    'appearance': [
        (2552, 3207),
    ],
    'ability_filter': [
        (3208, 3368),
    ],
}

for owner, ranges in blocks.items():
    content = headers[owner]
    for start, end in ranges:
        content += block(start, end)
    # Append movement evaluation from state.rs into movement.rs
    if owner == 'movement':
        content += state_block(515, 1174)
    content += footer()
    path = os.path.join(base, f'{owner}.rs')
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f'Wrote {path}')

# Rewrite state.rs without the movement block (lines 515..1174)
state_content = state_block(1, 514) + state_block(1175, len(all_state_lines))
state_content = state_content.replace(
    "use crate::ability::util::compare_counts;\n", ""
).replace(
    "use crate::card::{CardProperty, Condition, TriggerEvent};\n",
    "use crate::card::Condition;\n"
)
with open(state_src, 'w', encoding='utf-8') as f:
    f.write(state_content)
print(f'Rewrote {state_src}')

# Rewrite card.rs with only shared utilities (including revealed_cards_scoped)
card_keep = [
    "use super::ConditionContext;\n"
    "use crate::ability::enums::Zone;\n"
    "use crate::ability::util::{self, compare_counts};\n"
    "use crate::card::{Condition, HeartColor};\n"
    "use crate::core::constants::CountCast;\n"
    "use crate::HashSet;\n"
    "use smallvec::SmallVec;\n"
    "#[cfg(feature = \"no_std\")]\n"
    "use alloc::{\n"
    "    string::{String, ToString},\n"
    "    vec::Vec,\n"
    "};\n"
    "\n"
    "impl<'a> ConditionContext<'a> {\n"
]

keep_ranges = [
    (23, 83),     # resolve_condition_player through evaluate_check_self_condition
    (373, 399),   # eligible_cards
    (726, 771),   # revealed_cards_scoped, revealed_cards_scoped_to
    (1801, 1880), # modified_cost, count_distinct_cost, count_distinct_heart_types
    (3369, 3402), # zone_len
    (3789, 3807), # moved_card_pool
    (3847, 3885), # heart_count_at_or_all
]

for start, end in keep_ranges:
    card_keep.append(block(start, end))
card_keep.append(footer())

with open(src, 'w', encoding='utf-8') as f:
    f.write(''.join(card_keep))
print(f'Rewrote {src}')

# Fix private methods called from other modules
fixes = {
    os.path.join(base, 'group.rs'): [
        ('    fn sum_group_filtered_zone(', '    pub(crate) fn sum_group_filtered_zone('),
    ],
    os.path.join(base, 'card.rs'): [
        ('    fn eligible_cards', '    pub(crate) fn eligible_cards'),
        ('    fn revealed_cards_scoped(', '    pub(crate) fn revealed_cards_scoped('),
        ('    fn modified_cost(', '    pub(crate) fn modified_cost('),
        ('    fn count_distinct_cost(', '    pub(crate) fn count_distinct_cost('),
        ('    fn count_distinct_heart_types(', '    pub(crate) fn count_distinct_heart_types('),
        ('    fn moved_card_pool(', '    pub(crate) fn moved_card_pool('),
        ('    fn heart_count_at_or_all(', '    pub(crate) fn heart_count_at_or_all('),
    ],
}
for path, replacements in fixes.items():
    with open(path, encoding='utf-8') as f:
        content = f.read()
    for old, new in replacements:
        content = content.replace(old, new, 1)
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f'Fixed visibility in {path}')

# Remove recorded.rs if it exists (now folded into card.rs)
rec_path = os.path.join(base, 'recorded.rs')
if os.path.exists(rec_path):
    os.remove(rec_path)
    print(f'Removed {rec_path}')

# Ensure mod.rs registers all split modules and removes recorded
mod_path = os.path.join(base, 'mod.rs')
with open(mod_path, 'r', encoding='utf-8') as f:
    mod_content = f.read()
if 'mod appearance;' not in mod_content:
    if 'mod ability_filter;\n' in mod_content:
        mod_content = mod_content.replace('mod ability_filter;\n', 'mod ability_filter;\nmod appearance;\n')
    else:
        mod_content = mod_content.replace('mod card;\n', 'mod card;\nmod appearance;\n')
if 'mod recorded;\n' in mod_content:
    mod_content = mod_content.replace('mod recorded;\n', '')
with open(mod_path, 'w', encoding='utf-8') as f:
    f.write(mod_content)
print(f'Updated {mod_path}')
