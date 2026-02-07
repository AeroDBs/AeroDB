//! BTreeMap-based index structures
//!
//! Indexes use BTreeMap<IndexKey, Vec<StorageOffset>> for deterministic ordering.
//! Offsets are always sorted ascending.

use std::collections::BTreeMap;

/// Index key representing a serialized field value.
///
/// Supports String, Int (i64), Float (f64 bits for ordering), Bool.
/// Ordering is deterministic: Bool < Int < Float < String.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexKey {
    /// Boolean value (false < true)
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Float value (stored as bits for total ordering)
    Float(u64),
    /// String value
    String(String),
}

impl IndexKey {
    /// Create a key from a boolean
    pub fn from_bool(v: bool) -> Self {
        IndexKey::Bool(v)
    }

    /// Create a key from an integer
    pub fn from_int(v: i64) -> Self {
        IndexKey::Int(v)
    }

    /// Create a key from a float
    ///
    /// Uses bit representation for total ordering.
    pub fn from_float(v: f64) -> Self {
        // Convert to total-ordering bits
        let bits = v.to_bits();
        // Handle negative floats: flip all bits
        // Handle positive floats: flip sign bit
        let ordered = if (bits >> 63) == 1 {
            !bits // Negative: flip all bits
        } else {
            bits ^ (1 << 63) // Positive: flip sign bit
        };
        IndexKey::Float(ordered)
    }

    /// Create a key from a string
    pub fn from_string(v: impl Into<String>) -> Self {
        IndexKey::String(v.into())
    }

    /// Create a key from a JSON value
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::Bool(b) => Some(IndexKey::from_bool(*b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Some(IndexKey::from_int(i))
                } else if let Some(f) = n.as_f64() {
                    Some(IndexKey::from_float(f))
                } else {
                    None
                }
            }
            serde_json::Value::String(s) => Some(IndexKey::from_string(s)),
            _ => None, // Arrays and objects not indexed
        }
    }
}

/// Storage offset type
pub type StorageOffset = u64;

/// A single field index using BTreeMap for deterministic ordering.
#[derive(Debug, Default)]
pub struct IndexTree {
    /// Maps key values to sorted lists of offsets
    tree: BTreeMap<IndexKey, Vec<StorageOffset>>,
}

impl IndexTree {
    /// Creates a new empty index tree
    pub fn new() -> Self {
        Self {
            tree: BTreeMap::new(),
        }
    }

    /// Insert an offset for a key.
    ///
    /// Maintains sorted ascending order.
    pub fn insert(&mut self, key: IndexKey, offset: StorageOffset) {
        let offsets = self.tree.entry(key).or_default();

        // Insert maintaining sorted order
        match offsets.binary_search(&offset) {
            Ok(_) => {} // Already exists
            Err(pos) => offsets.insert(pos, offset),
        }
    }

    /// Remove an offset for a key.
    ///
    /// If the key has no more offsets, removes the key entirely.
    pub fn remove(&mut self, key: &IndexKey, offset: StorageOffset) {
        if let Some(offsets) = self.tree.get_mut(key) {
            if let Ok(pos) = offsets.binary_search(&offset) {
                offsets.remove(pos);
            }
            if offsets.is_empty() {
                self.tree.remove(key);
            }
        }
    }

    /// Lookup all offsets for an exact key match.
    ///
    /// Returns offsets sorted ascending.
    pub fn lookup_eq(&self, key: &IndexKey) -> Vec<StorageOffset> {
        self.tree.get(key).cloned().unwrap_or_default()
    }

