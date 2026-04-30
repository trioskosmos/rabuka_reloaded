import requests
import json

BASE_URL = "http://localhost:8080"

def debug_apis():
    """Debug available APIs to find card data"""
    
    # Try different endpoints
    endpoints = [
        '/api/get_card_registry',
        '/api/cards',
        '/api/card_database',
        '/api/game-state',
        '/api/actions'
    ]
    
    for endpoint in endpoints:
        print(f"\n=== Testing {endpoint} ===")
        try:
            response = requests.get(f"{BASE_URL}{endpoint}")
            print(f"Status: {response.status_code}")
            
            if response.status_code == 200:
                data = response.json()
                print(f"Type: {type(data)}")
                
                if isinstance(data, dict):
                    print(f"Keys: {list(data.keys())[:10]}")  # First 10 keys
                    if 'cards' in data:
                        cards = data['cards']
                        print(f"Cards count: {len(cards)}")
                        if cards:
                            sample_card = cards[0]
                            print(f"Sample card keys: {list(sample_card.keys())}")
                            if 'abilities' in sample_card:
                                print(f"Sample card abilities: {sample_card['abilities']}")
                            else:
                                print("No abilities in sample card")
                elif isinstance(data, list):
                    print(f"List length: {len(data)}")
                    if data:
                        sample = data[0]
                        print(f"Sample keys: {list(sample.keys())}")
                        if 'abilities' in sample:
                            print(f"Sample abilities: {sample['abilities']}")
                        else:
                            print("No abilities in sample")
                
                # Show first 500 characters of response
                response_text = response.text[:500]
                print(f"Response preview: {response_text}")
            else:
                print(f"Error: {response.text}")
                
        except Exception as e:
            print(f"Exception: {e}")

if __name__ == "__main__":
    debug_apis()
