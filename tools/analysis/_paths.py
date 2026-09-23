# -*- coding: utf-8 -*-
"""Shared repo-root path helpers for tools/analysis scripts."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TEST_OUTPUT = ROOT / "test_output"
CARDS_JSON = ROOT / "cards" / "cards.json"
ABILITIES_JSON = ROOT / "cards" / "abilities.json"
DECKS_DIR = ROOT / "web_ui" / "decks"
ARENA_TRACE = TEST_OUTPUT / "bot_arena_trace.csv"
ARENA_LOGS = TEST_OUTPUT / "arena_logs"
