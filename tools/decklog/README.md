# decklog scraper

Downloads deck recipes from `decklog.bushiroad.com` (Loveca, game id `11`) and converts
them to the plain `CARD-ID x QTY` text that `web_ui/deck_converter.html` and
`web_ui/js/modals/GameSetupModal.js` already parse.

**No login. No browser. No cookies. No dependencies** — stdlib Python 3 only.

## How it works

The site is a Vue SPA. Its pages render nothing useful server-side (the HTML is a
`<div id=app>` shell), but the SPA fetches deck data from a plain JSON endpoint that
answers any request:

```
GET  https://decklog.bushiroad.com/system/app/api/view/{code}
POST https://decklog.bushiroad.com/system/app/api/recipesearch/{page}
```

Two non-obvious details, both of which cost time to find:

- **CloudFront requires the SPA's `Referer` / `Origin` headers.** Without them the
  endpoint returns `[]` and looks like the deck is private. It is not.
- **Playwright's default headless-shell user agent gets a hard 403** from CloudFront.
  Irrelevant now, since no browser is used, but it is why browser-based approaches
  appear to require a login.

Response shape for a deck:

| field | meaning |
|---|---|
| `list[]` | main deck; `card_kind` `M` = member, `L` = live |
| `sub_list[]` | energy deck |
| `card_number` | card id, sometimes with a fullwidth `＋` (normalized to `+`) |
| `num` | count |
| `deck_param1` | regulation (`S` standard, `H` half deck, `O` other) |

## Usage

```powershell
# one or more specific decks
python tools/decklog/scrape_decklog.py fetch --code 1TG900 --code XGD0

# from a list of codes, one per line
python tools/decklog/scrape_decklog.py fetch --codes-file tools/decklog/codes.txt --limit 500

# convert to the web UI import format
python tools/decklog/scrape_decklog.py emit-import tools/decklog/decks.jsonl
```

Output is appended to `decks.jsonl` and de-duplicated by deck code, so re-running
resumes rather than re-downloading. `--delay` (default 1.5s) and `--jitter` throttle
requests; `main` deck totals that are not 30/40/60 are flagged in the log.

## Getting deck codes

Deck *content* by code is fully public — `fetch` needs no session at all. Discovering
*codes* is the only gated part, and it is gated on purpose:

- `recipesearch` answers anonymously (35,536 Loveca decks, 3,554 pages) but returns an
  opaque `enc` per deck instead of a code.
- `/system/app/api/detail/{enc}` is what resolves `enc` to a code. It returns `[]`
  anonymously, in every request variant tried (GET/POST, trailing slash, JSON body,
  form body, `+game_title_id`), and not even within the same session that produced the
  `enc`. No session cookies are set for anonymous browsing at all.
- `enc` is not recoverable by hashing: it is not `md5(id)`, `md5(code)`, or
  `md5(id+game_title_id)`.
- `/system/app/thumb/{enc}` *is* public and returns a rendered deck PNG, so images are
  deliberately open while the underlying data is not.

Because of that, the codes have to come from a logged-in session. Rather than hand over
credentials, `export_codes_browser.js` collects them inside your own browser — nothing
is shared.

### Step 1: test it first (about 10 seconds, non-destructive)

The script ships in **test mode**, which is the default. It walks page 50 and exercises
the entire chain for those 10 decks — search → `enc` → code → deck data:

1. Log in at `https://decklog.bushiroad.com/`, open `https://decklog.bushiroad.com/search?c=11`
2. F12 → Console → paste `export_codes_browser.js` → Enter

It prints a table of the 10 decks with resolved codes and card counts:

```
  code      title                          main  energy
  1TG900    僕光                               60      12  ok
```

A green run prints `=== TEST PASSED ===`. If rows show `(no code - detail/...)` you are
not logged in; if `main` is `ERR`, the data endpoint rejected the request. Nothing is
downloaded in test mode.

### Step 2: full run

Set `CONFIG.TEST_PAGES = []` and re-paste. There is nothing else to configure — the
script reads `count` and `page_count` from the first search response and sizes itself,
so it stays correct as new decks are published. Set `CONFIG.PAGES` to a positive number
only to scope a smaller run.

It downloads `codes.txt`, plus a `codes_partial_p<N>.txt` and
`search_meta_partial_p<N>.jsonl` checkpoint every 100 pages so a crash or a closed tab
costs at most 100 pages of work.

### Step 3: fetch every deck (still no session)

```powershell
python tools/decklog/scrape_decklog.py fetch --codes-file tools/decklog/codes.txt --target-hours 4
python tools/decklog/scrape_decklog.py build
```

`build` is what merges `search_meta.jsonl` into the derived records.

## Finishing in about 8 hours

The job is two phases of roughly equal request volume, so split the window evenly:

