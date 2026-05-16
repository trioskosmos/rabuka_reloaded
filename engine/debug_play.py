import urllib.request
import json
import time

URL_BASE = "http://127.0.0.1:8080/api"


def request(method, path, data=None):
    url = f"{URL_BASE}{path}"
    req = urllib.request.Request(url, method=method)
    req.add_header("Content-Type", "application/json")
    if data:
        req.data = json.dumps(data).encode("utf-8")
    try:
        with urllib.request.urlopen(req) as response:
            return json.loads(response.read().decode("utf-8"))
    except Exception as e:
        return {"error": str(e)}


def main():
    print("--- 1. Init ---")
    resp = request("POST", "/init", {})
    print(json.dumps(resp, indent=2))

    print("\n--- 2. Play 高海千歌 (1952) to Center ---")
    resp = request(
        "POST",
        "/execute-action",
        {
            "action_index": 0,  # Dummy, but needed
            "action_type": "play_member_to_stage",
            "card_id": 1952,
            "stage_area": "center",
        },
    )
    print(json.dumps(resp, indent=2))

    print("\n--- 3. Get State ---")
    state = request("GET", "/game-state")
    print(json.dumps(state, indent=2))

    print("\n--- 4. Play 桜内梨子 (1957) to Left ---")
    resp = request(
        "POST",
        "/execute-action",
        {
            "action_index": 0,
            "action_type": "play_member_to_stage",
            "card_id": 1957,
            "stage_area": "left",
        },
    )
    print(json.dumps(resp, indent=2))

    print("\n--- 5. Get State ---")
    state = request("GET", "/game-state")
    print(json.dumps(state, indent=2))

    print("\n--- 6. Use ability on 高海千歌 (1952) ---")
    resp = request(
        "POST",
        "/execute-action",
        {
            "action_index": 0,
            "action_type": "use_ability",
            "card_id": 1952,
            "stage_area": "center",
        },
    )
    print(json.dumps(resp, indent=2))


if __name__ == "__main__":
    main()
