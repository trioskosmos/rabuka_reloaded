//! Choice result handler registry — table-driven dispatch for provide_choice_result

use crate::ability::resolver::AbilityResolver;
use crate::ability::types::{Choice, ChoiceResult, ChoiceRoute, ExecutionContext};
use crate::game_state::GameState;

/// Trait for handling a specific choice result.
pub trait ChoiceResultHandler: Send + Sync {
    /// Handle this choice result combination.
    fn handle(&self, resolver: &mut AbilityResolver, gs: &mut GameState, choice: &Choice, result: &ChoiceResult, context: ExecutionContext) -> Result<(), String>;
    
    /// The choice type this handler matches.
    fn choice_type(&self) -> fn(&Choice) -> bool;
    
    /// The result type this handler matches.
    fn result_type(&self) -> fn(&ChoiceResult) -> bool;
}

/// Registry of choice result handlers.
pub struct ChoiceResultRegistry {
    handlers: Vec<Box<dyn ChoiceResultHandler>>,
}

impl ChoiceResultRegistry {
    pub fn new() -> Self {
        Self { handlers: Vec::new() }
    }
    
    pub fn register(mut self, handler: Box<dyn ChoiceResultHandler>) -> Self {
        self.handlers.push(handler);
        self
    }
    
    pub fn handle(&self, resolver: &mut AbilityResolver, gs: &mut GameState, choice: &Choice, result: &ChoiceResult, context: ExecutionContext) -> Result<(), String> {
        for handler in &self.handlers {
            if (handler.choice_type())(choice) && (handler.result_type())(result) {
                return handler.handle(resolver, gs, choice, result, context);
            }
        }
        Err("Choice result does not match pending choice".to_string())
    }
}

/// Global registry.
static CHOICE_RESULT_REGISTRY: std::sync::OnceLock<ChoiceResultRegistry> = std::sync::OnceLock::new();

pub fn init_choice_result_registry() -> &'static ChoiceResultRegistry {
    CHOICE_RESULT_REGISTRY.get_or_init(|| {
        ChoiceResultRegistry::new()
            .register(Box::new(SelectCardHandler))
            .register(Box::new(AnyNumberSkipHandler))
            .register(Box::new(OrderSkipHandler))
            .register(Box::new(GeneralSkipHandler))
            .register(Box::new(AreaSelectHandler))
            .register(Box::new(SelectTargetHandler))
            .register(Box::new(SelectPositionHandler))
            .register(Box::new(HeartColorHandler))
            .register(Box::new(HeartTypeHandler))
    })
}

// Handlers
struct SelectCardHandler;
impl ChoiceResultHandler for SelectCardHandler {
    fn choice_type(&self) -> fn(&Choice) -> bool {
        |c| matches!(c, Choice::SelectCard { .. })
    }
    
    fn result_type(&self) -> fn(&ChoiceResult) -> bool {
        |r| matches!(r, ChoiceResult::CardSelected { .. })
    }
    
    fn handle(&self, resolver: &mut AbilityResolver, gs: &mut GameState, choice: &Choice, result: &ChoiceResult, context: ExecutionContext) -> Result<(), String> {
        let Choice::SelectCard { zone, card_type, count, allow_skip, cost_limit, cost_limit_operator, cost_total, cost_total_operator, group, characters, filtered_indices, is_select_action, target_player_id, blind, destination, discard_remaining, .. } = choice else { unreachable!() };
        let ChoiceResult::CardSelected { indices } = result else { unreachable!() };
        
        resolver.handle_select_card(
            gs,
            choice,
            zone,
            context,
            crate::ability::choice::SelectionContext {
                card_type: card_type.clone(),
                count: *count,
                allow_skip: *allow_skip,
                indices: indices.to_vec(),
                cost_limit: *cost_limit,
                cost_limit_operator: cost_limit_operator.clone(),
                cost_total: *cost_total,
                cost_total_operator: cost_total_operator.clone(),
                group: group.clone(),
                characters: characters.clone(),
                filtered_indices: filtered_indices.clone(),
                is_select_action: *is_select_action,
                target_player_id: target_player_id.clone(),
                destination: destination.clone(),
                discard_remaining: *discard_remaining,
                blind: *blind,
                is_reveal: matches!(choice, Choice::SelectCard { is_reveal: true, .. }),
            },
        )
    }
}

