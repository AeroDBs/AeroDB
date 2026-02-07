# Phase 7 Control Plane Implementation

## Status: COMPLETE

The Phase 7 Control Plane Skeleton has been implemented, providing the operator control surface for AeroDB per the Phase 7 specification documents.

## implementation Guidelines

This implementation strictly adheres to:
- **PHASE7_AUTHORITY_MODEL.md**: Explicit authority levels (Observer, Operator, Auditor).
- **PHASE7_CONFIRMATION_MODEL.md**: Mandatory, ephemeral confirmation for mutations.
- **PHASE7_AUDITABILITY.md**: Durable, append-only audit logging.
- **PHASE7_ERROR_MODEL.md**: Deterministic, domain-specific errors.

## Components Implemented

### 1. Control Plane Core (`src/dx/api/control_plane/`)

This module isolates control plane logic from the kernel, acting as a "thin server" that validates authority and intent before dispatching to the kernel.

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports and organization |
| `authority.rs` | Definitions for `AuthorityLevel` (Observer, Operator, Auditor) `AuthorityContext` |
| `commands.rs` | Definitions for all 10 Phase 7 commands (Inspection, Diagnostic, Control) |
| `confirmation.rs` | Implementation of `ConfirmationFlow` and ephemeral `ConfirmationToken`s |
| `errors.rs` | `ControlPlaneError` with 4 domains: OperatorInput, Validation, KernelRejection, Transport |
| `types.rs` | Request/Response types (`CommandRequest`, `CommandResponse`) and state views |
| `handlers.rs` | `ControlPlaneHandler` for routing, authority checks, and interaction with the kernel |

### 2. Audit Logging (`src/observability/audit.rs`)

A new audit subsystem provides immutable, append-only logging of all control plane actions.

- **Types**: `AuditRecord`, `AuditAction`, `AuditOutcome`.
- **Sustainability**: `FileAuditLog` implements fsync-ed append-only persistence.
- **Completeness**: Records every attempt, confirmation, rejection, and execution result.

### 3. Observability (`src/dx/explain/control_plane.rs`)

Phase 7-specific explanations that are distinct from Phase 4 query explanations.

- **PreExecutionExplanation**: Generated *before* confirmation to show the operator exactly what will happen (consequences, invariant impacts, risks).
- **PostExecutionExplanation**: Generated *after* execution for audit trails.

### 4. CLI Integration (`src/cli/`)

The CLI has been extended to serve as the reference client for the control plane.

- **New Subcommand**: `aerodb control`
- **Actions**:
    - `inspect <cluster|node|replication|promotion>`: Read-only state views.
    - `diag <diagnostics|wal|snapshots>`: Read-only diagnostics.
    - `promote`, `demote`: Mutating commands requiring confirmation.
    - `force-promote`: Override command requiring enhanced confirmation (acknowledging risks).
- **Implementation**: `cli/commands.rs` implements a `control()` handler that acts as a thin client, creating an in-memory audit log for the session.

### 5. Verification (`tests/control_plane_invariants.rs`)

A comprehensive test suite verifies the critical invariants of the control plane:

- **Authority Enforcement**: Verifies that Observers cannot execute mutating commands.
- **Confirmation Safety**: Verifies that mutating commands return `AwaitingConfirmation` initially, and only proceed with a valid token.
- **Token Safety**: Verifies that tokens are single-use and command-specific.
- **Audit Completeness**: Verifies that actions generate appropriate audit records.
- **Error Determinism**: Verifies that errors include execution outcomes.

## Key Design Decisions

1.  **Ephemeral Confirmation**: Confirmation state is held in-memory (`ConfirmationFlow`) and does not survive control plane restarts. This prevents stale approvals from persisting.
2.  **Fail-Closed**: Any validation failure, authority mismatch, or ambiguity results in immediate rejection.
3.  **Passive Observability**: Inspection commands are strictly read-only and idempotent.
4.  **No Automation**: The control plane provides *tools* for human operators but offers no autonomic behaviors (no auto-failover, no rebalancing).

## Verification Results

- **Unit Tests**: All 893 library unit tests pass.
- **Invariant Tests**: The dedicated Phase 7 invariant tests pass.
- **Compilation**: The control plane compiles cleanly as part of the `aerodb` library.
