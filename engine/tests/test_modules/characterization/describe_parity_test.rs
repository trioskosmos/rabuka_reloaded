//! Describe-parity gate (audit F1).
//!
//! Every effect node of every baked ability must render through describe.rs
//! templates in BOTH languages. A fallback (`describe output == node text`)
//! means a parser action has no UI prompt template — the fix belongs in
//! describe.rs, not here. Also pins EN/JA arm parity so a template added for
//! one language but missing from the other fails CI.

use crate::helpers::*;
use rabuka_engine::ability::ability_store::AbilityRef;
use rabuka_engine::ability::abilities_gen::NUM_ABILITIES;
use rabuka_engine::ability::describe::{describe_effect_en, describe_effect_ja};
use rabuka_engine::core::card::AbilityEffect;

fn collect_nodes<'a>(eff: &'a AbilityEffect, out: &mut Vec<&'a AbilityEffect>) {
    out.push(eff);
    let c = &eff.compound;
    if let Some(ref v) = c.actions {
        for a in v {
            collect_nodes(a, out);
        }
    }
    for child in [
        &c.look_action,
        &c.select_action,
        &c.primary_effect,
        &c.followup_action,
        &c.optional_action,
        &c.conditional_action,
    ] {
        if let Some(ref b) = *child {
            collect_nodes(b, out);
        }
    }
    if let Some(ref b) = eff.alternative_effect_any() {
        collect_nodes(b, out);
    }
}

#[test]
fn describe_templates_cover_every_ability_node_en_and_ja() {
    let db = load_real_database();
    let _ = db; // keep helper import when only bytecode is walked

    let mut failures: Vec<String> = Vec::new();
    let mut nodes_visited = 0usize;

    for idx in 0..NUM_ABILITIES {
        let ability = AbilityRef(idx as u16).resolve();
        let Some(ref effect) = ability.effect else {
            continue;
        };
        let mut nodes = Vec::new();
        collect_nodes(effect, &mut nodes);
        nodes_visited += nodes.len();

        for node in nodes {
            // Nodes without text have nothing to fall back to and no prompt
            // to render — skip them.
            if node.text.is_empty() {
                continue;
            }
            let en = describe_effect_en(node);
            let ja = describe_effect_ja(node);
            let action = node.action.to_str();
            if *en == *node.text {
                failures.push(format!(
                    "ability {idx}: EN fallback for action '{action}' (text: {})",
                    node.text
                ));
            }
            if *ja == *node.text {
                failures.push(format!(
                    "ability {idx}: JA fallback for action '{action}' (text: {})",
                    node.text
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "describe fallback hit for {} node(s) ({nodes_visited} visited):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
