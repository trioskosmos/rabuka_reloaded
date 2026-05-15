from pydantic import BaseModel, Field
from typing import Literal, List, Optional, Union, Any

class Action(BaseModel):
    action: str
    text: Optional[str] = None

class MoveCardsAction(Action):
    action: Literal["move_cards"]
    source: Optional[str] = None
    destination: Optional[str] = None
    count: Optional[int] = None
    card_type: Optional[str] = None
    target: Optional[str] = None
    placement_order: Optional[str] = None
    any_number: Optional[bool] = None

class DrawCardAction(Action):
    action: Literal["draw_card"]
    source: Literal["deck"] = "deck"
    destination: Literal["hand"] = "hand"
    count: int = 1

class LookAtAction(Action):
    action: Literal["look_at"]
    source: str
    count: int

class SelectAction(Action):
    action: Literal["select_cards", "select"]
    discard_remaining: Optional[bool] = None
    destination: Optional[str] = None
    count: Optional[int] = None

class LookAndSelectAction(Action):
    action: Literal["look_and_select"]
    look_action: LookAtAction
    select_action: Union[SelectAction, MoveCardsAction]

class GainResourceAction(Action):
    action: Literal["gain_resource"]
    resource: Literal["blade", "heart", "generic"]
    count: int
    target: Optional[str] = None
    heart_color: Optional[str] = None
    per_unit: Optional[bool] = None
    per_unit_count: Optional[int] = None
    per_unit_type: Optional[str] = None
    location: Optional[str] = None
    duration: Optional[str] = None

class ChangeStateAction(Action):
    action: Literal["change_state"]
    state_change: Literal["wait", "active"]
    card_type: str
    count: Optional[int] = None
    target: Optional[str] = None
    optional: Optional[bool] = None

class ModifyScoreAction(Action):
    action: Literal["modify_score"]
    operation: Literal["add", "remove"]
    value: int

class ModifyRequiredHeartsAction(Action):
    action: Literal["modify_required_hearts"]
    operation: Literal["decrease", "increase"]
    heart_color: str
    count: int

class RevealAction(Action):
    action: Literal["reveal"]
    source: str
    count: int

class AppearAction(Action):
    action: Literal["appear"]
    source: str
    destination: Literal["stage"] = "stage"

class PositionChangeAction(Action):
    action: Literal["position_change"]
    target: Optional[str] = None

class FormationChangeAction(Action):
    action: Literal["formation_change"]

class DoNothingAction(Action):
    action: Literal["do_nothing"]

class UnknownAction(Action):
    action: Literal["unknown"]
    text: str

class SequentialAction(Action):
    action: Literal["sequential"]
    actions: List['AnyAction']

# And so on for all action types
AnyAction = Union[
    MoveCardsAction, 
    DrawCardAction, 
    LookAtAction, 
    SelectAction, 
    LookAndSelectAction, 
    GainResourceAction, 
    ChangeStateAction,
    ModifyScoreAction,
    ModifyRequiredHeartsAction,
    RevealAction,
    AppearAction,
    PositionChangeAction,
    FormationChangeAction,
    DoNothingAction,
    UnknownAction,
    SequentialAction
]

class Cost(BaseModel):
    type: str
    text: str
    optional: Optional[bool] = None

class MoveCardsCost(Cost):
    type: Literal["move_cards"]
    source: Optional[str] = None
    destination: Optional[str] = None
    count: Optional[int] = None
    card_type: Optional[str] = None
    self_cost: Optional[bool] = None

class PayEnergyCost(Cost):
    type: Literal["pay_energy"]
    energy: int
    count: int

class UnknownCost(Cost):
    type: Literal["unknown_cost"]

class ChangeStateCost(Cost):
    type: Literal["change_state"]
    state_change: Literal["wait", "active"]
    card_type: str
    count: Optional[int] = None
    target: Optional[str] = None
    optional: Optional[bool] = None

class SequentialCost(Cost):
    type: Literal["sequential_cost"]
    costs: List['AnyCost']

AnyCost = Union[MoveCardsCost, PayEnergyCost, UnknownCost, ChangeStateCost, SequentialCost]

class Ability(BaseModel):
    triggerless_text: str
    cost: Optional[AnyCost] = None
    effect: Optional[AnyAction] = None

SequentialAction.model_rebuild()
SequentialCost.model_rebuild()
