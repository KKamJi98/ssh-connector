//! A stand-in for `ssh` used by the tests and the golden fixture generator.
//!
//! It prints its arguments to stderr and exits with `FAKE_SSH_EXIT` (default
//! 0). `FAKE_SSH_SLEEP_MS` makes it wait first, for the interrupt tests.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    eprintln!("fake-ssh argv={args:?}");
    if let Some(ms) = std::env::var("FAKE_SSH_SLEEP_MS")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
    let code = std::env::var("FAKE_SSH_EXIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    std::process::exit(code);
}
