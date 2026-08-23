# -*- coding: utf-8 -*-
import io
import json
import subprocess

diff = subprocess.run(
    ["git", "diff", "-U2", "--", "../abilities.json"],
    capture_output=True,
    text=True,
    encoding="utf-8",
    errors="replace",
).stdout
hunks = [h for h in diff.split("@@") if "action" in h]
print("hunks with effect changes:", max(0, len(hunks) - 1))
# Show the full_text entries adjacent to changed hunks via re-parse comparison
old = json.loads(diff.splitlines()[0]) if False else None
