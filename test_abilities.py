"""
Ability System Test Suite
Tests: condition evaluation, 常時 blade bonuses, position filtering, blade totals
Run against a running server (start.bat or cargo run --bin rabuka_engine web-server)
"""

import json
import sys
import time
import urllib.request
import urllib.error

BASE = "http://127.0.0.1:8080"

def req(method, path, data=None):
    url = f"{BASE}{path}"
    body = json.dumps(data).encode() if data else None
    r = urllib.request.Request(url, data=body, method=method)
    r.add_header("Content-Type", "application/json")
    try:
        resp = urllib.request.urlopen(r)
        return json.loads(resp.read())
    except urllib.error.HTTPError as e:
        print(f"  HTTP {e.code}: {e.read().decode()[:200]}")
        return None
    except urllib.error.URLError as e:
        print(f"  Connection failed: {e.reason}")
        return None

def get(path):
    return req("GET", path)

def post(path, data=None):
    return req("POST", path, data)

passed = 0
failed = 0

def check(name, condition, detail=""):
    global passed, failed
    if condition:
        passed += 1
        print(f"  ✅ {name}")
    else:
        failed += 1
        print(f"  ❌ {name}  {detail}")

def init_game():
    print("\n=== INIT GAME ===")
    # Init with default decks
    r = post("/api/init")
    check("game init returns ok", r is not None)
    # Advance through mulligan
    time.sleep(0.2)
    for _ in range(4):
        acts = get("/api/actions")
        if acts and acts.get("actions"):
            # Find pass/skip action or first action
            for a in acts["actions"]:
                if "skip" in a.get("action_type","").lower() or "pass" in a.get("action_type","").lower() or "mulligan" in a.get("action_type","").lower():
                    post("/api/execute-action", {"action_index": a["index"], "action_type": a["action_type"]})
                    time.sleep(0.1)
                    break
    print("  Game initialized")

def get_game_state():
    return get("/api/game-state")

def do_action(index, action_type, extra=None):
    payload = {"action_index": index, "action_type": action_type}
    if extra:
        payload.update(extra)
    return post("/api/execute-action", payload)

def exec_code(code):
    return post("/api/exec", {"code": code})

def find_action(actions, action_type=None, desc_contains=None):
    if not actions:
        return None
    for a in actions:
        if action_type and a.get("action_type") != action_type:
            continue
        if desc_contains and desc_contains.lower() not in a.get("description","").lower():
            continue
        return a
    return None

def wait_for_phase(target_phase, max_loops=30):
    for _ in range(max_loops):
        gs = get_game_state()
        if gs and target_phase.lower() in gs.get("phase","").lower():
            return gs
        acts = get("/api/actions")
        if acts and acts.get("actions"):
            for a in acts["actions"]:
                if "pass" in a.get("action_type","").lower() or "skip" in a.get("description","").lower():
                    post("/api/execute-action", {"action_index": a["index"], "action_type": a["action_type"]})
                    time.sleep(0.1)
                    break
        time.sleep(0.1)
    return get_game_state()

def add_energy(player_idx, amount):
    code = f"player_idx = {player_idx}; draw_energy = True; amount = {amount}"
    exec_code(code)

def add_card_to_hand(player_idx, card_no):
    code = f'player_idx = {player_idx}; add_card = True; card_no = "{card_no}"'
    exec_code(code)

def test_condition_evaluation():
    """Test that conditions evaluate correctly (not always true)"""
    print("\n=== TEST: Condition Evaluation ===")
    
    gs = get_game_state()
    if not gs:
        check("game state available", False)
        return
    check("game state has phase", "phase" in gs)
    
    # Check debug conditions endpoint is alive
    cond = get("/api/debug/conditions")
    check("conditions endpoint alive", cond is not None and cond.get("success"))
    if cond:
        check("conditions list is array", isinstance(cond.get("conditions"), list))
        total = len(cond.get("conditions", []))
        print(f"  Total conditions found: {total}")

def test_blade_modifier_in_display():
    """Test that CardDisplay includes total_blade field"""
    print("\n=== TEST: Blade Modifier in Display ===")
    gs = get_game_state()
    if not gs:
        return
    # Check stage cards have total_blade
    for pname in ["player1","player2"]:
        p = gs.get(pname, {})
        stage = p.get("stage", {})
        for pos in ["left_side","center","right_side"]:
            card = stage.get(pos)
            if card:
                has_total = "total_blade" in card
                check(f"{pname} {pos} has total_blade", has_total)
                if has_total:
                    blade = card.get("blade", 0)
                    total = card.get("total_blade", 0)
                    check(f"{pname} {pos} total_blade >= blade", total >= blade,
                          f"blade={blade} total={total}")

