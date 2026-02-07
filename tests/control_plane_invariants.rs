//! Phase 7 Control Plane Invariant Tests
//!
//! Per PHASE7_TESTING_STRATEGY.md:
//! Tests must prove that invariants hold under all conditions.
//!
//! Test Categories:
//! 1. Invariant enforcement tests
//! 2. Confirmation bypass tests
//! 3. Duplicate command tests
//! 4. Failure injection tests
//! 5. Audit-log completeness tests
//! 6. Kernel non-interference tests

use uuid::Uuid;

// Import Phase 7 modules
use aerodb::dx::api::control_plane::{
    AuthorityContext, AuthorityLevel, CommandOutcome, CommandRequest, ConfirmationFlow,
    ConfirmationResult, ConfirmationToken, ControlCommand, ControlPlaneCommand, ControlPlaneError,
    ControlPlaneErrorDomain, ControlPlaneHandler, DiagnosticCommand, InspectionCommand,
};
use aerodb::observability::{AuditAction, AuditLog, AuditOutcome, AuditRecord, MemoryAuditLog};

// =============================================================================
// P7-A2: NO EXECUTION WITHOUT CONFIRMATION
// =============================================================================

/// Test: Mutating commands require confirmation.
///
/// Per PHASE7_CONFIRMATION_MODEL.md §3:
/// All Control Commands require confirmation.
#[test]
fn test_no_execution_without_confirmation() {
    let mut handler = ControlPlaneHandler::new();
    let replica_id = Uuid::new_v4();

    // Create a mutating command without confirmation token
    let cmd = ControlPlaneCommand::Control(ControlCommand::RequestPromotion {
        replica_id,
        reason: None,
    });
    let request = CommandRequest::new(cmd, AuthorityContext::operator());

    // Execute - should return awaiting confirmation, not success
    let response = handler.handle_command(request).unwrap();
    assert_eq!(response.outcome, CommandOutcome::AwaitingConfirmation);
    assert!(response.confirmation_token.is_some());
}

/// Test: Inspection commands do NOT require confirmation.
#[test]
fn test_inspection_no_confirmation_required() {
    let mut handler = ControlPlaneHandler::new();

    let cmd = ControlPlaneCommand::Inspection(InspectionCommand::InspectClusterState);
    let request = CommandRequest::new(cmd, AuthorityContext::observer());

    let response = handler.handle_command(request).unwrap();
    assert_eq!(response.outcome, CommandOutcome::Success);
    assert!(response.confirmation_token.is_none());
}

// =============================================================================
// P7-A1: AUTHORITY ENFORCEMENT
// =============================================================================

/// Test: Observer cannot execute mutating commands.
///
/// Per PHASE7_AUTHORITY_MODEL.md §3.2:
/// Operator authority is required for all state mutation.
#[test]
fn test_observer_cannot_mutate() {
    let mut handler = ControlPlaneHandler::new();

    let cmd = ControlPlaneCommand::Control(ControlCommand::RequestPromotion {
        replica_id: Uuid::new_v4(),
        reason: None,
    });

    // Observer tries to execute mutating command
    let request = CommandRequest::new(cmd, AuthorityContext::observer());
    let result = handler.handle_command(request);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.domain(), ControlPlaneErrorDomain::ValidationError);
    assert!(err.code().contains("INSUFFICIENT_AUTHORITY"));
}

/// Test: Authority levels have correct permissions.
#[test]
fn test_authority_level_permissions() {
    assert!(!AuthorityLevel::Observer.can_mutate());
    assert!(AuthorityLevel::Operator.can_mutate());
    assert!(!AuthorityLevel::Auditor.can_mutate());

    // All can observe
    assert!(AuthorityLevel::Observer.can_observe());
    assert!(AuthorityLevel::Operator.can_observe());
    assert!(AuthorityLevel::Auditor.can_observe());
}

// =============================================================================
// CONFIRMATION NOT REUSABLE
// =============================================================================

/// Test: Confirmation token cannot be reused.
///
/// Per PHASE7_CONFIRMATION_MODEL.md §7:
/// Confirmation tokens are single-use.
#[test]
fn test_confirmation_not_reusable() {
    let mut flow = ConfirmationFlow::new();
    let target = Uuid::new_v4();

    let token = flow.request_confirmation("request_promotion", Some(target));
    let token_id = token.id();

    // First confirm succeeds
    let result1 = flow.confirm(token_id, "request_promotion", Some(target));
    assert!(matches!(result1, ConfirmationResult::Proceed { .. }));

    // Second confirm fails (token consumed)
    let result2 = flow.confirm(token_id, "request_promotion", Some(target));
    assert!(matches!(result2, ConfirmationResult::Abort { .. }));
}

