"""Orchestration: grammar -> annotate -> map -> compare."""
import json
import sys
import os
from pathlib import Path

from . import annotator
from . import mapper

sys.path.insert(0, str(Path(__file__).parent.parent))


def parse_ability(triggerless_text: str) -> dict:
    if not triggerless_text.strip():
        return {'triggerless_text': triggerless_text}

    text = triggerless_text.strip().rstrip('。')
    ability = {'triggerless_text': triggerless_text}

    # Decompose into clauses
    clauses = annotator.decompose(text)

    # Classify each clause
    for c in clauses:
        annotator.classify(c)

    # Assemble into ability JSON
    result = mapper.build(clauses)
    ability.update(result)
    return ability
