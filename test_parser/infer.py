"""Action inference from field SETS — not a priority cascade.

The key insight: action type is determined by WHICH fields are present,
not by which text keyword matches first. The inference is a SET membership
test against known action field signatures.

Algorithm:
1. Extract ALL fields from text (field extractors are independent)
2. For each action signature, check if its 'required' fields are present
3. Score each match: (matched_required, matched_optional, keyword_match)
4. Pick the best match
"""

from __future__ import annotations
import re
from typing import Dict, Optional, Tuple
from schema import extract_all, ACTION_FIELD_SIGNATURES


def score_action(action_name: str, action_sig: Dict, fields: Dict, text: str) -> Tuple[int, int, int]:
    """Score how well an action matches the extracted fields.
    Returns (matched_required, matched_optional, keyword_hits)."""
    required = action_sig.get('required', [])
    optional = action_sig.get('optional', [])
    keywords = action_sig.get('keywords', [])
    defaults = action_sig.get('defaults', {})

    matched_required = sum(1 for f in required if f in fields)
    matched_optional = sum(1 for f in optional if f in fields)
    keyword_hits = sum(1 for kw in keywords if re.search(kw, text))

    # Penalize if required fields are MISSING that should be there
    # (e.g. move_cards without source+dest is a bad match)
    if len(required) > 0 and matched_required < len(required):
        return (-1, 0, 0)  # disqualify

    return (matched_required, matched_optional, keyword_hits)


def infer_action(fields: Dict, text: str) -> Tuple[str, Dict]:
    """Infer action type from field presence.
    Returns (action_name, augmented_fields_with_defaults)."""
    
    results = []
    for name, sig in ACTION_FIELD_SIGNATURES.items():
        score = score_action(name, sig, fields, text)
        if score[0] >= 0:  # not disqualified
            results.append((score, name, sig))
    
    # Sort by: required matches desc, optional matches desc, keyword hits desc
    results.sort(key=lambda r: (r[0][0], r[0][1], r[0][2]), reverse=True)
    
    if results:
        best = results[0]
        action_name = best[1]
        sig = best[2]
        
        # Apply defaults
        for k, v in sig.get('defaults', {}).items():
            fields.setdefault(k, v)
        
        return action_name, fields
    
    # No match → custom
    return 'custom', fields


def parse_action(text: str) -> Dict[str, Any]:
    """Parse an action text using schema-driven extraction + set-based inference."""
    fields = extract_all(text)
    action, fields = infer_action(fields, text)
    fields['action'] = action
    return fields