    /// Lookup offsets in a range [min, max] (inclusive).
    ///
    /// Returns offsets sorted ascending.
    /// If min is None, starts from the beginning.
    /// If max is None, goes to the end.
    pub fn lookup_range(
        &self,
        min: Option<&IndexKey>,
        max: Option<&IndexKey>,
    ) -> Vec<StorageOffset> {
        use std::ops::Bound;

        let min_bound: Bound<&IndexKey> = match min {
            Some(k) => Bound::Included(k),
            None => Bound::Unbounded,
        };
        let max_bound: Bound<&IndexKey> = match max {
            Some(k) => Bound::Included(k),
            None => Bound::Unbounded,
        };

        let mut result = Vec::new();
        for (_, offsets) in self.tree.range((min_bound, max_bound)) {
            result.extend(offsets);
        }

        // Sort to ensure deterministic order even when combining multiple keys
        result.sort();
        result
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.tree.clear();
    }

    /// Returns the number of distinct keys
    pub fn key_count(&self) -> usize {
        self.tree.len()
    }

    /// Returns the total number of offsets
    pub fn offset_count(&self) -> usize {
        self.tree.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_ordering() {
        let keys = vec![
            IndexKey::from_bool(false),
            IndexKey::from_bool(true),
            IndexKey::from_int(-100),
            IndexKey::from_int(0),
            IndexKey::from_int(100),
            IndexKey::from_string("aaa"),
            IndexKey::from_string("zzz"),
        ];

        // Verify ordering
        for i in 1..keys.len() {
            assert!(keys[i - 1] < keys[i], "Keys should be ordered");
        }
    }

    #[test]
    fn test_insert_and_lookup() {
        let mut tree = IndexTree::new();

        tree.insert(IndexKey::from_string("alice"), 100);
        tree.insert(IndexKey::from_string("alice"), 200);
        tree.insert(IndexKey::from_string("bob"), 300);

        let alice_offsets = tree.lookup_eq(&IndexKey::from_string("alice"));
        assert_eq!(alice_offsets, vec![100, 200]);

        let bob_offsets = tree.lookup_eq(&IndexKey::from_string("bob"));
        assert_eq!(bob_offsets, vec![300]);
    }

    #[test]
    fn test_offsets_sorted() {
        let mut tree = IndexTree::new();

        // Insert in reverse order
        tree.insert(IndexKey::from_int(42), 300);
        tree.insert(IndexKey::from_int(42), 100);
        tree.insert(IndexKey::from_int(42), 200);

        let offsets = tree.lookup_eq(&IndexKey::from_int(42));
        assert_eq!(offsets, vec![100, 200, 300]);
    }

    #[test]
    fn test_remove() {
        let mut tree = IndexTree::new();

        tree.insert(IndexKey::from_int(1), 100);
        tree.insert(IndexKey::from_int(1), 200);

        tree.remove(&IndexKey::from_int(1), 100);

        let offsets = tree.lookup_eq(&IndexKey::from_int(1));
        assert_eq!(offsets, vec![200]);

        // Remove last offset, key should be removed
        tree.remove(&IndexKey::from_int(1), 200);
        assert_eq!(tree.key_count(), 0);
    }

    #[test]
    fn test_lookup_range() {
        let mut tree = IndexTree::new();

        tree.insert(IndexKey::from_int(1), 100);
        tree.insert(IndexKey::from_int(2), 200);
        tree.insert(IndexKey::from_int(3), 300);
        tree.insert(IndexKey::from_int(4), 400);
        tree.insert(IndexKey::from_int(5), 500);

        let offsets = tree.lookup_range(Some(&IndexKey::from_int(2)), Some(&IndexKey::from_int(4)));
        assert_eq!(offsets, vec![200, 300, 400]);
    }

    #[test]
    fn test_from_json() {
        assert_eq!(
            IndexKey::from_json(&serde_json::json!(true)),
            Some(IndexKey::Bool(true))
        );
        assert_eq!(
            IndexKey::from_json(&serde_json::json!(42)),
            Some(IndexKey::Int(42))
        );
        assert_eq!(
            IndexKey::from_json(&serde_json::json!("hello")),
            Some(IndexKey::String("hello".to_string()))
        );
        assert_eq!(
            IndexKey::from_json(&serde_json::json!([1, 2, 3])),
            None // Arrays not indexed
        );
    }
}
