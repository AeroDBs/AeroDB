# EXPLANATION MODEL — PHASE 4

## Status

- Phase: **4**
- Authority: **Normative**
- Scope: **All explanations exposed via API or UI**
- Depends on:
  - PHASE4_VISION.md
  - PHASE4_INVARIANTS.md
  - OBSERVABILITY_API.md
  - SEMANTIC_EQUIVALENCE.md
  - FAILURE_MODEL_PHASE3.md

This document defines the **formal model for explanations** in AeroDB.
If an explanation cannot be expressed using this model, it MUST NOT be exposed.

---

## 1. Purpose

AeroDB does not merely *report outcomes*.
It must be able to **explain why those outcomes occurred**.

This document ensures that explanations are:

- Exact
- Verifiable
- Deterministic
- Evidence-based

Explanations are not summaries, guesses, or narratives.
They are **structured proofs over observed state**.

---

## 2. Core Principle

### Explanation-as-Evidence Principle

> Every explanation in AeroDB must be reducible  
> to concrete system state and explicit rules.

An explanation is valid only if:
- It references real identifiers
- It applies explicit rules
- It can be independently re-derived

If an explanation cannot be proven, it must not exist.

---

## 3. What an Explanation Is (and Is Not)

### 3.1 An Explanation IS

- A structured record
- A sequence of evaluated facts
- A traceable reasoning path
- A deterministic artifact

### 3.2 An Explanation IS NOT

- A human-friendly story
- A heuristic guess
- A performance hint
- A visualization-driven inference
- A simplification for convenience

If an explanation reads like prose, it is probably wrong.

---

## 4. Explanation Object Model

All explanations MUST conform to the following structure:

```json
{
  "explanation_type": "<type>",
  "observed_snapshot": {
    "snapshot_id": "<id>",
    "commit_id": <number>
  },
  "inputs": { ... },
  "rules_applied": [
    {
      "rule_id": "<stable identifier>",
      "description": "<exact rule reference>",
      "evaluation": "true | false",
      "evidence": { ... }
    }
  ],
  "conclusion": { ... }
}
````

Rules:

* `rule_id` MUST map to a documented invariant or rule
* `evaluation` MUST be explicit
* `evidence` MUST be raw state, not interpretation

---

## 5. Rule References

### 5.1 Rule Identity

Every rule referenced in explanations MUST:

* Have a stable identifier
* Map to one of:

  * INVARIANTS.md
  * PHASE*_INVARIANTS.md
  * MVCC rules
  * Replication rules
  * Failure model rules

Free-form rules are forbidden.

---

### 5.2 Rule Application Semantics

Rules MUST be applied:

* In a defined order
* Explicitly
* Without short-circuiting unless documented

Skipping a rule is a correctness violation.

---

## 6. Explanation Types (Mandatory)

The following explanation types MUST be supported.

---

### 6.1 Read Visibility Explanation

**Type:** `mvcc.read_visibility`

Explains:

* Why a specific document version is visible or invisible

Inputs:

* Document ID
* Snapshot CommitId

Rules Applied:

* MVCC visibility rules
* CommitId comparisons
* Tombstone handling

Conclusion:

* Visible version ID OR
* Explicit “no visible version” result

---

### 6.2 Query Execution Explanation

**Type:** `query.execution`

Explains:

* How a query was executed

Inputs:

* Query
* Snapshot CommitId

Rules Applied:

* Query planning rules
* Bounds enforcement
* Index advisory usage

Conclusion:

* Deterministic execution plan
* Guarantees enforced

---

### 6.3 Recovery Explanation

**Type:** `recovery.process`

Explains:

* How recovery proceeded after a crash

Inputs:

* Last known durable state

Rules Applied:

* Checkpoint selection rules
* WAL replay rules
* Checksum validation rules

Conclusion:

* Final recovered CommitId
* State validity result

---

### 6.4 Checkpoint Safety Explanation

**Type:** `checkpoint.safety`

Explains:

* Why a checkpoint is valid or not

Inputs:

* Checkpoint marker
* Snapshot metadata
* WAL offsets

Rules Applied:

* Durability ordering rules
* Snapshot completeness rules

Conclusion:

* Checkpoint accepted or rejected

---

### 6.5 Replication Safety Explanation (Conditional)

**Type:** `replication.safety`

Explains:

* Why a replica is safe to serve reads

Inputs:

* Replica WAL prefix
* Snapshot CommitId

Rules Applied:

* WAL prefix rule
* MVCC snapshot safety rule

Conclusion:

* Read-safe or not read-safe

---

## 7. Determinism Requirements

Explanations MUST be:

* Deterministic
* Stable across restarts
* Independent of timing
* Independent of UI context

If two identical states produce different explanations,
that is a correctness bug.

---

## 8. Failure and Partial-State Handling

### 8.1 Incomplete Evidence

If evidence is missing:

* The explanation MUST say so explicitly
* No inferred conclusion is allowed

Example:

```json
{
  "conclusion": {
    "status": "undetermined",
    "reason": "WAL segment missing"
  }
}
```

---

### 8.2 Failure During Explanation

If explanation generation fails:

* The failure MUST be surfaced
* Partial explanations MUST be marked incomplete

Silently dropping explanations is forbidden.

---

## 9. Relationship to UI

The UI:

* Renders explanations
* Does not generate explanations
* Does not reorder rules
* Does not simplify conclusions

All explanation logic lives in the core system or API layer.

---

## 10. Relationship to Observability API

* Explanations are delivered via `/v1/explain/*`
* API responses MUST embed explanation objects verbatim
* UI consumes explanations as immutable artifacts

No UI-side interpretation allowed.

---

## 11. Testing Requirements

Each explanation type MUST have tests that:

* Verify rule completeness
* Verify evidence accuracy
* Verify determinism
* Verify failure transparency

Golden-file tests are recommended.

---

## 12. Explicit Non-Goals

The Explanation Model does NOT aim to:

* Teach database theory
* Optimize readability
* Provide debugging shortcuts
* Replace documentation

It exists to provide **proof artifacts**, not tutorials.

---

## 13. Final Rule

> An explanation that cannot be proven
> is worse than no explanation at all.

AeroDB earns trust by refusing to guess.

---

END OF DOCUMENT
