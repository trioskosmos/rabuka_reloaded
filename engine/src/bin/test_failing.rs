//! Test runner for failing tests without affecting the main server
//! 
//! This binary runs specific failing tests to help debug issues
//! without requiring a full cargo test run that might interfere
//! with a running game server.

use std::process;

fn main() {
    println!("Running failing Honoka tests...");
    
    // Run specific failing tests
    let test_results = vec![
        run_test("honoka_q166_member_found_after_4_fillers"),
        run_test("honoka_q166_target_first_card"),
        run_test("honoka_q166_target_last_card"),
        run_test("honoka_q166_two_matches_only_one_added"),
    ];
    
    let passed = test_results.iter().filter(|&&(_, success)| success).count();
    let total = test_results.len();
    
    println!("\n=== Test Results ===");
    for (test_name, success) in test_results {
        let status = if success { "PASS" } else { "FAIL" };
        println!("{}: {}", test_name, status);
    }
    
    println!("\nSummary: {}/{} tests passed", passed, total);
    
    if passed == total {
        println!("All tests passed! 🎉");
    } else {
        println!("Some tests failed. Check output above for details.");
        process::exit(1);
    }
}

fn run_test(test_name: &str) -> (&str, bool) {
    println!("Running {}...", test_name);
    
    let output = process::Command::new("cargo")
        .args(&["test", test_name, "--", "--nocapture"])
        .output();
    
    match output {
        Ok(result) => {
            let success = result.status.success();
            if !success {
                println!("FAILED: {}", String::from_utf8_lossy(&result.stderr));
            }
            (test_name, success)
        }
        Err(e) => {
            println!("Error running test {}: {}", test_name, e);
            (test_name, false)
        }
    }
}