struct AnyNumberSkipHandler;
impl ChoiceResultHandler for AnyNumberSkipHandler {
    fn choice_type(&self) -> fn(&Choice) -> bool {
        |c| matches!(c, Choice::SelectCard { count: 0, allow_skip: true, .. })
    }
    
    fn result_type(&self) -> fn(&ChoiceResult) -> bool {
        |r| matches!(r, ChoiceResult::Skip)
    }
    
    fn handle(&self, resolver: &mut AbilityResolver, gs: &mut GameState, _choice: &Choice, _result: &ChoiceResult, _context: ExecutionContext) -> Result<(), String> {
        resolver.clear_choice_state_and_resume(gs)
    }
}

struct OrderSkipHandler;
impl ChoiceResultHandler for OrderSkipHandler {
    fn choice_type(&self) -> fn(&Choice) -> bool {
        |c| matches!(c, Choice::SelectTarget { target, allow_skip: false, .. } if target == "order")
    }
    
    fn result_type(&self) -> fn(&ChoiceResult) -> bool {
        |r| matches!(r, ChoiceResult::Skip)
    }
    
    fn handle(&self, _resolver: &mut AbilityResolver, _gs: &mut GameState, _choice: &Choice, _result: &ChoiceResult, _context: ExecutionContext) -> Result<(), String> {
        Err("Deck ordering cannot be skipped".to_string())
    }
}

struct GeneralSkipHandler;
impl ChoiceResultHandler for GeneralSkipHandler {
    fn choice_type(&self) -> fn(&Choice) -> bool {
        |c| matches!(c, Choice::SelectCard { .. } | Choice::SelectTarget { .. })
    }
    
    fn result_type(&self) -> fn(&ChoiceResult) -> bool {
        |r| matches!(r, ChoiceResult::Skip)
    }
    
    fn handle(&self, resolver: &mut AbilityResolver, gs: &mut GameState, _choice: &Choice, _result: &ChoiceResult, context: ExecutionContext) -> Result<(), String> {
        gs.ability_queue.take_pending_actions();
        resolver.clear_choice_state(gs);
        resolver.resume_execution(gs, context)
    }
}

struct AreaSelectHandler;
impl ChoiceResultHandler for AreaSelectHandler {
    fn choice_type(&self) -> fn(&Choice) -> bool {
        |c| matches!(c, Choice::SelectTarget { target, .. } if target == "area_select")
    }
    
    fn result_type(&self) -> fn(&ChoiceResult) -> bool {
        |r| matches!(r, ChoiceResult::TargetSelected { .. })
    }
    
    fn handle(&self, resolver: &mut AbilityResolver, gs: &mut GameState, choice: &Choice, result: &ChoiceResult, _context: ExecutionContext) -> Result<(), String> {
        let ChoiceResult::TargetSelected { target: selected } = result else { unreachable!() };
        let Choice::SelectTarget { options: Some(ref opts), .. } = choice else {
            resolver.selected_area = Some(selected.clone());
            resolver.clear_choice_state(gs);
            return resolver.resume_pending_actions(gs);
        };
        
        let area = if let Ok(idx) = selected.parse::<usize>() {
            opts.get(idx).map(|s| s.as_str()).unwrap_or("left").to_string()
        } else if opts.contains(selected) {
            selected.clone()
        } else {
            opts[0].clone()
        };
        resolver.selected_area = Some(area);
        resolver.clear_choice_state(gs);
        resolver.resume_pending_actions(gs)
    }
}

