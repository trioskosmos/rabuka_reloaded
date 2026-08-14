#![cfg(feature = "3ds")]

// Action-list formatters (engine_duplication.md 1.5 action_list.rs).
// Icon-rich single-line action description for the image-mode action list.

use rabuka_engine::game_setup;
use rabuka_engine::game_state::GameState;

use crate::i18n;
use crate::i18n::Lang;
use crate::lang::{current_lang, tl};
use crate::ui::text::truncate_aware_segments;
use crate::util::{cn_or_empty, tl_area};

/// Icon-rich single-line description for the image-mode action list.

/// Icon-rich single-line description for the image-mode action list.
pub(crate) fn format_action_line_image(act: &game_setup::Action, gs: &GameState) -> String {
    match act.action_type {
        game_setup::ActionType::Pass => tl("Pass"),
        game_setup::ActionType::PlayMemberToStage => {
            let cn = cn_or_empty(act);
            let name = i18n::card_display_name(
                &act.parameters
                    .as_ref()
                    .and_then(|p| p.card_name.clone())
                    .unwrap_or_default(),
                current_lang(),
            );
            let base_cost = act
                .parameters
                .as_ref()
                .and_then(|p| p.base_cost)
                .unwrap_or(0);
            let area = act
                .parameters
                .as_ref()
                .and_then(|p| p.stage_area.clone())
                .unwrap_or_default();
            let area_label = tl_area(&area);
            if !cn.is_empty() {
                if base_cost > 0 {
                    format!(
                        "{{{{icon_energy.png|E}}}}{} [{}] {} {}",
                        base_cost, cn, name, area_label
                    )
                } else {
                    format!("[{}] {} {}", cn, name, area_label)
                }
            } else {
                if base_cost > 0 {
                    format!(
                        "{{{{icon_energy.png|E}}}}{} {} {}",
                        base_cost, name, area_label
                    )
                } else {
                    format!("{} {}", name, area_label)
                }
            }
        }
        game_setup::ActionType::UseAbility => {
            let name = i18n::card_display_name(
                &act.parameters
                    .as_ref()
                    .and_then(|p| p.card_name.clone())
                    .unwrap_or_default(),
                current_lang(),
            );
            let cost = act
                .parameters
                .as_ref()
                .and_then(|p| p.final_cost.or(p.base_cost))
                .unwrap_or(0);
            let area = act
                .parameters
                .as_ref()
                .and_then(|p| p.stage_area.clone())
                .unwrap_or_default();
            let area_label = tl_area(&area);
            let abil = act
                .parameters
                .as_ref()
                .and_then(|p| p.source_ability.clone())
                .unwrap_or_default();
            let abil_short = truncate_aware_segments(&abil, 28);
            let cn = cn_or_empty(act);
            if !cn.is_empty() {
                if cost > 0 {
                    format!(
                        "{{{{icon_energy.png|E}}}}{} [{}] {} {} {}",
                        cost, cn, name, area_label, abil_short
                    )
                } else {
                    format!("[{}] {} {} {}", cn, name, area_label, abil_short)
                }
            } else {
                if cost > 0 {
                    format!(
                        "{{{{icon_energy.png|E}}}}{} {} {} {}",
                        cost, name, area_label, abil_short
                    )
                } else {
                    format!("{} {} {}", name, area_label, abil_short)
                }
            }
        }
        _ => {
            let cn = cn_or_empty(act);
            let name = i18n::card_display_name(
                &act.parameters
                    .as_ref()
                    .and_then(|p| p.card_name.clone())
                    .unwrap_or_default(),
                current_lang(),
            );
            let line = if let Some(sel) = act.selected {
                let label = if sel {
                    tl("selected_label")
                } else {
                    tl("unselected_label")
                };
                if !cn.is_empty() && !name.is_empty() {
                    format!("[{}] [{}] {}", label, cn, name)
                } else if !cn.is_empty() {
                    format!("[{}] [{}]", label, cn)
                } else {
                    format!("[{}] {}", label, name)
                }
            } else {
                let desc = act
                    .display_desc(current_lang() == Lang::Japanese)
                    .to_string();
                let ability_text = if act.action_type == game_setup::ActionType::ChoiceOption {
                    gs.get_pending_choice()
                        .and_then(|c| {
                            use rabuka_engine::ability::types::Choice;
                            if let Choice::SelectAutoAbility { options, .. } = c {
                                act.parameters
                                    .as_ref()
                                    .and_then(|p| p.card_id)
                                    .and_then(|idx| options.get(idx as usize))
                                    .map(|o| o.ability_text.clone())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let display = if !ability_text.is_empty() {
                    ability_text
                } else {
                    desc
                };
                // Text-only choice options (e.g. skip, yes/no, pay/skip cost) are
                // identified by card_no sentinels; don't prepend their bracketed
                // tag (e.g. [skip]) to the localized label (e.g. スキップ).
                let text_only = crate::ui::text::is_text_only(act);
                if cn.is_empty() || text_only {
                    display
                } else if !name.is_empty() {
                    format!("[{}] {} {}", cn, name, display)
                } else {
                    format!("[{}] {}", cn, display)
                }
            };
            line
        }
    }
}
