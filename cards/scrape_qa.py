"""Scrape the official Q&A list and append any new entries to qa_data.json.

Follows /question/search_ex pagination, parses qa-Item blocks matching the
existing qa_data.json schema (id, date, question, answer, related_cards).
"""

import json
import re
import time
import urllib.parse
import urllib.request
from pathlib import Path

BASE = "https://llofficial-cardgame.com"
QA_JSON = Path(__file__).parent / "qa_data.json"

SEARCH_URL = (
    f"{BASE}/question/searchresults/"
    "?keyword=&keyword_type%5B%5D=all&search_type=and&title=&card_kind=&work_title="
)


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=20) as resp:
        return resp.read().decode("utf-8", errors="replace")


def get_max_page(html):
    m = re.search(r"max_page\s*=\s*(\d+)", html)
    return int(m.group(1)) if m else 1


def unescape(s):
    return (
        s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#039;", "'")
        .replace("&quot;", '"')
    )


def parse_items(html):
    items = []
    # Each qa-Item block
    blocks = re.split(r'<div class="qa-Item qa-Wrapper">', html)[1:]
    for block in blocks:
        entry = {}
        m = re.search(r'faq-Heading">(Q\d+)\s*\((\d{4}\.\d{2}\.\d{2})\)', block)
        if not m:
            # some pages may omit date
            m = re.search(r'faq-Heading">(Q\d+)', block)
            entry["id"] = m.group(1) if m else ""
            entry["date"] = ""
        else:
            entry["id"] = m.group(1)
            entry["date"] = m.group(2)

        m = re.search(r'<p class="question-Detail">(.*?)</p>', block, re.DOTALL)
        entry["question"] = (
            re.sub(
                r"<br\s*/?>", "\n", unescape(re.sub(r"<[^>]+>", "", m.group(1)))
            ).strip()
            if m
            else ""
        )

        m = re.search(r'<p class="answer-Detail">(.*?)</p>', block, re.DOTALL)
        entry["answer"] = (
            re.sub(
                r"<br\s*/?>", "\n", unescape(re.sub(r"<[^>]+>", "", m.group(1)))
            ).strip()
            if m
            else ""
        )

        # related cards: [CARD_NO ： NAME]
        related = []
        for m in re.finditer(r"\[([^：:]+?) ：\s*([^\]]+)\]", block):
            related.append(
                {
                    "card_no": m.group(1).strip(),
                    "name": m.group(2).strip(),
                }
            )
        entry["related_cards"] = related

        if entry["id"]:
            items.append(entry)
    return items


def main():
    existing = json.loads(QA_JSON.read_text(encoding="utf-8"))
    existing_ids = {e["id"] for e in existing}
    print(f"Existing QA count: {len(existing)}")

    all_items = []
    html = fetch(SEARCH_URL)
    max_page = get_max_page(html)
    print(f"max_page: {max_page}")

    all_items.extend(parse_items(html))
    for pg in range(2, max_page + 1):
        url = (
            f"{BASE}/question/search_ex"
            f"?keyword=&keyword_type%5B0%5D=all&search_type=and"
            f"&title=&card_kind=&work_title=&page={pg}"
            f"&t={int(time.time() * 1000)}"
        )
        try:
            data = fetch(url)
        except Exception as e:
            print(f"  page {pg} failed: {e}")
            continue
        all_items.extend(parse_items(data))
        time.sleep(0.2)

    print(f"Scraped {len(all_items)} total QA entries")
    print(f"Unique ids: {len({i['id'] for i in all_items})}")

    # Find new ones not already present
    seen = set()
    new_entries = []
    for item in all_items:
        if item["id"] in existing_ids or item["id"] in seen:
            continue
        seen.add(item["id"])
        new_entries.append(item)

    # Sort new by numeric desc, then append to keep desc ordering by newest first
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

    # quick dump for verification of new ones
    out = [e for e in new_entries if True]
    (Path(__file__).parent / "_new_qa.json").write_text(
        json.dumps(new_entries, ensure_ascii=False, indent=2), encoding="utf-8"
    )


if __name__ == "__main__":
    main()
