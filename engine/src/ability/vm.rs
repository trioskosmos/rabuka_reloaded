use super::abilities_gen::{Opcode, BYTECODE, NUM_ABILITIES, OFFSETS, STRINGS};
use crate::ability::enums::{ActionType, EffectCardType, EffectState};
use crate::card::{
    ek_box_new, Ability, AbilityCost, AbilityEffect, Condition, ConditionCardType, EffectKind,
    Operator,
};
use crate::core::types::ArcStr;

fn read_u8(c: &mut &[u8]) -> u8 {
    if c.is_empty() {
        return 0;
    }
    let b = c[0];
    *c = &c[1..];
    b
}
fn read_u16(c: &mut &[u8]) -> u16 {
    if c.len() < 2 {
        return 0;
    }
    let v = u16::from_le_bytes([c[0], c[1]]);
    *c = &c[2..];
    v
}
fn read_i8(c: &mut &[u8]) -> i8 {
    if c.is_empty() {
        return 0;
    }
    let b = c[0] as i8;
    *c = &c[1..];
    b
}
fn read_str(c: &mut &[u8]) -> Option<&'static str> {
    let idx = read_u16(c) as usize;
    if idx == 0xFFFF {
        None
    } else if idx < STRINGS.len() {
        Some(STRINGS[idx])
    } else {
        None
    }
}

/// Decode a length-prefixed list of interned strings: `u8 count` followed by
/// `count` little-endian `u16` string-table indices. Used for `Vec<String>`
/// effect/condition fields (e.g. `heart_colors`, `group_names`).
fn read_str_list(c: &mut &[u8]) -> Vec<String> {
    let n = read_u8(c) as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let idx = read_u16(c) as usize;
        if idx != 0xFFFF && idx < STRINGS.len() {
            out.push(STRINGS[idx].to_string());
        }
    }
    out
}

// ── Presence-aware scalar readers ──
// Absent fields are encoded as the 0xFF (or 0xFFFF) sentinel and decode to
// `None`, so the bytecode path matches the JSON loader (which leaves unset
// fields as `None`).
fn read_u8_opt(c: &mut &[u8]) -> Option<u8> {
    let b = read_u8(c);
    if b == 0xFF {
        None
    } else {
        Some(b)
    }
}
fn read_u16_opt(c: &mut &[u8]) -> Option<u16> {
    let v = read_u16(c);
    if v == 0xFFFF {
        None
    } else {
        Some(v)
    }
}
fn read_bool_opt(c: &mut &[u8]) -> Option<bool> {
    match read_u8(c) {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}
fn read_zone_opt(c: &mut &[u8]) -> Option<&'static str> {
    read_u8_opt(c).map(decode_zone)
}
fn read_player_opt(c: &mut &[u8]) -> Option<&'static str> {
    read_u8_opt(c).map(decode_player)
}
fn read_card_type_opt(c: &mut &[u8]) -> Option<&'static str> {
    read_u8_opt(c).map(decode_card_type)
}
fn read_resource_opt(c: &mut &[u8]) -> Option<&'static str> {
    read_u8_opt(c).map(decode_resource)
}
fn read_heart_opt(c: &mut &[u8]) -> Option<&'static str> {
    read_u8_opt(c).map(decode_heart)
}
fn read_duration_opt(c: &mut &[u8]) -> Option<&'static str> {
    read_u8_opt(c).map(decode_duration)
}
fn read_operator_opt(c: &mut &[u8]) -> Option<&'static str> {
    read_u8_opt(c).map(decode_operator)
}
fn read_state_opt(c: &mut &[u8]) -> Option<EffectState> {
    read_u8_opt(c).map(decode_state)
}
fn read_i8_opt(c: &mut &[u8]) -> Option<i8> {
    let b = read_u8(c);
    if b == 0xFF {
        None
    } else {
        Some(b as i8)
    }
}

include!("vm_gen.rs");

pub fn ability_count() -> usize {
    NUM_ABILITIES
}

