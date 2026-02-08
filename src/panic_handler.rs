//! Panic Handler
//!
//! HARDENING: Controlled panic handling for production safety.
//!
//! - Captures panic info
//! - Writes to crash log before terminating
//! - Never silently swallows panics

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic::{self, PanicInfo};
use std::path::PathBuf;

/// Initialize the production panic handler
///
/// HARDENING: All panics are logged before termination.
/// This ensures operators can diagnose crashes.
pub fn init_panic_handler(data_dir: Option<PathBuf>) {
    let crash_log_path = data_dir.map(|d| d.join("crash.log"));
    
    panic::set_hook(Box::new(move |info| {
        handle_panic(info, crash_log_path.as_ref());
    }));
}

fn handle_panic(info: &PanicInfo<'_>, crash_log_path: Option<&PathBuf>) {
    // Build panic message
    let message = format_panic_info(info);
    
    // Always print to stderr
    eprintln!("\n{}", "=".repeat(80));
    eprintln!("AERODB FATAL ERROR - PANIC");
    eprintln!("{}", "=".repeat(80));
    eprintln!("{}", message);
    eprintln!("{}", "=".repeat(80));
    
    // Write to crash log if path available
    if let Some(path) = crash_log_path {
        if let Err(e) = write_crash_log(path, &message) {
            eprintln!("Failed to write crash log: {}", e);
        } else {
            eprintln!("Crash log written to: {}", path.display());
        }
    }
    
    eprintln!("\nThis is a bug. Please report it with the above information.");
}

fn format_panic_info(info: &PanicInfo<'_>) -> String {
    let mut msg = String::new();
    
    // Timestamp
    let now = chrono::Utc::now().to_rfc3339();
    msg.push_str(&format!("Timestamp: {}\n", now));
    
    // Location
    if let Some(location) = info.location() {
        msg.push_str(&format!(
            "Location: {}:{}:{}\n",
            location.file(),
            location.line(),
            location.column()
        ));
    }
    
    // Message
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        msg.push_str(&format!("Message: {}\n", s));
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        msg.push_str(&format!("Message: {}\n", s));
    } else {
        msg.push_str("Message: <unknown panic payload>\n");
    }
    
    // Backtrace (if available)
    let backtrace = std::backtrace::Backtrace::capture();
    match backtrace.status() {
        std::backtrace::BacktraceStatus::Captured => {
            msg.push_str(&format!("\nBacktrace:\n{}", backtrace));
        }
        _ => {
            msg.push_str("\nBacktrace: <not captured, set RUST_BACKTRACE=1>\n");
        }
    }
    
    msg
}

fn write_crash_log(path: &PathBuf, message: &str) -> std::io::Result<()> {
    // Ensure parent dir exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Append to crash log
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    
    writeln!(file, "\n{}", "=".repeat(80))?;
    writeln!(file, "{}", message)?;
    file.sync_all()?;
    
    Ok(())
}

/// Wrapper for unwrap that provides better panic messages
///
/// HARDENING: Use this instead of bare unwrap() for better debugging.
#[macro_export]
macro_rules! safe_unwrap {
    ($expr:expr, $msg:literal) => {
        match $expr {
            Some(val) => val,
            None => panic!("safe_unwrap failed at {}:{}: {}", file!(), line!(), $msg),
        }
    };
    ($expr:expr) => {
        match $expr {
            Some(val) => val,
            None => panic!("safe_unwrap failed at {}:{}", file!(), line!()),
        }
    };
}

/// Wrapper for Result unwrap that provides better panic messages
#[macro_export]
macro_rules! safe_unwrap_result {
    ($expr:expr, $msg:literal) => {
        match $expr {
            Ok(val) => val,
            Err(e) => panic!("safe_unwrap_result failed at {}:{}: {} (error: {:?})", file!(), line!(), $msg, e),
        }
    };
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => panic!("safe_unwrap_result failed at {}:{} (error: {:?})", file!(), line!(), e),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_format_panic_info_captures_location() {
        // We can't easily test PanicInfo directly, but we can test the format function
        // by checking that the formatted string contains expected elements
    }
    
    #[test]
    fn test_write_crash_log() {
        let temp = TempDir::new().unwrap();
        let crash_log = temp.path().join("crash.log");
        
        write_crash_log(&crash_log, "Test crash message").unwrap();
        
        let content = fs::read_to_string(&crash_log).unwrap();
        assert!(content.contains("Test crash message"));
    }
    
    #[test]
    fn test_crash_log_appends() {
        let temp = TempDir::new().unwrap();
        let crash_log = temp.path().join("crash.log");
        
        write_crash_log(&crash_log, "First crash").unwrap();
        write_crash_log(&crash_log, "Second crash").unwrap();
        
        let content = fs::read_to_string(&crash_log).unwrap();
        assert!(content.contains("First crash"));
        assert!(content.contains("Second crash"));
    }
    
    #[test]
    fn test_safe_unwrap_some() {
        let val: Option<i32> = Some(42);
        assert_eq!(safe_unwrap!(val), 42);
    }
    
    #[test]
    fn test_safe_unwrap_result_ok() {
        let val: Result<i32, &str> = Ok(42);
        assert_eq!(safe_unwrap_result!(val), 42);
    }
}
