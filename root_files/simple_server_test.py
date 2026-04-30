
import requests
import json
import time

BASE_URL = "http://localhost:8080"

def test_server_basic():
    """Test basic server functionality"""
    print("Testing basic server functionality...")
    
    # Test server status
    try:
        response = requests.get(f"{BASE_URL}/api/status", timeout=5)
        if response.status_code == 200:
            print("Server status endpoint working")
            return True
        else:
            print(f"Server status returned: {response.status_code}")
    except Exception as e:
        print(f"Server status failed: {e}")
    
    return False

def test_game_state():
    """Test game state endpoint"""
    print("Testing game state endpoint...")
    
    try:
        response = requests.get(f"{BASE_URL}/api/game-state", timeout=5)
        if response.status_code == 200:
            state = response.json()
            print(f"Game state retrieved - Turn: {state.get('turn', 'N/A')}, Phase: {state.get('phase', 'N/A')}")
            return True
        else:
            print(f"Game state returned: {response.status_code}")
    except Exception as e:
        print(f"Game state failed: {e}")
    
    return False

def test_actions():
    """Test actions endpoint"""
    print("Testing actions endpoint...")
    
    try:
        response = requests.get(f"{BASE_URL}/api/actions", timeout=5)
        if response.status_code == 200:
            actions = response.json()
            print(f"Actions retrieved: {len(actions)} actions available")
            return True
        else:
            print(f"Actions returned: {response.status_code}")
    except Exception as e:
        print(f"Actions failed: {e}")
    
    return False

if __name__ == "__main__":
    print("=== SIMPLE SERVER TEST ===")
    
    # Wait a moment for server to start
    time.sleep(2)
    
    # Run tests
    status_ok = test_server_basic()
    state_ok = test_game_state()
    actions_ok = test_actions()
    
    print(f"\n=== TEST RESULTS ===")
    print(f"Server Status: {'PASS' if status_ok else 'FAIL'}")
    print(f"Game State: {'PASS' if state_ok else 'FAIL'}")
    print(f"Actions: {'PASS' if actions_ok else 'FAIL'}")
    
    if status_ok and state_ok and actions_ok:
        print("All tests passed - server is working!")
    else:
        print("Some tests failed - server needs attention")