pub fn get_ability(idx: usize) -> Option<Ability> {
    if idx >= NUM_ABILITIES {
        return None;
    }
    let start = OFFSETS[idx] as usize;
    let end = OFFSETS[idx + 1] as usize;
    if start >= end {
        return Some(Ability::default());
    }
    let mut cursor = &BYTECODE[start..end];
    let mut cost_eff: Option<AbilityEffect> = None;
    let mut effect: Option<AbilityEffect> = None;

    while !cursor.is_empty() {
        // Peek at the opcode without consuming it. Cost opcodes (0x80+) route to
        // the cost slot; everything else is an effect (including the
        // compound/control shapes 0x60-0x65, 0x70/0x71), decoded uniformly by
        // `decode_effect_from_slice`, which reads the opcode byte itself.
        let op_val = cursor[0];

        if op_val >= 0x80 {
            let op_val = read_u8(&mut cursor);
            match Opcode::try_from(op_val) {
                Ok(op) => cost_eff = Some(decode_cost_op(op, &mut cursor)),
                Err(_) => break,
            }
            continue;
        }

        let eff = decode_effect_from_slice(&mut cursor);
        if eff.action == ActionType::Custom && eff.kind.is_none() && eff.effect_steps.is_none()
            && eff.condition.is_none() && eff.compound.actions.is_none()
        {
            // Unrecognized opcode produced an empty effect; stop to avoid desync.
            break;
        }
        // Chain multiple top-level effects into a sequential wrapper, matching
        // the JSON loader's behaviour.
        effect = match effect.take() {
            None => Some(eff),
            Some(prev) => {
                let mut seq = if prev.action == ActionType::Sequential && prev.kind.is_none() {
                    prev
                } else {
                    AbilityEffect {
                        action: "sequential".into(),
                        effect_steps: Some(vec![Box::new(prev)]),
                        ..Default::default()
                    }
                };
                seq.effect_steps
                    .get_or_insert_with(Vec::new)
                    .push(Box::new(eff));
                seq.action = "sequential".into();
                Some(seq)
            }
        };
    }

    Some(Ability {
        full_text: String::new(),
        triggerless_text: None,
        triggers: None,
        use_limit: None,
        is_null: false,
        cost: cost_eff.map(|ce| Box::new(AbilityCost(ce))),
        effect: effect.map(Box::new),
        keywords: None,
    })
}

fn set_direct_fields(eff: &mut AbilityEffect) {
    if let Some(ref ek) = eff.kind.as_ref().map(|k| k.as_ref()) {
        match ek {
            EffectKind::DrawCards {
                target_count,
                source,
                ..
            } => {
                eff.count = *target_count;
                eff.source.clone_from(source);
            }
            EffectKind::MoveCards {
                count,
                source,
                destination,
                target,
                ..
            } => {
                eff.count = *count;
                eff.source.clone_from(source);
                eff.destination.clone_from(destination);
                eff.target.clone_from(target);
            }
            EffectKind::GainResource { value, .. } => {
                eff.count = *value;
            }
            EffectKind::ModifyScore { value, target, .. } => {
                eff.count = *value;
                eff.target.clone_from(target);
            }
            EffectKind::ChangeState { target, .. } => {
                eff.target.clone_from(target);
            }
            EffectKind::SelectTarget { target, .. } => {
                eff.target.clone_from(target);
            }
            EffectKind::PositionOp { target, .. } => {
                eff.target.clone_from(target);
            }
            _ => {}
        };
    }
}

