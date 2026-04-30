#!/usr/bin/env python3

import subprocess
import sys
import os

def test_baton_touch_energy():
    """Test that baton touch energy calculation works correctly"""
    
    # Change to engine directory
    os.chdir("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\engine")
    
    # Run a simple baton touch test
    try:
        result = subprocess.run([
            "cargo", "test", "test_q24_baton_touch_procedure", 
            "--", "--nocapture"
        ], capture_output=True, text=True, timeout=30)
        
        print("STDOUT:")
        print(result.stdout)
        
        if result.stderr:
            print("STDERR:")
            print(result.stderr)
            
        print(f"Return code: {result.returncode}")
        
        # Check if test passed
        if "test q24_baton_touch_procedure ... ok" in result.stdout:
            print("✅ Baton touch test PASSED")
            return True
        else:
            print("❌ Baton touch test FAILED")
            return False
            
    except subprocess.TimeoutExpired:
        print("❌ Test timed out")
        return False
    except Exception as e:
        print(f"❌ Error running test: {e}")
        return False

if __name__ == "__main__":
    print("Testing baton touch energy calculation fix...")
    success = test_baton_touch_energy()
    sys.exit(0 if success else 1)
