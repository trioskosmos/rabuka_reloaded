use super::abilities_gen::{Opcode, BYTECODE, NUM_ABILITIES, OFFSETS, STRINGS};
use crate::card::{ek_box_new, Ability, AbilityCost, AbilityEffect, Condition, EffectKind};

fn read_u8(cursor: &mut &[u8]) -> u8 {
    let b = cursor[0];
    *cursor = &cursor[1..];
    b
}
fn read_u16(cursor: &mut &[u8]) -> u16 {
    let v = u16::from_le_bytes([cursor[0], cursor[1]]);
    *cursor = &cursor[2..];
    v
}
fn read_i8(cursor: &mut &[u8]) -> i8 {
    let b = cursor[0] as i8;
    *cursor = &cursor[1..];
    b
}
fn read_str(cursor: &mut &[u8]) -> Option<&'static str> {
    let idx = read_u16(cursor) as usize;
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
        } else {
            let eff = decode_op_into(op, &mut cursor);
            effect = match effect.take() {
                None => Some(eff),
                Some(mut seq) => {
                    let mut steps = seq.effect_steps.take().unwrap_or_default();
                    steps.push(Box::new(eff));
                    seq.effect_steps = Some(steps);
                    seq.action = "sequential".into();
                    Some(seq)
                }
            };
        }
    }

    Some(Ability {
        full_text: String::new(),
        triggerless_text: String::new(),
        triggers: None,
        use_limit: None,
        is_null: false,
        cost: cost_eff.map(|ce| Box::new(AbilityCost(ce))),
        effect: effect.map(Box::new),
        keywords: None,
    })
}

