import os
import subprocess

src = os.path.normpath('engine/src/ability/choice.rs')
base = os.path.normpath('engine/src/ability/choice')

# Always load pristine choice.rs from git HEAD
try:
    cmd = ['git', 'show', 'HEAD:engine/src/ability/choice.rs']
    orig_text = subprocess.check_output(cmd, encoding='utf-8')
    all_lines = orig_text.splitlines(keepends=True)
except Exception:
    with open(src, encoding='utf-8') as f:
        all_lines = f.readlines()

def block(start, end):
    return ''.join(all_lines[start - 1 : end])

os.makedirs(base, exist_ok=True)

# 1. mod.rs
mod_header = (
    "use crate::core::constants::CountCast;\n"
    "use super::condition::ConditionContext;\n"
    "use super::enums::{ActionType, Zone};\n"
    "use super::types::{\n"
    "    Choice, ChoiceBuilder, ChoiceResult, ChoiceRoute, ExecutionContext, LookAndSelectStep,\n"
    "};\n"
    "use super::util;\n"
    "pub(super) use crate::ability::enums;\n"
    "pub(super) use crate::ability::compound;\n"
    "pub(super) use crate::ability::resolver;\n"
    "use crate::ability::debug::ABILITY_DEBUG;\n"
    "use crate::ability::queue::ConditionalChoice;\n"
    "use crate::card::AbilityEffect;\n"
    "use crate::game_state::GameState;\n"
    "#[cfg(not(feature = \"no_std\"))]\n"
    "use std::borrow::Cow;\n"
    "#[cfg(feature = \"no_std\")]\n"
    "use alloc::{\n"
    "    borrow::Cow,\n"
    "    boxed::Box,\n"
    "    string::{String, ToString},\n"
    "    vec::Vec,\n"
    "};\n"
    "use core::sync::atomic::Ordering;\n"
    "use smallvec::SmallVec;\n"
    "\n"
    "mod payment;\n"
    "mod position;\n"
    "mod result_handlers;\n"
    "mod select_card;\n"
    "mod stage_discard;\n"
    "mod target;\n"
    "\n"
    "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n"
    "enum Continuation {\n"
    "    Immediate,\n"
    "    DeferredSelectCard,\n"
    "    DeferredOther,\n"
    "}\n"
    "\n"
    "pub(crate) struct SelectionContext {\n"
    "    pub card_type: Option<String>,\n"
    "    pub count: usize,\n"
    "    pub allow_skip: bool,\n"
    "    pub indices: Vec<usize>,\n"
    "    pub cost_limit: Option<u8>,\n"
    "    pub cost_limit_operator: Option<String>,\n"
    "    pub cost_total: Option<u8>,\n"
    "    pub cost_total_operator: Option<String>,\n"
    "    pub group: Option<String>,\n"
    "    pub characters: Option<Vec<String>>,\n"
    "    pub filtered_indices: Option<Vec<usize>>,\n"
    "    pub is_select_action: bool,\n"
    "    pub target_player_id: Option<String>,\n"
    "    pub destination: Option<String>,\n"
    "    pub discard_remaining: Option<bool>,\n"
    "    pub blind: bool,\n"
    "    pub is_reveal: bool,\n"
    "}\n"
    "\n"
    "impl SelectionContext {\n"
    "    pub fn mfi(&self, indices: &[usize]) -> Vec<usize> {\n"
    "        match &self.filtered_indices {\n"
    "            Some(fi) => indices.iter().filter_map(|&i| fi.get(i).copied()).collect(),\n"
    "            None => indices.to_vec(),\n"
    "        }\n"
    "    }\n"
    "}\n"
    "\n"
    "impl super::resolver::AbilityResolver {\n"
)

mod_content = mod_header + block(63, 440) + block(3932, 4006)

with open(os.path.join(base, 'mod.rs'), 'w', encoding='utf-8') as f:
    f.write(mod_content)
print('Wrote mod.rs')

# Standard subfile header/footer
def subfile(content_blocks):
    return (
        "use super::*;\n"
        "\n"
        "impl super::resolver::AbilityResolver {\n"
        + ''.join(content_blocks)
        + "}\n"
    )

# 2. payment.rs: 592..1006
with open(os.path.join(base, 'payment.rs'), 'w', encoding='utf-8') as f:
    f.write(subfile([block(592, 1006)]))
print('Wrote payment.rs')

# 3. select_card.rs: 1007..2128
with open(os.path.join(base, 'select_card.rs'), 'w', encoding='utf-8') as f:
    f.write(subfile([block(1007, 2128)]))
print('Wrote select_card.rs')

# 4. stage_discard.rs: 441..591 and 2129..2733
with open(os.path.join(base, 'stage_discard.rs'), 'w', encoding='utf-8') as f:
    f.write(subfile([block(441, 591), block(2129, 2733)]))
print('Wrote stage_discard.rs')

# 5. target.rs: 2734..3158 and 3801..3931
with open(os.path.join(base, 'target.rs'), 'w', encoding='utf-8') as f:
    f.write(subfile([block(2734, 3158), block(3801, 3931)]))
print('Wrote target.rs')

# 6. position.rs: 3159..3800
with open(os.path.join(base, 'position.rs'), 'w', encoding='utf-8') as f:
    f.write(subfile([block(3159, 3800)]))
print('Wrote position.rs')