struct SelectTargetHandler;
impl ChoiceResultHandler for SelectTargetHandler {
    fn choice_type(&self) -> fn(&Choice) -> bool {
        |c| matches!(c, Choice::SelectTarget { .. })
    }
    
    fn result_type(&self) -> fn(&ChoiceResult) -> bool {
        |r| matches!(r, ChoiceResult::TargetSelected { .. })
    }
    
    fn handle(&self, resolver: &mut AbilityResolver, gs: &mut GameState, choice: &Choice, result: &ChoiceResult, _context: ExecutionContext) -> Result<(), String> {
        let Choice::SelectTarget { target, .. } = choice else { unreachable!() };
        let ChoiceResult::TargetSelected { target: selected } = result else { unreachable!() };
        
        if target == "choice_string" {
            // Read conditional_choice from the queue entry
            let conditional_choice = gs.ability_queue.current_entry()
                .and_then(|e| e.conditional_choice.clone());
            
            // Check choice_card_no to determine which handler to use
            let choice_card_no = gs.ability_queue.current_entry()
                .and_then(|e| e.choice_card_no.clone());
            
            if matches!(choice_card_no, Some(ChoiceRoute::ChoiceString)) {
                crate::ability::compound::conditional_on::handle_choice_string_selection(
                    resolver, gs, selected, conditional_choice
                )
            } else {
                crate::ability::compound::conditional_on::handle_choice_string_store(
                    resolver, gs, selected, conditional_choice
                )
            }
        } else {
            resolver.handle_select_target(gs, target, selected)
        }
    }
}

struct SelectPositionHandler;
impl ChoiceResultHandler for SelectPositionHandler {
    fn choice_type(&self) -> fn(&Choice) -> bool {
        |c| matches!(c, Choice::SelectPosition { .. })
    }
    
    fn result_type(&self) -> fn(&ChoiceResult) -> bool {
        |r| matches!(r, ChoiceResult::PositionSelected { .. })
    }
    
    fn handle(&self, resolver: &mut AbilityResolver, gs: &mut GameState, _choice: &Choice, result: &ChoiceResult, context: ExecutionContext) -> Result<(), String> {
        let ChoiceResult::PositionSelected { position } = result else { unreachable!() };
        resolver.handle_select_position(gs, position, context)
    }
}

struct HeartColorHandler;
impl ChoiceResultHandler for HeartColorHandler {
    fn choice_type(&self) -> fn(&Choice) -> bool {
        |c| matches!(c, Choice::SelectHeartColor { .. })
    }
    
    fn result_type(&self) -> fn(&ChoiceResult) -> bool {
        |r| matches!(r, ChoiceResult::HeartColorSelected { .. })
    }
    
    fn handle(&self, resolver: &mut AbilityResolver, gs: &mut GameState, _choice: &Choice, result: &ChoiceResult, _context: ExecutionContext) -> Result<(), String> {
        let ChoiceResult::HeartColorSelected { colors } = result else { unreachable!() };
        let Choice::SelectHeartColor { count, .. } = _choice else { unreachable!() };
        resolver.handle_heart_selection(gs, *count as u8, colors)
    }
}

struct HeartTypeHandler;
impl ChoiceResultHandler for HeartTypeHandler {
    fn choice_type(&self) -> fn(&Choice) -> bool {
        |c| matches!(c, Choice::SelectHeartType { .. })
    }
    
    fn result_type(&self) -> fn(&ChoiceResult) -> bool {
        |r| matches!(r, ChoiceResult::HeartTypeSelected { .. })
    }
    
    fn handle(&self, resolver: &mut AbilityResolver, gs: &mut GameState, _choice: &Choice, result: &ChoiceResult, _context: ExecutionContext) -> Result<(), String> {
        let ChoiceResult::HeartTypeSelected { types: colors } = result else { unreachable!() };
        let Choice::SelectHeartType { count, .. } = _choice else { unreachable!() };
        resolver.handle_heart_selection(gs, *count as u8, colors)
    }
}