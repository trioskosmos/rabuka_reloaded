"""PEG-like grammar rules for ability text syntax.

Each rule has a .match(text, pos=0) method that returns (end_pos, result) or None.
Rules compose via Seq, OneOf, Optional, Repeat classes.
"""
import re
from typing import Any, Callable, List, Optional, Tuple


class Rule:
    def match(self, text: str, pos: int = 0) -> Optional[Tuple[int, Any]]: ...


class Str(Rule):
    """Match a literal string."""
    def __init__(self, s: str):
        self.s = s
    def match(self, text: str, pos: int = 0):
        if text.startswith(self.s, pos):
            return pos + len(self.s), self.s
        return None


class Re(Rule):
    """Match a regex pattern."""
    def __init__(self, pattern: str, group: int = 0):
        self.pat = re.compile(pattern)
        self.group = group
    def match(self, text: str, pos: int = 0):
        m = self.pat.match(text, pos)
        if m:
            return m.end(), m.group(self.group)
        return None


class Seq(Rule):
    """Match a sequence of rules in order."""
    def __init__(self, *rules):
        self.rules = rules
    def match(self, text: str, pos: int = 0):
        results = []
        for rule in self.rules:
            m = rule.match(text, pos)
            if m is None:
                return None
            pos, res = m
            results.append(res)
        return pos, results


class OneOf(Rule):
    """Try each rule in order, return first match."""
    def __init__(self, *rules):
        self.rules = rules
    def match(self, text: str, pos: int = 0):
        for rule in self.rules:
            m = rule.match(text, pos)
            if m is not None:
                return m
        return None


class Opt(Rule):
    """Optionally match the rule."""
    def __init__(self, rule):
        self.rule = rule
    def match(self, text: str, pos: int = 0):
        m = self.rule.match(text, pos)
        if m:
            return m
        return pos, None


class Many(Rule):
    """Match the rule zero or more times."""
    def __init__(self, rule, sep=None):
        self.rule = rule
        self.sep = sep
    def match(self, text: str, pos: int = 0):
        results = []
        first = True
        while pos < len(text):
            if not first and self.sep:
                m = self.sep.match(text, pos)
                if m is None:
                    break
                pos = m[0]
            m = self.rule.match(text, pos)
            if m is None:
                break
            pos, res = m
            if res is not None:
                results.append(res)
            first = False
        return pos, results


class Fn(Rule):
    """Rule with custom match function."""
    def __init__(self, fn: Callable):
        self.fn = fn
    def match(self, text: str, pos: int = 0):
        return self.fn(text, pos)


class Map(Rule):
    """Transform the result of another rule."""
    def __init__(self, rule, fn: Callable):
        self.rule = rule
        self.fn = fn
    def match(self, text: str, pos: int = 0):
        m = self.rule.match(text, pos)
        if m:
            res = self.fn(m[1])
            # If map function explicitly returns None, treat it as a parse failure
            if res is None:
                return None
            return m[0], res
        return None


class Ref(Rule):
    """Forward reference to another rule (lazy)."""
    def __init__(self):
        self.target = None
    def resolve(self, target):
        self.target = target
    def match(self, text: str, pos: int = 0):
        return self.target.match(text, pos) if self.target else None


def keyword(kw: str) -> Rule:
    """Match a keyword (followed by non-word char or end)."""
    return Re(r'(?:' + re.escape(kw) + r')(?=\W|$)', group=0)
