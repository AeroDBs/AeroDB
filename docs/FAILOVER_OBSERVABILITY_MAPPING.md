# FAILOVER_OBSERVABILITY_MAPPING.md — Failover & Promotion

## Status
- Phase: **6**
- Authority: **Normative**
- Depends on:
  - FAILOVER_VISION.md
  - FAILOVER_SCOPE.md
  - FAILOVER_INVARIANTS.md
  - FAILOVER_ARCHITECTURE.md
  - FAILOVER_FAILURE_MODEL.md
  - FAILOVER_STATE_MACHINE.md
- Frozen Dependencies: **Phase 4 Observability (Frozen)**

---

## 1. Purpose

This document maps **Phase 6 behavior** to the existing
**observability and explanation infrastructure**.

It defines:
- What events are emitted
- What metrics are exposed
- What explanations are available

Observability in Phase 6 is **passive only**.
It must never influence control flow or decisions.

---

## 2. Observability Principles (Inherited)

Phase 6 observability MUST obey all Phase 4 rules:

- Side-effect free
- Deterministic ordering
- Non-authoritative
- No feedback loops
- No gating behavior

Observability **describes** what happened; it never **decides** what happens.

---

## 3. Event Emission Mapping

Phase 6 introduces the following **new event types**.

### 3.1 Promotion Lifecycle Events

#### `replication.promotion.requested`

Emitted when:
- A promotion request is accepted for processing

Fields:
- `requested_replica_id`
- `current_primary_id` (if known)
- `reason` (operator-provided, optional)

---

#### `replication.promotion.validation_started`

Emitted when:
- Transition to `PromotionValidating` occurs

Fields:
- `requested_replica_id`

---

#### `replication.promotion.validation_failed`

Emitted when:
- Promotion is denied during validation

Fields:
- `requested_replica_id`
- `failed_invariant`
- `failure_reason`

---

#### `replication.promotion.validation_succeeded`

Emitted when:
- Promotion validation completes successfully

Fields:
- `requested_replica_id`

---

#### `replication.promotion.transition_started`

Emitted when:
- Authority transition begins

Fields:
- `requested_replica_id`

---

#### `replication.promotion.transition_completed`

Emitted when:
- Authority transition completes successfully

Fields:
- `new_primary_id`

---

### 3.2 Crash-Related Promotion Events

#### `replication.promotion.aborted_on_crash`

Emitted on recovery if:
- A promotion was in progress but not completed

Fields:
- `last_known_state`
- `requested_replica_id` (if known)

---

## 4. Metrics Mapping

Phase 6 may expose the following **passive metrics**.

### 4.1 Counters

- `replication_promotion_requests_total`
- `replication_promotion_success_total`
- `replication_promotion_denied_total`
- `replication_promotion_crash_abort_total`

---

### 4.2 Gauges

- `replication_current_primary` (labelled by replica_id)
- `replication_promotion_in_progress` (0 or 1)

---

### 4.3 Histograms (Optional)

- `replication_promotion_validation_duration_ms`
- `replication_promotion_transition_duration_ms`

Metrics MUST:
- Be monotonic where applicable
- Never influence behavior
- Never be required for correctness

---

## 5. Explanation Engine Integration

Every promotion attempt MUST produce an explanation artifact.

### 5.1 Explanation on Success

Explanation MUST include:
- Promotion request parameters
- Validation checks performed
- Invariants satisfied
- Authority transition result

---

### 5.2 Explanation on Failure

Explanation MUST include:
- Promotion request parameters
- Exact invariant violated
- Deterministic reason for denial
- No speculative or heuristic language

---

### 5.3 Explanation Stability

Given identical inputs:
- Explanation output MUST be identical
- Ordering MUST be deterministic
- Language MUST be stable

---

## 6. Observability During Failure

If observability emission fails:
- Promotion behavior MUST NOT change
- Promotion MUST NOT be retried
- Failure MUST be logged as best-effort only

Observability failure is **never fatal** to correctness.

---

## 7. Forbidden Observability Behavior

Phase 6 MUST NOT:
- Block promotion on logging failure
- Retry promotion due to metrics
- Adapt behavior based on telemetry
- Hide or compress failure details

---

## 8. Completeness Criteria

Observability mapping is complete when:
- Every Phase 6 state transition emits at least one event
- Every promotion attempt is explainable
- No observability signal affects decisions
- All signals are deterministic and auditable

---

END OF DOCUMENT