| phase | requests | window | notes |
|---|---|---|---|
| `export_codes_browser.js` (`TARGET_HOURS`) | ~39,094 | ~4h | `PAGE_CONCURRENCY: 4` |
| `scrape_decklog.py fetch --target-hours` | 35,536 | ~4h | ~405ms average |

Neither phase hardcodes a deck or page count: the browser script reads `page_count` from
the API, and `fetch` derives its delay from however many codes are left in the file.

**The code phase is latency-bound, not rate-limited.** Measured sequentially it runs
~12.3s per page (about 1.1s per request), which would take ~12h. That is response
latency, not a server-imposed limit, so `PAGE_CONCURRENCY` resolves a page's decks in
parallel:

| `PAGE_CONCURRENCY` | projected | total |
|---|---|---|
| 1 | ~12.3s/page | ~12.1h |
| 2 | ~6.6s/page | ~6.5h |
| 4 | ~3.8s/page | ~3.7h |
| 8 | ~2.3s/page | ~2.3h |

Pacing is **deadline-based**, not sleep-based: each page sleeps only the slack between
what it actually cost and its share of the remaining budget. Slow responses therefore
push the ETA out instead of the finish time, and if the run falls behind it drops its
sleeps and catches up rather than silently overrunning. The progress line shows
`s/page vs Ns budget`, the deadline ETA, and a `[behind: server-limited]` marker with the
true projection when it can't hit the target.

This is the phase running under your account, so raise `PAGE_CONCURRENCY` cautiously.
`TARGET_HOURS` and `--target-hours` are the only timing knobs. If a window implies less
than 0.25s between requests, `fetch` says so and floors the rate instead of hammering.

## Verification and recovery

`verify` audits the harvest instead of trusting it:

```powershell
# offline audit: counts, duplicates, totals, card IDs vs cards/cards.json
python tools/decklog/scrape_decklog.py verify tools/decklog/decks.jsonl

# plus re-fetch a random sample live and diff against what was stored
python tools/decklog/scrape_decklog.py verify tools/decklog/decks.jsonl --spot-check 20
```

It reports duplicate codes, the distribution of main/energy totals, decks that aren't
30/40/60, and — the important one — **card IDs present in the decks but absent from
`cards/cards.json`**, which is what would silently break deck loading in the engine. The
spot check re-downloads N random decks and compares the full card map, so corruption or
silent format drift gets caught rather than discovered later.

Recovery:

- `fetch` appends and flushes per record and de-duplicates against `decks.jsonl`, so
  interrupting it and re-running resumes where it stopped.
- Codes come from `codes_partial_p<N>.txt` if `codes.txt` never arrives.
- Keep every `codes_partial_*.txt`; feed several to `fetch` via `--codes-file` (it
  de-duplicates) if a run was interrupted across sessions.

If you already have rendered deck pages saved to disk, the same regex the web UI uses
can be run over them directly, which is the existing manual paste workflow in batch form:

```powershell
python tools/decklog/scrape_decklog.py convert-html pages/ --out combined.txt
```

## Raw first, conversion second

Conversion is never allowed to be the only copy of the data. `fetch` writes the verbatim
API response to `raw/<code>.json` **before** deriving anything from it, and everything
downstream is reproducible from those files with no network at all:

```
tools/decklog/
  raw/<code>.json        verbatim API response, the source of truth
  decks.jsonl            derived records, one JSON per line
  decks.txt              derived human-readable lists, same content as emit-import
  search_meta.jsonl      owner / publish_date / fav_count from the logged-in search
  codes.txt              deck codes
```

Rebuild every derived file offline — useful after changing the converter, or to prove
the derived data still matches the source:

```powershell
python tools/decklog/scrape_decklog.py build
```

`build` merges `search_meta.jsonl` when present. That file matters because the
anonymous data endpoint returns `owner: false`, empty `memo`, and no `publish_date`;
those fields only come back on the search response, which your browser session can see.

### What gets captured

Beyond the counts, each record keeps everything the API sent:

| scope | fields |
|---|---|
| deck | `title`, `memo`, `regulation`, `fav_count`, `id`, `status`, `comment`, `has_recipe_error`, `is_publish`, `publish_date`, `owner_name` (via search meta) |
| card | `num`, `name`, `rare`, `card_kind`, `blade`, `blade_heart`, `img`, `max`, `type`, `slot`, `sub`, `p_param`, `g_param` |

`members` / `lives` / `energy` are the convenience views; `cards` is the full per-card
map including `blade` and `blade_heart`.

### Streaming output

Both derived files are appended and flushed per deck, so you can read, grep or `Ctrl+C`
at any point during a multi-hour run and keep everything already harvested. `decks.txt`
is byte-identical to what `emit-import` would produce at the end — `build` and
`emit-import` share one formatter so the three paths cannot drift.

## Rate limits and enforcement

Measured against the live endpoint, not assumed:

