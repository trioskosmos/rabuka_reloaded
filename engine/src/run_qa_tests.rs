// Binary to run QA data tests independently
// Delegates all logic to the shared qa_test_suite module in the library crate.

fn main() {
    rabuka_engine::qa_test_suite::run_all();
}
