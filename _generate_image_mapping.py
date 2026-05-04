import json, os, re
from pathlib import Path

CARDS_JSON = Path("cards/cards.json")
WEBP_DIR = Path("web_ui/img/cards_webp")
OUTPUT = Path("web_ui/js/card_image_mapping.json")

def load_cards():
    with open(CARDS_JSON, encoding="utf-8") as f:
        raw = json.load(f)
    if isinstance(raw, dict):
        return list(raw.keys())
    elif isinstance(raw, list):
        return [c.get("card_no", "") for c in raw]
    return []

def load_webp_files():
    files = set()
    for f in os.listdir(WEBP_DIR):
        if f.endswith(".webp"):
            files.add(f.replace(".webp", ""))
    return files

def normalize(s):
    return re.sub(r'[^a-zA-Z0-9]', '', s).lower()

def build_mapping(card_nos, webp_names):
    mapping = {}

    # Index webp files by normalized form (strip all non-alnum)
    webp_index = {}
    for w in webp_names:
        norm = normalize(w)
        webp_index.setdefault(norm, []).append(w)

    for cn in card_nos:
        if not cn:
            continue

        # Build candidate filenames from this card_no
        normalized_cn = normalize(cn)

        # 1) Exact match
        if cn in webp_names:
            mapping[cn] = f"img/cards_webp/{cn}.webp"
            continue

        # 2) Normalized match (ignoring fullwidth + → 2, dashes, etc)
        if normalized_cn in webp_index:
            mapping[cn] = f"img/cards_webp/{webp_index[normalized_cn][0]}.webp"
            continue

        # 3) Fullwidth plus → 2 suffix (e.g. PR-010-PR＋ → PR-010-PR2)
        alt = cn.replace("\uff0b", "2")
        if alt in webp_names:
            mapping[cn] = f"img/cards_webp/{alt}.webp"
            continue
        if normalize(alt) in webp_index:
            mapping[cn] = f"img/cards_webp/{webp_index[normalize(alt)][0]}.webp"
            continue

        # 4) Rarity-as-segment insertion: PL!-bp3-001-P → PL!-bp3-P-001-P.webp
        parts = cn.split("-")
        if len(parts) >= 4:
            for segment_pos in range(1, len(parts) - 1):
                for rarity in ["P", "R", "N", "L", "SEC", "SECE", "SECL", "PE", "PR", "RM", "AR", "RE", "LLE", "P2", "R2", "PE2", "SD"]:
                    candidate = "-".join(parts[:segment_pos]) + f"-{rarity}-" + "-".join(parts[segment_pos:])
                    if candidate in webp_names:
                        mapping[cn] = f"img/cards_webp/{candidate}.webp"
                        break
                if cn in mapping:
                    break

        # 5) Rarity suffix 2: LL-bp1-001-R → LL-bp1-001-R2.webp
        if cn not in mapping:
            for w in webp_names:
                wn = normalize(w)
                if wn.startswith(normalized_cn) and len(wn) == len(normalized_cn) + 1:
                    mapping[cn] = f"img/cards_webp/{w}.webp"
                    break

        # 6) Fallback: any webp with same normalized core
        if cn not in mapping:
            for w in webp_names:
                wn = normalize(w)
                if wn == normalized_cn:
                    mapping[cn] = f"img/cards_webp/{w}.webp"
                    break

    return mapping

def main():
    card_nos = load_cards()
    webp_names = load_webp_files()
    print(f"Cards: {len(card_nos)}, WebP files: {len(webp_names)}")

    mapping = build_mapping(card_nos, webp_names)
    print(f"Mapped: {len(mapping)}")

    # Merge with existing mapping
    existing = {}
    if OUTPUT.exists():
        with open(OUTPUT, encoding="utf-8") as f:
            existing = json.load(f)
        print(f"Existing mapping: {len(existing)} entries")

    existing.update(mapping)
    print(f"Final mapping: {len(existing)} entries")

    unmapped = [cn for cn in card_nos if cn and cn not in existing and not cn.startswith("PYHN")]
    if unmapped:
        print(f"\nStill unmapped ({len(unmapped)}):")
        for cn in sorted(unmapped)[:20]:
            print(f"  {repr(cn)}")

    with open(OUTPUT, "w", encoding="utf-8") as f:
        json.dump(existing, f, ensure_ascii=False, indent=2)
    print(f"\nWritten to {OUTPUT}")

if __name__ == "__main__":
    main()
