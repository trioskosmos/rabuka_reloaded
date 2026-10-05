"""One command, end to end: every Loveca deck from decklog.

    python tools/decklog/harvest_all.py

Phases, all automatic once `login.py` has been done once:

  1. codes   walk the search pages and resolve enc -> deck code in the logged-in
             browser. Saves codes.txt + search_meta.jsonl as it goes, and recycles the
             tab every RECYCLE_EVERY pages because Chrome's renderer runs out of sockets
             after a few thousand requests to one origin (net::ERR_INSUFFICIENT_RESOURCES).
  2. cards   fetch every deck's card list anonymously over plain HTTP with a thread
             pool. Writes raw/<code>.json (verbatim source) and html/<code>.html (the
             decklog DOM shape, so the web UI converter can paste it).
  3. verify  audit the harvest and print the headline numbers.

Every phase resumes. Re-running picks up where it stopped.
"""

import argparse
import importlib.util
import json
import random
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROFILE = HERE / ".chrome-profile"
CODES = HERE / "codes.txt"
META = HERE / "search_meta.jsonl"
RAW = HERE / "raw"
HTML = HERE / "html"
ANALYSIS = HERE / "analysis"

API = "https://decklog.bushiroad.com/system/app/api"
GAME = 11
UA = ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
      "(KHTML, like Gecko) Chrome/153.0.0.0 Safari/537.36")

RECYCLE_EVERY = 150
DETAIL_CONCURRENCY = 5
SEARCH_CONCURRENCY = 2
REQUEST_PAUSE_MS = 40


def load_sibling(name):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def build_param(keyword=""):
    return {"content": str(GAME), "deck_param1": "", "deck_param2": "",
            "keyword": keyword, "keyword_or": "", "keyword_not": "",
            "user_title": "", "deckcode": "", "exact_match": 0,
            "search_main_deck": 0, "exclude_error": 1, "order": "date"}


def read_done():
    codes, meta = [], {}
    if CODES.exists():
        codes = [l.strip() for l in CODES.read_text(encoding="utf-8").splitlines() if l.strip()]
    if META.exists():
        for l in META.read_text(encoding="utf-8").splitlines():
            if l.strip():
                try:
                    r = json.loads(l)
                    if r.get("code"):
                        meta[r["code"]] = r
                except json.JSONDecodeError:
                    pass
    return codes, meta


class Session:
    """Logged-in browser context, with automatic tab recycling.

    A fresh tab means a fresh renderer, which is the only reliable cure for
    ERR_INSUFFICIENT_RESOURCES. Recycling here is what makes an unattended run
    possible at all.
    """

    def __init__(self, pw):
        self.pw = pw
        self.ctx = None
        self.page = None
        self.reqs = 0
        self.open_context()

    def open_context(self):
        self.close()
        self.ctx = self.pw.chromium.launch_persistent_context(
            user_data_dir=str(PROFILE), headless=True,
            args=["--disable-blink-features=AutomationControlled", "--no-sandbox"],
        )
        self.ctx.set_default_timeout(45000)
        self.page = self.ctx.new_page()

    def close(self):
        for closer in (self.ctx,):
            try:
                if closer:
                    closer.close()
            except Exception:
                pass
        self.ctx = None
        self.page = None

    def recycle(self):
        # Swap the tab, not the browser: the socket exhaustion is per-renderer, so a
        # fresh tab is the cure and it costs a fraction of relaunching the context.
        # If the whole context died, relaunch it - progress lives on disk, not in here.
        try:
            if self.page:
                self.page.close()
        except Exception:
            pass
        try:
            self.page = self.ctx.new_page()
        except Exception:
            self.open_context()

    def post(self, path, body=None):
        for attempt in range(3):
            try:
                res = self.page.evaluate(
                    """async ({path, body}) => {
                        const r = await fetch('https://decklog.bushiroad.com/system/app/api' + path, {
                            method: 'POST',
                            credentials: 'include',
                            headers: body ? {'Content-Type': 'application/json'} : {},
                            body: body ? JSON.stringify(body) : undefined,
                        });
                        const t = await r.text();
                        return t && t !== '[]' ? t : null;
                    }""",
                    {"path": path, "body": body},
                )
                self.reqs += 1
                if res:
                    return json.loads(res)
                return None
            except Exception:
                self.recycle()
                time.sleep(1.0 + attempt)
        return None

    def logged_in(self):
        try:
            return "is_logined" in self.page.content()
        except Exception:
            return False


