from typing import Any, Callable, List, Optional, Tuple, Union
import re


class ParseError(Exception):
    def __init__(self, message: str, position: int):
        self.message = message
        self.position = position
        super().__init__(f"{message} at position {position}")


class Parser:
    def parse(
        self, text: str, position: int = 0, debug: bool = False
    ) -> Tuple[Any, int]:
        raise NotImplementedError


class Str(Parser):
    def __init__(self, pattern: str):
        self.pattern = pattern

    def parse(
        self, text: str, position: int = 0, debug: bool = False
    ) -> Tuple[Any, int]:
        if debug:
            print(f"  [Str] Attempting '{self.pattern}' at pos {position}")
        if text.startswith(self.pattern, position):
            if debug:
                print(f"    [Str] MATCHED '{self.pattern}'")
            return self.pattern, position + len(self.pattern)
        if debug:
            print(f"    [Str] FAILED '{self.pattern}'")
        raise ParseError(f"Expected '{self.pattern}'", position)


class Regex(Parser):
    def __init__(self, pattern: str):
        self.re = re.compile(pattern)

    def parse(
        self, text: str, position: int = 0, debug: bool = False
    ) -> Tuple[Any, int]:
        if debug:
            print(f"  [Regex] Attempting '{self.re.pattern}' at pos {position}")
        match = self.re.match(text, position)
        if match:
            if debug:
                print(f"    [Regex] MATCHED '{match.group(0)}'")
            return match.group(0), match.end()
        if debug:
            print(f"    [Regex] FAILED '{self.re.pattern}'")
        raise ParseError(f"Regex '{self.re.pattern}' did not match", position)


class OneOf(Parser):
    def __init__(self, *parsers: Parser):
        self.parsers = parsers

    def parse(
        self, text: str, position: int = 0, debug: bool = False
    ) -> Tuple[Any, int]:
        if debug:
            print(f"  [OneOf] Trying choices at pos {position}")
        for i, parser in enumerate(self.parsers):
            if debug:
                print(f"    [OneOf] Choice {i}:")
            try:
                result, next_pos = parser.parse(text, position, debug=debug)
                if debug:
                    print(f"    [OneOf] Choice {i} SUCCEEDED")
                return result, next_pos
            except ParseError:
                if debug:
                    print(f"    [OneOf] Choice {i} FAILED")
                continue
        raise ParseError("No parser in OneOf matched", position)


class Seq(Parser):
    def __init__(self, *parsers: Parser):
        self.parsers = parsers

    def parse(
        self, text: str, position: int = 0, debug: bool = False
    ) -> Tuple[Any, int]:
        if debug:
            print(f"  [Seq] Starting sequence at pos {position}")
        results = []
        current_pos = position
        for i, parser in enumerate(self.parsers):
            if debug:
                print(f"    [Seq] Step {i}:")
            try:
                result, next_pos = parser.parse(text, current_pos, debug=debug)
                results.append(result)
                current_pos = next_pos
            except ParseError as e:
                if debug:
                    print(f"    [Seq] Step {i} FAILED at {e.position}")
                raise ParseError(f"Sequence failed", e.position)
        if debug:
            print(f"  [Seq] SEQUENCE SUCCEEDED")
        return results, current_pos


class Opt(Parser):
    def __init__(self, parser: Parser):
        self.parser = parser

    def parse(
        self, text: str, position: int = 0, debug: bool = False
    ) -> Tuple[Any, int]:
        if debug:
            print(f"  [Opt] Attempting optional part at pos {position}")
        try:
            result, next_pos = self.parser.parse(text, position, debug=debug)
            if debug:
                print(f"    [Opt] SUCCEEDED")
            return result, next_pos
        except ParseError:
            if debug:
                print(f"    [Opt] FAILED (returning None)")
            return None, position


class Many(Parser):
    def __init__(self, parser: Parser):
        self.parser = parser

    def parse(
        self, text: str, position: int = 0, debug: bool = False
    ) -> Tuple[List[Any], int]:
        if debug:
            print(f"  [Many] Starting loop at pos {position}")
        results = []
        current_pos = position
        while True:
            try:
                result, next_pos = self.parser.parse(text, current_pos, debug=debug)
                results.append(result)
                current_pos = next_pos
            except ParseError:
                if debug:
                    print(f"    [Many] Loop terminated")
                break
        if debug:
            print(f"  [Many] SUCCEEDED (found {len(results)} items)")
        return results, current_pos


class Map(Parser):
    def __init__(self, parser: Parser, func: Callable[[Any], Any]):
        self.parser = parser
        self.func = func

    def parse(
        self, text: str, position: int = 0, debug: bool = False
    ) -> Tuple[Any, int]:
        result, next_pos = self.parser.parse(text, position, debug=debug)
        if debug:
            print(f"  [Map] Transforming result...")
        return self.func(result), next_pos


class Capture(Parser):
    def __init__(self, parser: Parser):
        self.parser = parser

    def parse(
        self, text: str, position: int = 0, debug: bool = False
    ) -> Tuple[Tuple[Any, str], int]:
        start_pos = position
        result, next_pos = self.parser.parse(text, position, debug=debug)
        matched_text = text[start_pos:next_pos]
        return (result, matched_text), next_pos


def token(parser: Parser, ws_parser: Parser) -> Parser:
    class TokenParser(Parser):
        def __init__(self, p: Parser, w: Parser):
            self.p = p
            self.w = w

        def parse(
            self, text: str, position: int = 0, debug: bool = False
        ) -> Tuple[Any, int]:
            if debug:
                print(f"  [Token] Consuming whitespace after parser...")
            res, next_pos = self.p.parse(text, position, debug=debug)
            _, final_pos = self.w.parse(text, next_pos, debug=debug)
            return res, final_pos

    return TokenParser(parser, ws_parser)