fn decode_effect_from_slice(cursor: &mut &[u8]) -> AbilityEffect {
    if cursor.is_empty() {
        return AbilityEffect::default();
    }
    let op_val = read_u8(cursor);

    // look_and_select (0x70) + optional select_cards sub-op (0x71)
    if op_val == 0x70 {
        let count = read_u8(cursor);
        let source = decode_zone(read_u8(cursor));
        let target = decode_player(read_u8(cursor));
        let mut steps = Vec::new();
        steps.push(Box::new(AbilityEffect {
            action: "look_at".into(),
            count: Some(count as u32),
            source: Some(source.into()),
            target: Some(target.into()),
            ..Default::default()
        }));
        if !cursor.is_empty() && cursor[0] == 0x71 {
            let _ = read_u8(cursor);
            let sc = read_u8(cursor);
            let dest = decode_zone(read_u8(cursor));
            let _dr = read_u8(cursor);
            steps.push(Box::new(AbilityEffect {
                action: "select_cards".into(),
                count: Some(sc as u32),
                destination: Some(dest.into()),
                ..Default::default()
            }));
        }
        return AbilityEffect {
            action: "look_and_select".into(),
            count: Some(count as u32),
            source: Some(source.into()),
            target: Some(target.into()),
            effect_steps: Some(steps),
            ..Default::default()
        };
    }
    if op_val == 0x71 || op_val == 0x62 {
        // sub-opcode markers (handled by their parent wrapper); emit empty.
        return AbilityEffect::default();
    }

    // sequential (0x60)
    if op_val == 0x60 {
        let n = read_u8(cursor);
        let mut steps = Vec::with_capacity(n as usize);
        for _ in 0..n {
            steps.push(Box::new(decode_effect_from_slice(cursor)));
        }
        return AbilityEffect {
            action: "sequential".into(),
            effect_steps: Some(steps),
            ..Default::default()
        };
    }

    // conditional_alternative (0x61)
    if op_val == 0x61 {
        let cond_len = read_u16(cursor) as usize;
        let (cc_data, rest1) = cursor.split_at(cond_len);
        let mut cc = cc_data;
        *cursor = rest1;
        let cond = decode_condition(&mut cc);
        let body_len = read_u16(cursor) as usize;
        let (body, rest2) = cursor.split_at(body_len);
        *cursor = rest2;
        let alt_len = read_u16(cursor) as usize;
        let alt = if alt_len > 0 {
            let (ac_data, rest3) = cursor.split_at(alt_len);
            let mut ac = ac_data;
            *cursor = rest3;
            Some(Box::new(decode_effect_from_slice(&mut ac)))
        } else {
            None
        };
        let primary = decode_effect_from_slice(&mut &body[..]);
        return match alt {
            Some(a) => AbilityEffect {
                action: "conditional_alternative".into(),
                condition: Some(Box::new(cond)),
                effect_steps: Some(vec![Box::new(primary), a]),
                ..Default::default()
            },
            None => {
                // No alternative branch: this is a plain condition-gated effect.
                // Preserve the trigger condition on the primary effect so the
                // resolver's condition gate fires correctly (matches the JSON
                // loader, which keeps `effect.condition` on a condition-wrapped
                // sequential). Dropping it would make the ability fire
                // unconditionally.
                let mut p = primary;
                p.condition = Some(Box::new(cond));
                p
            }
        };
    }

    // choice (0x65)
    if op_val == 0x65 {
        let count = read_u8(cursor);
        let group_names = read_str(cursor);
        let cc_len = read_u16(cursor) as usize;
        let choice_cond = (cc_len > 0).then(|| {
            let (cd, r) = cursor.split_at(cc_len);
            *cursor = r;
            let mut cc = cd;
            Box::new(decode_condition(&mut cc))
        });
        let ac_len = read_u16(cursor) as usize;
        let alt_cond = (ac_len > 0).then(|| {
            let (ad, r) = cursor.split_at(ac_len);
            *cursor = r;
            let mut ac = ad;
            Box::new(decode_condition(&mut ac))
        });
        let alt_count_type = read_u8(cursor);
        let num_opts = read_u8(cursor);
        let mut options = Vec::with_capacity(num_opts as usize);
        for _ in 0..num_opts {
            options.push(Box::new(decode_effect_from_slice(cursor)));
        }
        let mut ek = default_compoundEffect();
        if let EffectKind::CompoundEffect {
            options: ref mut bc_o,
            alternative_count_type: ref mut bc_act,
            group_names: ref mut bc_gn,
            choice_condition: ref mut bc_cc,
            alternative_condition: ref mut bc_ac,
            ..
        } = &mut ek
        {
            *bc_o = Some(options.iter().map(|o| (*o).clone()).collect());
            *bc_act = (alt_count_type != 0).then(|| "any_number".into());
            if let Some(s) = group_names {
                *bc_gn = Some(Box::new(vec![s.to_string()]));
            }
            *bc_cc = choice_cond;
            *bc_ac = alt_cond;
        }
        return AbilityEffect {
            action: "choice".into(),
            count: Some(count as u32),
            kind: Some(ek_box_new(ek)),
            compound: crate::card::CompoundBranch {
                actions: Some(options.iter().map(|o| (*o).clone()).collect()),
                ..Default::default()
            },
            ..Default::default()
        };
    }

    // generic effect / cost opcodes present in the Opcode enum
    if let Ok(op) = Opcode::try_from(op_val) {
        if op_val >= 0x80 {
            return decode_cost_op(op, cursor);
        }
        if let Some(kind) = decode_effect_kind(op, cursor) {
            let action = action_for_op(op);
            let mut ae = AbilityEffect {
                action: action.into(),
                kind: Some(ek_box_new(*kind)),
                ..Default::default()
            };
            set_direct_fields(&mut ae);
            return ae;
        }
    }
    AbilityEffect::default()
}

