import argparse
import copy
import json
from pathlib import Path
import subprocess
import tempfile


def run(executable, engine, arguments, expected=None):
    result = subprocess.run([str(executable), *map(str, arguments)], cwd=engine,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            encoding="utf-8", errors="strict")
    if expected is None:
        if result.returncode:
            raise AssertionError(result.stderr)
    elif result.returncode == 0 or expected not in result.stderr:
        raise AssertionError(f"Expected failure containing {expected!r}: {result.stderr}")
    return result


def write_trace(path, records):
    path.write_text("".join(json.dumps(item, ensure_ascii=False) + "\n" for item in records),
                    encoding="utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("executable", type=Path)
    parser.add_argument("--engine", type=Path, default=Path(__file__).resolve().parent.parent / "engine")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    executable = args.executable.resolve()
    deck = args.engine.resolve().parent / "web_ui/decks/muse_cup.txt"
    with tempfile.TemporaryDirectory(prefix="replay-smoke-") as directory:
        root = Path(directory)
        trace = root / "trace.jsonl"
        record = run(executable, args.engine, ["record", trace, deck, 2, 42, 7])
        replay = run(executable, args.engine, ["replay", trace, deck])
        print(record.stderr, end="")
        print(replay.stderr, end="")
        original = trace.read_bytes()
        run(executable, args.engine, ["record", trace, deck, 2, 42, 7], "already exists")
        assert trace.read_bytes() == original
        records = [json.loads(line) for line in original.decode("utf-8").splitlines()]
        cases = []
        wrong = copy.deepcopy(records)
        wrong[0]["deck_text"] += "changed"
        cases.append((wrong, "header/deck_text"))
        wrong = copy.deepcopy(records)
        wrong[0]["build_identity"]["assets_sha256"] = "changed"
        cases.append((wrong, "build_identity/assets_sha256"))
        wrong = copy.deepcopy(records)
        wrong[1]["initial"]["turn"] = 99
        cases.append((wrong, "game/0/initial/turn"))
        wrong = copy.deepcopy(records)
        action = next(step for step in wrong[1]["steps"] if step["operation"] == "action")
        action["selected"] = None
        cases.append((wrong, "Recorded semantic action is not available"))
        cases.append((records[:-1], "Truncated trace"))
        cases.append((records + [{}], "Trailing trace records"))
        for number, (data, expected) in enumerate(cases):
            bad = root / f"bad-{number}.jsonl"
            write_trace(bad, data)
            run(executable, args.engine, ["replay", bad, deck], expected)
        if args.output:
            with args.output.open("x", encoding="utf-8", newline="\n") as output:
                output.write(replay.stdout)
        print(f"PASS: two-game cross-process replay and {len(cases) + 1} negative cases")


if __name__ == "__main__":
    main()
