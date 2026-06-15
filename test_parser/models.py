from __future__ import annotations
from typing import List, Optional, Union, Literal, Dict, Any
from pydantic import BaseModel, Field


class BaseParserModel(BaseModel):
    text: str


class Cost(BaseParserModel):
    type: str
    value: Optional[int] = None


class Condition(BaseParserModel):
    type: str
    # Common fields
    target: Optional[str] = None
    location: Optional[str] = None
    locations: Optional[List[str]] = None
    card_type: Optional[str] = None
    count: Optional[int] = None
    comparison_operator: Optional[str] = None
    unit: Optional[str] = None

    # Compound
    conditions: Optional[List[Condition]] = None
    logical_operator: Optional[Literal["and", "or"]] = None

    # Temporal
    temporal: Optional[str] = None
    phase: Optional[str] = None

    # Characters & Groups
    characters: Optional[List[str]] = None
    group_names: Optional[List[str]] = None
    all_members: Optional[bool] = None

    # State & Energy
    state: Optional[str] = None
    resource_type: Optional[str] = None
    negation: Optional[bool] = None
    heart_colors: Optional[List[str]] = None

    # Comparison details
    comparison_target: Optional[str] = None
    comparison_type: Optional[str] = None
    aggregate: Optional[str] = None

    # Others
    card_property: Optional[str] = None
    distinct: Optional[bool] = None
    exclude_self: Optional[bool] = None
    all_areas: Optional[bool] = None
    cost_limit: Optional[int] = None
    position: Optional[str] = None
    movement: Optional[str] = None


class Action(BaseParserModel):
    type: str
    target: Optional[str] = None
    location: Optional[str] = None
    destination: Optional[str] = None
    source: Optional[str] = None
    count: Optional[int] = None
    card_type: Optional[str] = None
    optional: Optional[bool] = None
    
    # Characters & Groups
    characters: Optional[List[str]] = None
    group_names: Optional[List[str]] = None
    
    # Sequential/Compound
    actions: Optional[List[Action]] = None
    
    # For look_and_select
    look_action: Optional[Action] = None
    select_action: Optional[Action] = None
    discard_remaining: Optional[bool] = None
    
    # Resource related
    resource: Optional[str] = None
    heart_colors: Optional[List[str]] = None
    
    # State related
    state_change: Optional[str] = None
    
    # Others
    duration: Optional[str] = None
    cost_limit: Optional[int] = None
    position: Optional[str] = None
    exclude_self: Optional[bool] = None


class UnknownAction(Action):
    type: Literal["unknown"] = "unknown"
    raw_text: str


class Ability(BaseModel):
    name: Optional[str] = None
    cost: Optional[Union[Cost, List[Cost], Action]] = None
    condition: Optional[Condition] = None
    effects: List[Action] = Field(default_factory=list)
    raw_text: Optional[str] = None