fn decode_cost_op(op: Opcode, cursor: &mut &[u8]) -> AbilityEffect {
    match op {
        Opcode::MoveCardsCost => {
            let src = decode_zone(read_u8(cursor));
            let dest = decode_zone(read_u8(cursor));
            let ct = decode_card_type(read_u8(cursor));
            let _sc = read_u8(cursor);
            let count = read_u8(cursor);
            let optional = read_u8(cursor) != 0;
            let any_number = read_u8(cursor) != 0;
            let group_names = read_str_list(cursor);
            let characters = read_str_list(cursor);
            let mut ek = default_moveCards();
            if let EffectKind::MoveCards {
                source: ref mut s,
                destination: ref mut d,
                card_type: ref mut c,
                count: ref mut n,
                any_number: ref mut an,
                group_names: ref mut gn,
                characters: ref mut ch,
                ..
            } = &mut ek
            {
                *s = Some(src.into());
                *d = Some(dest.into());
                *c = Some(EffectCardType::from_str(ct));
                *n = Some(count as u32);
                *an = Some(any_number);
                if !group_names.is_empty() {
                    *gn = Some(Box::new(group_names));
                }
                if !characters.is_empty() {
                    *ch = Some(Box::new(characters));
                }
            }
            AbilityEffect {
                action: "move_cards".into(),
                source: Some(src.into()),
                destination: Some(dest.into()),
                count: Some(count as u32),
                optional: Some(optional),
                kind: Some(ek_box_new(ek)),
                ..Default::default()
            }
        }
        Opcode::Tap => AbilityEffect {
            action: "tap".into(),
            ..Default::default()
        },
        Opcode::Rest => {
            let c = read_u8(cursor);
            let mut ek = default_changeState();
            if let EffectKind::ChangeState {
                state_change: ref mut s,
                ..
            } = &mut ek
            {
                *s = Some(EffectState::Other(ArcStr::from("rest")));
            }
            AbilityEffect {
                action: "rest".into(),
                count: Some(c as u32),
                kind: Some(ek_box_new(ek)),
                ..Default::default()
            }
        }
        Opcode::EnergyCost => {
            let a = read_u8(cursor);
            let _ = read_u8(cursor);
            let mut ek = default_moveCards();
            if let EffectKind::MoveCards {
                energy_count: ref mut e,
                ..
            } = &mut ek
            {
                *e = Some(a as u32);
            }
            AbilityEffect {
                action: "pay_energy".into(),
                count: Some(a as u32),
                kind: Some(ek_box_new(ek)),
                ..Default::default()
            }
        }
        Opcode::DiscardCost => {
            let c = read_u8(cursor);
            let ct = decode_card_type(read_u8(cursor));
            let optional = read_u8(cursor) != 0;
            let any_number = read_u8(cursor) != 0;
            let group_names = read_str_list(cursor);
            let characters = read_str_list(cursor);
            let mut ek = default_moveCards();
            if let EffectKind::MoveCards {
                count: ref mut n,
                card_type: ref mut t,
                any_number: ref mut an,
                group_names: ref mut gn,
                characters: ref mut ch,
                ..
            } = &mut ek
            {
                *n = Some(c as u32);
                *t = Some(EffectCardType::from_str(ct));
                *an = Some(any_number);
                if !group_names.is_empty() {
                    *gn = Some(Box::new(group_names));
                }
                if !characters.is_empty() {
                    *ch = Some(Box::new(characters));
                }
            }
            AbilityEffect {
                action: "discard".into(),
                count: Some(c as u32),
                optional: Some(optional),
                kind: Some(ek_box_new(ek)),
                ..Default::default()
            }
        }
        Opcode::PlaceEnergyUnderMemberCost => {
            let c = read_u8(cursor);
            let mut ek = default_moveCards();
            if let EffectKind::MoveCards {
                count: ref mut n, ..
            } = &mut ek
            {
                *n = Some(c as u32);
            }
            AbilityEffect {
                action: "place_energy_under_member".into(),
                count: Some(c as u32),
                kind: Some(ek_box_new(ek)),
                ..Default::default()
            }
        }
        Opcode::PayEnergyCost => {
            let a = read_u8(cursor);
            let _ = read_u8(cursor);
            let mut ek = default_moveCards();
            if let EffectKind::MoveCards {
                energy_count: ref mut e,
                ..
            } = &mut ek
            {
                *e = Some(a as u32);
            }
            AbilityEffect {
                action: "pay_energy".into(),
                count: Some(a as u32),
                kind: Some(ek_box_new(ek)),
                ..Default::default()
            }
        }
        Opcode::ChangeStateCost => {
            let s = decode_state(read_u8(cursor));
            let _ = read_u8(cursor);
            let _ = read_u8(cursor);
            let mut ek = default_changeState();
            if let EffectKind::ChangeState {
                state_change: ref mut st,
                ..
            } = &mut ek
            {
                *st = Some(s);
            }
            AbilityEffect {
                action: "change_state".into(),
                kind: Some(ek_box_new(ek)),
                ..Default::default()
            }
        }
        Opcode::SequentialCost => {
            let n = read_u8(cursor);
            let mut steps = Vec::new();
            for _ in 0..n {
                if !cursor.is_empty() {
                    let sv = read_u8(cursor);
                    if let Ok(so) = Opcode::try_from(sv) {
                        steps.push(Box::new(decode_cost_op(so, cursor)));
                    }
                }
            }
            AbilityEffect {
                action: "sequential_cost".into(),
                compound: crate::card::CompoundBranch {
                    actions: Some(steps),
                    ..Default::default()
                },
                ..Default::default()
            }
        }
        Opcode::RevealCost => {
            let _ = read_u8(cursor);
            let _ = read_u8(cursor);
            let _ = read_u8(cursor);
            AbilityEffect {
                action: "reveal".into(),
                ..Default::default()
            }
        }
        Opcode::ChoiceCondition => {
            let n = read_u8(cursor);
            let mut opts = Vec::new();
            for _ in 0..n {
                if !cursor.is_empty() {
                    let sv = read_u8(cursor);
                    if let Ok(so) = Opcode::try_from(sv) {
                        opts.push(Box::new(decode_cost_op(so, cursor)));
                    }
                }
            }
            AbilityEffect {
                action: "choice".into(),
                compound: crate::card::CompoundBranch {
                    actions: Some(opts),
                    ..Default::default()
                },
                ..Default::default()
            }
        }
        _ => AbilityEffect {
            action: "".into(),
            ..Default::default()
        },
    }
}

