//! Persistent store for NanoDB using sled
//!
//! This module provides a persistent implementation
//! of the database store, storing path-value pairs on disk.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use sled::Db;
use bincode::{serialize, deserialize};


use crate::path::Path;
use crate::value::Value;
use crate::errors::{Result, StoreError};
use crate::index::{PathIndex, PersistentPrefixIndex};
use crate::wildcard_index::WildcardIndex;
use crate::index_batcher::{IndexBatcher, BatcherConfig, BatcherStats};

/// A persistent store for the database using sled
pub struct PersistentStore {
    /// The underlying sled database
    db: Arc<Db>,
    /// Index for prefix searches
    prefix_index: Arc<RwLock<PersistentPrefixIndex>>,
    /// Batcher for prefix index operations
    prefix_batcher: Arc<Mutex<IndexBatcher<PersistentPrefixIndex, RwLock<PersistentPrefixIndex>>>>,
    /// Index for wildcard searches
    wildcard_index: Arc<RwLock<WildcardIndex>>,
    /// Batcher for wildcard index operations  
    wildcard_batcher: Arc<Mutex<IndexBatcher<WildcardIndex, RwLock<WildcardIndex>>>>,
    /// Batcher configuration
    batcher_config: BatcherConfig,
}

impl PersistentStore {
    /// Open a persistent store at the given path
    pub fn open<P: Into<PathBuf>>(path: P) -> Result<Self> {
        let db = sled::open(path.into())
            .map_err(|e| StoreError::Internal(format!("Failed to open database: {}", e)))?;
        
        // Create the prefix index
        let prefix_index = Arc::new(RwLock::new(PersistentPrefixIndex::new(&db)?));
        
        // Create the wildcard index
        let wildcard_index = Arc::new(RwLock::new(WildcardIndex::new(&db)?));
        
        // Create default batcher configuration
        let batcher_config = BatcherConfig::default();
        
        // Create batchers
        let prefix_batcher = Arc::new(Mutex::new(
            IndexBatcher::new_rwlock(Arc::clone(&prefix_index), batcher_config.clone())
        ));
        
        let wildcard_batcher = Arc::new(Mutex::new(
            IndexBatcher::new_rwlock(Arc::clone(&wildcard_index), batcher_config.clone())
        ));
        
        let store = PersistentStore {
            db: Arc::new(db),
            prefix_index,
            prefix_batcher,
            wildcard_index,
            wildcard_batcher,
            batcher_config,
        };
        
        // Build initial indexes if the database already contains data
        store.rebuild_indexes()?;
        
        Ok(store)
    }
    /// Rebuild all indexes from scratch
    fn rebuild_indexes(&self) -> Result<()> {
        // Clear indexes
        {
            let mut prefix_idx = self.prefix_index.write().unwrap();
            prefix_idx.clear()?;
            
            let mut wildcard_idx = self.wildcard_index.write().unwrap();
            wildcard_idx.clear()?;
        }
        
        // Iterate through all paths and add them to indexes
        for item in self.db.iter() {
            let (key_bytes, _) = item
                .map_err(|e| StoreError::Internal(format!("Failed to iterate database: {}", e)))?;
            
            // Deserialize the path
            let path: Path = deserialize(&key_bytes)
                .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
            
            // Add to indexes
            {
                let mut prefix_idx = self.prefix_index.write().unwrap();
                prefix_idx.add_path(&path)?;
                
                let mut wildcard_idx = self.wildcard_index.write().unwrap();
                wildcard_idx.add_path(&path)?;
            }
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
        
        // Update indexes using batchers
        {
            let mut prefix_batcher = self.prefix_batcher.lock().unwrap();
            prefix_batcher.batch_add(path.clone())?;
            
            let mut wildcard_batcher = self.wildcard_batcher.lock().unwrap();
            wildcard_batcher.batch_add(path)?;
        }
        
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
        
        // Update indexes using batchers
        {
            let mut prefix_batcher = self.prefix_batcher.lock().unwrap();
            prefix_batcher.batch_remove(path.clone())?;
            
            let mut wildcard_batcher = self.wildcard_batcher.lock().unwrap();
            wildcard_batcher.batch_remove(path.clone())?;
        }
        
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
        
        // Use the wildcard index to find matching paths
        let matching_paths = {
            let wildcard_idx = self.wildcard_index.read().unwrap();
            wildcard_idx.find_matches(pattern)?
        };
        
        // Get the values for all matching paths
        for path in matching_paths {
            if let Ok(value) = self.get(&path) {
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
        // Flush database to disk
        self.db.flush()
            .map_err(|e| StoreError::Internal(format!("Failed to flush database: {}", e)))?;
        
        // Flush index batchers
        {
            let mut prefix_batcher = self.prefix_batcher.lock().unwrap();
            prefix_batcher.flush()?;
            
            let mut wildcard_batcher = self.wildcard_batcher.lock().unwrap();
            wildcard_batcher.flush()?;
        }
        
        Ok(())
    }

    pub fn batcher_stats(&self) -> Result<(BatcherStats, BatcherStats)> {
        let prefix_stats = {
            let batcher = self.prefix_batcher.lock().unwrap();
            batcher.stats()
        };
        
        let wildcard_stats = {
            let batcher = self.wildcard_batcher.lock().unwrap();
            batcher.stats()
        };
        
        Ok((prefix_stats, wildcard_stats))
    }

    pub fn configure_batcher(&mut self, config: BatcherConfig) -> Result<()> {
        self.batcher_config = config.clone();
        
        // Flush existing batchers before reconfiguring
        {
            let mut prefix_batcher = self.prefix_batcher.lock().unwrap();
            prefix_batcher.flush()?;
            
            let mut wildcard_batcher = self.wildcard_batcher.lock().unwrap();
            wildcard_batcher.flush()?;
        }
        
        // Create new batchers with the updated configuration
        let prefix_batcher = Arc::new(Mutex::new(
            IndexBatcher::new_rwlock(Arc::clone(&self.prefix_index), config.clone())
        ));
        
        let wildcard_batcher = Arc::new(Mutex::new(
            IndexBatcher::new_rwlock(Arc::clone(&self.wildcard_index), config)

        ));
        
        self.prefix_batcher = prefix_batcher;
        self.wildcard_batcher = wildcard_batcher;
        
        Ok(())
    }
}