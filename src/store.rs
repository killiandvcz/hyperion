//! In-memory store for NanoDB
//!
//! This module provides a simple in-memory implementation
//! of the database store, mapping paths to values.

use std::collections::HashMap;
use crate::path::Path;
use crate::value::Value;
use crate::errors::{Result, StoreError};

/// An in-memory store for the database
#[derive(Debug, Default)]
pub struct MemoryStore {
    /// Map of paths to values
    data: HashMap<Path, Value>,
}

impl MemoryStore {
    /// Create a new empty memory store
    pub fn new() -> Self {
        MemoryStore {
            data: HashMap::new(),
        }
    }
    
    /// Set a value at the given path
    pub fn set(&mut self, path: Path, value: Value) -> Result<()> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot set value at empty path".to_string()));
        }
        
        self.data.insert(path, value);
        Ok(())
    }
    
    /// Get a value at the given path
    pub fn get(&self, path: &Path) -> Result<Value> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot get value at empty path".to_string()));
        }
        
        self.data.get(path)
        .cloned()
        .ok_or_else(|| StoreError::NotFound(path.clone()))
    }
    
    /// Delete a value at the given path
    pub fn delete(&mut self, path: &Path) -> Result<()> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot delete value at empty path".to_string()));
        }
        
        if self.data.remove(path).is_none() {
            return Err(StoreError::NotFound(path.clone()));
        }
        
        Ok(())
    }
    
    /// Check if a path exists in the store
    pub fn exists(&self, path: &Path) -> bool {
        self.data.contains_key(path)
    }
    
    /// List all paths that start with the given prefix
    pub fn list_prefix(&self, prefix: &Path) -> Vec<&Path> {
        self.data.keys()
        .filter(|p| p.starts_with(prefix))
        .collect()
    }
    
    /// Get all values under a prefix (for entity reconstruction)
    pub fn get_prefix(&self, prefix: &Path) -> HashMap<&Path, &Value> {
        self.data.iter()
        .filter(|(p, _)| p.starts_with(prefix))
        .collect()
    }
    
    /// Count the number of paths in the store
    pub fn count(&self) -> usize {
        self.data.len()
    }
    
    /// Count the number of paths under a prefix
    pub fn count_prefix(&self, prefix: &Path) -> usize {
        self.list_prefix(prefix).len()
    }
    
    /// Query paths that match a pattern (which may contain wildcards)
    // Dans store.rs, modifiez la fonction query et les fonctions associées:
    
    pub fn query<'a>(&'a self, pattern: &'a Path) -> HashMap<&'a Path, &'a Value> {
        if !pattern.has_wildcards() {
            // If there are no wildcards, this is just a simple get
            if let Some(value) = self.data.get(pattern) {
                let mut result = HashMap::new();
                result.insert(pattern, value);
                return result;
            }
            return HashMap::new();
        }
        
        // Find all paths that match the pattern
        self.data.iter()
        .filter(|(path, _)| path.matches(pattern))
        .collect()
    }
    
    pub fn query_values<'a>(&'a self, pattern: &'a Path) -> Vec<&'a Value> {
        self.query(pattern).values().cloned().collect()
    }
    
    pub fn query_paths<'a>(&'a self, pattern: &'a Path) -> Vec<&'a Path> {
        self.query(pattern).keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[test]
    fn test_set_get() {
        let mut store = MemoryStore::new();
        
        let path = Path::from_str("users.u-123456.username").unwrap();
        let value = Value::String("alice".to_string());
        
        store.set(path.clone(), value.clone()).unwrap();
        
        let retrieved = store.get(&path).unwrap();
        assert_eq!(retrieved, value);
    }
    
    #[test]
    fn test_delete() {
        let mut store = MemoryStore::new();
        
        let path = Path::from_str("users.u-123456.username").unwrap();
        let value = Value::String("alice".to_string());
        
        store.set(path.clone(), value).unwrap();
        assert!(store.exists(&path));
        
        store.delete(&path).unwrap();
        assert!(!store.exists(&path));
        
        // Should return NotFound error when trying to get deleted path
        assert!(matches!(store.get(&path), Err(StoreError::NotFound(_))));
    }
    
    #[test]
    fn test_list_prefix() {
        let mut store = MemoryStore::new();
        
        // Add several paths under the same prefix
        store.set(Path::from_str("users.u-123456.username").unwrap(), 
        Value::String("alice".to_string())).unwrap();
        store.set(Path::from_str("users.u-123456.email").unwrap(), 
        Value::String("alice@example.com".to_string())).unwrap();
        store.set(Path::from_str("users.u-123456.profile.bio").unwrap(), 
        Value::String("Software developer".to_string())).unwrap();
        
        // Add a path with a different prefix
        store.set(Path::from_str("posts.p-789012.title").unwrap(), 
        Value::String("Hello World".to_string())).unwrap();
        
        let prefix = Path::from_str("users.u-123456").unwrap();
        let paths = store.list_prefix(&prefix);
        
        assert_eq!(paths.len(), 3);
        assert!(paths.iter().all(|p| p.starts_with(&prefix)));
        
        // Count should match
        assert_eq!(store.count_prefix(&prefix), 3);
        assert_eq!(store.count(), 4);
    }
    
    #[test]
    fn test_query_single_wildcard() {
        let mut store = MemoryStore::new();
        
        // Add several user emails
        store.set(Path::from_str("users.u-123456.email").unwrap(), 
        Value::String("alice@example.com".to_string())).unwrap();
        store.set(Path::from_str("users.u-789012.email").unwrap(), 
        Value::String("bob@example.com".to_string())).unwrap();
        store.set(Path::from_str("users.u-345678.email").unwrap(), 
        Value::String("charlie@example.com".to_string())).unwrap();
        
        // Add some other user data
        store.set(Path::from_str("users.u-123456.username").unwrap(), 
        Value::String("alice".to_string())).unwrap();
        
        // Query all user emails
        let pattern = Path::from_str("users.*.email").unwrap();
        let results = store.query(&pattern);
        
        assert_eq!(results.len(), 3);
        
        // All results should be email values
        for (path, value) in &results {
            assert!(path.to_string().ends_with(".email"));
            assert!(value.is_string());
        }
    }
    
    #[test]
    fn test_query_multi_wildcard() {
        let mut store = MemoryStore::new();
        
        // Add nested bio fields at different levels
        store.set(Path::from_str("users.u-123456.bio").unwrap(), 
        Value::String("Alice's bio".to_string())).unwrap();
        store.set(Path::from_str("users.u-789012.profile.bio").unwrap(), 
        Value::String("Bob's bio".to_string())).unwrap();
        store.set(Path::from_str("users.u-345678.profile.details.bio").unwrap(), 
        Value::String("Charlie's bio".to_string())).unwrap();
        
        // Add some non-bio fields
        store.set(Path::from_str("users.u-123456.email").unwrap(), 
        Value::String("alice@example.com".to_string())).unwrap();
        
        // Query all bio fields at any level
        let pattern = Path::from_str("users.**.bio").unwrap();
        let results = store.query(&pattern);
        
        assert_eq!(results.len(), 3);
        
        // All results should be bio values
        for (path, _) in &results {
            assert!(path.to_string().ends_with(".bio"));
        }
    }
}