fn decode_zone(v: u8) -> &'static str {
    // Must stay in sync with ZONE_ENCODE in cards/compile_abilities.py.
    match v {
        0 => "hand",
        1 => "stage",
        2 => "center",
        3 => "left",
        4 => "right",
        5 => "discard",
        6 => "waitroom",
        7 => "energy",
        8 => "energy_zone",
        9 => "deck",
        10 => "deck_top",
        11 => "deck_bottom",
        12 => "success_zone",
        13 => "live_card_zone",
        14 => "success_live_zone",
        15 => "energy_deck",
        16 => "empty_area",
        17 => "same_area",
        18 => "under_member",
        19 => "looked_at",
        20 => "revealed_cards",
        21 => "selected_cards",
        22 => "resolution",
        23 => "exclusion_zone",
        24 => "deck_or_discard",
        _ => "hand",
    }
}
fn decode_player(v: u8) -> &'static str {
    match v {
        0 => "self",
        1 => "opponent",
        2 => "both",
        3 => "owner",
        _ => "self",
    }
}
fn decode_card_type(v: u8) -> &'static str {
    match v {
        0 => "card",
        1 => "member_card",
        2 => "live_card",
        3 => "energy_card",
        4 => "event_card",
        5 => "character_card",
        6 => "baton_touch_card",
        7 => "climax_card",
        _ => "card",
    }
}
fn decode_resource(v: u8) -> &'static str {
    match v {
        0 => "heart",
        1 => "blade",
        2 => "yell",
        3 => "shield",
        4 => "energy",
        _ => "heart",
    }
}
fn decode_heart(v: u8) -> &'static str {
    match v {
        0 => "smile",
        1 => "pure",
        2 => "cool",
        3 => "active",
        4 => "natural",
        5 => "elegant",
        _ => "smile",
    }
}
fn decode_state(v: u8) -> EffectState {
    match v {
        0 => EffectState::Other(ArcStr::from("rest")),
        1 => EffectState::Other(ArcStr::from("stand")),
        2 => EffectState::Other(ArcStr::from("reverse")),
        3 => EffectState::Wait,
        _ => EffectState::Other(ArcStr::from("rest")),
    }
}
fn decode_duration(v: u8) -> &'static str {
    match v {
        0 => "this_turn",
        1 => "until_end_of_live",
        2 => "permanent",
        3 => "until_used",
        4 => "next_turn",
        _ => "this_turn",
    }
}
fn decode_operator(v: u8) -> &'static str {
    match v {
        0 => "=",
        1 => "!=",
        2 => ">",
        3 => ">=",
        4 => "<",
        5 => "<=",
        _ => ">=",
    }
}

