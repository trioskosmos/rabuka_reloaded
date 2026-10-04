// Three-request probe. Answers the only question that matters for request budget:
// does /detail/{enc} already contain the card list, or only the deck code?
//
// If it contains cards, we never need the anonymous view endpoint at all, which halves
// the total request count. If it only returns the code, we know the floor is
// 3,554 search + 35,536 detail + 35,536 view.
//
// Run this on the logged-in search page. It sends at most 3 requests total.

const API = "https://decklog.bushiroad.com/system/app/api";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function post(path, body) {
  const res = await fetch(API + path, {
    method: "POST",
    credentials: "include",
    headers: body ? { "Content-Type": "application/json" } : {},
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  if (!text || text === "[]") return { empty: true, status: res.status };
  try {
    return { data: JSON.parse(text), status: res.status };
  } catch {
    return { raw: text.slice(0, 400), status: res.status };
  }
}

(async () => {
  const param = {
    content: "11", deck_param1: "", deck_param2: "", keyword: "", keyword_or: "",
    keyword_not: "", user_title: "", deckcode: "", exact_match: 0,
    search_main_deck: 0, exclude_error: 1, order: "date",
  };

  const sr = await post("/recipesearch/900", { param });
  const items = (sr.data && sr.data.list) || [];
  console.log(`probe page 900 -> ${items.length} decks`);
  if (!items.length) {
    console.log("nothing to probe (not logged in, or page out of range)");
    return;
  }

  // PROBE 1: what does a single detail response actually contain?
  const d = await post(`/detail/${items[0].enc}`);
  const data = d.data;
  console.log("\n=== PROBE 1: detail response shape ===");
  if (!data) {
    console.log("empty/blocked:", d);
    return;
  }
  console.log("top-level keys:", Object.keys(data).join(", "));
  console.log("deck_id:", data.deck_id, " title:", data.title);
  for (const k of ["list", "sub_list", "cards", "p_list"]) {
    if (k in data) console.log(`  HAS "${k}" -> ${Array.isArray(data[k]) ? data[k].length + " entries" : typeof data[k]}`);
  }
  if (Array.isArray(data.list) && data.list.length) {
    console.log("  first card entry:", JSON.stringify(data.list[0]).slice(0, 260));
  }
  console.log(
    "\nVERDICT: " +
    (Array.isArray(data.list) && data.list.length
      ? "detail RETURNS CARDS -> anonymous view phase is unnecessary"
      : "detail returns only metadata -> view phase required for card data")
  );

  // PROBE 2: does detail accept several encs at once?
  const encs = items.slice(0, 3).map((i) => i.enc);
  const multi = await post("/detail", { encs });
  console.log("\n=== PROBE 2: bulk detail ===");
  console.log("POST /detail with 3 encs ->",
    multi.data ? Object.keys(multi.data).slice(0, 8) : (multi.raw || "empty"));

  // PROBE 3: is there a bulk view that takes multiple codes?
  const one = data.deck_id;
  if (one) {
    const bulk = await post(`/view/${one}/list`, { deck_id: one });
    console.log("\n=== PROBE 3: bulk view ===");
    console.log(`POST /view/${one}/list ->`, bulk.data ? "has data" : (bulk.raw || "empty"));
  }

  console.log("\n3 probes done.");
})();