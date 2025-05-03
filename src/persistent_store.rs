//! Persistent store for NanoDB using sled
//!
//! This module provides a persistent implementation
//! of the database store, storing path-value pairs on disk.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use sled::{Db, IVec};
use bincode::{serialize, deserialize};


use crate::path::Path;
use crate::value::Value;
use crate::errors::{Result, StoreError};
use crate::index::{PathIndex, PersistentPrefixIndex};

/// A persistent store for the database using sled
pub struct PersistentStore {
    /// The underlying sled database
    db: Arc<Db>,
    /// Index for prefix searches
    prefix_index: Arc<RwLock<PersistentPrefixIndex>>,
}

impl PersistentStore {
    /// Open a persistent store at the given path
    pub fn open<P: Into<PathBuf>>(path: P) -> Result<Self> {
        let db = sled::open(path.into())
        .map_err(|e| StoreError::Internal(format!("Failed to open database: {}", e)))?;
        
        // Create and load the persistent index
        let prefix_index = PersistentPrefixIndex::new(&db)?;
        
        Ok(PersistentStore {
            db: Arc::new(db),
            prefix_index: Arc::new(RwLock::new(prefix_index)),
        })
    }
    
    /// Rebuild the index from scratch using all paths in the database
    fn rebuild_index(&self) -> Result<()> {
        let mut index = self.prefix_index.write().unwrap();
        index.clear()?;
        
        // Iterate through all keys and add them to the index
        for item in self.db.iter() {
            let (key_bytes, _) = item
            .map_err(|e| StoreError::Internal(format!("Failed to iterate database: {}", e)))?;
            
            // Deserialize the path
            let path: Path = deserialize(&key_bytes)
            .map_err(|e| StoreError::Internal(format!("Failed to deserialize path: {}", e)))?;
            
            // Add to index
            index.add_path(&path)?;
        }
        
        Ok(())
    }
    
    /// Set a value at the given path
    pub fn set(&self, path: Path, value: Value) -> Result<()> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot set value at empty path".to_string()));
        }
        
        // Serialize the path and value
        let path_bytes = serialize(&path)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
        
        let value_bytes = serialize(&value)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
        
        // Store in the database
        self.db.insert(path_bytes, value_bytes)
            .map_err(|e| StoreError::Internal(format!("Failed to insert data: {}", e)))?;
        
        // Update the index
        let mut index = self.prefix_index.write().unwrap();
        index.add_path(&path)?;
        
        Ok(())
    }
    
    /// Get a value at the given path
    pub fn get(&self, path: &Path) -> Result<Value> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot get value at empty path".to_string()));
        }
        
        // Serialize the path to use as key
        let path_bytes = serialize(path)
        .map_err(|e| StoreError::Internal(format!("Failed to serialize path: {}", e)))?;
        
        // Retrieve from the database
        let value_bytes = self.db.get(path_bytes)
        .map_err(|e| StoreError::Internal(format!("Failed to retrieve data: {}", e)))?
        .ok_or_else(|| StoreError::NotFound(path.clone()))?;
        
        // Deserialize the value
        let value: Value = deserialize(&value_bytes)
        .map_err(|e| StoreError::Internal(format!("Failed to deserialize value: {}", e)))?;
        
        Ok(value)
    }
    
    /// Delete a value at the given path
    pub fn delete(&self, path: &Path) -> Result<()> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot delete value at empty path".to_string()));
        }
        
        // Serialize the path to use as key
        let path_bytes = serialize(path)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
        
        // Remove from the database
        let result = self.db.remove(path_bytes)
            .map_err(|e| StoreError::Internal(format!("Failed to delete data: {}", e)))?;
        
        if result.is_none() {
            return Err(StoreError::NotFound(path.clone()));
        }
        
        // Update the index
        let mut index = self.prefix_index.write().unwrap();
        index.remove_path(path)?;
        
        Ok(())
    }
    
    /// Check if a path exists in the store
    pub fn exists(&self, path: &Path) -> Result<bool> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot check empty path".to_string()));
        }
        
        // Serialize the path to use as key
        let path_bytes = serialize(path)
        .map_err(|e| StoreError::Internal(format!("Failed to serialize path: {}", e)))?;
        
        // Check if the key exists
        let result = self.db.contains_key(path_bytes)
        .map_err(|e| StoreError::Internal(format!("Failed to check key: {}", e)))?;
        
        Ok(result)
    }
    
    /// List all paths that start with the given prefix
    pub fn list_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        // Use the persistent index for prefix lookup
        let index = self.prefix_index.read().unwrap();
        index.find_prefix(prefix)
    }
    
    /// Get all values under a prefix (for entity reconstruction)
    pub fn get_prefix(&self, prefix: &Path) -> Result<Vec<(Path, Value)>> {
        let mut results = Vec::new();
        
        // Iterate through all items in the database
        for item in self.db.iter() {
            let (key_bytes, value_bytes) = item
            .map_err(|e| StoreError::Internal(format!("Failed to iterate database: {}", e)))?;
            
            // Deserialize the path
            let path: Path = deserialize(&key_bytes)
            .map_err(|e| StoreError::Internal(format!("Failed to deserialize path: {}", e)))?;
            
            // Check if it starts with the prefix
            if path.starts_with(prefix) {
                // Deserialize the value
                let value: Value = deserialize(&value_bytes)
                .map_err(|e| StoreError::Internal(format!("Failed to deserialize value: {}", e)))?;
                
                results.push((path, value));
            }
        }
        
        Ok(results)
    }
    
    /// Query paths that match a pattern (which may contain wildcards)
    pub fn query(&self, pattern: &Path) -> Result<Vec<(Path, Value)>> {
        let mut results = Vec::new();
        
        // If there are no wildcards, we can do a simple get
        if !pattern.has_wildcards() {
            if let Ok(value) = self.get(pattern) {
                results.push((pattern.clone(), value));
            }
            return Ok(results);
        }
        
        // With wildcards, we need to scan all paths
        for item in self.db.iter() {
            let (key_bytes, value_bytes) = item
            .map_err(|e| StoreError::Internal(format!("Failed to iterate database: {}", e)))?;
            
            // Deserialize the path
            let path: Path = deserialize(&key_bytes)
            .map_err(|e| StoreError::Internal(format!("Failed to deserialize path: {}", e)))?;
            
            // Check if it matches the pattern
            if path.matches(pattern) {
                // Deserialize the value
                let value: Value = deserialize(&value_bytes)
                .map_err(|e| StoreError::Internal(format!("Failed to deserialize value: {}", e)))?;
                
                results.push((path, value));
            }
        }
        
        Ok(results)
    }
    
    /// Count the number of paths in the store
    pub fn count(&self) -> Result<usize> {
        let count = self.db.len();
        
        Ok(count as usize)
    }
    
    /// Count the number of paths under a prefix
    pub fn count_prefix(&self, prefix: &Path) -> Result<usize> {
        let paths = self.list_prefix(prefix)?;
        Ok(paths.len())
    }
    
    /// Flush changes to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush()
        .map_err(|e| StoreError::Internal(format!("Failed to flush database: {}", e)))?;
        
        Ok(())
    }
}