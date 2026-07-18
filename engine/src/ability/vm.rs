use super::abilities_gen::{Opcode, BYTECODE, NUM_ABILITIES, OFFSETS, STRINGS};
use crate::ability::enums::{ActionType, EffectCardType, EffectState};
use crate::card::{
    ek_box_new, Ability, AbilityCost, AbilityEffect, Condition, ConditionCardType, EffectKind,
    Operator,
};
use crate::core::types::ArcStr;

fn read_u8(c: &mut &[u8]) -> u8 {
    let b = c[0];
    *c = &c[1..];
    b
}
fn read_u16(c: &mut &[u8]) -> u16 {
    let v = u16::from_le_bytes([c[0], c[1]]);
    *c = &c[2..];
    v
}
fn read_i8(c: &mut &[u8]) -> i8 {
    let b = c[0] as i8;
    *c = &c[1..];
    b
}
fn read_str(c: &mut &[u8]) -> Option<&'static str> {
    let idx = read_u16(c) as usize;
    if idx == 0xFFFF {
        None
    } else {
        Some(STRINGS[idx])
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
        let op_val = read_u8(&mut cursor);
        let op = match Opcode::try_from(op_val) {
            Ok(o) => o,
            Err(_) => break,
        };

        if op_val >= 0x80 {
            cost_eff = Some(decode_cost_op(op, &mut cursor));
        } else if op_val == 0x60 {
            let n = read_u8(&mut cursor);
            let mut steps = Vec::with_capacity(n as usize);
            for _ in 0..n {
                steps.push(Box::new(decode_effect_from_slice(&mut cursor)));
            }
            effect = Some(AbilityEffect {
                action: "sequential".into(),
                effect_steps: Some(steps),
                ..Default::default()
            });
        } else if op_val == 0x61 {
            let cond_len = read_u16(&mut cursor) as usize;
            let (cc_data, rest1) = cursor.split_at(cond_len);
            let mut cc = cc_data;
            cursor = rest1;
            let cond = decode_condition(&mut cc);
            let body_len = read_u16(&mut cursor) as usize;
            let (body, rest2) = cursor.split_at(body_len);
            cursor = rest2;
            let alt_len = read_u16(&mut cursor) as usize;
            let alt = if alt_len > 0 {
                let (ac_data, rest3) = cursor.split_at(alt_len);
                let mut ac = ac_data;
                cursor = rest3;
                Some(Box::new(decode_effect_from_slice(&mut ac)))
            } else {
                None
            };
            let primary = decode_effect_from_slice(&mut &body[..]);
            effect = Some(match alt {
                Some(a) => AbilityEffect {
                    action: "conditional_alternative".into(),
                    condition: Some(Box::new(cond)),
                    effect_steps: Some(vec![Box::new(primary), a]),
                    ..Default::default()
                },
                None => primary,
            });
        } else if op_val == 0x63 {
            effect = Some(AbilityEffect {
                action: "conditional_on_optional".into(),
                optional: Some(read_u8(&mut cursor) != 0),
                ..Default::default()
            });
        } else if op_val == 0x64 {
            effect = Some(AbilityEffect {
                action: "conditional_on_result".into(),
                ..Default::default()
            });
        } else if op_val == 0x65 {
            let count = read_u8(&mut cursor);
            let group_names = read_str(&mut cursor);
            let cc_len = read_u16(&mut cursor) as usize;
            let choice_cond = (cc_len > 0).then(|| {
                let (cd, r) = cursor.split_at(cc_len);
                cursor = r;
                let mut cc = cd;
                Box::new(decode_condition(&mut cc))
            });
            let ac_len = read_u16(&mut cursor) as usize;
            let alt_cond = (ac_len > 0).then(|| {
                let (ad, r) = cursor.split_at(ac_len);
                cursor = r;
                let mut ac = ad;
                Box::new(decode_condition(&mut ac))
            });
            let alt_count_type = read_u8(&mut cursor);
            let num_opts = read_u8(&mut cursor);
            let mut options = Vec::with_capacity(num_opts as usize);
            for _ in 0..num_opts {
                options.push(Box::new(decode_effect_from_slice(&mut cursor)));
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
            effect = Some(AbilityEffect {
                action: "choice".into(),
                count: Some(count as u32),
                kind: Some(ek_box_new(ek)),
                compound: crate::card::CompoundBranch {
                    actions: Some(options.iter().map(|o| (*o).clone()).collect()),
                    ..Default::default()
                },
                ..Default::default()
            });
        } else if op_val == 0x70 {
            let count = read_u8(&mut cursor);
            let source = decode_zone(read_u8(&mut cursor));
            let target = decode_player(read_u8(&mut cursor));
            let mut steps = Vec::new();
            steps.push(Box::new(AbilityEffect {
                action: "look_at".into(),
                count: Some(count as u32),
                source: Some(source.into()),
                target: Some(target.into()),
                ..Default::default()
            }));
            if !cursor.is_empty() && cursor[0] == Opcode::SelectCards as u8 {
                let _ = read_u8(&mut cursor);
                let sc = read_u8(&mut cursor);
                let dest = decode_zone(read_u8(&mut cursor));
                let _dr = read_u8(&mut cursor);
                steps.push(Box::new(AbilityEffect {
                    action: "select_cards".into(),
                    count: Some(sc as u32),
                    destination: Some(dest.into()),
                    ..Default::default()
                }));
            }
            effect = Some(AbilityEffect {
                action: "look_and_select".into(),
                count: Some(count as u32),
                source: Some(source.into()),
                target: Some(target.into()),
                effect_steps: Some(steps),
                ..Default::default()
            });
        } else if op_val == 0x62 {
            // conditional_alternative sub-type marker (handled by 0x61 wrapper)
        } else if op_val == 0x71 {
            // select_cards sub-opcode (handled by 0x70)
        } else {
            if let Some(mut ek) = decode_effect_kind(op, &mut cursor) {
                let action = action_for_op(op);
                let mut ae = AbilityEffect {
                    action: action.into(),
                    kind: Some(ek_box_new((*ek).clone())),
                    ..Default::default()
                };
                set_direct_fields(&mut ae);
                effect = match effect.take() {
                    None => Some(ae),
                    Some(mut seq) => {
                        seq.effect_steps
                            .get_or_insert_with(Vec::new)
                            .push(Box::new(ae));
                        seq.action = "sequential".into();
                        Some(seq)
                    }
                };
            }
        }
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
            let mut ek = default_moveCards();
            if let EffectKind::MoveCards {
                source: ref mut s,
                destination: ref mut d,
                card_type: ref mut c,
                count: ref mut n,
                ..
            } = &mut ek
            {
                *s = Some(src.into());
                *d = Some(dest.into());
                *c = Some(ct.into());
                *n = Some(count as u32);
            }
            AbilityEffect {
                action: "move_cards".into(),
                source: Some(src.into()),
                destination: Some(dest.into()),
                count: Some(count as u32),
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
            let mut ek = default_moveCards();
            if let EffectKind::MoveCards {
                count: ref mut n,
                card_type: ref mut t,
                ..
            } = &mut ek
            {
                *n = Some(c as u32);
                *t = Some(ct.into());
            }
            AbilityEffect {
                action: "discard".into(),
                count: Some(c as u32),
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
    match v {
        0 => "hand",
        1 => "stage",
        2 => "center",
        3 => "left",
        4 => "right",
        5 => "discard",
        6 => "waitroom",
        7 => "energy",
        9 => "deck",
        10 => "deck_top",
        11 => "deck_bottom",
        12 => "success_zone",
        13 => "live_card_zone",
        15 => "energy_deck",
        18 => "under_member",
        19 => "looked_at",
        20 => "revealed_cards",
        21 => "selected_cards",
        22 => "resolution",
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
