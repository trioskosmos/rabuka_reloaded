"""Phase B: extract the Step::Setup match arms from the 3ds bin into per-phase
handler functions in src/setup.rs. Arm bodies are copied verbatim (re-indented)
so semantic equivalence is preserved by construction."""

import re
import os

ROOT = os.path.dirname(os.path.abspath(__file__))
BIN = os.path.join(ROOT, "src", "bin", "rabuka_3ds.rs")
OUT = os.path.join(ROOT, "src", "setup.rs")


def read(p):
    with open(p, encoding="utf-8") as f:
        return f.read()


def write(p, s):
    with open(p, "w", encoding="utf-8", newline="\n") as f:
        f.write(s)


def find_body_end(src, open_idx):
    depth = 0
    i = open_idx
    n = len(src)
    while i < n:
        c = src[i]
        if c == '"':
            i += 1
            while i < n and src[i] != '"':
                if src[i] == "\\":
                    i += 1
                i += 1
            i += 1
            continue
        if c == "'":
            i += 1
            if i < n and src[i] == "\\":
                i += 1
            if i < n:
                i += 1
            i += 1
            continue
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            while i < n and src[i] != "\n":
                i += 1
            continue
        if c == "/" and i + 1 < n and src[i + 1] == "*":
            i += 2
            while i + 1 < n and not (src[i] == "*" and src[i + 1] == "/"):
                i += 1
            i += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n


def reindent(expr_text, target_first=4):
    lines = expr_text.split("\n")
    nonblank = [ln for ln in lines if ln.strip()]
    if not nonblank:
        return expr_text
    min_indent = min(len(ln) - len(ln.lstrip()) for ln in nonblank)
    out = []
    for ln in lines:
        if not ln.strip():
            out.append("")
        else:
            out.append(" " * target_first + ln[min_indent:])
    return "\n".join(out)


# Per-arm: (handler name, params after (cards, decks))
SIGS = {
    "PickMode": (
        "pick_mode",
        [("keys", "u32"), ("n", "usize"), ("was_dirty", "bool"), ("cur", "usize")],
    ),
    "PickDeck": (
        "pick_deck",
        [
            ("keys", "u32"),
            ("n", "usize"),
            ("was_dirty", "bool"),
            ("cur", "usize"),
            ("vs_ai", "bool"),
            ("is_multiplayer", "bool"),
        ],
    ),
    "MultiplayerDeck": (
        "multiplayer_deck",
        [("keys", "u32"), ("n", "usize"), ("was_dirty", "bool"), ("cur", "usize")],
    ),
    "PickDeck2": (
        "pick_deck2",
        [
            ("keys", "u32"),
            ("n", "usize"),
            ("was_dirty", "bool"),
            ("cur", "usize"),
            ("p1_idx", "usize"),
            ("vs_ai", "bool"),
        ],
    ),
    "Loading": (
        "loading",
        [("p1_idx", "usize"), ("p2_idx", "usize"), ("vs_ai", "bool")],
    ),
    "Testing": ("testing", [("keys", "u32")]),
    "QrScan": ("qr_scan", [("keys", "u32"), ("was_dirty", "bool"), ("ctx", "usize")]),
    "QrResult": (
        "qr_result",
        [("keys", "u32"), ("was_dirty", "bool"), ("cards_read", "Vec<String>")],
    ),
    "QrNotDeck": (
        "qr_not_deck",
        [
            ("keys", "u32"),
            ("was_dirty", "bool"),
            ("scanned_text", "String"),
            ("frames_left", "u32"),
        ],
    ),
    "DeckViewer": (
        "deck_viewer",
        [
            ("keys", "u32"),
            ("was_dirty", "bool"),
            ("card_ids", "&Vec<i16>"),
            ("mut offset", "usize"),
            ("vs_ai", "bool"),
            ("is_multiplayer", "bool"),
            ("viewing_card", "&mut Option<i16>"),
            ("card_db", "&Arc<CardDatabase>"),
            ("atlas", "&CardAtlas"),
        ],
    ),
    "ControlGuide": (
        "control_guide",
        [("keys", "u32"), ("was_dirty", "bool"), ("page", "usize")],
    ),
    "MultiplayerPickRole": (
        "multiplayer_pick_role",
        [
            ("keys", "u32"),
            ("was_dirty", "bool"),
            ("deck_idx", "usize"),
            ("cur", "usize"),
        ],
    ),
    "MultiplayerHostWait": (
        "multiplayer_host_wait",
        [("keys", "u32"), ("was_dirty", "bool"), ("p1_idx", "usize")],
    ),
    "MultiplayerClientScan": (
        "multiplayer_client_scan",
        [
            ("keys", "u32"),
            ("was_dirty", "bool"),
            ("p1_idx", "usize"),
            ("frames", "u32"),
        ],
    ),
    "MultiplayerClientHostSelect": (
        "multiplayer_client_host_select",
        [
            ("keys", "u32"),
            ("p1_idx", "usize"),
            ("hosts", "&Vec<u16>"),
            ("cursor", "usize"),
        ],
    ),
    "MultiplayerSyncDeck": (
        "multiplayer_sync_deck",
        [
            ("was_dirty", "bool"),
            ("p1_idx", "usize"),
            ("p2_idx", "usize"),
            ("is_host", "bool"),
        ],
    ),
    "MultiplayerLoading": (
        "multiplayer_loading",
        [
            ("p1_idx", "usize"),
            ("p2_idx", "usize"),
            ("is_host", "bool"),
            ("deck_sync_bytes", "Option<Vec<u8>>"),
            ("seed", "u64"),
        ],
    ),
}

