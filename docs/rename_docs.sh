#!/usr/bin/env bash
set -euo pipefail

DRY_RUN=0
if [[ "${1:-}" == "--dry-run" ]]; then
  DRY_RUN=1
  echo "Running in DRY-RUN mode (no files will be renamed)"
fi

rename_file() {
  local src="$1"
  local dst="$2"

  if [[ ! -f "$src" ]]; then
    echo "ERROR: Source file not found: $src"
    exit 1
  fi

  if [[ -f "$dst" ]]; then
    echo "ERROR: Destination already exists: $dst"
    exit 1
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "mv \"$src\" \"$dst\""
  else
    mv "$src" "$dst"
  fi
}

echo "=== Renaming Core / Phase 1 docs ==="
rename_file VISION.md CORE_VISION.md
rename_file INVARIANTS.md CORE_INVARIANTS.md
rename_file SCOPE.md CORE_SCOPE.md
rename_file RELIABILITY.md CORE_RELIABILITY.md
rename_file WAL.md CORE_WAL.md
rename_file STORAGE.md CORE_STORAGE.md
rename_file SNAPSHOT.md CORE_SNAPSHOT.md
rename_file CHECKPOINT.md CORE_CHECKPOINT.md
rename_file BACKUP.md CORE_BACKUP.md
rename_file RESTORE.md CORE_RESTORE.md
rename_file BOOT.md CORE_BOOT.md
rename_file LIFECYCLE.md CORE_LIFECYCLE.md
rename_file SCHEMA.md CORE_SCHEMA.md
rename_file QUERY.md CORE_QUERY.md
rename_file API_SPEC.md CORE_API_SPEC.md
rename_file ERRORS.md CORE_ERRORS.md

echo "=== Renaming MVCC docs (Phase 2A) ==="
rename_file MVCC.md MVCC_MODEL.md
rename_file MVCC_VISIBILITY.md MVCC_VISIBILITY_RULES.md
rename_file MVCC_WAL_INTERACTION.md MVCC_WAL_INTEGRATION.md
rename_file MVCC_SNAPSHOT_INTEGRATION.md MVCC_SNAPSHOT_MODEL.md
rename_file MVCC_GC.md MVCC_GC_MODEL.md
rename_file MVCC_TESTING_STRATEGY.md MVCC_TESTING.md
# These are already correctly named, no change:
# MVCC_FAILURE_MATRIX.md
# MVCC_COMPATIBILITY.md

echo "=== Renaming Replication docs (Phase 2B) ==="
rename_file PHASE2_REPLICATION_VISION.md REPL_VISION.md
rename_file PHASE2_REPLICATION_INVARIANTS.md REPL_INVARIANTS.md
rename_file PHASE2_REPLICATION_PROOFS.md REPL_PROOFS.md
rename_file REPLICATION_MODEL.md REPL_MODEL.md
rename_file REPLICATION_LOG_FLOW.md REPL_WAL_FLOW.md
rename_file REPLICATION_READ_SEMANTICS.md REPL_READ_RULES.md
rename_file REPLICATION_RECOVERY.md REPL_RECOVERY_MODEL.md
rename_file REPLICATION_SNAPSHOT_TRANSFER.md REPL_SNAPSHOT_TRANSFER.md
rename_file REPLICATION_FAILURE_MATRIX.md REPL_FAILURE_MATRIX.md
rename_file REPLICATION_COMPATIBILITY.md REPL_COMPATIBILITY.md
rename_file PHASE2_REPLICATION_READINESS.md PHASE2_REPL_READINESS.md

echo "=== Renaming Performance docs (Phase 3) ==="
rename_file PHASE3_VISION.md PERF_VISION.md
rename_file PHASE3_INVARIANTS.md PERF_INVARIANTS.md
rename_file PHASE3_PROOF_RULES.md PERF_PROOF_RULES.md
rename_file PERFORMANCE_BASELINE.md PERF_BASELINE.md
rename_file SEMANTIC_EQUIVALENCE.md PERF_SEMANTIC_EQUIVALENCE.md
rename_file FAILURE_MODEL_PHASE3.md PERF_FAILURE_MODEL.md
rename_file ROLLBACK_AND_DISABLEMENT.md PERF_DISABLEMENT.md
rename_file GROUP_COMMIT.md PERF_GROUP_COMMIT.md
rename_file WAL_BATCHING.md PERF_WAL_BATCHING.md
rename_file READ_PATH_OPTIMIZATION.md PERF_READ_PATH.md
rename_file INDEX_ACCELERATION.md PERF_INDEX_ACCELERATION.md
rename_file CHECKPOINT_PIPELINING.md PERF_CHECKPOINT_PIPELINING.md
rename_file REPLICA_READ_FAST_PATH.md PERF_REPLICA_READ_FAST_PATH.md
rename_file MEMORY_LAYOUT_OPTIMIZATION.md PERF_MEMORY_LAYOUT.md
rename_file PERFORMANCE_OBSERVABILITY.md PERF_OBSERVABILITY.md

echo "=== Renaming Developer Experience docs (Phase 4) ==="
rename_file PHASE4_VISION.md DX_VISION.md
rename_file PHASE4_INVARIANTS.md DX_INVARIANTS.md
rename_file OBSERVABILITY_API.md DX_OBSERVABILITY_API.md
rename_file ADMIN_UI_ARCHITECTURE.md DX_ADMIN_UI_ARCH.md
rename_file EXPLANATION_MODEL.md DX_EXPLANATION_MODEL.md
rename_file OBSERVABILITY.md DX_OBSERVABILITY_PRINCIPLES.md

echo "=== Renaming Meta / Planning docs ==="
rename_file PLAN.md PROJECT_PLAN.md

echo "=== Rename completed successfully ==="
if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "(Dry-run only; no changes were made)"
fi
