import json
import collections


def simplify(obj):
    if obj is None:
        return None
    if isinstance(obj, list):
        return [simplify(item) for item in obj]
    if isinstance(obj, dict):
        new_obj = {}
        # Fields to ignore (filters)
        ignore_fields = {
            "text",
            "card_type",
            "group_names",
            "cost_limit",
            "cost_limit_operator",
            "position",
            "parenthetical",
        }
        for k, v in obj.items():
            if k in ignore_fields:
                continue
            new_obj[k] = simplify(v)
        return new_obj
    return obj


def get_canonical_key(ability):
    # We only care about triggers, cost, and effect
    core = {
        "triggers": ability.get("triggers"),
        "use_limit": ability.get("use_limit"),
        "cost": simplify(ability.get("cost")),
        "effect": simplify(ability.get("effect")),
    }
    # Convert to string for hashing/counting
    return json.dumps(core, sort_keys=True)


def main():
    with open("cards/abilities.json", "r", encoding="utf-8") as f:
        data = json.load(f)

    abilities = data.get("unique_abilities", [])
    counts = collections.Counter()

    # Map canonical key back to one of the original full_texts for readability
    key_to_example = {}

    for ab in abilities:
        if ab.get("is_null"):
            continue
        key = get_canonical_key(ab)
        counts[key] += 1
        if key not in key_to_example:
            key_to_example[key] = ab.get("full_text")

    results = []
    for key, count in counts.most_common():
        results.append(
            {
                "example_text": key_to_example[key],
                "canonical_form": json.loads(key),
                "count": count,
            }
        )

    output = {"total_truly_unique": len(results), "abilities": results}

    with open("cards/unique_abilities_filtered.json", "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"Found {len(results)} truly unique abilities.")


if __name__ == "__main__":
    main()
