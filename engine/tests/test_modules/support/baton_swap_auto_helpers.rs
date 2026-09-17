use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

pub const MOVER: &str = "PL!SP-bp5-006-R";
pub const FILLER: &str = "PL!-sd1-010-SD";
pub const NIJI_LIVE: &str = "PL!N-bp1-026-L";
pub const LIELIA_LIVE: &str = "PL!SP-bp1-023-L";
pub const NIJI_MEMBER: &str = "PL!N-bp7-001-R";
pub const NO_BLADE_HEART: &str = "PL!SP-bp1-021-N";
pub const ENERGY: &str = "LL-E-001-SD";

pub fn resolve_auto_choices_accepting_optionals(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { count, .. } => {
                if *count > 0 && *count < 10 {
                    game.select_indices(&(0..*count).collect::<Vec<_>>());
                } else {
                    game.select_indices(&[0]);
                }
            }
            Choice::SelectTarget { target, options, .. }
                if target == "conditional_optional" =>
            {
                // Accept the optional (do the follow-up action).
                game.select_choice_option(1);
            }
            Choice::SelectTarget { target, .. }
                if target == "position|destination" || target == "area_select" =>
            {
                let acts = game.generated_actions();
                if acts.is_empty() {
                    game.select_indices(&[]);
                } else {
                    game.select_generated(0);
                }
            }
            _ => game.select_indices(&[0]),
        }
    }
}

pub fn replace_member_by_baton_touch(game: &mut TestGame, replaced: i16, arriver: i16, area: MemberArea) {
    game.give_energy(30);
    game.state.player1.stage.set_area(area, replaced);
    game.state.player1.hand.cards.push(arriver);
    game.play_to_stage(arriver, area);
}

pub fn append_twenty_filler_cards(game: &mut TestGame) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
}

pub fn activate_mill_three_position_swap(game: &mut TestGame, target_area: MemberArea) {
    let mover = game.id(MOVER);
    // The mover's 起動 cost mills 3 from deck top; ensure there's material.
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
    let mover_area = match target_area {
        MemberArea::LeftSide => MemberArea::RightSide,
        _ => MemberArea::LeftSide,
    };
    game.add_to_stage(mover_area, mover);
    game.give_energy(10);

    game.activate_ability(mover);
    game.drain_auto_ability_choices();
    let acts = game.generated_actions();
    let target_area_str = match target_area {
        MemberArea::LeftSide => "left",
        MemberArea::Center => "center",
        MemberArea::RightSide => "right",
    };
    let idx = acts
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some(target_area_str))
        .unwrap_or_else(|| panic!("no swap option to {}", target_area_str));
    game.select_generated(idx);
    game.drain_auto_ability_choices();
}

pub fn stage_member_and_swap_area(game: &mut TestGame, target: i16, target_area: MemberArea) {
    game.add_to_stage(target_area, target);
    activate_mill_three_position_swap(game, target_area);
}

pub fn heart_modifier(game: &TestGame, cid: i16, hc: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(cid, hc)
}

pub fn blade_modifier(game: &TestGame, cid: i16) -> i32 {
    game.state.mods.get_blade_modifier(cid)
}
