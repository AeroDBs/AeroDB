//! Phase 7 Confirmation Model
//!
//! Per PHASE7_CONFIRMATION_MODEL.md:
//! - Dangerous or irreversible actions require explicit confirmation
//! - Confirmation is explicit, contemporaneous, specific, and not reusable
//! - Override commands require enhanced confirmation
//!
//! Confirmation is a SAFETY BOUNDARY, not a usability feature.

use std::fmt;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Confirmation token — ephemeral, non-reusable proof of intent.
///
/// Per PHASE7_CONFIRMATION_MODEL.md §4:
/// - Confirmation must be contemporaneous
/// - Confirmation cannot be reused
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationToken {
    /// Unique token ID.
    id: Uuid,

    /// Command this token confirms.
    command_name: String,

    /// Target ID (node/replica) for the command.
    target_id: Option<Uuid>,

    /// When this token was created.
    created_at: SystemTime,

    /// Whether this token has been consumed.
    consumed: bool,
}

impl ConfirmationToken {
    /// Create a new confirmation token for a command.
    pub fn new(command_name: impl Into<String>, target_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            command_name: command_name.into(),
            target_id,
            created_at: SystemTime::now(),
            consumed: false,
        }
    }

    /// Get the token ID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the command name.
    pub fn command_name(&self) -> &str {
        &self.command_name
    }

    /// Check if this token is valid for a specific command.
    ///
    /// Per PHASE7_CONFIRMATION_MODEL.md §4.3:
    /// Confirmation must apply to exactly one command.
    pub fn is_valid_for(&self, command_name: &str, target_id: Option<Uuid>) -> bool {
        !self.consumed && self.command_name == command_name && self.target_id == target_id
    }

    /// Check if this token has expired.
    ///
    /// Tokens are ephemeral and expire after a short period.
    /// Per PHASE7_STATE_MODEL.md §6: Ephemeral state must not survive restarts.
    pub fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at
            .elapsed()
            .map(|d| d > max_age)
            .unwrap_or(true)
    }

    /// Consume this token, marking it as used.
    ///
    /// Returns true if successfully consumed, false if already consumed.
    pub fn consume(&mut self) -> bool {
        if self.consumed {
            false
        } else {
            self.consumed = true;
            true
        }
    }

    /// Check if consumed.
    pub fn is_consumed(&self) -> bool {
        self.consumed
    }
}

/// Enhanced confirmation for override commands.
///
/// Per PHASE7_CONFIRMATION_MODEL.md §8:
/// Override commands require explicit acknowledgement of overridden invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnhancedConfirmation {
    /// Base confirmation token.
    pub token: ConfirmationToken,

    /// Invariants that will be overridden.
    pub overridden_invariants: Vec<String>,

    /// Explicit risk acknowledgements from operator.
    pub acknowledged_risks: Vec<String>,

    /// Clear statement of responsibility transfer.
    pub responsibility_accepted: bool,
}

impl EnhancedConfirmation {
    /// Create enhanced confirmation.
    pub fn new(
        command_name: impl Into<String>,
        target_id: Option<Uuid>,
        overridden_invariants: Vec<String>,
    ) -> Self {
        Self {
            token: ConfirmationToken::new(command_name, target_id),
            overridden_invariants,
            acknowledged_risks: Vec::new(),
            responsibility_accepted: false,
        }
    }

    /// Add risk acknowledgement.
    pub fn acknowledge_risk(mut self, risk: impl Into<String>) -> Self {
        self.acknowledged_risks.push(risk.into());
        self
    }

    /// Accept responsibility.
    pub fn accept_responsibility(mut self) -> Self {
        self.responsibility_accepted = true;
        self
    }

    /// Check if this enhanced confirmation is complete.
    ///
    /// Per PHASE7_CONFIRMATION_MODEL.md §8:
    /// If enhanced confirmation is incomplete, the override MUST be rejected.
    pub fn is_complete(&self) -> bool {
        !self.token.is_consumed()
            && self.responsibility_accepted
            && !self.overridden_invariants.is_empty()
            && self.overridden_invariants.len() == self.acknowledged_risks.len()
    }
}

/// Status of a confirmation flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationStatus {
    /// Awaiting confirmation from operator.
    Pending,

    /// Operator has confirmed.
    Confirmed,

    /// Operator has rejected/cancelled.
    Rejected,

    /// Confirmation window expired.
    Expired,

    /// Token was already consumed.
    AlreadyConsumed,
}

impl fmt::Display for ConfirmationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfirmationStatus::Pending => write!(f, "PENDING"),
            ConfirmationStatus::Confirmed => write!(f, "CONFIRMED"),
            ConfirmationStatus::Rejected => write!(f, "REJECTED"),
            ConfirmationStatus::Expired => write!(f, "EXPIRED"),
            ConfirmationStatus::AlreadyConsumed => write!(f, "ALREADY_CONSUMED"),
        }
    }
}

/// Result of a confirmation attempt.
#[derive(Debug, Clone)]
pub enum ConfirmationResult {
    /// Confirmation successful, proceed with execution.
    Proceed { token_id: Uuid },

    /// Confirmation rejected or failed.
    Abort { reason: String },
}