fn decode_op_into(op: Opcode, cursor: &mut &[u8]) -> AbilityEffect {
    match op {
        Opcode::Sequential => {
            let count = read_u8(cursor);
            let mut steps = Vec::with_capacity(count as usize);
            for _ in 0..count {
                if !cursor.is_empty() {
                    let sub_op_val = read_u8(cursor);
                    if let Ok(sub_op) = Opcode::try_from(sub_op_val) {
                        steps.push(Box::new(decode_op_into(sub_op, cursor)));
                    }
                }
            }
            return AbilityEffect {
                action: "sequential".into(),
                effect_steps: Some(steps),
                ..Default::default()
            };
        }
        Opcode::Conditional => {
            let cond_len = read_u16(cursor) as usize;
            *cursor = &cursor[cond_len.min(cursor.len())..];
            let body_len = read_u16(cursor) as usize;
            let body = &cursor[..body_len];
            *cursor = &cursor[body_len..];
            let alt_len = read_u16(cursor) as usize;
            let alt = if alt_len > 0 {
                let mut ac = &cursor[..alt_len];
                *cursor = &cursor[alt_len..];
                let aop_val = read_u8(&mut ac);
                if let Ok(aop) = Opcode::try_from(aop_val) {
                    Some(Box::new(decode_op_into(aop, &mut ac)))
                } else {
                    None
                }
            } else {
                None
            };
            let primary = decode_effect_from_slice(body);
            if let Some(alt_eff) = alt {
                return AbilityEffect {
                    action: "conditional_alternative".into(),
                    effect_steps: Some(vec![Box::new(primary), alt_eff]),
                    ..Default::default()
                };
            }
            return primary;
        }
        Opcode::LookAt => {
            let count = read_u8(cursor);
            let source = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let mut steps: Vec<Box<AbilityEffect>> = Vec::new();
            steps.push(Box::new(AbilityEffect {
                action: "look_at".into(),
                count: Some(count as u32),
                source: Some(source.into()),
                target: Some(target.into()),
                ..Default::default()
            }));
            if !cursor.is_empty() && cursor[0] == Opcode::SelectCards as u8 {
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
        Opcode::DrawCard | Opcode::DrawUntilCount => {
            let count = read_u8(cursor);
            let source = decode_zone(read_u8(cursor));
            return AbilityEffect {
                action: action_for_op(op).into(),
                count: Some(count as u32),
                source: Some(source.into()),
                ..Default::default()
            };
        }
        Opcode::MoveCards => {
            let count = read_u8(cursor);
            let source = decode_zone(read_u8(cursor));
            let dest = decode_zone(read_u8(cursor));
            let _ct = read_u8(cursor);
            let target = decode_player(read_u8(cursor));
            return AbilityEffect {
                action: "move_cards".into(),
                count: Some(count as u32),
                source: Some(source.into()),
                destination: Some(dest.into()),
                target: Some(target.into()),
                ..Default::default()
            };
        }
        Opcode::GainResource => {
            // Consume all 6 bytes: resource, count, heart, duration, str_idx(u16)
            let _res = read_u8(cursor);
            let count = read_u8(cursor);
            let _heart = read_u8(cursor);
            let _dur = read_u8(cursor);
            let _str = read_u16(cursor);
            return AbilityEffect {
                action: "gain_resource".into(),
                count: Some(count as u32),
                ..Default::default()
            };
        }
        Opcode::ModifyScore => {
            let val = read_i8(cursor) as u32;
            let _pu = read_u8(cursor);
            let target = decode_player(read_u8(cursor));
            return AbilityEffect {
                action: "modify_score".into(),
                count: Some(val),
                target: Some(target.into()),
                ..Default::default()
            };
        }
        Opcode::ChangeState => {
            let _state = read_u8(cursor);
            let target = decode_player(read_u8(cursor));
            return AbilityEffect {
                action: "change_state".into(),
                target: Some(target.into()),
                ..Default::default()
            };
        }
        Opcode::PositionChange => {
            let target = decode_player(read_u8(cursor));
            return AbilityEffect {
                action: "position_change".into(),
                target: Some(target.into()),
                ..Default::default()
            };
        }
        Opcode::ChooseTargetPlayer => {
            let target = decode_player(read_u8(cursor));
            return AbilityEffect {
                action: "choose_target_player".into(),
                target: Some(target.into()),
                ..Default::default()
            };
        }
        Opcode::Restriction => {
            return AbilityEffect {
                action: "restriction".into(),
                ..Default::default()
            }
        }
        Opcode::PlaceEnergyUnderMember => {
            return AbilityEffect {
                action: "place_energy_under_member".into(),
                count: Some(read_u8(cursor) as u32),
                ..Default::default()
            };
        }
        Opcode::ModifyRequiredHearts => {
            let val = read_i8(cursor) as u32;
            let target = decode_player(read_u8(cursor));
            return AbilityEffect {
                action: "modify_required_hearts".into(),
                count: Some(val),
                target: Some(target.into()),
                ..Default::default()
            };
        }
        Opcode::ModifyRequiredHeartsGlobal => {
            let val = read_i8(cursor) as u32;
            return AbilityEffect {
                action: "modify_required_hearts_global".into(),
                count: Some(val),
                ..Default::default()
            };
        }
        Opcode::ModifyCost => {
            let val = read_i8(cursor) as u32;
            let target = decode_player(read_u8(cursor));
            return AbilityEffect {
                action: "modify_cost".into(),
                count: Some(val),
                target: Some(target.into()),
                ..Default::default()
            };
        }
        Opcode::SetBladeType => {
            let _ = read_u8(cursor);
            return AbilityEffect {
                action: "set_blade_type".into(),
                ..Default::default()
            };
        }
        Opcode::SetBladeCount | Opcode::SetHeartType => {
            return AbilityEffect {
                action: action_for_op(op).into(),
                count: Some(read_u8(cursor) as u32),
                ..Default::default()
            };
        }
        Opcode::GainAbility => {
            read_u16(cursor);
            let _d = read_u8(cursor);
            return AbilityEffect {
                action: "gain_ability".into(),
                ..Default::default()
            };
        }
        Opcode::GainAbilityFromSource => {
            read_u16(cursor);
            return AbilityEffect {
                action: "gain_ability_from_source".into(),
                ..Default::default()
            };
        }
        Opcode::ModifyYellCount => {
            let val = read_i8(cursor) as u32;
            return AbilityEffect {
                action: "modify_yell_count".into(),
                count: Some(val),
                ..Default::default()
            };
        }
        Opcode::InvalidateAbility => {
            return AbilityEffect {
                action: "invalidate_ability".into(),
                ..Default::default()
            }
        }
        Opcode::SuppressAbilityTrigger => {
            return AbilityEffect {
                action: "suppress_ability_trigger".into(),
                ..Default::default()
            }
        }
        Opcode::ActivateAbility => {
            return AbilityEffect {
                action: "activate_ability".into(),
                ..Default::default()
            }
        }
        Opcode::PlayBatonTouch => {
            return AbilityEffect {
                action: "play_baton_touch".into(),
                ..Default::default()
            }
        }
        Opcode::SetCardIdentity => {
            let _ = read_u16(cursor);
            return AbilityEffect {
                action: "set_card_identity".into(),
                ..Default::default()
            };
        }
        Opcode::ConditionalOnOptional => {
            return AbilityEffect {
                action: "conditional_on_optional".into(),
                optional: Some(read_u8(cursor) != 0),
                ..Default::default()
            };
        }
        Opcode::ConditionalOnResult => {
            return AbilityEffect {
                action: "conditional_on_result".into(),
                ..Default::default()
            };
        }
        _ => {}
    }
    decode_simple_effect(op, cursor);
    AbilityEffect {
        action: action_for_op(op).into(),
        ..Default::default()
    }
}

fn action_for_op(op: Opcode) -> &'static str {
    match op {
        Opcode::DrawCard => "draw_card",
        Opcode::MoveCards => "move_cards",
        Opcode::GainResource => "gain_resource",
        Opcode::ModifyScore => "modify_score",
        Opcode::ChangeState => "change_state",
        Opcode::PositionChange => "position_change",
        Opcode::ModifyRequiredHearts => "modify_required_hearts",
        Opcode::ModifyCost => "modify_cost",
        Opcode::SetBladeType => "set_blade_type",
        Opcode::SetBladeCount => "set_blade_count",
        Opcode::SetHeartType => "set_heart_type",
        Opcode::GainAbility => "gain_ability",
        Opcode::Restriction => "restriction",
        Opcode::ChooseTargetPlayer => "choose_target_player",
        Opcode::PlaceEnergyUnderMember => "place_energy_under_member",
        Opcode::DrawUntilCount => "draw_until_count",
        Opcode::ModifyYellCount => "modify_yell_count",
        Opcode::InvalidateAbility => "invalidate_ability",
        Opcode::SuppressAbilityTrigger => "suppress_ability_trigger",
        Opcode::ActivateAbility => "activate_ability",
        Opcode::PlayBatonTouch => "play_baton_touch",
        Opcode::ModifyRequiredHeartsGlobal => "modify_required_hearts_global",
        Opcode::GainAbilityFromSource => "gain_ability_from_source",
        Opcode::SetCardIdentity => "set_card_identity",
        _ => "",
    }
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
                source: ref mut _bc_source,
                destination: ref mut _bc_destination,
                count: ref mut _bc_count,
                card_type: ref mut _bc_card_type,
                ..
            } = &mut ek
            {
                *_bc_source = Some(src.into());
                *_bc_destination = Some(dest.into());
                *_bc_count = Some(count as u32);
                *_bc_card_type = Some(ct.into());
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
            let count = read_u8(cursor);
            AbilityEffect {
                action: "rest".into(),
                count: Some(count as u32),
                ..Default::default()
            }
        }
        Opcode::Energy => {
            let amt = read_u8(cursor);
            let _color = read_u8(cursor);
            AbilityEffect {
                action: "pay_energy".into(),
                count: Some(amt as u32),
                ..Default::default()
            }
        }
        Opcode::Discard => {
            let count = read_u8(cursor);
            let _ct = decode_card_type(read_u8(cursor));
            AbilityEffect {
                action: "discard".into(),
                count: Some(count as u32),
                ..Default::default()
            }
        }
        Opcode::PlaceEnergyUnderMemberCost => {
            let count = read_u8(cursor);
            AbilityEffect {
                action: "place_energy_under_member".into(),
                count: Some(count as u32),
                ..Default::default()
            }
        }
        Opcode::PayEnergy => {
            let amt = read_u8(cursor);
            let _opt = read_u8(cursor);
            AbilityEffect {
                action: "pay_energy".into(),
                count: Some(amt as u32),
                ..Default::default()
            }
        }
        Opcode::ChangeStateCost => {
            let state = decode_state(read_u8(cursor));
            let _opt = read_u8(cursor);
            let _self = read_u8(cursor);
            let mut ek = default_changeState();
            if let EffectKind::ChangeState {
                state_change: ref mut _bc_state_change,
                ..
            } = &mut ek
            {
                *_bc_state_change = Some(state.into());
            }
            AbilityEffect {
                action: "change_state".into(),
                kind: Some(ek_box_new(ek)),
                ..Default::default()
            }
        }
        Opcode::Tap => AbilityEffect {
            action: "tap".into(),
            ..Default::default()
        },
        Opcode::Rest => {
            let count = read_u8(cursor);
            AbilityEffect {
                action: "rest".into(),
                count: Some(count as u32),
                ..Default::default()
            }
        }
        Opcode::Energy => {
            let amt = read_u8(cursor);
            let _color = read_u8(cursor);
            AbilityEffect {
                action: "pay_energy".into(),
                count: Some(amt as u32),
                ..Default::default()
            }
        }
        Opcode::Discard => {
            let count = read_u8(cursor);
            let ct = decode_card_type(read_u8(cursor));
            AbilityEffect {
                action: "discard".into(),
                count: Some(count as u32),
                kind: Some(ek_box_new(default_moveCards())),
                ..Default::default()
            }
        }
        Opcode::PlaceEnergyUnderMemberCost => {
            let count = read_u8(cursor);
            AbilityEffect {
                action: "place_energy_under_member".into(),
                count: Some(count as u32),
                ..Default::default()
            }
        }
        Opcode::PayEnergy => {
            let amt = read_u8(cursor);
            let _opt = read_u8(cursor);
            AbilityEffect {
                action: "pay_energy".into(),
                count: Some(amt as u32),
                ..Default::default()
            }
        }
        Opcode::SequentialCost => {
            let n = read_u8(cursor);
            let mut steps = Vec::with_capacity(n as usize);
            for _ in 0..n {
                if !cursor.is_empty() {
                    let sub_op_val = read_u8(cursor);
                    if let Ok(sub_op) = Opcode::try_from(sub_op_val) {
                        steps.push(Box::new(decode_cost_op(sub_op, cursor)));
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
        Opcode::Reveal => {
            let _src = decode_zone(read_u8(cursor));
            let _ct = decode_card_type(read_u8(cursor));
            let _count = read_u8(cursor);
            AbilityEffect {
                action: "reveal".into(),
                ..Default::default()
            }
        }
        Opcode::ChoiceCondition => {
            let _n = read_u8(cursor);
            AbilityEffect {
                action: "choice".into(),
                ..Default::default()
            }
        }
        _ => {
            decode_simple_cost(op, cursor);
            let action = cost_action_for_op(op);
            AbilityEffect {
                action: action.into(),
                ..Default::default()
            }
        }
    }
}

fn decode_simple_cost(op: Opcode, cursor: &mut &[u8]) {
    match op {
        Opcode::Tap => {}
        Opcode::Rest => {
            let _ = read_u8(cursor);
        }
        Opcode::Energy => {
            let _ = read_u8(cursor);
            let _ = read_u8(cursor);
        }
        Opcode::Discard => {
            let _ = read_u8(cursor);
            let _ = read_u8(cursor);
        }
        Opcode::PlaceEnergyUnderMemberCost => {
            let _ = read_u8(cursor);
        }
        Opcode::Reveal => {
            let _ = read_u8(cursor);
            let _ = read_u8(cursor);
            let _ = read_u8(cursor);
        }
        Opcode::ChoiceCondition => {
            let _ = read_u8(cursor);
        }
        _ => {}
    }
}

fn cost_action_for_op(op: Opcode) -> &'static str {
    match op {
        Opcode::MoveCardsCost => "move_cards",
        Opcode::Tap => "tap",
        Opcode::Rest => "rest",
        Opcode::Energy => "pay_energy",
        Opcode::Discard => "discard",
        Opcode::PlaceEnergyUnderMemberCost => "place_energy_under_member",
        Opcode::PayEnergy => "pay_energy",
        Opcode::ChangeStateCost => "change_state",
        Opcode::SequentialCost => "sequential_cost",
        Opcode::Reveal => "reveal",
        Opcode::ChoiceCondition => "choice",
        _ => "",
    }
}

fn decode_effect_from_slice(data: &[u8]) -> AbilityEffect {
    let mut cursor = data;
    if cursor.is_empty() {
        return AbilityEffect::default();
    }
    let op_val = read_u8(&mut cursor);
    if let Ok(op) = Opcode::try_from(op_val) {
        decode_op_into(op, &mut cursor)
    } else {
        AbilityEffect::default()
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
fn decode_state(v: u8) -> &'static str {
    match v {
        0 => "rest",
        1 => "stand",
        2 => "reverse",
        3 => "wait",
        _ => "rest",
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
