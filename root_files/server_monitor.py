
import requests
import time
import subprocess
import sys

def monitor_server():
    """Monitor server health"""
    base_url = "http://localhost:8080"
    
    for i in range(30):  # Monitor for 30 seconds
        try:
            response = requests.get(f"{base_url}/api/status", timeout=2)
            if response.status_code == 200:
                print(f"Server is healthy (check {i+1}/30)")
                return True
        except:
            print(f"Server not responding (check {i+1}/30)")
        
        time.sleep(1)
    
    print("Server failed to respond within 30 seconds")
    return False

if __name__ == "__main__":
    monitor_server()
