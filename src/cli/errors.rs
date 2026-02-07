//! CLI-specific error types
//!
//! All CLI errors are FATAL per ERRORS.md

use std::fmt;
use std::io;

/// CLI error codes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliErrorCode {
    /// Configuration file error
    ConfigError,
    /// I/O error (stdin/stdout)
    IoError,
    /// Already initialized
    AlreadyInitialized,
    /// Not initialized
    NotInitialized,
    /// Boot failed
    BootFailed,
}

impl CliErrorCode {
    /// Get the error code string
    pub fn code(&self) -> &'static str {
        match self {
            Self::ConfigError => "AERO_CLI_CONFIG_ERROR",
            Self::IoError => "AERO_CLI_IO_ERROR",
            Self::AlreadyInitialized => "AERO_CLI_ALREADY_INITIALIZED",
            Self::NotInitialized => "AERO_CLI_NOT_INITIALIZED",
            Self::BootFailed => "AERO_CLI_BOOT_FAILED",
        }
    }
}

/// CLI error
#[derive(Debug)]
pub struct CliError {
    code: CliErrorCode,
    message: String,
}

impl CliError {
    /// Create a new CLI error
    pub fn new(code: CliErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Config error
    pub fn config_error(msg: impl Into<String>) -> Self {
        Self::new(CliErrorCode::ConfigError, msg)
    }

    /// I/O error
    pub fn io_error(msg: impl Into<String>) -> Self {
        Self::new(CliErrorCode::IoError, msg)
    }

    /// Already initialized
    pub fn already_initialized() -> Self {
        Self::new(
            CliErrorCode::AlreadyInitialized,
            "Data directory already initialized",
        )
    }

    /// Not initialized
    pub fn not_initialized() -> Self {
        Self::new(
            CliErrorCode::NotInitialized,
            "Data directory not initialized. Run 'aerodb init' first.",
        )
    }

    /// Boot failed
    pub fn boot_failed(msg: impl Into<String>) -> Self {
        Self::new(CliErrorCode::BootFailed, msg)
    }

    /// Get the error code
    pub fn code(&self) -> &CliErrorCode {
        &self.code
    }

    /// Get the error code string
    pub fn code_str(&self) -> &'static str {
        self.code.code()
    }

    /// Get the error message
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.code(), self.message)
    }
}

impl std::error::Error for CliError {}

impl From<io::Error> for CliError {
    fn from(e: io::Error) -> Self {
        Self::io_error(e.to_string())
    }
}

impl From<serde_json::Error> for CliError {
    fn from(e: serde_json::Error) -> Self {
        Self::io_error(format!("JSON error: {}", e))
    }
}

/// CLI result type
pub type CliResult<T> = Result<T, CliError>;