- **No rate limiting is advertised or enforced at 5 req/s.** A 40-request burst at
  200ms spacing produced no `429`, no `Retry-After`, and no `X-RateLimit-*` headers.
  Latency stayed at 66–110ms with no upward drift (one 3.6s outlier).
- **Responses are `Cache-Control: no-store` with `X-Cache: Miss from cloudfront`**, so
  every request reaches the Apache origin. There is no CDN absorbing the load — 35k
  requests means 35k origin hits. That, not a limiter, is the thing to be gentle about.
- **CloudFront WAF is real and already caught us once**: Playwright's default
  headless-shell user agent gets a hard `403`. Rules can change at any time, so treat
  the UA spoof as necessary rather than decorative.
- Origin is CakePHP (`Set-Cookie: CAKEPHP=...`) — an application session, currently
  anonymous and unused by the data fetch.

On enforcement: the English Terms prohibit unauthorized access to the service and Bushiroad
reserves the right to suspend access at any time, without notice. I found no clause
specifically addressing automated retrieval of deck data. Bushiroad's general site terms
prohibit unauthorized reproduction and redistribution of their content, so the decks are
fine to use locally but not to republish wholesale. For context, at least one third-party
tracker (hocg-logs.holotune.jp) states publicly that it pulls deck data from DECK LOG via
its public API — so this is a known, working path, not an exploit.

The practical risk split is worth being precise about:

- **The data phase carries no credentials.** No session, no account, nothing to lose but
  your IP. Worst realistic outcome is IP-level throttling.
- **The code-collection phase runs under your real account.** That is where account
  risk lives, which is why it defaults to a test and checkpoints every 100 pages.

```json
{"code":"1TG900","url":"...","title":"僕光","regulation":"S","game_title_id":11,
 "members":{"PL!-BP3-014-N":4},"lives":{"PL!-BP4-022-L":2},
 "energy":{"LL-E-001-SD":12},"total":60,"energy_total":12}
```

## Analysis half

Runs on the harvested data with no network access. All three read `raw/` (or a
`search_meta*.jsonl`) and never mutate it.

```powershell
# whole-corpus report + deduped outputs
python tools/decklog/analyze_combos.py --owners tools/decklog/search_meta.jsonl

# one deck in detail, by position in the harvest or by code
python tools/decklog/one_deck.py --index 337
python tools/decklog/one_deck.py --code XGD0

# audit the search metadata alone (field completeness, dates, owners)
python tools/decklog/analyze_meta.py C:/Users/trios/Downloads/search_meta_partial_p100.jsonl
```

`analyze_combos.py` writes to `analysis/`:

| file | contents |
|---|---|
| `analysis_report.txt` | the full findings |
| `combos.jsonl` | one record per unique combo: cards, copy count, every deck code, owners, distinct energy options |
| `deduped_decks.txt` | one deck per unique combo in web-UI import format |
| `unique_combos.txt` | bare combo keys, one per line |

### What counts as "the same deck"

A **combo** is the card list with rarity stripped and energy cards excluded:

- **Rarity is cosmetic.** The card number is everything up to the 3-digit number;
  whatever follows is the printing — `N`, `R`, `M`, `P`, `PR`, `SEC`, `SD`, `SD2`,
  `RM`, `AR`, `L`, `R＋`, `P＋`. All collapse onto the card number, so decks differing
  only in foil are one combo. A typical 60-card list is ~22-33 distinct cards after
  collapsing.
- **Energy is excluded** because every energy card does the same job, so two decks are
  the same combo even if they run different energy. `combos.jsonl` still records how many
  distinct energy options each combo was seen with.

### Two attribution traps

Both produced wrong answers before they were handled:

1. **Hidden owner names are not "one owner".** `player_name` is empty on ~21% of decks,
   concentrated on the most-copied archetype lists — the 12x `ジャンヌ・ダルク` cluster is
   entirely hidden. Counting hidden as a single owner reports a widely-shared list as one
   person's private collection, so the report splits duplicate combos into
   `all owners hidden` / `1 visible owner` / `N visible owners`.
2. **`fav_count` is not usable.** It comes back `0` for 97% of decks (max 1), which is not
   plausible for a site with a visible heart counter. Treat it as absent, not as a
   popularity signal.

### Clustering

Exact-combo dedupe is not enough, because most real duplication is *near*-duplicate: the
same archetype with one card swapped. Decks are bucketed by live-card set (the 12 live
slots define the archetype), then greedily clustered within each bucket by Jaccard
similarity over member cards (`--threshold`, default `0.8`). That yields archetype
clusters instead of a pile of near-identical rows.

Measured 1.1s per 1000 decks, so the full corpus analyses in well under a minute.

## Notes

- These are other players' deck lists. Fine for local testing and bot training; don't
  redistribute them wholesale.
- Keep `--delay` at 1.5s or higher. A few hundred decks covers the archetype space —
  a full-site pull is the part that gets an IP throttled, and it isn't needed.