# Dispatcher match arm: (pattern text) -> handler + call args
DISPATCH = {
    "PickMode": ("PickMode(cur)", ["cards", "decks", "keys", "n", "was_dirty", "cur"]),
    "PickDeck": (
        "PickDeck(cur, vs_ai, is_multiplayer)",
        ["cards", "decks", "keys", "n", "was_dirty", "cur", "vs_ai", "is_multiplayer"],
    ),
    "MultiplayerDeck": (
        "MultiplayerDeck(cur)",
        ["cards", "decks", "keys", "n", "was_dirty", "cur"],
    ),
    "PickDeck2": (
        "PickDeck2(cur, p1_idx, vs_ai)",
        ["cards", "decks", "keys", "n", "was_dirty", "cur", "p1_idx", "vs_ai"],
    ),
    "Loading": (
        "Loading(p1_idx, p2_idx, vs_ai)",
        ["cards", "decks", "p1_idx", "p2_idx", "vs_ai"],
    ),
    "Testing": ("Testing", ["cards", "decks", "keys"]),
    "QrScan": ("QrScan(ctx)", ["cards", "decks", "keys", "was_dirty", "ctx"]),
    "QrResult": (
        "QrResult(cards_read)",
        ["cards", "decks", "keys", "was_dirty", "cards_read"],
    ),
    "QrNotDeck": (
        "QrNotDeck(scanned_text, frames_left)",
        ["cards", "decks", "keys", "was_dirty", "scanned_text", "frames_left"],
    ),
    "DeckViewer": (
        "DeckViewer(ref card_ids, offset, _, vs_ai, is_multiplayer, ref mut viewing_card, ref card_db, ref atlas)",
        [
            "cards",
            "decks",
            "keys",
            "was_dirty",
            "card_ids",
            "offset",
            "vs_ai",
            "is_multiplayer",
            "viewing_card",
            "card_db",
            "atlas",
        ],
    ),
    "ControlGuide": (
        "ControlGuide(page)",
        ["cards", "decks", "keys", "was_dirty", "page"],
    ),
    "MultiplayerPickRole": (
        "MultiplayerPickRole(deck_idx, cur)",
        ["cards", "decks", "keys", "was_dirty", "deck_idx", "cur"],
    ),
    "MultiplayerHostWait": (
        "MultiplayerHostWait(p1_idx)",
        ["cards", "decks", "keys", "was_dirty", "p1_idx"],
    ),
    "MultiplayerClientScan": (
        "MultiplayerClientScan(p1_idx, frames)",
        ["cards", "decks", "keys", "was_dirty", "p1_idx", "frames"],
    ),
    "MultiplayerClientHostSelect": (
        "MultiplayerClientHostSelect(p1_idx, ref hosts, cursor)",
        ["cards", "decks", "keys", "p1_idx", "hosts", "cursor"],
    ),
    "MultiplayerSyncDeck": (
        "MultiplayerSyncDeck(p1_idx, p2_idx, is_host)",
        ["cards", "decks", "was_dirty", "p1_idx", "p2_idx", "is_host"],
    ),
    "MultiplayerLoading": (
        "MultiplayerLoading(p1_idx, p2_idx, is_host, deck_sync_bytes, seed)",
        ["cards", "decks", "p1_idx", "p2_idx", "is_host", "deck_sync_bytes", "seed"],
    ),
}


