"""
Heart-icon vocabulary readers.

The regexes and the `heart_id` normalizer they build on live in
parser_utils (shared with cost_parser); this module owns the
parser-side readers that turn rendered icons into the heart-colour
lists the condition and effect cascades consume. Keeping them apart
from the parse logic means the only code that spells heart-icon
markup is here and parser_utils — nothing in the cascades has to.

Every reader is pure: text in, structured value out. No parser state,
no forward references into the cascades, so this module is a leaf
parser.py imports from.
"""

import re

from parser_utils import (
    HEART_ICON,
    HEART_ICON_ID,
    HEART_LABEL,
    HEART_REF,
    heart_id,
)


def _heart_icons(text):
    """Every rendered heart icon in `text`, in order of appearance."""
    return re.findall(HEART_ICON, text)


def _heart_count(text):
    """How many heart icons `text` renders."""
    return len(_heart_icons(text))


def _heart_id_list(text):
    """Every heart colour named by a rendered icon, in order, duplicates kept."""
    return [heart_id(n) for n in re.findall(HEART_ICON_ID, text)]


def _heart_ids(text):
    """The distinct heart colours `text` names, sorted."""
    return sorted(set(_heart_id_list(text)))


def _heart_ids_in_order(text):
    """The distinct heart colours `text` names, in first-appearance order.

    Conditions want this (the order reads as the order the clause lists them);
    effect patches want it too, because the engine treats the list positionally.
    """
    return list(dict.fromkeys(_heart_id_list(text)))


def _heart_ids_gained(text):
    """The heart colours `text` grants, in order, duplicates kept.

    「{{heart_03.png|heart03}}を得る」 grants a heart, so the icon is one the
    player receives; 「{{heart_03.png|heart03}}を持つ」 merely requires one and
    is a condition, not a count. Only the former is a gain.
    """
    return [
        f"heart{m.group(1).zfill(2)}"
        for m in re.finditer(HEART_ICON_ID, text)
        if "持つ" not in text[m.end() : m.end() + 12]
    ]


def _heart_label_counts(text):
    """How many times each heart colour label appears in `text`, in order."""
    counts = {}
    for label in re.findall(HEART_LABEL, text):
        counts[label] = counts.get(label, 0) + 1
    return counts


def _heart_ref_ids(text, unique=False):
    """Heart colours from the `heart_NN` ids written in `text`, as `heartNN`."""
    ids = [heart_id(n) for n in re.findall(HEART_REF, text)]
    return list(dict.fromkeys(ids)) if unique else ids


def _count_heart_refs(text, allowed=None):
    """Per-colour counts of the `heart_NN` ids in `text`, optionally filtered."""
    counts = {}
    for heart_id in _heart_ref_ids(text):
        if allowed is None or heart_id in allowed:
            counts[heart_id] = counts.get(heart_id, 0) + 1
    return counts


def _uniform_count(counts):
    """The single value every colour agrees on, or the smallest if they differ.

    Required-heart clauses are written with the same number per colour, so the
    usual answer is "they all agree"; `min` keeps a malformed clause from
    over-granting when they do not.
    """
    if not counts:
        return None
    values = list(counts.values())
    return values[0] if len(set(values)) == 1 else min(values)