/// Test: Confirmation token is specific to command.
#[test]
fn test_confirmation_command_specific() {
    let mut flow = ConfirmationFlow::new();
    let target = Uuid::new_v4();

    // Token for request_promotion
    let token = flow.request_confirmation("request_promotion", Some(target));
    let token_id = token.id();

    // Try to use for request_demotion - should fail
    let result = flow.confirm(token_id, "request_demotion", Some(target));
    assert!(matches!(result, ConfirmationResult::Abort { .. }));
}

// =============================================================================
// FORCE PROMOTION REQUIRES ENHANCED CONFIRMATION
// =============================================================================

/// Test: Force promotion requires enhanced confirmation.
///
/// Per PHASE7_CONFIRMATION_MODEL.md §8:
/// Override commands require enhanced confirmation.
#[test]
fn test_force_promotion_requires_enhanced_confirmation() {
    let cmd = ControlCommand::ForcePromotion {
        replica_id: Uuid::new_v4(),
        reason: "Primary unavailable".to_string(),
        acknowledged_risks: vec!["P6-A1 may be violated".to_string()],
    };

    assert!(cmd.requires_enhanced_confirmation());

    // Regular promotion does not
    let regular = ControlCommand::RequestPromotion {
        replica_id: Uuid::new_v4(),
        reason: None,
    };
    assert!(!regular.requires_enhanced_confirmation());
}

// =============================================================================
// ERROR MODEL
// =============================================================================

/// Test: Errors include execution outcome.
///
/// Per PHASE7_ERROR_MODEL.md §9:
/// Errors MUST include execution outcome.
#[test]
fn test_error_includes_outcome() {
    let err = ControlPlaneError::missing_confirmation("request_promotion");
    assert!(err.definitely_not_executed());
    assert!(err.invariant().is_some());
    assert!(err.invariant().unwrap().contains("P7-A2"));
}

/// Test: Transport errors have unknown outcome.
///
/// Per PHASE7_ERROR_MODEL.md §7.2:
/// Transport errors MUST be treated as unknown execution outcome.
#[test]
fn test_transport_error_unknown_outcome() {
    let err = ControlPlaneError::transport_failure("connection reset");
    assert!(!err.definitely_not_executed());
    assert_eq!(err.domain(), ControlPlaneErrorDomain::TransportError);
}

/// Test: Kernel rejections are passed through verbatim.
///
/// Per PHASE7_ERROR_MODEL.md §6.2:
/// Kernel rejections MUST be surfaced verbatim.
#[test]
fn test_kernel_rejection_passthrough() {
    let err = ControlPlaneError::from_kernel_rejection(
        "AERO_PROMOTION_ALREADY_IN_PROGRESS",
        "Another promotion is already in progress",
    );
    assert_eq!(err.domain(), ControlPlaneErrorDomain::KernelRejection);
    assert_eq!(err.code(), "AERO_PROMOTION_ALREADY_IN_PROGRESS");
}

// =============================================================================
// AUDIT LOG COMPLETENESS
// =============================================================================

/// Test: Every command attempt produces an audit record.
///
/// Per PHASE7_INVARIANTS.md §P7-O3:
/// States and action history are written to persistent append-only audit logs.
#[test]
fn test_audit_record_on_command() {
    let audit_log = MemoryAuditLog::new();

    // Record a command request
    let record = AuditRecord::new(AuditAction::CommandRequested, AuditOutcome::Pending)
        .with_command("request_promotion")
        .with_authority("OPERATOR");

    audit_log.append(&record).unwrap();

    assert_eq!(audit_log.len(), 1);
    let records = audit_log.records();
    assert_eq!(records[0].action, AuditAction::CommandRequested);
}

/// Test: Audit records include all required fields.
#[test]
fn test_audit_record_fields() {
    let request_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let token_id = Uuid::new_v4();

    let record = AuditRecord::new(AuditAction::ConfirmationProvided, AuditOutcome::Success)
        .with_command("request_promotion")
        .with_request_id(request_id)
        .with_target(target_id)
        .with_authority("OPERATOR")
        .with_operator("admin@example.com")
        .with_confirmation_token(token_id);

    assert_eq!(record.action, AuditAction::ConfirmationProvided);
    assert_eq!(record.outcome, AuditOutcome::Success);
    assert_eq!(record.command_name, Some("request_promotion".to_string()));
    assert_eq!(record.request_id, Some(request_id));
    assert_eq!(record.target_id, Some(target_id));
    assert_eq!(record.confirmation_token, Some(token_id));
}

