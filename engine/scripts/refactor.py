"""Refactor AbilityResolver: remove game_state field, add gs param.

Strategy: pure text replacement. Three passes per file:
  1. Replace self.game_state -> gs in method bodies
  2. Add gs: &mut GameState to every fn signature in impl blocks
  3. Add gs, as first arg to all self.method() calls
"""

import re, os

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

FILES = [
    "src/ability/resolver.rs",
    "src/ability/choice.rs",
    "src/ability/effects.rs",
    "src/ability/move_cards.rs",
    "src/ability/cost.rs",
    "src/ability/compound.rs",
    "src/ability/look.rs",
]

SKIP = {"new", "take_looked_at", "get_pending_choice", "card_db", "match_cards_in_zone"}

SELF_GS = re.compile(r"\bself\.game_state\b")
SELF_CALL = re.compile(r"\bself\.(\w+)\s*\(")
IMPL_OPEN = re.compile(r"^\s*impl\s+(super::resolver::)?AbilityResolver\s*\{")


def process_file(path):
    with open(path, "r", encoding="utf-8") as f:
        orig = f.read()
    content = orig

    # ---- PASS 1: body replacement ----
    # Collapse self\n.game_state first
    content = re.sub(r"self\s*\n\s*\.game_state", "self.game_state", content)
    content = SELF_GS.sub("gs", content)

    # ---- PASS 2: add gs param to fn signatures inside impl block ----
    lines = content.splitlines(keepends=True)
    result = []

    i = 0
    while i < len(lines):
        line = lines[i]
        # Find impl AbilityResolver {
        if IMPL_OPEN.match(line.strip()):
            result.append(line)
            i += 1
            depth = 1
            # Process until closing brace
            while i < len(lines) and depth > 0:
                # Track brace depth
                for ch in lines[i]:
                    if ch == "{":
                        depth += 1
                    elif ch == "}":
                        depth -= 1

                if depth <= 0:
                    result.append(lines[i])
                    i += 1
                    break

                # Check for fn definition
                m = re.match(r"^(\s*(pub\s+)?fn\s+(\w+))\s*\(", lines[i])
                if not m:
                    result.append(lines[i])
                    i += 1
                    continue

                fn_name = m.group(3)
                if fn_name in SKIP:
                    result.append(lines[i])
                    i += 1
                    continue

                # Collect the full fn signature (including return type and opening {)
                sig_lines = []
                paren_depth = 0
                sig_done = False
                j = i
                while j < len(lines) and not sig_done:
                    sig_lines.append(lines[j])
                    for ch in lines[j]:
                        if ch == "(":
                            paren_depth += 1
                        elif ch == ")":
                            paren_depth -= 1
                            if paren_depth <= 0:
                                # Check what follows the )
                                rest = lines[j].split(")")[1] if ")" in lines[j] else ""
                                # Strip to find { or ->
                                body_check = rest.split("//")[0]
                                if "{" in body_check or body_check.strip().startswith(
                                    "->"
                                ):
                                    sig_done = True
                                    break
                    if sig_done:
                        break
                    # Also check if next line starts fn or closes impl
                    if j + 1 < len(lines) and (
                        re.match(r"^\s*(pub\s+)?fn\s+", lines[j + 1])
                        or lines[j + 1].strip().startswith("}")
                    ):
                        sig_done = True
                        break
                    j += 1

                # Now insert gs param into the combined signature
                combined = (
                    "".join(sig_lines)
                    .replace("\r\n", " ")
                    .replace("\n", " ")
                    .replace("\r", " ")
                )

                if "&mut self" in combined:
                    idx = combined.index("&mut self") + len("&mut self")
                    before = combined[:idx]
                    after = combined[idx:].lstrip()
                    if after.startswith(","):
                        after = after[1:].lstrip()
                    if after.startswith(")"):
                        combined = f"{before}, gs: &mut GameState{after}"
                    else:
                        combined = f"{before}, gs: &mut GameState, {after}"
                elif "&self" in combined:
                    idx = combined.index("&self") + len("&self")
                    before = combined[:idx]
                    after = combined[idx:].lstrip()
                    if after.startswith(","):
                        after = after[1:].lstrip()
                    if after.startswith(")"):
                        combined = f"{before}, gs: &mut GameState{after}"
                    else:
                        combined = f"{before}, gs: &mut GameState, {after}"

                result.append(combined.rstrip() + "\n")
                i = j + 1  # skip past collected sig lines
            continue
        result.append(line)
        i += 1

    content = "".join(result)

    # ---- PASS 3: add gs, to self.method() calls ----
    def call_replacer(m):
        name = m.group(1)
        if name in SKIP:
            return m.group(0)
        return f"self.{name}(gs, "

    content = SELF_CALL.sub(call_replacer, content)

    # ---- PASS 4: add import ----
    if "use crate::game_state" not in content:
        lines = content.splitlines(keepends=True)
        last_use = -1
        for k, ln in enumerate(lines):
            if ln.startswith("use "):
                last_use = k
        if last_use >= 0:
            ins = last_use + 1
            while ins < len(lines) and lines[ins].startswith("use "):
                ins += 1
            lines.insert(ins, "use crate::game_state::GameState;\n")
        content = "".join(lines)

    if content != orig:
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        return True
    return False


def main():
    for rel in FILES:
        path = os.path.join(BASE, rel)
        ok = process_file(path)
        print(f"  {'Modified' if ok else 'No changes'}: {rel}")


if __name__ == "__main__":
    main()
