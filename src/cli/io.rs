//! JSON I/O handling for CLI
//!
//! Per API_SPEC.md:
//! - Input: single JSON object via stdin
//! - Output: single JSON object via stdout
//! - UTF-8 only

use std::io::{self, BufRead, Write};

use serde_json::Value;

use super::errors::{CliError, CliResult};

/// Read a JSON request from stdin
pub fn read_request() -> CliResult<Value> {
    let stdin = io::stdin();
    let mut line = String::new();

    stdin.lock().read_line(&mut line)?;

    if line.trim().is_empty() {
        return Err(CliError::io_error("Empty input"));
    }

    let value: Value = serde_json::from_str(&line)?;
    Ok(value)
}

/// Read multiple JSON requests from stdin (for start command)
pub fn read_requests() -> impl Iterator<Item = CliResult<Value>> {
    let stdin = io::stdin();
    stdin.lock().lines().map(|line| {
        let line = line.map_err(CliError::from)?;
        if line.trim().is_empty() {
            return Err(CliError::io_error("Empty line"));
        }
        serde_json::from_str(&line).map_err(CliError::from)
    })
}

/// Write a success response to stdout
pub fn write_response(data: Value) -> CliResult<()> {
    let response = serde_json::json!({
        "status": "ok",
        "data": data
    });

    let mut stdout = io::stdout();
    serde_json::to_writer(&mut stdout, &response)?;
    writeln!(stdout)?;
    stdout.flush()?;

    Ok(())
}

/// Write an error response to stdout
pub fn write_error(code: &str, message: &str) -> CliResult<()> {
    let response = serde_json::json!({
        "status": "error",
        "code": code,
        "message": message
    });

    let mut stdout = io::stdout();
    serde_json::to_writer(&mut stdout, &response)?;
    writeln!(stdout)?;
    stdout.flush()?;

    Ok(())
}

/// Write a raw JSON string to stdout
pub fn write_json(json_str: &str) -> CliResult<()> {
    let mut stdout = io::stdout();
    writeln!(stdout, "{}", json_str)?;
    stdout.flush()?;

    Ok(())
}
