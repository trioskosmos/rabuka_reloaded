"""Deep-dive analysis of specific parser issues."""
import json, os

abilities_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'abilities.json')
with open(abilities_path, 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']

# 1. Verify keyword searches
print("=== KEYWORD SEARCH VERIFICATION ===")
for name, kw in [
    ('tsuika', '追加'),
    ('turn_end', 'ターン終了時まで'),
    ('next_turn', '次のターン'),
]:
    matches = [a for a in abilities if kw in a.get('full_text', '')]
    print(f"  {name} ({kw}): {len(matches)} matches")

# 2. Activation condition parsing detail
print("\n=== ACTIVATION CONDITION (のみ) DETAILS ===")
for a in abilities:
    ft = a.get('full_text', '')
    if 'のみ起動' in ft or 'のみ発動' in ft:
        eff = a.get('effect') or {}
        has_ac = 'activation_condition' in eff
        has_acp = 'activation_condition_parsed' in eff
        acp_type = eff.get('activation_condition_parsed', {}).get('type', 'N/A') if has_acp else 'N/A'
        ac = eff.get('activation_condition', '')
        ea = eff.get('action', 'N/A')
        print(f"  card_count={a['card_count']} effect_action={ea} has_ac={has_ac} has_acp={has_acp} acp_type={acp_type}")
        print(f"    ft: {ft[:120]}")
        if has_ac:
            print(f"    ac: {ac[:100]}")

# 3. Re-yell detail
print("\n=== RE-YELL DETAILS ===")
for a in abilities:
    ft = a.get('full_text', '')
    if 'もう一度エール' in ft or 'もう1度エール' in ft:
        eff = a.get('effect') or {}
        print(f"  card_count={a['card_count']}")
        print(f"  ft: {ft[:120]}")
        print(f"  top action: {eff.get('action')}")
        if eff.get('action') == 'sequential':
            for i, sub in enumerate(eff.get('actions', [])):
                sa = sub.get('action', '?')
                st = sub.get('text', '')[:80]
                print(f"    sub[{i}]: action={sa} text={st}")

# 4. Invalidate detail
print("\n=== INVALIDATE ABILITY DETAILS ===")
for a in abilities:
    ft = a.get('full_text', '')
    if '無効にする' in ft:
        eff = a.get('effect') or {}
        print(f"  card_count={a['card_count']}")
        print(f"  ft: {ft[:120]}")
        print(f"  top action: {eff.get('action')}")
        if eff.get('action') == 'sequential':
            for i, sub in enumerate(eff.get('actions', [])):
                sa = sub.get('action', '?')
                st = sub.get('text', '')[:80]
                print(f"    sub[{i}]: action={sa} text={st}")

# 5. Check if "ターン終了時まで" is in any abilities
print("\n=== TURN END SEARCH ===")
for a in abilities:
    if 'ターン終了時まで' in a.get('full_text', ''):
        print(f"  [{a['card_count']}] {a['full_text'][:120]}")
        break
else:
    print("  No abilities found with ターン終了時まで")

# 6. Check "追加" in full_text
print("\n=== 追加 SEARCH ===")
for a in abilities:
    if '追加' in a.get('full_text', ''):
        print(f"  [{a['card_count']}] {a['full_text'][:120]}")

# 7. Count abilities with null/absent effect
null_effects = [a for a in abilities if a.get('effect') is None or a.get('effect') == {}]
print(f"\n=== NULL/EMPTY EFFECTS: {len(null_effects)} ===")
for a in null_effects:
    print(f"  [{a['card_count']}] {a.get('full_text', '')[:100]}")

# 8. Check abilities where このカードを + 登場させる (self make-appear)
print("\n=== SELF MAKE-APPEAR (このカードを...登場させる) ===")
for a in abilities:
    ft = a.get('full_text', '')
    if 'このカードを' in ft and '登場させる' in ft:
        eff = a.get('effect') or {}
        has_st = eff.get('self_target', False)
        print(f"  card_count={a['card_count']} self_target={has_st} action={eff.get('action')}")
        print(f"    ft: {ft[:120]}")

print("\nDONE")
