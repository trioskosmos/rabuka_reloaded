"""Scrape the official Q&A list and append any new entries to qa_data.json.

Follows the site's JSON list endpoint (`/manage/rule-qa/list`), which replaced
the old server-rendered `/question/searchresults/` HTML blocks, and maps each
item onto the existing qa_data.json schema (id, date, question, answer,
related_cards).
"""

import json
import re
import time
import urllib.request
from pathlib import Path

BASE = "https://llofficial-cardgame.com"
QA_JSON = Path(__file__).parent / "qa_data.json"

LIST_URL = f"{BASE}/manage/rule-qa/list"
PER_PAGE = 100


def fetch(url):
    req = urllib.request.Request(
        url, headers={"User-Agent": "Mozilla/5.0", "Accept": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8", errors="replace"))


def normalise_text(value):
    return (value or "").replace("\r\n", "\n").replace("\r", "\n").strip()


def parse_related(card_names):
    """`[CARD_NO ： NAME] [CARD_NO ： NAME]` → the existing related_cards shape."""
    related = []
    for m in re.finditer(r"\[([^：:]+?) ：\s*([^\]]+)\]", card_names or ""):
        related.append(
            {
                "card_no": m.group(1).strip(),
                "name": m.group(2).strip(),
            }
        )
    return related


def to_entry(item):
    stamp = item.get("update_time") or item.get("regist_time") or ""
    return {
        "id": f"Q{item['id']}",
        "date": stamp.replace("/", "."),
        "question": normalise_text(item.get("question")),
        "answer": normalise_text(item.get("answer")),
        "related_cards": parse_related(item.get("card_names")),
    }


def scrape_all():
    items = []
    page = 1
    total = None
    while True:
        url = (
            f"{LIST_URL}?page={page}&per_page={PER_PAGE}&status=published"
            f"&t={int(time.time() * 1000)}"
        )
        payload = fetch(url)
        if total is None:
            total = int(payload.get("total") or 0)
            print(f"total: {total}")
        batch = payload.get("items") or []
        items.extend(batch)
        if not batch or len(items) >= total:
            break
        page += 1
        time.sleep(0.2)
    return items


def main():
    existing = json.loads(QA_JSON.read_text(encoding="utf-8"))
    existing_ids = {e["id"] for e in existing}
    print(f"Existing QA count: {len(existing)}")

    raw = scrape_all()
    print(f"Scraped {len(raw)} total QA entries")

    seen = set()
    new_entries = []
    for item in raw:
        entry = to_entry(item)
        if entry["id"] in existing_ids or entry["id"] in seen:
            continue
        seen.add(entry["id"])
        new_entries.append(entry)

    new_entries.sort(key=lambda i: int(i["id"][1:]), reverse=True)
    merged = existing + new_entries
    merged.sort(key=lambda i: int(i["id"][1:]), reverse=True)

    print(f"New QAs to add: {len(new_entries)}")
    for n in new_entries:
        print(f"  {n['id']} ({n.get('date', '')}) - {n['question'][:50]}")

    if new_entries:
        QA_JSON.write_text(
            json.dumps(merged, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        print(f"\nWrote {len(merged)} entries to qa_data.json")
    else:
        print("\nNo new QAs found - nothing written")

    (Path(__file__).parent / "_new_qa.json").write_text(
        json.dumps(new_entries, ensure_ascii=False, indent=2), encoding="utf-8"
    )


if __name__ == "__main__":
    main()