def phase_codes(args):
    from concurrent.futures import ThreadPoolExecutor

    print("=" * 70)
    print("PHASE 1  codes  (logged-in browser)")
    print("=" * 70)

    codes, meta = read_done()
    print(f"resuming with {len(codes)} codes already harvested")

    if not PROFILE.exists():
        raise SystemExit("no profile: run login.py once first")

    from playwright.sync_api import sync_playwright
    with sync_playwright() as pw:
        s = Session(pw)
        if not s.logged_in():
            raise SystemExit(
                "not logged in. Run: python tools/decklog/login.py  (one time)")

        probe = s.post(f"/recipesearch/1", {"param": build_param()})
        if not probe:
            raise SystemExit("search returned nothing; is the session valid?")
        total = probe.get("count") or 0
        pages = probe.get("page_count") or 0
        per_page = len(probe.get("list") or []) or 10
        print(f"{total:,} decks over {pages:,} pages ({per_page}/page)")

        have = set(codes)
        stop = args.max_codes or total
        start_page = 1 + (len(have) // per_page)
        print(f"starting from page {start_page} (already have {len(have)} codes)")

        cf = CODES.open("a", encoding="utf-8")
        mf = META.open("a", encoding="utf-8")

        def resolve(item):
            d = s.post(f"/detail/{item['enc']}")
            return item, (d or {}).get("deck_id"), d

        page = start_page
        added = 0
        try:
            while page <= pages and len(have) < stop:
                got = s.post(f"/recipesearch/{page}", {"param": build_param()})
                items = (got or {}).get("list") or []
                if not items:
                    page += 1
                    continue

                with ThreadPoolExecutor(max_workers=args.detail_concurrency) as pool:
                    for item, code, _ in pool.map(resolve, items):
                        if not code or code in have:
                            continue
                        have.add(code)
                        added += 1
                        cf.write(code + "\n")
                        mf.write(json.dumps({
                            "code": code, "enc": item.get("enc"),
                            "title": item.get("title") or "",
                            "player_name": item.get("player_name") or "",
                            "publish_date": item.get("publish_date") or "",
                            "fav_count": item.get("fav_count") or 0,
                            "comment": item.get("comment") or "",
                            "regulation": item.get("deck_param1") or "",
                        }, ensure_ascii=False) + "\n")

                if page % 10 == 0:
                    cf.flush()
                    mf.flush()
                    rate = s.reqs / max(1, (page - start_page + 1))
                    print(f"  page {page}/{pages}  codes={len(have)} (+{added})  "
                          f"reqs={s.reqs}")

                page += 1
                if page % RECYCLE_EVERY == 0:
                    s.recycle()
                    print(f"  recycled the tab at page {page} (fresh renderer)")
                    cf.flush()
                    mf.flush()
                time.sleep(args.pause)
        finally:
            cf.close()
            mf.close()

    print(f"phase 1 done: {len(have)} codes, {added} new")
    return len(have)


def phase_cards(args):
    from concurrent.futures import ThreadPoolExecutor, as_completed

    sl = load_sibling("scrape_decklog")
    codes, _ = read_done()
    codes = list(dict.fromkeys(codes))
    done = {p.stem for p in RAW.glob("*.json")} if RAW.exists() else set()
    todo = [c for c in codes if c not in done]
    print("=" * 70)
    print(f"PHASE 2  cards  ({len(todo)} to fetch, {len(done)} already held)")
    print("=" * 70)
    if not todo:
        print("nothing to do")
        return

    def pull(code):
        try:
            data = sl.fetch_deck(code)
            sl.save_raw(RAW, code, data)
            sl.save_html(HTML, code, data)
            return code, sl.record_from_raw(data, code), None
        except Exception as exc:
            return code, None, exc

    ok = fail = 0
    with CODES.open("a", encoding="utf-8"):
        pass
    jf = (HERE / "decks.jsonl").open("a", encoding="utf-8")
    tf = (HERE / "decks.txt").open("a", encoding="utf-8")
    try:
        with ThreadPoolExecutor(max_workers=args.workers) as pool:
            futures = [pool.submit(pull, c) for c in todo]
            for i, fut in enumerate(as_completed(futures), 1):
                code, rec, exc = fut.result()
                if exc is not None:
                    fail += 1
                    if fail <= 20:
                        print(f"  {code}: FAILED {exc}")
                    continue
                jf.write(json.dumps(rec, ensure_ascii=False) + "\n")
                tf.write(sl.deck_block(rec))
                ok += 1
                if i % 250 == 0:
                    jf.flush()
                    tf.flush()
                    print(f"  {i}/{len(todo)}  ok={ok} fail={fail}")
                time.sleep(args.card_delay + random.uniform(0, args.card_delay))
    finally:
        jf.close()
        tf.close()
    print(f"phase 2 done: ok={ok} fail={fail}")
    print(f"raw json : {RAW}")
    print(f"html     : {HTML}")


def phase_verify(args):
    sl = load_sibling("scrape_decklog")
    print("=" * 70)
    print("PHASE 3  verify")
    print("=" * 70)
    args.jsonl = str(HERE / "decks.jsonl")
    args.cards_db = "cards/cards.json"
    args.spot_check = args.spot_check
    args.seed = 0
    args.delay = 0.3
    args.allow_unknown = False
    args.regulation = "S"
    args.energy = 12
    sl.cmd_verify(args)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--skip-codes", action="store_true")
    ap.add_argument("--skip-cards", action="store_true")
    ap.add_argument("--max-codes", type=int, default=0)
    ap.add_argument("--detail-concurrency", type=int, default=DETAIL_CONCURRENCY)
    ap.add_argument("--workers", type=int, default=8)
    ap.add_argument("--pause", type=float, default=0.04)
    ap.add_argument("--card-delay", type=float, default=0.05)
    ap.add_argument("--spot-check", type=int, default=15)
    args = ap.parse_args()

    if not args.skip_codes:
        phase_codes(args)
    if not args.skip_cards:
        phase_cards(args)
    phase_verify(args)
    print("\nall phases complete")


if __name__ == "__main__":
    sys.exit(main())