from typing import List, Optional, Union, Any, Literal
from pydantic import BaseModel, Field

# --- Sub-models ---
class AnyAction(BaseModel):
    action: str
    text: str
    duration: Optional[str] = None
    per_unit: Optional[bool] = None
    per_unit_type: Optional[str] = None

class DrawCardAction(AnyAction):
    action: Literal["draw_card"] = "draw_card"
    count: int

class MoveCardsAction(AnyAction):
    action: Literal["move_cards"] = "move_cards"
    source: str
    destination: str
    count: Optional[int] = None
    any_number: Optional[bool] = False

class LookAtAction(AnyAction):
    action: Literal["look_at"] = "look_at"
    source: str
    count: int

class LookAndSelectAction(AnyAction):
    action: Literal["look_and_select"] = "look_and_select"
    source: str
    destination: str
    look_count: int
    select_count: int

class SelectAction(AnyAction):
    action: Literal["select", "select_cards"] = "select"
    count: Optional[Union[int, str]] = None
    target: Optional[str] = None
    destination: Optional[str] = None

class GainResourceAction(AnyAction):
    action: Literal["gain_resource"] = "gain_resource"
    resource: str
    count: int

class ModifyScoreAction(AnyAction):
    action: Literal["modify_score"] = "modify_score"
    operation: str
    value: int
    target: str

class ChangeStateAction(AnyAction):
    action: Literal["change_state"] = "change_state"
    state_change: str
    card_type: str
    target: str
    count: Optional[int] = 1

class AppearAction(AnyAction):
    action: Literal["appear"] = "appear"
    source: str

class RevealAction(AnyAction):
    action: Literal["reveal"] = "reveal"
    source: str
    count: int

class ModifyRequiredHeartsAction(AnyAction):
    action: Literal["modify_required_hearts"] = "modify_required_hearts"
    operation: str
    heart_color: str
    count: int

class PositionChangeAction(AnyAction):
    action: Literal["position_change"] = "position_change"

class FormationChangeAction(AnyAction):
    action: Literal["formation_change"] = "formation_change"

class TreatAsAction(AnyAction):
    action: Literal["treat_as"] = "treat_as"
    groups: List[str]

class AbilityDisableAction(AnyAction):
    action: Literal["ability_disable"] = "ability_disable"
    target: Optional[str] = None
    ability_type: Optional[str] = None

class AbilityTriggerAction(AnyAction):
    action: Literal["ability_trigger"] = "ability_trigger"
    trigger_condition: Optional[str] = None
    ability_type: Optional[str] = None

class DoNothingAction(AnyAction):
    action: Literal["do_nothing"] = "do_nothing"

class UnknownAction(AnyAction):
    action: Literal["unknown"] = "unknown"

class SequentialAction(AnyAction):
    action: Literal["sequential"] = "sequential"
    actions: List[Union[DrawCardAction, MoveCardsAction, LookAtAction, SelectAction, 
                        GainResourceAction, ModifyScoreAction, ChangeStateAction, 
                        AppearAction, RevealAction, ModifyRequiredHeartsAction,
                        PositionChangeAction, FormationChangeAction, DoNothingAction,
                        TreatAsAction, AbilityDisableAction, AbilityTriggerAction, UnknownAction, 'SequentialAction']]

# --- Costs ---
class AnyCost(BaseModel):
    type: str
    text: str

class PayEnergyCost(AnyCost):
    type: Literal["pay_energy"] = "pay_energy"
    energy: int
    count: int

class MoveCardsCost(AnyCost):
    type: Literal["move_cards"] = "move_cards"
    source: str
    destination: str
    count: int
    optional: bool = False

class ChangeStateCost(AnyCost):
    type: Literal["change_state"] = "change_state"
    state_change: str
    card_type: str
    optional: bool = False
    self_cost: bool = False

class RevealCost(AnyCost):
    type: Literal["reveal_cost"] = "reveal_cost"
    count: Optional[int] = None

class ChoiceCost(AnyCost):
    type: Literal["choice_cost"] = "choice_cost"
    costs: List[AnyCost]

class UnknownCost(AnyCost):
    type: Literal["unknown_cost"] = "unknown_cost"

class SequentialCost(AnyCost):
    type: Literal["sequential_cost"] = "sequential_cost"
    costs: List[Union[PayEnergyCost, MoveCardsCost, ChangeStateCost, RevealCost, ChoiceCost, UnknownCost]]

# --- Top Level ---
class AbilityBlock(BaseModel):
    cost: Optional[Union[PayEnergyCost, MoveCardsCost, ChangeStateCost, RevealCost, ChoiceCost, UnknownCost, SequentialCost]] = None
    condition: Optional[str] = None
    effect: Union[DrawCardAction, MoveCardsAction, LookAtAction, SelectAction, 
                  GainResourceAction, ModifyScoreAction, ChangeStateAction, 
                  AppearAction, RevealAction, ModifyRequiredHeartsAction,
                  PositionChangeAction, FormationChangeAction, DoNothingAction,
                  TreatAsAction, AbilityDisableAction, AbilityTriggerAction, UnknownAction, SequentialAction]

class Ability(BaseModel):
    triggerless_text: str
    blocks: List[AbilityBlock] = []
    # Legacy fields
    cost: Optional[Union[PayEnergyCost, MoveCardsCost, ChangeStateCost, RevealCost, ChoiceCost, UnknownCost, SequentialCost]] = None
    effect: Optional[Union[DrawCardAction, MoveCardsAction, LookAtAction, SelectAction, 
                          GainResourceAction, ModifyScoreAction, ChangeStateAction, 
                          AppearAction, RevealAction, ModifyRequiredHeartsAction,
                          PositionChangeAction, FormationChangeAction, DoNothingAction,
                          TreatAsAction, AbilityDisableAction, AbilityTriggerAction, UnknownAction, SequentialAction]] = None
