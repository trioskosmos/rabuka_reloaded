"""Priority-based rule registry for structured text matching.
Replaces fragile lambda-list-rebuilt-per-call + implicit-priority patterns."""

from __future__ import annotations
import logging
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, Optional, Union

logger = logging.getLogger(__name__)

# --- Rule definition ---

@dataclass
class Rule:
    """A single dispatch rule with explicit priority (higher = checked first)."""
    priority: int
    name: str
    match: Callable           # def match(text, state?) -> bool
    apply: Optional[Callable] = None  # def apply(text, state) -> None
    help: str = ""

    def run(self, text: str, state: Dict) -> bool:
        try:
            return self.match(text, state)
        except TypeError:
            return self.match(text)
        except Exception as e:
            logger.warning(f"Rule '{self.name}' raised {e}")
            return False

    def execute(self, text: str, state: Dict) -> None:
        if self.apply:
            try:
                self.apply(text, state)
            except Exception as e:
                logger.error(f"Rule '{self.name}' apply() raised {e}")


# --- Registry ---

class RuleRegistry:
    """Ordered collection of rules. Insertion-sorted by priority descending."""

    def __init__(self, rules: Optional[list[Rule]] = None):
        self._rules: list[Rule] = []
        if rules:
            for r in rules:
                self.add(r)

    def add(self, rule: Rule) -> None:
        """Insert rule in priority order (descending)."""
        i = 0
        while i < len(self._rules) and self._rules[i].priority >= rule.priority:
            i += 1
        self._rules.insert(i, rule)

    def match(self, text: str, state: Dict) -> Optional[Rule]:
        """Return the first matching rule, or None."""
        for rule in self._rules:
            if rule.run(text, state):
                if logger.isEnabledFor(logging.DEBUG):
                    logger.debug(f"  ✓ {rule.name} (prio {rule.priority})")
                return rule
        return None

    def dispatch(self, text: str, state: Dict, default: str = "custom") -> str:
        """Run the first matching rule's apply() on state. Returns action name."""
        rule = self.match(text, state)
        if rule:
            rule.execute(text, state)
            state['action'] = rule.name
            return rule.name

        if logger.isEnabledFor(logging.INFO):
            logger.info(f"  ⚠ NO RULE for '{text[:60]}...'")
        state['action'] = default
        return default

    @property
    def names(self) -> list[str]:
        return [r.name for r in self._rules]

    def __len__(self) -> int:
        return len(self._rules)

    def __iter__(self):
        return iter(self._rules)

    def __repr__(self) -> str:
        lines = [f"  {r.priority:3d}  {r.name:25s}  {r.help}" for r in self._rules]
        return f"RuleRegistry({len(self._rules)} rules):\n" + "\n".join(lines)


# --- Convenience builder ---

def registry(*entries: Union[Rule, tuple]) -> RuleRegistry:
    """Build a registry from Rules or (priority, name, match, apply?, help?) tuples."""
    reg = RuleRegistry()
    for e in entries:
        if isinstance(e, Rule):
            reg.add(e)
        else:
            priority, name, match = e[0], e[1], e[2]
            apply = e[3] if len(e) > 3 else None
            help_text = e[4] if len(e) > 4 else ""
            reg.add(Rule(priority, name, match, apply, help_text))
    return reg
