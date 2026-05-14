import re, os

root = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\src"

# Read all files
paths = [
    r"core\game_state\abilities.rs",
    r"ability\resolver.rs",
    r"ability\choice.rs",
    r"ability\compound.rs",
    r"turn\actions.rs",
]
files = {}
for p in paths:
    full = os.path.join(root, p)
    with open(full, encoding="utf-8") as f:
        files[p] = f.read()

# === abilities.rs ===
c = files[r"core\game_state\abilities.rs"]
old_init = """                        cost_paid: false,
                        pending_choice_result: None,
                        choice_card_no: None,
                        conditional_choice: None,
                        execution_context: None,
                        selected_card_ids: Vec::new(),
                            effect_started: false,
                        optional_cost_was_paid: false,"""
new_init = """                        state: crate::ability_queue::AbilityState::Pending,
                        pending_choice_result: None,
                        choice_card_no: None,
                        conditional_choice: None,
                        execution_context: None,
                        selected_card_ids: Vec::new(),"""
c = c.replace(old_init, new_init)

c = c.replace(
    "let cost_already_paid = entry.cost_paid;",
    "let already = entry.state == crate::ability_queue::AbilityState::CostReady\n"
    "    || entry.state == crate::ability_queue::AbilityState::EffectActive\n"
    "    || entry.state == crate::ability_queue::AbilityState::EffectChoicePending;",
)

c = c.replace(
    "                    e.effect_started = true;",
    "                    if matches!(e.state, crate::ability_queue::AbilityState::Pending) {\n"
    "                        e.state = crate::ability_queue::AbilityState::CostReady;\n"
    "                    }",
)
# the cost_already_paid variable name changed
c = c.replace("if cost_already_paid", "if already")
c = c.replace("cost_already_paid &&", "already &&")
c = c.replace("!cost_already_paid &&", "!already &&")
# But NOT the one in the comment
c = c.replace("// The already variable captures", "// The already variable captures")
c = c.replace("cost_already_paid = entry.", "__already_placeholder__")
c = c.replace(
    "cost_already_paid", "_already_val"
)  # remaining references (use old for function scope)
# Hmm, this is getting messy. Let me be more surgical.
files[r"core\game_state\abilities.rs"] = c

# Let me check what happened
print("=== abilities.rs changes ===")
idx = c.find("let already = entry.state")
if idx >= 0:
    print(f"already check at {idx}")
idx = c.find("if already")
if idx >= 0:
    print(f"already ref at {idx}")
idx = c.find("_already_val")
if idx >= 0:
    print(f"_already_val at {idx} - BAD!")
idx = c.find("__already_placeholder__")
if idx >= 0:
    print(f"__already_placeholder__ at {idx} - BAD!")