/// Confirmation flow manager.
///
/// Per PHASE7_STATE_MODEL.md §6:
/// Confirmation state is ephemeral and must not survive restarts.
#[derive(Debug, Default)]
pub struct ConfirmationFlow {
    /// Pending confirmation tokens (ephemeral, in-memory only).
    pending_tokens: Vec<ConfirmationToken>,

    /// Maximum age for confirmation tokens.
    max_token_age: Duration,
}

impl ConfirmationFlow {
    /// Create a new confirmation flow manager.
    pub fn new() -> Self {
        Self {
            pending_tokens: Vec::new(),
            max_token_age: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Create a confirmation token for a command.
    ///
    /// Per PHASE7_CONFIRMATION_MODEL.md §5:
    /// 1. Operator issues command
    /// 2. System generates pre-execution explanation
    /// 3. System requests confirmation (this creates the token)
    pub fn request_confirmation(
        &mut self,
        command_name: impl Into<String>,
        target_id: Option<Uuid>,
    ) -> ConfirmationToken {
        let token = ConfirmationToken::new(command_name, target_id);
        self.pending_tokens.push(token.clone());
        token
    }

    /// Confirm a pending command.
    ///
    /// Per PHASE7_CONFIRMATION_MODEL.md §4.2:
    /// Confirmation must occur immediately after explanation.
    pub fn confirm(
        &mut self,
        token_id: Uuid,
        command_name: &str,
        target_id: Option<Uuid>,
    ) -> ConfirmationResult {
        // Find and validate the token
        let token_idx = self.pending_tokens.iter().position(|t| t.id() == token_id);

        match token_idx {
            None => ConfirmationResult::Abort {
                reason: "Confirmation token not found".to_string(),
            },
            Some(idx) => {
                let token = &mut self.pending_tokens[idx];

                // Check expiry
                if token.is_expired(self.max_token_age) {
                    self.pending_tokens.remove(idx);
                    return ConfirmationResult::Abort {
                        reason: "Confirmation token expired".to_string(),
                    };
                }

                // Check validity for this command
                if !token.is_valid_for(command_name, target_id) {
                    return ConfirmationResult::Abort {
                        reason: "Confirmation token does not match command".to_string(),
                    };
                }

                // Consume the token
                if !token.consume() {
                    return ConfirmationResult::Abort {
                        reason: "Confirmation token already consumed".to_string(),
                    };
                }

                let id = token.id();
                self.pending_tokens.remove(idx);

                ConfirmationResult::Proceed { token_id: id }
            }
        }
    }

    /// Reject/cancel a pending confirmation.
    pub fn reject(&mut self, token_id: Uuid) {
        self.pending_tokens.retain(|t| t.id() != token_id);
    }

    /// Clean up expired tokens.
    pub fn cleanup_expired(&mut self) {
        self.pending_tokens
            .retain(|t| !t.is_expired(self.max_token_age));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirmation_token_created_valid() {
        let token = ConfirmationToken::new("request_promotion", Some(Uuid::new_v4()));
        assert!(!token.is_consumed());
        assert!(!token.is_expired(Duration::from_secs(60)));
    }

    #[test]
    fn test_confirmation_token_consume_once() {
        let mut token = ConfirmationToken::new("request_promotion", None);
        assert!(token.consume());
        assert!(!token.consume()); // Second consume fails
        assert!(token.is_consumed());
    }

    #[test]
    fn test_confirmation_token_validity() {
        let target = Uuid::new_v4();
        let token = ConfirmationToken::new("request_promotion", Some(target));

        assert!(token.is_valid_for("request_promotion", Some(target)));
        assert!(!token.is_valid_for("request_demotion", Some(target)));
        assert!(!token.is_valid_for("request_promotion", None));
    }

    #[test]
    fn test_enhanced_confirmation_incomplete_by_default() {
        let conf = EnhancedConfirmation::new(
            "force_promotion",
            Some(Uuid::new_v4()),
            vec!["P6-A1".to_string()],
        );
        assert!(!conf.is_complete());
    }

    #[test]
    fn test_enhanced_confirmation_complete() {
        let conf = EnhancedConfirmation::new(
            "force_promotion",
            Some(Uuid::new_v4()),
            vec!["P6-A1".to_string()],
        )
        .acknowledge_risk("Primary may still be active")
        .accept_responsibility();

        assert!(conf.is_complete());
    }

    #[test]
    fn test_confirmation_flow() {
        let mut flow = ConfirmationFlow::new();
        let target = Uuid::new_v4();

        let token = flow.request_confirmation("request_promotion", Some(target));
        let token_id = token.id();

        match flow.confirm(token_id, "request_promotion", Some(target)) {
            ConfirmationResult::Proceed { .. } => {}
            ConfirmationResult::Abort { reason } => {
                panic!("Expected proceed, got abort: {}", reason);
            }
        }
    }

    #[test]
    fn test_confirmation_not_reusable() {
        let mut flow = ConfirmationFlow::new();
        let target = Uuid::new_v4();

        let token = flow.request_confirmation("request_promotion", Some(target));
        let token_id = token.id();

        // First confirm succeeds
        assert!(matches!(
            flow.confirm(token_id, "request_promotion", Some(target)),
            ConfirmationResult::Proceed { .. }
        ));

        // Second confirm fails (token consumed/removed)
        assert!(matches!(
            flow.confirm(token_id, "request_promotion", Some(target)),
            ConfirmationResult::Abort { .. }
        ));
    }
}
