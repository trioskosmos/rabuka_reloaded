# -*- coding: utf-8 -*-
"""Aggregate success-live-zone cards, final scores, and turn counts from audit JSONL.

Usage:
  python analyze_success_zones.py out.txt audit1.jsonl [audit2.jsonl ...]
Each audit file should be labeled by deck — pass as path (deck name taken from
filename or sibling run metadata).
"""
import json
import sys
from collections import Counter, defaultdict


def load(path):
    games = {}  # game -> dict
    success_cards = defaultdict(list)  # game -> [(player, card_no, name)]
    decisions = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            ev = rec.get("event")
            if ev == "game_end":
                g = rec.get("game")
                z = rec.get("success_counts") or [0, 0]
                games[g] = {
                    "succ": list(z),
                    "turn": rec.get("turn", 0),
                    "decisions": rec.get("decisions", 0),
                }
            elif ev == "run_start":
                # optional deck label
                games_meta = rec.get("deck") or rec.get("metadata") or {}
            elif ev == "decision":
                own = (rec.get("view") or {}).get("own") or {}
                succ = own.get("success") or []
                p = rec.get("policy_player")
                g = rec.get("game")
                t = rec.get("turn")
                for c in succ:
                    if not c:
                        continue
                    card_no = c.get("card_no")
                    name = c.get("card_name") or c.get("name")
                    success_cards[(g, p)].append((t, card_no, name))
                decisions.append(rec)
    return games, success_cards, decisions


def main():
    out_path = sys.argv[1]
    paths = sys.argv[2:]
    if not paths:
        print("usage: analyze_success_zones.py out.txt audit.jsonl ...")
        sys.exit(1)

    lines = []
    lines.append("# Success live zones / turns / scores by deck audit\n")
    lines.append(f"audits: {len(paths)}\n\n")

    summary_rows = []
    all_card_counter = Counter()
    all_card_by_deck = {}

    for path in paths:
        # deck label from filename (e.g. deck_5CP3Z_idou.jsonl)
        base = path.replace("\\", "/").split("/")[-1]
        label = base
        if label.startswith("audit_") and label.endswith(".jsonl"):
            label = label[len("audit_"):-len(".jsonl")]
        elif label.endswith(".jsonl"):
            label = label[:-len(".jsonl")]

        games, success_cards, decisions = load(path)
        if not games:
            lines.append(f"## {label}\n(no game_end events in {path})\n\n")
            continue

        turns = [g["turn"] for g in games.values()]
        # winner: success[0] vs success[1] — policy_player is decision side;
        # game_end success_counts is [p1, p2] presumably
        finals = list(games.values())
        p1_w = sum(1 for g in finals if g["succ"][0] > g["succ"][1])
        p2_w = sum(1 for g in finals if g["succ"][1] > g["succ"][0])
        draws = sum(1 for g in finals if g["succ"][0] == g["succ"][1])
        avg_turn = sum(turns) / len(turns)
        # score histogram of finals
        score_pairs = Counter(tuple(g["succ"]) for g in finals)

        # success cards: prefer last observation per (game, player) — zone fills over time
        # Actually each decision records current success zone; take max-length set per game/player
        last_zone = {}
        # Re-scan is cheaper: from success_cards list order (append by decision order),
        # keep the last full zone snapshot: for each game/player, last time we saw N cards
        # success_cards[(g,p)] is a flat list of (t, card_no, name) appends across decisions.
        # Better: re-parse and keep final zone from last decision per (game,player).
        # We already appended duplicates across turns — dedupe by taking cards from last
        # decision that had success for that key. Rebuild properly:
        final_zone = {}
        for rec in decisions:
            own = (rec.get("view") or {}).get("own") or {}
            succ = own.get("success") or []
            key = (rec.get("game"), rec.get("policy_player"))
            final_zone[key] = [
                (c.get("card_no"), c.get("card_name") or c.get("name") or c.get("card_no"))
                for c in succ if c
            ]

        # card frequency across final zones
        card_freq = Counter()
        card_name = {}
        for (g, p), cards in final_zone.items():
            for cno, name in cards:
                card_freq[cno] += 1
                if name:
                    card_name[cno] = name
                all_card_counter[cno] += 1
        all_card_by_deck[label] = card_freq

        lines.append(f"## {label}\n")
        lines.append(f"- games: {len(finals)}\n")
        lines.append(f"- avg turns: {avg_turn:.2f} (min {min(turns)} max {max(turns)})\n")
        lines.append(f"- P1 wins {p1_w} | P2 wins {p2_w} | draws {draws}\n")
        lines.append(f"- final success pairs: {dict(sorted(score_pairs.items()))}\n")
        lines.append("\n### Success-zone cards (count of final-zone appearances)\n")
        lines.append("| card_no | name | times |\n|---|---|---:|\n")
        for cno, n in card_freq.most_common(30):
            lines.append(f"| {cno} | {card_name.get(cno, '')} | {n} |\n")
        if not card_freq:
            lines.append("| (none) | | |\n")
        lines.append("\n")
        summary_rows.append((label, len(finals), avg_turn, p1_w, p2_w, draws, card_freq))

    lines.append("\n## Cross-deck summary\n\n")
    lines.append("| deck | games | avg turns | P1 W | P2 W | draws | top success card |\n")
    lines.append("|---|---:|---:|---:|---:|---:|---|\n")
    for label, n, at, w1, w2, dr, freq in summary_rows:
        top = freq.most_common(1)
        top_s = f"{top[0][0]} ({top[0][1]}x)" if top else "-"
        lines.append(f"| {label} | {n} | {at:.2f} | {w1} | {w2} | {dr} | {top_s} |\n")

    lines.append("\n## Success-card frequency across ALL decks\n\n")
    lines.append("| card_no | total |\n|---|---:|\n")
    for cno, n in all_card_counter.most_common(40):
        lines.append(f"| {cno} | {n} |\n")

    # pairwise deck comparison on shared success cards (Jaccard-ish overlap)
    lines.append("\n## Deck pairwise top-card overlap (Jaccard on success-card sets)\n\n")
    labels = [r[0] for r in summary_rows]
    lines.append("| | " + " | ".join(labels) + " |\n")
    lines.append("|---" + "|---" * len(labels) + "|\n")
    for a in labels:
        set_a = set(all_card_by_deck[a])
        row = [a]
        for b in labels:
            set_b = set(all_card_by_deck[b])
            if not set_a and not set_b:
                row.append("n/a")
            else:
                j = len(set_a & set_b) / len(set_a | set_b)
                row.append(f"{j:.2f}")
        lines.append("| " + " | ".join(row) + " |\n")

    with open(out_path, "w", encoding="utf-8") as f:
        f.writelines(lines)
    print(f"wrote {out_path}")


if __name__ == "__main__":
    main()
