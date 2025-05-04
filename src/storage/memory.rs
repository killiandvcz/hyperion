//! In-memory store for Hyperion
//!
//! This module provides a simple in-memory implementation
//! of the database store, mapping paths to values.

use std::any::Any;
use std::collections::HashMap;
use crate::core::path::Path;
use crate::core::value::Value;
use crate::core::errors::{Result, StoreError};
use crate::core::store::Store;

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
}

impl Store for MemoryStore {
    fn set(&mut self, path: Path, value: Value) -> Result<()> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot set value at empty path".to_string()));
        }
        
        self.data.insert(path, value);
        Ok(())
    }
    
    fn get(&self, path: &Path) -> Result<Value> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot get value at empty path".to_string()));
        }
        
        self.data.get(path)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(path.clone()))
    }
    
    fn delete(&mut self, path: &Path) -> Result<()> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot delete value at empty path".to_string()));
        }
        
        if self.data.remove(path).is_none() {
            return Err(StoreError::NotFound(path.clone()));
        }
        
        Ok(())
    }
    
    fn exists(&self, path: &Path) -> Result<bool> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot check empty path".to_string()));
        }
        
        Ok(self.data.contains_key(path))
    }
    
    fn list_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        let paths = self.data.keys()
            .filter(|p| p.starts_with(prefix))
            .cloned()
            .collect();
        
        Ok(paths)
    }
    
    fn get_prefix(&self, prefix: &Path) -> Result<Vec<(Path, Value)>> {
        let pairs = self.data.iter()
            .filter(|(p, _)| p.starts_with(prefix))
            .map(|(p, v)| (p.clone(), v.clone()))
            .collect();
        
        Ok(pairs)
    }
    
    fn query(&self, pattern: &Path) -> Result<Vec<(Path, Value)>> {
        if !pattern.has_wildcards() {
            // If there are no wildcards, this is just a simple get
            if let Ok(value) = self.get(pattern) {
                return Ok(vec![(pattern.clone(), value)]);
            }
            return Ok(Vec::new());
        }
        
        // Find all paths that match the pattern
        let pairs = self.data.iter()
            .filter(|(path, _)| path.matches(pattern))
            .map(|(p, v)| (p.clone(), v.clone()))
            .collect();
        
        Ok(pairs)
    }
    
    fn count(&self) -> Result<usize> {
        Ok(self.data.len())
    }
    
    fn count_prefix(&self, prefix: &Path) -> Result<usize> {
        let count = self.data.keys()
            .filter(|p| p.starts_with(prefix))
            .count();
        
        Ok(count)
    }
    
    fn flush(&self) -> Result<()> {
        // No-op for in-memory store
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}