def main():
    src = read(BIN)
    marker = "Step::Setup(ref cards, ref decks, ref phase, ref dirty) => {"
    idx = src.index(marker)
    mj = src.index("match phase.clone() {", idx)
    mopen = src.index("{", mj)
    mend = find_body_end(src, mopen)

    starts = [
        m.start() + mopen + 20
        for m in re.finditer(r"(?m)^ {20}SetupPhase::", src[mopen:mend])
    ]
    arms = []
    for s in starts:
        name_m = re.match(r"SetupPhase::(\w+)", src[s:])
        assert name_m is not None, "regex is anchored on SetupPhase:: at " + str(s)
        arm_name = name_m.group(1)
        arrow = src.index("=>", s)
        after = src[arrow + 2 :]
        k = 0
        while after[k] in " \t\n":
            k += 1
        expr_full_start = arrow + 2 + k
        body_open = src.index("{", expr_full_start)
        body_end = find_body_end(src, body_open)
        expr = src[expr_full_start:body_end]
        arms.append((arm_name, expr))

    out = []
    out.append('#![cfg(feature = "3ds")]')
    out.append("")
    out.append("// Setup state machine: one handler function per SetupPhase.")
    out.append(
        "// Each handler returns the next Step. Bodies were moved verbatim from the"
    )
    out.append("// Step::Setup match arm in the bin (see extract_setup.py).")
    out.append("")
    out.append("use std::collections::{HashMap, HashSet};")
    out.append("use std::sync::Arc;")
    out.append("")
    out.append("use rabuka_engine::card::{Card, CardDatabase};")
    out.append("use rabuka_engine::card_loader::CardLoader;")
    out.append("use rabuka_engine::deck_builder::DeckBuilder;")
    out.append("use rabuka_engine::deck_parser::{DeckEntry, DeckList, DeckParser};")
    out.append("use rabuka_engine::game_setup;")
    out.append("use rabuka_engine::game_state::GameState;")
    out.append("use rabuka_engine::player::Player;")
    out.append("")
    out.append("use crate::dprintln;")
    out.append("use crate::ffi::*;")
    out.append("use crate::i18n::Lang;")
    out.append("use crate::lang::{current_lang, set_lang, tl, tl_fmt};")
    out.append("use crate::steps::{Overlay, SetupPhase, Step};")
    out.append("use crate::uds;")
    out.append("use crate::ui::card_atlas::CardAtlas;")
    out.append("use crate::ui::colors::*;")
    out.append(
        "use crate::ui::grid::{card_grid_input, render_card_detail, render_card_grid, GridAction};"
    )
    out.append("use crate::ui::hint::render_hint_bar;")
    out.append("use crate::util::{base64_decode, looks_like_b64, ticks_to_ms};")
    out.append("")
    out.append("/// On-device test suite — runs QA checks in limited 3DS memory.")
    out.append('/// Accessed via "Run Tests" menu. Each test returns a result line.')
    out.append(
        "fn run_on_device_tests(cards: Arc<Vec<Card>>, decks: Vec<DeckList>) -> Vec<String> {"
    )
    out.append("    let mut r: Vec<String> = Vec::new();")
    out.append("    let t0 = unsafe { _3ds_system_tick() };")
    out.append('    r.push(format!("CARDS: {}", cards.len()));')
    out.append("    let mut cards_vec = (*cards).clone();")
    out.append("    CardLoader::attach_abilities(&mut cards_vec);")
    out.append(
        "    let wa = cards_vec.iter().filter(|c| !c.abilities.is_empty()).count();"
    )
    out.append("    r.push(if wa > 0 {")
    out.append('        format!("ABILITIES: {} (OK)", wa)')
    out.append("    } else {")
    out.append('        "ABILITIES: NONE (FAIL!)".into()')
    out.append("    });")
    out.append('    r.push(format!("DECKS: {}", decks.len()));')
    out.append("    if let Some(c) = cards.first() {")
    out.append("        let nl = c.name.len();")
    out.append(
        '        r.push(format!("CARD[0]: {} ({}ch) OK", &c.name[..nl.min(20)], nl));'
    )
    out.append("    } else {")
    out.append('        r.push("CARD[0]: NONE (FAIL!)".into());')
    out.append("    }")
    out.append("    let he = cards.iter().any(|c| {")
    out.append("        let cn: &str = &c.card_no;")
    out.append('        cn.contains("LL-E-005")')
    out.append("    });")
    out.append("    r.push(if he {")
    out.append('        "ENERGY: found (OK)".into()')
    out.append("    } else {")
    out.append('        "ENERGY: missing (FAIL!)".into()')
    out.append("    });")
    out.append("    if decks.len() >= 2 {")
    out.append(
        "        match rabuka_engine::game_setup::test_ai_vs_ai(&cards_vec, &decks[0], &decks[1], 5) {"
    )
    out.append('            Ok(n) => r.push(format!("AI PLAY: {} actions (OK)", n)),')
    out.append('            Err(e) => r.push(format!("AI PLAY: FAIL {}", e)),')
    out.append("        }")
    out.append("    } else {")
    out.append('        r.push("AI PLAY: skip (need 2 decks)".into());')
    out.append("    }")
    out.append("    let ms = ticks_to_ms(unsafe { _3ds_system_tick() } - t0);")
    out.append('    r.push(format!("TIME: {}ms", ms));')
    out.append('    r.push("=== DONE ===".into());')
    out.append("    r")
    out.append("}")
    out.append("")

    seen = set()
    for arm_name, expr in arms:
        fn_name, params = SIGS[arm_name]
        seen.add(arm_name)
        sig = ", ".join(f"{n}: {t}" for n, t in params)
        out.append(
            f"fn {fn_name}(cards: &Arc<Vec<Card>>, decks: &Vec<DeckList>{', ' if params else ''}{sig}) -> Step {{"
        )
        out.append(reindent(expr))
        out.append("}")
        out.append("")

    missing = [a for a in SIGS if a not in seen]
    if missing:
        raise SystemExit(f"MISSING ARMS: {missing}")

    # Dispatcher
    out.append("/// Route one setup frame to the handler for the active SetupPhase.")
    out.append("pub fn setup_step(")
    out.append("    cards: &Arc<Vec<Card>>,")
    out.append("    decks: &Vec<DeckList>,")
    out.append("    phase: &SetupPhase,")
    out.append("    keys: u32,")
    out.append("    dirty: bool,")
    out.append(") -> Step {")
    out.append("    let n = decks.len();")
    out.append("    let was_dirty = dirty;")
    out.append("    let new_step = match phase.clone() {")
    for arm_name, expr in arms:
        fn_name, _ = SIGS[arm_name]
        pattern, call = DISPATCH[arm_name]
        out.append(f"        SetupPhase::{pattern} => {fn_name}({', '.join(call)}),")
    out.append("    };")
    out.append("    new_step")
    out.append("}")

    write(OUT, "\n".join(out) + "\n")
    print("WROTE", OUT, f"({len(arms)} arms)")


if __name__ == "__main__":
    main()
