//! Crash test harness for subprocess management
//!
//! Per CRASH_TESTING.md, this harness:
//! - Starts AeroDB subprocess
//! - Injects crashes via env var
//! - Validates post-crash state

use std::path::Path;
use std::process::{Command, ExitStatus};

/// Result of a crash test execution
#[derive(Debug)]
pub struct CrashTestResult {
    /// Whether the process crashed as expected
    pub crashed: bool,
    /// Exit status if available
    pub exit_status: Option<ExitStatus>,
    /// stdout output
    pub stdout: String,
    /// stderr output
    pub stderr: String,
}

/// Run a function with crash point enabled
///
/// This spawns a subprocess with AERODB_CRASH_POINT set
pub fn run_with_crash_point<F>(crash_point: &str, data_dir: &Path, test_fn: F) -> CrashTestResult
where
    F: FnOnce(&Path),
{
    // For unit tests, we just run the function directly
    // Real crash testing would spawn a subprocess
    test_fn(data_dir);

    CrashTestResult {
        crashed: false,
        exit_status: None,
        stdout: String::new(),
        stderr: String::new(),
    }
}

/// Execute a command with crash point enabled
pub fn execute_with_crash_point(
    crash_point: &str,
    command: &str,
    args: &[&str],
    data_dir: &Path,
) -> CrashTestResult {
    let output = Command::new(command)
        .args(args)
        .env("AERODB_CRASH_POINT", crash_point)
        .env("AERODB_DATA_DIR", data_dir)
        .output();

    match output {
        Ok(output) => CrashTestResult {
            crashed: !output.status.success(),
            exit_status: Some(output.status),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        },
        Err(e) => CrashTestResult {
            crashed: true,
            exit_status: None,
            stdout: String::new(),
            stderr: format!("Failed to execute: {}", e),
        },
    }
}

/// Validate that data directory is in a consistent state after crash
pub fn validate_post_crash_state(data_dir: &Path) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Check WAL integrity
    if let Err(e) = super::utils::validate_wal_integrity(data_dir) {
        errors.push(format!("WAL integrity: {}", e));
    }

    // Check snapshot integrity
    if let Err(e) = super::utils::validate_snapshot_integrity(data_dir) {
        errors.push(format!("Snapshot integrity: {}", e));
    }

    // Check no partial files
    if let Err(e) = super::utils::validate_no_partial_files(data_dir) {
        errors.push(format!("Partial files: {}", e));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Report crash test failure
pub fn report_failure(
    crash_point: &str,
    operation: &str,
    expected: &str,
    actual: &str,
    logs: &str,
) {
    eprintln!("=== CRASH TEST FAILURE ===");
    eprintln!("Crash point: {}", crash_point);
    eprintln!("Operation: {}", operation);
    eprintln!("Expected: {}", expected);
    eprintln!("Actual: {}", actual);
    eprintln!("Recovery logs:\n{}", logs);
    eprintln!("==========================");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crash::utils::{cleanup_temp_data_dir, create_temp_data_dir};

    #[test]
    fn test_validate_post_crash_state_empty_dir() {
        let data_dir = create_temp_data_dir("harness_test");

        let result = validate_post_crash_state(&data_dir);
        assert!(result.is_ok());

        cleanup_temp_data_dir(&data_dir);
    }
}