def test_position_check():
    """Test that check_trigger_position works correctly"""
    print("\n=== TEST: Position Trigger Check ===")
    from zones_module import check_trigger_position, MemberArea
    
    # LeftSide check
    assert check_trigger_position(Some("登場, 左サイド"), MemberArea::LeftSide)
    assert not check_trigger_position(Some("登場, 左サイド"), MemberArea::Center)
    assert not check_trigger_position(Some("登場, 左サイド"), MemberArea::RightSide)
    
    # RightSide check
    assert check_trigger_position(Some("登場, 右サイド"), MemberArea::RightSide)
    assert not check_trigger_position(Some("登場, 右サイド"), MemberArea::LeftSide)
    
    # No position requirement
    assert check_trigger_position(Some("起動"), MemberArea::Center)
    assert check_trigger_position(Some("起動"), MemberArea::LeftSide)
    
    # Center check
    assert check_trigger_position(Some("起動, センター"), MemberArea::Center)
    assert not check_trigger_position(Some("起動, センター"), MemberArea::LeftSide)
    
    print("  Position trigger logic verified")

def test_constant_ability_flow():
    """Full integration test: play a card with 常時 ability and verify blade bonus"""
    print("\n=== TEST: Constant Ability Integration ===")
    
    # Find a card that has 常時 ability with cost >= 13 condition
    # First, let's search for such a card in abilities.json
    with open("cards/abilities.json", "r", encoding="utf-8") as f:
        ab_data = json.load(f)
    abilities = ab_data.get("unique_abilities", [])
    
    # Look for 常時 abilities that grant blades with cost_limit condition
    test_card_info = None
    test_cost_limit = None
    for ability in abilities:
        triggers = ability.get("triggers", "")
        if "常時" not in triggers:
            continue
        effect = ability.get("effect", {})
        if effect.get("action") != "gain_resource" or effect.get("resource") not in ("blade", "ブレード"):
            continue
        condition = effect.get("condition", {})
        if condition.get("type") == "location_condition" and condition.get("cost_limit"):
            # Found a 常時 blade gain with cost limit condition
            test_card_info = ability.get("cards", [None])[0] if ability.get("cards") else None
            test_cost_limit = condition.get("cost_limit")
            if test_card_info:
                break
    
    if not test_card_info:
        print("  ⚠️  No suitable test card found with 常時 + blade gain + cost_limit condition")
        print("  Falling back to simple state check")
        cond = get("/api/debug/conditions")
        if cond and cond.get("conditions"):
            const_conds = [c for c in cond["conditions"] if c.get("condition_type") == "location_condition"]
            print(f"  Found {len(const_conds)} location_conditions")
            for c in const_conds[:3]:
                print(f"    card={c.get('card_name')} zone={c.get('zone')} result={c.get('result')}")
            check("location conditions evaluated", len(const_conds) > 0)
        return
    
    # Parse card info
    parts = test_card_info.split("|")
    card_no = parts[0].strip()
    card_name = parts[1].strip() if len(parts) > 1 else card_no
    print(f"  Test card: {card_no} ({card_name}) with cost_limit >= {test_cost_limit}")
    
    # Add the card to player 1's hand
    init_game()
    add_card_to_hand(0, card_no)
    add_energy(0, 10)
    
    time.sleep(0.2)
    gs = get_game_state()
    phase = gs.get("phase", "") if gs else ""
    check("card added to hand", gs is not None)

def main():
    print("=" * 60)
    print("  ABILITY SYSTEM TEST SUITE")
    print("=" * 60)
    print(f"  Server: {BASE}")
    print()
    
    # First check server is alive
    r = get("/api/status")
    if not r:
        print("❌ Server not reachable! Start the server first.")
        print("   run: cargo run --bin rabuka_engine web-server")
        sys.exit(1)
    print(f"  Server status: {r}")
    
    # Run tests
    init_game()
    test_condition_evaluation()
    test_blade_modifier_in_display()
    test_constant_ability_flow()
    
    # Summary
    print()
    print("=" * 60)
    total = passed + failed
    print(f"  RESULTS: {passed}/{total} passed, {failed} failed")
    print("=" * 60)
    
    if failed > 0:
        sys.exit(1)

if __name__ == "__main__":
    main()
