//! Index Determinism Tests
//!
//! Tests for index invariants:
//! - Index rebuild is deterministic
//! - Index lookups return consistent results
//! - Primary key index always exists

use aerodb::index::{DocumentInfo, IndexManager};
use serde_json::json;
use std::collections::HashSet;

// =============================================================================
// Helper Functions
// =============================================================================

fn make_doc(id: &str, name: &str, offset: u64) -> DocumentInfo {
    DocumentInfo {
        document_id: id.to_string(),
        schema_id: "test".to_string(),
        schema_version: "1.0".to_string(),
        is_tombstone: false,
        body: json!({ "_id": id, "name": name }),
        offset,
    }
}

// =============================================================================
// Primary Key Index Tests
// =============================================================================

/// PK index returns correct offset after apply_write.
#[test]
fn test_pk_lookup_correct_offset() {
    let mut manager = IndexManager::pk_only();

    let doc = make_doc("find_me", "Name", 42);
    manager.apply_write(&doc);

    let offsets = manager.lookup_pk("find_me");
    assert_eq!(offsets, vec![42]);
}

/// Missing PK returns empty.
#[test]
fn test_pk_lookup_missing() {
    let mut manager = IndexManager::pk_only();

    let doc = make_doc("exists", "Name", 0);
    manager.apply_write(&doc);

    let offsets = manager.lookup_pk("not_exists");
    assert!(offsets.is_empty());
}

/// Multiple documents indexed correctly.
#[test]
fn test_multiple_documents() {
    let mut manager = IndexManager::pk_only();

    manager.apply_write(&make_doc("doc1", "Alice", 0));
    manager.apply_write(&make_doc("doc2", "Bob", 100));
    manager.apply_write(&make_doc("doc3", "Charlie", 200));

    assert_eq!(manager.lookup_pk("doc1"), vec![0]);
    assert_eq!(manager.lookup_pk("doc2"), vec![100]);
    assert_eq!(manager.lookup_pk("doc3"), vec![200]);
}

// =============================================================================
// Deterministic Lookup Tests
// =============================================================================

/// Same lookup returns same result.
#[test]
fn test_lookup_deterministic() {
    let mut manager = IndexManager::pk_only();
    manager.apply_write(&make_doc("test", "Name", 50));

    // Multiple lookups must return same result
    for _ in 0..100 {
        let offsets = manager.lookup_pk("test");
        assert_eq!(offsets, vec![50]);
    }
}

// =============================================================================
// Secondary Index Tests
// =============================================================================

/// Field index with secondary index enabled.
#[test]
fn test_field_index_lookup() {
    let mut indexed = HashSet::new();
    indexed.insert("name".to_string());

    let mut manager = IndexManager::new(indexed);
    manager.apply_write(&make_doc("doc1", "Alice", 0));
    manager.apply_write(&make_doc("doc2", "Bob", 100));

    let offsets = manager.lookup_eq("name", &json!("Alice"));
    assert_eq!(offsets, vec![0]);

    let offsets = manager.lookup_eq("name", &json!("Bob"));
    assert_eq!(offsets, vec![100]);
}

/// No secondary index for unindexed field.
#[test]
fn test_unindexed_field_empty() {
    // Only PK indexed, no "name" index
    let mut manager = IndexManager::pk_only();
    manager.apply_write(&make_doc("doc1", "Alice", 0));

    // Should return empty for unindexed field
    let offsets = manager.lookup_eq("name", &json!("Alice"));
    assert!(offsets.is_empty());
}

// =============================================================================
// Apply Write and Delete Tests
// =============================================================================

/// Apply write updates index.
#[test]
fn test_apply_write_updates_index() {
    let mut manager = IndexManager::pk_only();

    let doc = DocumentInfo {
        document_id: "new_doc".to_string(),
        schema_id: "test".to_string(),
        schema_version: "1.0".to_string(),
        is_tombstone: false,
        body: json!({ "_id": "new_doc" }),
        offset: 500,
    };

    manager.apply_write(&doc);

    let offsets = manager.lookup_pk("new_doc");
    assert_eq!(offsets, vec![500]);
}

/// Apply delete removes from index.
#[test]
fn test_apply_delete_removes_from_index() {
    let mut manager = IndexManager::pk_only();
    manager.apply_write(&make_doc("to_delete", "Name", 100));

    // Should exist
    assert_eq!(manager.lookup_pk("to_delete"), vec![100]);

    // Delete it
    manager.apply_delete("to_delete", &json!({ "_id": "to_delete" }));

    // Should be gone
    assert!(manager.lookup_pk("to_delete").is_empty());
}

/// Update replaces previous entry.
#[test]
fn test_update_replaces_entry() {
    let mut manager = IndexManager::pk_only();

    // Initial write
    manager.apply_write(&make_doc("doc", "OldName", 100));
    assert_eq!(manager.lookup_pk("doc"), vec![100]);

    // Update at new offset
    manager.apply_write(&make_doc("doc", "NewName", 200));

    // Should have new offset only
    let offsets = manager.lookup_pk("doc");
    assert_eq!(offsets, vec![200]);
}

// =============================================================================
// Offset Order Tests
// =============================================================================

/// Offsets are returned in consistent order.
#[test]
fn test_offsets_consistent_order() {
    let mut manager = IndexManager::pk_only();

    manager.apply_write(&make_doc("z", "Last", 300));
    manager.apply_write(&make_doc("a", "First", 100));
    manager.apply_write(&make_doc("m", "Middle", 200));

    // Each lookup should return consistent results
    let result1 = manager.all_offsets_pk_order();
    let result2 = manager.all_offsets_pk_order();

    assert_eq!(result1, result2);
}

/// Empty manager returns empty offsets.
#[test]
fn test_empty_manager() {
    let manager = IndexManager::pk_only();

    let offsets = manager.all_offsets_pk_order();
    assert!(offsets.is_empty());

    let lookup = manager.lookup_pk("anything");
    assert!(lookup.is_empty());
}
