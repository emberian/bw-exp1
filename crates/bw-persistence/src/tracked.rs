//! Tracked DashMap with automatic dirty detection.
//!
//! Wraps DashMap to automatically track which entities have been mutated.
//! Uses hash comparison to detect actual changes (not just mutable access).

use std::hash::{Hash, Hasher, DefaultHasher};
use std::ops::{Deref, DerefMut};

use dashmap::{DashMap, DashSet};
use dashmap::mapref::one::{Ref, RefMut};

/// A DashMap that tracks which entries have been modified.
///
/// When an entry is accessed mutably via `get_mut()`, a hash is computed.
/// On drop of the mutable reference, the hash is compared. If different,
/// the key is added to the dirty set.
pub struct TrackedDashMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Hash,
{
    inner: DashMap<K, V>,
    dirty: DashSet<K>,
}

impl<K, V> TrackedDashMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Hash,
{
    /// Create a new empty TrackedDashMap.
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
            dirty: DashSet::new(),
        }
    }

    /// Get a read-only reference to a value.
    ///
    /// Does not mark the entry as dirty.
    pub fn get(&self, key: &K) -> Option<Ref<'_, K, V>> {
        self.inner.get(key)
    }

    /// Get a mutable reference to a value.
    ///
    /// Computes a hash of the value. On drop, if the hash has changed,
    /// the key is marked as dirty.
    pub fn get_mut(&self, key: &K) -> Option<TrackedRefMut<'_, K, V>> {
        self.inner.get_mut(key).map(|guard| {
            let original_hash = compute_hash(&*guard);
            TrackedRefMut {
                guard,
                original_hash,
                key: key.clone(),
                dirty: &self.dirty,
            }
        })
    }

    /// Insert a new key-value pair.
    ///
    /// New inserts are always marked as dirty.
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        self.dirty.insert(key.clone());
        self.inner.insert(key, value)
    }

    /// Remove a key-value pair.
    ///
    /// Removes from dirty set as well.
    pub fn remove(&self, key: &K) -> Option<(K, V)> {
        self.dirty.remove(key);
        self.inner.remove(key)
    }

    /// Check if the map contains a key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Iterate over all entries (read-only).
    pub fn iter(&self) -> dashmap::iter::Iter<'_, K, V> {
        self.inner.iter()
    }

    /// Drain all dirty keys, returning them as a vector.
    ///
    /// Clears the dirty set after draining.
    pub fn drain_dirty(&self) -> Vec<K> {
        let keys: Vec<K> = self.dirty.iter().map(|r| r.clone()).collect();
        self.dirty.clear();
        keys
    }

    /// Check if any entries are dirty.
    pub fn has_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Get the count of dirty entries.
    pub fn dirty_count(&self) -> usize {
        self.dirty.len()
    }
}

impl<K, V> Default for TrackedDashMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A mutable reference that tracks changes via hash comparison.
pub struct TrackedRefMut<'a, K, V>
where
    K: Eq + Hash + Clone,
    V: Hash,
{
    guard: RefMut<'a, K, V>,
    original_hash: u64,
    key: K,
    dirty: &'a DashSet<K>,
}

impl<K, V> Deref for TrackedRefMut<'_, K, V>
where
    K: Eq + Hash + Clone,
    V: Hash,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<K, V> DerefMut for TrackedRefMut<'_, K, V>
where
    K: Eq + Hash + Clone,
    V: Hash,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl<K, V> Drop for TrackedRefMut<'_, K, V>
where
    K: Eq + Hash + Clone,
    V: Hash,
{
    fn drop(&mut self) {
        let new_hash = compute_hash(&*self.guard);
        if new_hash != self.original_hash {
            self.dirty.insert(self.key.clone());
        }
    }
}

/// Compute a hash of any hashable value.
fn compute_hash<T: Hash>(val: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    val.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Hash, PartialEq)]
    struct TestEntity {
        id: u32,
        value: String,
    }

    #[test]
    fn test_insert_marks_dirty() {
        let map: TrackedDashMap<u32, TestEntity> = TrackedDashMap::new();

        map.insert(1, TestEntity { id: 1, value: "hello".to_string() });

        assert!(map.has_dirty());
        assert_eq!(map.dirty_count(), 1);

        let dirty = map.drain_dirty();
        assert_eq!(dirty, vec![1]);
        assert!(!map.has_dirty());
    }

    #[test]
    fn test_get_mut_no_change_not_dirty() {
        let map: TrackedDashMap<u32, TestEntity> = TrackedDashMap::new();
        map.insert(1, TestEntity { id: 1, value: "hello".to_string() });
        map.drain_dirty(); // Clear from insert

        // Get mutable but don't change
        {
            let _guard = map.get_mut(&1).unwrap();
            // Guard drops without modification
        }

        assert!(!map.has_dirty());
    }

    #[test]
    fn test_get_mut_with_change_marks_dirty() {
        let map: TrackedDashMap<u32, TestEntity> = TrackedDashMap::new();
        map.insert(1, TestEntity { id: 1, value: "hello".to_string() });
        map.drain_dirty(); // Clear from insert

        // Get mutable and change
        {
            let mut guard = map.get_mut(&1).unwrap();
            guard.value = "world".to_string();
        }

        assert!(map.has_dirty());
        assert_eq!(map.dirty_count(), 1);
    }

    #[test]
    fn test_read_only_get_not_dirty() {
        let map: TrackedDashMap<u32, TestEntity> = TrackedDashMap::new();
        map.insert(1, TestEntity { id: 1, value: "hello".to_string() });
        map.drain_dirty(); // Clear from insert

        // Read-only access
        {
            let guard = map.get(&1).unwrap();
            assert_eq!(guard.value, "hello");
        }

        assert!(!map.has_dirty());
    }

    #[test]
    fn test_remove_clears_dirty() {
        let map: TrackedDashMap<u32, TestEntity> = TrackedDashMap::new();
        map.insert(1, TestEntity { id: 1, value: "hello".to_string() });

        assert!(map.has_dirty());

        map.remove(&1);

        assert!(!map.has_dirty());
    }
}