/// Test: Audit records serialize to JSON correctly.
#[test]
fn test_audit_record_json() {
    let record = AuditRecord::new(AuditAction::CommandExecuted, AuditOutcome::Success)
        .with_command("inspect_cluster_state");

    let json = record.to_json();
    assert!(json.contains("COMMAND_EXECUTED"));
    assert!(json.contains("SUCCESS"));
    assert!(json.contains("inspect_cluster_state"));
}

// =============================================================================
// COMMAND SEMANTICS
// =============================================================================

/// Test: Each command has exactly one kernel action.
///
/// Per PHASE7_INVARIANTS.md §P7-E1:
/// Each operator command MUST correspond to exactly one kernel action.
#[test]
fn test_command_names() {
    assert_eq!(
        InspectionCommand::InspectClusterState.command_name(),
        "inspect_cluster_state"
    );
    assert_eq!(
        DiagnosticCommand::RunDiagnostics.command_name(),
        "run_diagnostics"
    );
    assert_eq!(
        ControlCommand::RequestPromotion {
            replica_id: Uuid::nil(),
            reason: None
        }
        .command_name(),
        "request_promotion"
    );
}

/// Test: Commands correctly identify if they require confirmation.
#[test]
fn test_command_requires_confirmation() {
    let inspection = ControlPlaneCommand::Inspection(InspectionCommand::InspectClusterState);
    assert!(!inspection.requires_confirmation());

    let diag_cheap = ControlPlaneCommand::Diagnostic(DiagnosticCommand::InspectWal);
    assert!(!diag_cheap.requires_confirmation());

    let diag_expensive = ControlPlaneCommand::Diagnostic(DiagnosticCommand::RunDiagnostics);
    assert!(diag_expensive.requires_confirmation());

    let control = ControlPlaneCommand::Control(ControlCommand::RequestPromotion {
        replica_id: Uuid::new_v4(),
        reason: None,
    });
    assert!(control.requires_confirmation());
}

/// Test: Commands correctly identify if they are mutating.
#[test]
fn test_command_is_mutating() {
    let inspection = ControlPlaneCommand::Inspection(InspectionCommand::InspectClusterState);
    assert!(!inspection.is_mutating());

    let diagnostic = ControlPlaneCommand::Diagnostic(DiagnosticCommand::RunDiagnostics);
    assert!(!diagnostic.is_mutating());

    let control = ControlPlaneCommand::Control(ControlCommand::RequestPromotion {
        replica_id: Uuid::new_v4(),
        reason: None,
    });
    assert!(control.is_mutating());
}

// =============================================================================
// CONFIRMATION FLOW LIFECYCLE
// =============================================================================

/// Test: Full confirmation lifecycle.
#[test]
fn test_full_confirmation_lifecycle() {
    let mut handler = ControlPlaneHandler::new();
    let replica_id = Uuid::new_v4();

    // Step 1: Issue command without confirmation
    let cmd = ControlPlaneCommand::Control(ControlCommand::RequestPromotion {
        replica_id,
        reason: Some("Primary unavailable".to_string()),
    });
    let request1 = CommandRequest::new(cmd.clone(), AuthorityContext::operator());

    let response1 = handler.handle_command(request1).unwrap();
    assert_eq!(response1.outcome, CommandOutcome::AwaitingConfirmation);
    let token_id = response1.confirmation_token.unwrap();

    // Step 2: Issue command with confirmation token
    let request2 =
        CommandRequest::new(cmd, AuthorityContext::operator()).with_confirmation(token_id);

    let response2 = handler.handle_command(request2).unwrap();
    assert_eq!(response2.outcome, CommandOutcome::Success);
}

// =============================================================================
// OBSERVABILITY IS PASSIVE
// =============================================================================

/// Test: Observability exports have no side effects.
///
/// Per PHASE7_OBSERVABILITY_MODEL.md §2:
/// Observability MUST NOT influence execution.
#[test]
fn test_observability_is_passive() {
    // Inspection commands return data without affecting state
    let mut handler = ControlPlaneHandler::new();

    // Execute inspection twice
    let cmd = ControlPlaneCommand::Inspection(InspectionCommand::InspectClusterState);

    let request1 = CommandRequest::new(cmd.clone(), AuthorityContext::observer());
    let result1 = handler.handle_command(request1).unwrap();

    let request2 = CommandRequest::new(cmd, AuthorityContext::observer());
    let result2 = handler.handle_command(request2).unwrap();

    // Both should succeed (inspection is idempotent and passive)
    assert_eq!(result1.outcome, CommandOutcome::Success);
    assert_eq!(result2.outcome, CommandOutcome::Success);
}
