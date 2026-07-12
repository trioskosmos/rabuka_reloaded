use crate::card::{build_kind_from_action, Ability, AbilityEffect, Card, Condition, EffectKind};
use serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::string::String;
use std::sync::Arc;
use std::vec::Vec;

pub struct CardLoader;

impl CardLoader {
    pub fn load_cards_from_file(path: &Path) -> Result<Vec<Card>, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let abilities_path = path.parent().unwrap().join("abilities.json");
        let abilities_contents = std::fs::read_to_string(&abilities_path).ok();

        Self::load_cards_from_strs(&contents, abilities_contents.as_deref())
    }

    /// Parse cards from embedded string content (no file I/O at runtime).
    pub fn load_cards_from_strs(
        cards_json: &str,
        abilities_json: Option<&str>,
    ) -> Result<Vec<Card>, String> {
        // Try parsing as array first
        let mut cards: Vec<Card> = match serde_json::from_str::<Vec<Card>>(cards_json) {
            Ok(cards) => cards,
            Err(e1) => {
                // If that fails, try parsing as object (map) and convert to array
                let card_map: HashMap<String, Card> = serde_json::from_str(cards_json)
                    .map_err(|e| format!("Vec: {}; Object: {}", e1, e))?;
                card_map.into_values().collect()
            }
        };

        // Load abilities if provided
        if let Some(abilities_str) = abilities_json {
            if let Ok(abilities_data) = Self::load_abilities_from_str(abilities_str) {
                cards = Self::attach_abilities(cards, &abilities_data);
            }
        }

        Ok(cards)
    }

    #[allow(dead_code)]
    fn load_abilities_from_file(path: &Path) -> Result<serde_json::Value, String> {
        let mut file =
            File::open(path).map_err(|e| format!("Failed to open abilities file: {}", e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read abilities file: {}", e))?;
        Self::load_abilities_from_str(&contents)
    }

    pub fn load_abilities_from_str(contents: &str) -> Result<serde_json::Value, String> {
        let data: serde_json::Value = serde_json::from_str(contents)
            .map_err(|e| format!("Failed to parse abilities JSON: {}", e))?;
        Ok(data)
    }

    pub fn attach_abilities(mut cards: Vec<Card>, abilities_data: &serde_json::Value) -> Vec<Card> {
        let ability_map = Self::build_abilities_map_shared(abilities_data);
        for card in &mut cards {
            if let Some(card_abilities) = ability_map.get(&card.card_no) {
                card.abilities = card_abilities.clone();
            }
        }
        cards
    }

    /// Build a card_no -> Vec<Arc<Ability>> map from the parsed abilities JSON Value.
    pub fn build_abilities_map_shared(
        abilities_data: &serde_json::Value,
    ) -> HashMap<String, Vec<Arc<Ability>>> {
        let mut ability_map: HashMap<String, Vec<Arc<Ability>>> = HashMap::new();

        if let Some(unique_abilities) = abilities_data
            .get("unique_abilities")
            .and_then(|v| v.as_array())
        {
            for ability_entry in unique_abilities {
                let mut entry = ability_entry.clone();
                if let Some(obj) = entry.as_object_mut() {
                    if let Some(tc_val) = obj.remove("trigger_condition") {
                        match obj.get_mut("condition") {
                            Some(cond_val) => {
                                let mut merged = serde_json::Map::new();
                                merged.insert("type".into(), "compound".into());
                                merged.insert("operator".into(), "and".into());
                                merged.insert(
                                    "conditions".into(),
                                    serde_json::Value::Array(vec![cond_val.take(), tc_val]),
                                );
                                obj.insert("condition".into(), serde_json::Value::Object(merged));
                            }
                            None => {
                                obj.insert("condition".into(), tc_val);
                            }
                        }
                    }
                }
                // Extract effect JSON, action, and sub-actions BEFORE consuming entry
                let effect_entry = entry.get("effect").cloned();
                let effect_action = effect_entry.as_ref().and_then(|ej| {
                    ej.get("type")
                        .or_else(|| ej.get("action"))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                });
                let effect_actions = effect_entry
                    .as_ref()
                    .and_then(|ej| ej.get("actions"))
                    .and_then(|a| a.as_array())
                    .cloned();
                // Extract cost JSON BEFORE consuming entry
                let cost_entry = entry.get("cost").cloned();

                if let Ok(mut ability) = serde_json::from_value::<Ability>(entry) {
                    // Populate EffectKind from the original effect JSON
                    if let Some(ref mut effect) = ability.effect {
                        if let Some(ref act) = effect_action {
                            if let Some(ref ej) = effect_entry {
                                if let Some(kind) = build_kind_from_action(act, ej) {
                                    effect.kind = Some(kind);
                                }
                            }
                        }
                    }
                    // Recursively populate EffectKind for ALL nested sub-effects
                    // MUST run before fixed_actions so dynamic_count_any() works.
                    if let Some(ref mut effect) = ability.effect {
                        if let Some(ref json_effect) = effect_entry {
                            shared_populate_nested(effect, json_effect);
                        }
                    }

                    if let Some(ref mut effect) = ability.effect {
                        if let Some(ref actions) = effect.compound.actions.clone() {
                            let fixed_actions: Vec<crate::card::AbilityEffect> = actions
                                .iter()
                                .map(|action| {
                                    let mut fixed_action = action.clone();
                                    if (fixed_action.action == "draw"
                                        || fixed_action.action == "draw_card")
                                        && fixed_action.count.is_none()
                                        && fixed_action.dynamic_count_any().is_none()
                                    {
                                        fixed_action.count = Some(1);
                                    }
                                    fixed_action
                                })
                                .collect();
                            effect.compound.actions = Some(fixed_actions);
                        }
                    }
                    fn shared_populate_nested(
                        effect: &mut AbilityEffect,
                        json_val: &serde_json::Value,
                    ) {
                        if let Some(kind) = build_kind_from_action(&effect.action, json_val) {
                            effect.kind = Some(kind);
                        }
                        if let Some(ref mut sub) = effect.compound.look_action {
                            if let Some(sub_json) = json_val.get("look_action") {
                                shared_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut sub) = effect.compound.select_action {
                            if let Some(sub_json) = json_val.get("select_action") {
                                shared_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut sub) = effect.compound.followup_action {
                            if let Some(sub_json) = json_val.get("followup_action") {
                                shared_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut sub) = effect.compound.primary_effect {
                            if let Some(sub_json) = json_val.get("primary_effect") {
                                shared_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut sub) = effect.compound.optional_action {
                            if let Some(sub_json) = json_val.get("optional_action") {
                                shared_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut sub) = effect.compound.conditional_action {
                            if let Some(sub_json) = json_val.get("conditional_action") {
                                shared_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut actions) = effect.compound.actions {
                            if let Some(json_actions) =
                                json_val.get("actions").and_then(|a| a.as_array())
                            {
                                for (i, action) in actions.iter_mut().enumerate() {
                                    if i < json_actions.len() {
                                        shared_populate_nested(action, &json_actions[i]);
                                    }
                                }
                            }
                        }
                        if let Some(ref mut steps) = effect.effect_steps {
                            if let Some(json_steps) =
                                json_val.get("effect_steps").and_then(|a| a.as_array())
                            {
                                for (i, step) in steps.iter_mut().enumerate() {
                                    if i < json_steps.len() {
                                        shared_populate_nested(step, &json_steps[i]);
                                    }
                                }
                            }
                        }
                        if let Some(ref mut cond) = effect.condition {
                            if let Some(cond_json) = json_val.get("condition") {
                                shared_populate_condition(cond, cond_json);
                            }
                        }
                        match effect.kind.as_mut() {
                            Some(EffectKind::LookReveal {
                                ref mut options,
                                ref mut resource_on_select,
                                ..
                            }) => {
                                if let Some(ref mut opts) = options {
                                    if let Some(json_opts) =
                                        json_val.get("options").and_then(|a| a.as_array())
                                    {
                                        for (i, opt) in opts.iter_mut().enumerate() {
                                            if i < json_opts.len() {
                                                shared_populate_nested(opt, &json_opts[i]);
                                            }
                                        }
                                    }
                                }
                                if let Some(ref mut ros) = resource_on_select {
                                    if let Some(ros_json) = json_val.get("resource_on_select") {
                                        shared_populate_nested(ros, ros_json);
                                    }
                                }
                            }
                            Some(EffectKind::CompoundEffect {
                                ref mut options,
                                ref mut alternative_effect,
                                ..
                            }) => {
                                if let Some(ref mut opts) = options {
                                    if let Some(json_opts) =
                                        json_val.get("options").and_then(|a| a.as_array())
                                    {
                                        for (i, opt) in opts.iter_mut().enumerate() {
                                            if i < json_opts.len() {
                                                shared_populate_nested(opt, &json_opts[i]);
                                            }
                                        }
                                    }
                                }
                                if let Some(ref mut ae) = alternative_effect {
                                    if let Some(ae_json) = json_val.get("alternative_effect") {
                                        shared_populate_nested(ae, ae_json);
                                    }
                                }
                            }
                            Some(EffectKind::AbilityOp {
                                ref mut gained_effect,
                                ..
                            }) => {
                                if let Some(ref mut ge) = gained_effect {
                                    if let Some(ge_json) = json_val.get("gained_effect") {
                                        shared_populate_nested(ge, ge_json);
                                    }
                                }
                            }
                            Some(EffectKind::CustomOp {
                                ref mut opponent_action,
                                ..
                            }) => {
                                if let Some(ref mut oa) = opponent_action {
                                    if let Some(oa_json) = json_val.get("opponent_action") {
                                        shared_populate_nested(oa, oa_json);
                                    }
                                }
                            }
                            Some(EffectKind::MiscOp {
                                ref mut options, ..
                            }) => {
                                if let Some(ref mut opts) = options {
                                    if let Some(json_opts) =
                                        json_val.get("options").and_then(|a| a.as_array())
                                    {
                                        for (i, opt) in opts.iter_mut().enumerate() {
                                            if i < json_opts.len() {
                                                shared_populate_nested(opt, &json_opts[i]);
                                            }
                                        }
                                    }
                                }
                            }
                            Some(EffectKind::SelectTarget {
                                ref mut options, ..
                            }) => {
                                if let Some(ref mut opts) = options {
                                    if let Some(json_opts) =
                                        json_val.get("options").and_then(|a| a.as_array())
                                    {
                                        for (i, opt) in opts.iter_mut().enumerate() {
                                            if i < json_opts.len() {
                                                shared_populate_nested(opt, &json_opts[i]);
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    fn shared_populate_condition(
                        cond: &mut Condition,
                        cond_json: &serde_json::Value,
                    ) {
                        if let Some(ref mut opts) = cond.options {
                            if let Some(json_opts) =
                                cond_json.get("options").and_then(|a| a.as_array())
                            {
                                for (i, opt) in opts.iter_mut().enumerate() {
                                    if i < json_opts.len() {
                                        shared_populate_nested(opt, &json_opts[i]);
                                    }
                                }
                            }
                        }
                        if let Some(ref mut eff) = cond.effect {
                            if let Some(eff_json) = cond_json.get("effect") {
                                shared_populate_nested(eff, eff_json);
                            }
                        }
                        if let Some(ref mut conditions) = cond.conditions {
                            if let Some(json_conditions) =
                                cond_json.get("conditions").and_then(|a| a.as_array())
                            {
                                for (i, sub_cond) in conditions.iter_mut().enumerate() {
                                    if i < json_conditions.len() {
                                        shared_populate_condition(sub_cond, &json_conditions[i]);
                                    }
                                }
                            }
                        }
                        if let Some(ref mut sub_cond) = cond.condition {
                            if let Some(sub_cond_json) = cond_json.get("condition") {
                                shared_populate_condition(sub_cond, sub_cond_json);
                            }
                        }
                    }

                    // Populate EffectKind for the cost
                    if let Some(ref mut cost) = ability.cost {
                        if let Some(ref cj) = cost_entry {
                            let cost_action = cj
                                .get("type")
                                .or_else(|| cj.get("action"))
                                .and_then(|v| v.as_str());
                            if let Some(act) = cost_action {
                                if let Some(kind) = build_kind_from_action(act, cj) {
                                    cost.0.kind = Some(kind);
                                }
                            }
                        }
                        // Populate EffectKind for sequential_cost sub-costs
                        if let Some(ref costs) = cost.0.compound.actions.clone() {
                            let cost_sub_actions = cost_entry
                                .as_ref()
                                .and_then(|cj| cj.get("costs").or_else(|| cj.get("actions")))
                                .and_then(|a| a.as_array())
                                .cloned();
                            if let Some(ref json_costs) = cost_sub_actions {
                                let mut fixed = costs.clone();
                                for (i, sub) in costs.iter().enumerate() {
                                    if i < json_costs.len() {
                                        if let Some(k) =
                                            build_kind_from_action(&sub.action, &json_costs[i])
                                        {
                                            if i < fixed.len() {
                                                fixed[i].kind = Some(k);
                                            }
                                        }
                                    }
                                }
                                cost.0.compound.actions = Some(fixed);
                            }
                        }
                    }

                    let shared = Arc::new(ability);

                    if let Some(card_list) = ability_entry.get("cards").and_then(|v| v.as_array()) {
                        for card_entry in card_list {
                            if let Some(card_str) = card_entry.as_str() {
                                if let Some(card_no) = card_str.split(" | ").next() {
                                    ability_map
                                        .entry(card_no.to_string())
                                        .or_default()
                                        .push(Arc::clone(&shared));
                                }
                            }
                        }
                    }
                }
            }
        }
        ability_map
    }

    /// Build a card_no → Vec<Ability> map from the parsed abilities JSON Value.
    /// Exposed so the desktop gen_abilities_map tool can pre-bake it for 3DS.
    pub fn build_abilities_map(
        abilities_data: &serde_json::Value,
    ) -> HashMap<String, Vec<Ability>> {
        let mut ability_map: HashMap<String, Vec<Ability>> = HashMap::new();

        if let Some(unique_abilities) = abilities_data
            .get("unique_abilities")
            .and_then(|v| v.as_array())
        {
            for ability_entry in unique_abilities {
                let mut entry = ability_entry.clone();
                if let Some(obj) = entry.as_object_mut() {
                    if let Some(tc_val) = obj.remove("trigger_condition") {
                        match obj.get_mut("condition") {
                            Some(cond_val) => {
                                let mut merged = serde_json::Map::new();
                                merged.insert("type".into(), "compound".into());
                                merged.insert("operator".into(), "and".into());
                                merged.insert(
                                    "conditions".into(),
                                    serde_json::Value::Array(vec![cond_val.take(), tc_val]),
                                );
                                obj.insert("condition".into(), serde_json::Value::Object(merged));
                            }
                            None => {
                                obj.insert("condition".into(), tc_val);
                            }
                        }
                    }
                }
                // Extract effect JSON, action, and sub-actions BEFORE consuming entry
                let effect_entry = entry.get("effect").cloned();
                let effect_action = effect_entry.as_ref().and_then(|ej| {
                    ej.get("type")
                        .or_else(|| ej.get("action"))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                });
                let effect_actions = effect_entry
                    .as_ref()
                    .and_then(|ej| ej.get("actions"))
                    .and_then(|a| a.as_array())
                    .cloned();
                // Extract cost JSON BEFORE consuming entry
                let cost_entry = entry.get("cost").cloned();

                if let Ok(mut ability) = serde_json::from_value::<Ability>(entry) {
                    // Populate EffectKind from the original effect JSON
                    if let Some(ref mut effect) = ability.effect {
                        if let Some(ref act) = effect_action {
                            if let Some(ref ej) = effect_entry {
                                if let Some(kind) = build_kind_from_action(act, ej) {
                                    effect.kind = Some(kind);
                                }
                            }
                        }
                    }
                    if let Some(ref mut effect) = ability.effect {
                        if let Some(ref actions) = effect.compound.actions.clone() {
                            let fixed_actions: Vec<crate::card::AbilityEffect> = actions
                                .iter()
                                .map(|action| {
                                    let mut fixed_action = action.clone();
                                    if (fixed_action.action == "draw"
                                        || fixed_action.action == "draw_card")
                                        && fixed_action.count.is_none()
                                        && fixed_action.dynamic_count_any().is_none()
                                    {
                                        fixed_action.count = Some(1);
                                    }
                                    fixed_action
                                })
                                .collect();
                            effect.compound.actions = Some(fixed_actions);
                        }
                    }

                    // Recursively populate EffectKind for ALL nested sub-effects
                    // MUST run before fixed_actions so dynamic_count_any() works.
                    if let Some(ref mut effect) = ability.effect {
                        if let Some(ref json_effect) = effect_entry {
                            map_populate_nested(effect, json_effect);
                        }
                    }
                    fn map_populate_nested(
                        effect: &mut AbilityEffect,
                        json_val: &serde_json::Value,
                    ) {
                        if let Some(kind) = build_kind_from_action(&effect.action, json_val) {
                            effect.kind = Some(kind);
                        }
                        if let Some(ref mut sub) = effect.compound.look_action {
                            if let Some(sub_json) = json_val.get("look_action") {
                                map_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut sub) = effect.compound.select_action {
                            if let Some(sub_json) = json_val.get("select_action") {
                                map_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut sub) = effect.compound.followup_action {
                            if let Some(sub_json) = json_val.get("followup_action") {
                                map_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut sub) = effect.compound.primary_effect {
                            if let Some(sub_json) = json_val.get("primary_effect") {
                                map_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut sub) = effect.compound.optional_action {
                            if let Some(sub_json) = json_val.get("optional_action") {
                                map_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut sub) = effect.compound.conditional_action {
                            if let Some(sub_json) = json_val.get("conditional_action") {
                                map_populate_nested(sub, sub_json);
                            }
                        }
                        if let Some(ref mut actions) = effect.compound.actions {
                            if let Some(json_actions) =
                                json_val.get("actions").and_then(|a| a.as_array())
                            {
                                for (i, action) in actions.iter_mut().enumerate() {
                                    if i < json_actions.len() {
                                        map_populate_nested(action, &json_actions[i]);
                                    }
                                }
                            }
                        }
                        if let Some(ref mut steps) = effect.effect_steps {
                            if let Some(json_steps) =
                                json_val.get("effect_steps").and_then(|a| a.as_array())
                            {
                                for (i, step) in steps.iter_mut().enumerate() {
                                    if i < json_steps.len() {
                                        map_populate_nested(step, &json_steps[i]);
                                    }
                                }
                            }
                        }
                        if let Some(ref mut cond) = effect.condition {
                            if let Some(cond_json) = json_val.get("condition") {
                                map_populate_condition(cond, cond_json);
                            }
                        }
                        match effect.kind.as_mut() {
                            Some(EffectKind::LookReveal {
                                ref mut options,
                                ref mut resource_on_select,
                                ..
                            }) => {
                                if let Some(ref mut opts) = options {
                                    if let Some(json_opts) =
                                        json_val.get("options").and_then(|a| a.as_array())
                                    {
                                        for (i, opt) in opts.iter_mut().enumerate() {
                                            if i < json_opts.len() {
                                                map_populate_nested(opt, &json_opts[i]);
                                            }
                                        }
                                    }
                                }
                                if let Some(ref mut ros) = resource_on_select {
                                    if let Some(ros_json) = json_val.get("resource_on_select") {
                                        map_populate_nested(ros, ros_json);
                                    }
                                }
                            }
                            Some(EffectKind::CompoundEffect {
                                ref mut options,
                                ref mut alternative_effect,
                                ..
                            }) => {
                                if let Some(ref mut opts) = options {
                                    if let Some(json_opts) =
                                        json_val.get("options").and_then(|a| a.as_array())
                                    {
                                        for (i, opt) in opts.iter_mut().enumerate() {
                                            if i < json_opts.len() {
                                                map_populate_nested(opt, &json_opts[i]);
                                            }
                                        }
                                    }
                                }
                                if let Some(ref mut ae) = alternative_effect {
                                    if let Some(ae_json) = json_val.get("alternative_effect") {
                                        map_populate_nested(ae, ae_json);
                                    }
                                }
                            }
                            Some(EffectKind::SelectTarget {
                                ref mut options, ..
                            }) => {
                                if let Some(ref mut opts) = options {
                                    if let Some(json_opts) =
                                        json_val.get("options").and_then(|a| a.as_array())
                                    {
                                        for (i, opt) in opts.iter_mut().enumerate() {
                                            if i < json_opts.len() {
                                                map_populate_nested(opt, &json_opts[i]);
                                            }
                                        }
                                    }
                                }
                            }
                            Some(EffectKind::AbilityOp {
                                ref mut gained_effect,
                                ..
                            }) => {
                                if let Some(ref mut ge) = gained_effect {
                                    if let Some(ge_json) = json_val.get("gained_effect") {
                                        map_populate_nested(ge, ge_json);
                                    }
                                }
                            }
                            Some(EffectKind::CustomOp {
                                ref mut opponent_action,
                                ..
                            }) => {
                                if let Some(ref mut oa) = opponent_action {
                                    if let Some(oa_json) = json_val.get("opponent_action") {
                                        map_populate_nested(oa, oa_json);
                                    }
                                }
                            }
                            Some(EffectKind::MiscOp {
                                ref mut options, ..
                            }) => {
                                if let Some(ref mut opts) = options {
                                    if let Some(json_opts) =
                                        json_val.get("options").and_then(|a| a.as_array())
                                    {
                                        for (i, opt) in opts.iter_mut().enumerate() {
                                            if i < json_opts.len() {
                                                map_populate_nested(opt, &json_opts[i]);
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    fn map_populate_condition(cond: &mut Condition, cond_json: &serde_json::Value) {
                        if let Some(ref mut opts) = cond.options {
                            if let Some(json_opts) =
                                cond_json.get("options").and_then(|a| a.as_array())
                            {
                                for (i, opt) in opts.iter_mut().enumerate() {
                                    if i < json_opts.len() {
                                        map_populate_nested(opt, &json_opts[i]);
                                    }
                                }
                            }
                        }
                        if let Some(ref mut eff) = cond.effect {
                            if let Some(eff_json) = cond_json.get("effect") {
                                map_populate_nested(eff, eff_json);
                            }
                        }
                        if let Some(ref mut conditions) = cond.conditions {
                            if let Some(json_conditions) =
                                cond_json.get("conditions").and_then(|a| a.as_array())
                            {
                                for (i, sub_cond) in conditions.iter_mut().enumerate() {
                                    if i < json_conditions.len() {
                                        map_populate_condition(sub_cond, &json_conditions[i]);
                                    }
                                }
                            }
                        }
                        if let Some(ref mut sub_cond) = cond.condition {
                            if let Some(sub_cond_json) = cond_json.get("condition") {
                                map_populate_condition(sub_cond, sub_cond_json);
                            }
                        }
                    }

                    // Populate EffectKind for the cost
                    if let Some(ref mut cost) = ability.cost {
                        if let Some(ref cj) = cost_entry {
                            let cost_action = cj
                                .get("type")
                                .or_else(|| cj.get("action"))
                                .and_then(|v| v.as_str());
                            if let Some(act) = cost_action {
                                if let Some(kind) = build_kind_from_action(act, cj) {
                                    cost.0.kind = Some(kind);
                                }
                            }
                        }
                        // Populate EffectKind for sequential_cost sub-costs
                        if let Some(ref costs) = cost.0.compound.actions.clone() {
                            let cost_sub_actions = cost_entry
                                .as_ref()
                                .and_then(|cj| cj.get("costs").or_else(|| cj.get("actions")))
                                .and_then(|a| a.as_array())
                                .cloned();
                            if let Some(ref json_costs) = cost_sub_actions {
                                let mut fixed = costs.clone();
                                for (i, sub) in costs.iter().enumerate() {
                                    if i < json_costs.len() {
                                        if let Some(k) =
                                            build_kind_from_action(&sub.action, &json_costs[i])
                                        {
                                            if i < fixed.len() {
                                                fixed[i].kind = Some(k);
                                            }
                                        }
                                    }
                                }
                                cost.0.compound.actions = Some(fixed);
                            }
                        }
                    }

                    if let Some(card_list) = ability_entry.get("cards").and_then(|v| v.as_array()) {
                        for card_entry in card_list {
                            if let Some(card_str) = card_entry.as_str() {
                                if let Some(card_no) = card_str.split(" | ").next() {
                                    ability_map
                                        .entry(card_no.to_string())
                                        .or_default()
                                        .push(ability.clone());
                                }
                            }
                        }
                    }
                } else {
                    log::debug!(
                        "Failed to deserialize ability entry: {}",
                        serde_json::to_string_pretty(ability_entry).unwrap_or_default()
                    );
                }
            }
        }
        ability_map
    }

    /// Apply a pre-baked deduplicated abilities index to a card list.
    /// abilities_index: flat list of unique Ability objects.
    /// card_index: card_no → indices into abilities_index.
    /// Used on 3DS with the gen_abilities_map pre-baked compact format.
    pub fn apply_abilities_index(
        mut cards: Vec<Card>,
        abilities_index: &[Ability],
        card_index: &HashMap<String, Vec<usize>>,
    ) -> Vec<Card> {
        // Build shared Arc<Ability> pool from the index to avoid per-card clones
        let shared_pool: Vec<Arc<Ability>> = abilities_index
            .iter()
            .map(|a| Arc::new(a.clone()))
            .collect();
        for card in &mut cards {
            if let Some(indices) = card_index.get(&card.card_no) {
                card.abilities = indices
                    .iter()
                    .filter_map(|&i| shared_pool.get(i))
                    .map(|a| Arc::clone(a))
                    .collect();
            }
        }
        cards
    }
}
