import json
import re
import os


def strip_tags(text):
    if not text:
        return ""
    return re.sub(r"\{\{[^|]*\|([^}]*)\}\}", r"\1", text)


def parse_rules(file_path):
    """
    Parses rules.txt, ignoring the Table of Contents and extracting
    rules from the '総合ルール本文' section.
    """
    rules = []
    current_rule_id = None
    current_text = []

    with open(file_path, "r", encoding="utf-8") as f:
        lines = f.readlines()

    # Find where the actual rules start
    start_index = 0
    for i, line in enumerate(lines):
        if "総合ルール本文" in line:
            start_index = i + 1
            break

    for line in lines[start_index:]:
        line = line.strip()
        if not line:
            continue

        # Skip page markers
        if line.startswith("=== PAGE"):
            continue

        # Match rule numbers like 1.1, 1.1.1, 6.2.1.4.
        # We want to ensure it's at the start of the line.
        match = re.match(r"^(\d+(?:\.\d+)*)\.?\s*(.*)", line)
        if match:
            if current_rule_id:
                rules.append({"id": current_rule_id, "text": " ".join(current_text)})

            current_rule_id = match.group(1)
            current_text = [match.group(2)]
        else:
            if current_rule_id:
                current_text.append(line)

    if current_rule_id:
        rules.append({"id": current_rule_id, "text": " ".join(current_text)})

    return rules


def get_comprehensive_mappings():
    """
    Comprehensive mapping of keywords to rule IDs.
    """
    return {
        "6.2.1.4": [r"じゃんけん", r"先攻", r"後攻", r"どちらのプレイヤーが先攻"],
        "4.5.5": [r"メンバーの下", r"重ねて置く", r"下に置く"],
        "9.6.2.3.2": [r"バトンタッチ"],
        "11.10": [r"ポジションチェンジ"],
        "11.11": [r"フォーメーションチェンジ"],
        "9.5.1": [r"チェックタイミング"],
        "9.5.2": [r"プレイタイミング"],
        "4.10": [r"成功ライブカード置き場"],
        "10.2": [r"リフレッシュ"],
        "11.5": [r"ライブ開始時"],
        "11.6": [r"ライブ成功時"],
        "8.2": [r"ライブカードセットフェイズ"],
        "8.3": [r"パフォーマンスフェイズ"],
        "8.4": [r"ライブ勝敗判定フェイズ"],
        "4.3.2": [
            r"ウェイト状態",
            r"アクティブ状態",
            r"ウェイトにする",
            r"アクティブにする",
        ],
        "4.12": [r"控え室"],
        "4.11": [r"手札"],
        "4.8": [r"デッキ"],
        "4.7": [r"エネルギー置き場", r"エネルギーを支払"],
        "8.1": [r"ライブフェイズ"],
        "2.10": [r"スコア"],
        "2.9": [r"ハート"],
        "2.8": [r"ブレード"],
        "4.5.2.1": [r"センターエリア"],
        "11.4": [r"登場"],
        "9.1.1.3": [r"常時"],
        "9.1.1.2": [r"自動"],
        "9.1.1.1": [r"起動"],
    }


def map_qa_to_rules(qa_text, rule_mappings):
    matched_rules = set()

    # 1. Explicit references in parentheses: (9.5.1)
    refs = re.findall(r"\((\d+(?:\.\d+)*)\)", qa_text)
    for ref in refs:
        matched_rules.add(ref)

    # 2. Keyword mapping
    for rule_id, keywords in rule_mappings.items():
        for kw in keywords:
            if re.search(kw, qa_text):
                matched_rules.add(rule_id)

    return matched_rules


def main():
    rules_path = "engine/rules/rules.txt"
    qa_path = "cards/qa_data.json"
    out_path = "docs/rules_expanded.txt"

    if not os.path.exists("docs"):
        os.makedirs("docs")

    all_rules = parse_rules(rules_path)
    rule_mappings = get_comprehensive_mappings()

    with open(qa_path, "r", encoding="utf-8") as f:
        qa_data = json.load(f)

    # Map each QA to rule IDs
    rule_to_qa = {}  # Rule_ID -> list of QA_IDs
    qa_mapped = set()

    for entry in qa_data:
        qa_id = entry.get("id", "Unknown")
        q = strip_tags(entry.get("question", ""))
        a = strip_tags(entry.get("answer", ""))
        text = f"{q} {a}"

        rules = map_qa_to_rules(text, rule_mappings)
        for r in rules:
            if r not in rule_to_qa:
                rule_to_qa[r] = []
            rule_to_qa[r].append(qa_id)
            qa_mapped.add(qa_id)

    # Final output construction
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(
            "================================================================================\n"
        )
        f.write(
            "ラブライブ！シリーズ オフィシャルカードゲーム 総合ルール & QA 統合ドキュメント\n"
        )
        f.write(
            "================================================================================\n\n"
        )

        for rule in all_rules:
            rid = rule["id"]
            rtext = rule["text"]

            f.write(f"--- Rule {rid} ---\n")
            f.write(f"{rtext}\n")

            if rid in rule_to_qa:
                f.write("\n  [Relevant QA]\n")
                for qa_id in rule_to_qa[rid]:
                    qa_entry = next(
                        (item for item in qa_data if item.get("id") == qa_id), None
                    )
                    if qa_entry:
                        q = strip_tags(qa_entry.get("question", ""))
                        a = strip_tags(qa_entry.get("answer", ""))
                        f.write(f"    ({qa_id}) Q: {q}\n")
                        f.write(f"            A: {a}\n\n")
            else:
                f.write("\n  (No related QA entries found)\n")

            f.write("\n" + "-" * 40 + "\n\n")

        # Handle unmapped QAs
        unmapped = [item for item in qa_data if item.get("id") not in qa_mapped]
        if unmapped:
            f.write(
                "\n\n================================================================================\n"
            )
            f.write("UNMAPPED QA ENTRIES (Could not be linked to a specific rule)\n")
            f.write(
                "================================================================================\n\n"
            )
            for entry in unmapped:
                qa_id = entry.get("id", "Unknown")
                q = strip_tags(entry.get("question", ""))
                a = strip_tags(entry.get("answer", ""))
                f.write(f"[{qa_id}]\nQ: {q}\nA: {a}\n\n")


if __name__ == "__main__":
    main()
