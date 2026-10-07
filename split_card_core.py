import os
import subprocess

src = os.path.normpath('engine/src/core/card.rs')
base = os.path.normpath('engine/src/core/card')

# Always load pristine card.rs from git HEAD
try:
    cmd = ['git', 'show', 'HEAD:engine/src/core/card.rs']
    orig_text = subprocess.check_output(cmd, encoding='utf-8')
    all_lines = orig_text.splitlines(keepends=True)
except Exception:
    with open(src, encoding='utf-8') as f:
        all_lines = f.readlines()

def block(start, end):
    return ''.join(all_lines[start - 1 : end])

os.makedirs(base, exist_ok=True)

# 1. mod.rs
mod_content = (
    "use crate::core::constants::CountCast;\n"
    "use crate::ability::ability_store::AbilityRef;\n"
    "pub(crate) use crate::ability::enums::{\n"
    "    ActionType, ConditionType, EffectState, PlacementTarget, TargetPlayer, Zone,\n"
    "};\n"
    "use crate::core::types::ArcStr;\n"
    "#[cfg(feature = \"serde_support\")]\n"
    "use crate::BTreeMap;\n"
    "use crate::{HashMap, HashSet};\n"
    "#[cfg(feature = \"serde_support\")]\n"
    "use serde::{Deserialize, Serialize};\n"
    "use smallvec::SmallVec;\n"
    "\n"
    "#[cfg(feature = \"no_std\")]\n"
    "use alloc::{borrow::Cow, boxed::Box, string::String, string::ToString, vec::Vec};\n"
    "#[cfg(not(feature = \"no_std\"))]\n"
    "use std::borrow::Cow;\n"
    "use crate::Arc;\n"
    "\n"
    "pub mod condition;\n"
    "pub mod data;\n"
    "pub mod effect;\n"
    "\n"
    "pub use condition::*;\n"
    "pub use data::*;\n"
    "pub use effect::*;\n"
)

with open(os.path.join(base, 'mod.rs'), 'w', encoding='utf-8') as f:
    f.write(mod_content)
print('Wrote core/card/mod.rs')

# 2. data.rs: lines 1..1069 and 4765..len(all_lines)
# Note: lines 1..37 are imports and EkBox, but EkBox belongs with EffectKind in effect.rs.
# Let's keep the data blocks with `use super::*;`
data_content = (
    "use super::*;\n"
    "\n"
    + block(38, 1069)
    + block(4765, len(all_lines))
)
with open(os.path.join(base, 'data.rs'), 'w', encoding='utf-8') as f:
    f.write(data_content)
print('Wrote core/card/data.rs')

# 3. effect.rs: EkBox (lines 20..36) and lines 1070..3071
effect_content = (
    "use super::*;\n"
    "\n"
    + block(20, 36)
    + "\n"
    + block(1070, 3071)
)
with open(os.path.join(base, 'effect.rs'), 'w', encoding='utf-8') as f:
    f.write(effect_content)
print('Wrote core/card/effect.rs')

# 4. condition.rs: lines 3072..4764
condition_content = (
    "use super::*;\n"
    "\n"
    + block(3072, 4764)
)
with open(os.path.join(base, 'condition.rs'), 'w', encoding='utf-8') as f:
    f.write(condition_content)
print('Wrote core/card/condition.rs')
