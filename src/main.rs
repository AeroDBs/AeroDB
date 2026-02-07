//! AeroDB CLI entry point
//!
//! This is a minimal entrypoint that:
//! 1. Parses CLI arguments (via cli::run)
//! 2. Dispatches to CLI commands (via cli::run)
//! 3. Prints errors to stderr
//! 4. Exits with non-zero on failure
//!
//! Per BOOT.md, main.rs must NOT:
//! - Load configuration
//! - Initialize subsystems
//! - Perform recovery
//! - Open files or spawn threads
//!
//! All logic is delegated to the CLI module.

use aerodb::cli;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