fn decode_effect_card_type(v: u8) -> EffectCardType {
    match v {
        0 => EffectCardType::MemberCard,
        1 => EffectCardType::LiveCard,
        2 => EffectCardType::EnergyCard,
        _ => EffectCardType::Other(ArcStr::from("")),
    }
}
fn decode_action_type(v: u8) -> ActionType {
    match v {
        0 => ActionType::DrawCard,
        1 => ActionType::MoveCards,
        2 => ActionType::GainResource,
        3 => ActionType::ModifyScore,
        4 => ActionType::ChangeState,
        5 => ActionType::PositionChange,
        6 => ActionType::Custom,
        7 => ActionType::SetBladeCount,
        8 => ActionType::GainAbility,
        9 => ActionType::Restriction,
        10 => ActionType::Select,
        11 => ActionType::Reveal,
        _ => ActionType::Custom,
    }
}

impl From<&str> for EffectCardType {
    fn from(s: &str) -> Self {
        EffectCardType::from_str(s)
    }
}
impl From<&str> for EffectState {
    fn from(s: &str) -> Self {
        EffectState::from_str(s)
    }
}
impl From<&str> for ActionType {
    fn from(s: &str) -> Self {
        ActionType::from_str(s).unwrap_or(ActionType::Custom)
    }
}
