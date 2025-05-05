// src/storage/persistent.rs

use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use sled::Db;
use bincode::{serialize, deserialize};
use tokio::sync::OnceCell;

use crate::core::path::Path;
use crate::core::value::Value;
use crate::core::errors::{Result, StoreError};
use crate::core::store::Store;
use crate::core::index::{IndexSystem, IndexStats};

/// A persistent store for the database using sled
pub struct PersistentStore {
    /// The underlying sled database
    db: Arc<Db>,
    /// Unified index system
    index_system: IndexSystem,
    /// Statistics (cached to avoid async calls in sync contexts)
    cached_stats: OnceCell<IndexStats>,
}

impl PersistentStore {
    /// Open a persistent store at the given path
    pub async fn open_async<P: Into<PathBuf>>(path: P) -> Result<Self> {
        // Open the sled database
        let db = sled::open(path.into())
            .map_err(|e| StoreError::Internal(format!("Failed to open database: {}", e)))?;
        let db_arc = Arc::new(db);
        
        // Create the index system
        let index_system = IndexSystem::new(Arc::clone(&db_arc))?;
        
        let store = PersistentStore {
            db: db_arc,
            index_system,
            cached_stats: OnceCell::new(),
        };
        
        // Build initial indexes if the database already contains data
        store.rebuild_indexes_async().await?;
        
        Ok(store)
    }
    
    /// Open a persistent store synchronously (for non-async contexts)
    pub fn open<P: Into<PathBuf>>(path: P) -> Result<Self> {
        // Create a temporary runtime for synchronous initialization
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| StoreError::Internal(format!("Failed to create temporary runtime: {}", e)))?;
        
        // Use the runtime to call the async version
        rt.block_on(Self::open_async(path))
    }
    
    /// Rebuild all indexes from scratch
    async fn rebuild_indexes_async(&self) -> Result<()> {
        println!("Rebuilding indexes from existing data...");
        
        // Iterate through all paths in the database and add them to indexes
        for item in self.db.iter() {
            let (key_bytes, _) = item
                .map_err(|e| StoreError::Internal(format!("Failed to iterate database: {}", e)))?;
            
            // Deserialize the path
            let path: Path = deserialize(&key_bytes)
                .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
            
            // Add to indexes asynchronously
            self.index_system.add_path(path).await?;
        }
        
        // Flush the indexes to ensure all operations are complete
        self.index_system.flush().await?;
        println!("Index rebuilding complete.");
        
        Ok(())
    }
    
    /// Get index statistics
    pub async fn index_stats_async(&self) -> Result<IndexStats> {
        // Get combined stats from the index system
        let stats = self.index_system.stats();
        
        // Cache the stats
        let _ = self.cached_stats.set(stats.clone());
        
        Ok(stats)
    }
    
    /// Get index statistics (sync version)
    pub fn index_stats(&self) -> Result<IndexStats> {
        // If we have cached stats, return them
        if let Some(stats) = self.cached_stats.get() {
            return Ok(stats.clone());
        }
        
        // Otherwise, return the current stats
        Ok(self.index_system.stats().clone())
    }
}

impl Store for PersistentStore {
    fn set(&mut self, path: Path, value: Value) -> Result<()> {
        if path.is_empty() {
            return Err(StoreError::InvalidOperation("Cannot set value at empty path".to_string()));
        }
        
        println!("PersistentStore: Setting value at path: {:?}", path);
        
        // Serialize the path and value
        let path_bytes = serialize(&path)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
        
        let value_bytes = serialize(&value)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
        
        // Store in the database
        self.db.insert(path_bytes, value_bytes)
            .map_err(|e| StoreError::Internal(format!("Failed to insert data: {}", e)))?;
        
        // Flush to ensure data is persisted
        self.db.flush()
            .map_err(|e| StoreError::Internal(format!("Failed to flush database: {}", e)))?;
        
        // Update indexes asynchronously
        let path_clone = path.clone();
        let value_clone = value.clone();
        let index_system = self.index_system.clone();
        
        // Spawn a task to handle indexing
        tokio::spawn(async move {
            if let Err(e) = index_system.add_path_with_value(path_clone, value_clone).await {
                println!("Error updating value index: {:?}", e);
            }
        });
        
        Ok(())
    }
    
    fn get(&self, path: &Path) -> Result<Value> {
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
    
    fn delete(&mut self, path: &Path) -> Result<()> {
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
        
        // Flush to ensure data removal is persisted
        self.db.flush()
            .map_err(|e| StoreError::Internal(format!("Failed to flush database: {}", e)))?;
        
        // Update indexes asynchronously
        let path_clone = path.clone();
        let index_system = self.index_system.clone();
        
        // Spawn a task to handle index removal
        tokio::spawn(async move {
            if let Err(e) = index_system.remove_path(path_clone).await {
                println!("Error removing from index: {:?}", e);
            }
        });
        
        Ok(())
    }

    fn exists(&self, path: &Path) -> Result<bool> {
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
    
    fn list_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        println!("PersistentStore: Listing paths with prefix: {:?}", prefix);
        
        // Use the index system to find paths by prefix
        let paths = self.index_system.find_by_prefix(prefix)?;
        
        println!("PersistentStore: Found {} paths with prefix", paths.len());
        Ok(paths)
    }
    
    fn get_prefix(&self, prefix: &Path) -> Result<Vec<(Path, Value)>> {
        println!("PersistentStore: Getting all values with prefix: {:?}", prefix);
        
        let mut results = Vec::new();
        
        // Find all paths with this prefix
        let paths = self.list_prefix(prefix)?;
        
        // Get the value for each path
        for path in paths {
            if let Ok(value) = self.get(&path) {
                results.push((path, value));
            }
        }
        
        println!("PersistentStore: Found {} value pairs", results.len());
        Ok(results)
    }
    
    fn query(&self, pattern: &Path) -> Result<Vec<(Path, Value)>> {
        println!("PersistentStore: Querying with pattern: {:?}", pattern);
        
        let mut results = Vec::new();
        
        // If there are no wildcards, we can do a simple get
        if !pattern.has_wildcards() {
            if let Ok(value) = self.get(pattern) {
                results.push((pattern.clone(), value));
            }
            return Ok(results);
        }
        
        // Use the index system to find matching paths
        let matching_paths = self.index_system.find_by_pattern(pattern)?;
        
        // Get the values for all matching paths
        for path in matching_paths {
            if let Ok(value) = self.get(&path) {
                results.push((path, value));
            }
        }
        
        println!("PersistentStore: Found {} matches", results.len());
        Ok(results)
    }

    fn count(&self) -> Result<usize> {
        let count = self.db.len();
        Ok(count as usize)
    }
    
    fn count_prefix(&self, prefix: &Path) -> Result<usize> {
        let paths = self.list_prefix(prefix)?;
        Ok(paths.len())
    }

    fn flush(&self) -> Result<()> {
        // Flush database to disk
        self.db.flush()
            .map_err(|e| StoreError::Internal(format!("Failed to flush database: {}", e)))?;
        
        // Flush indexes (non-blocking)
        let index_system = self.index_system.clone();
        tokio::spawn(async move {
            let _ = index_system.flush().await;
        });
        
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for PersistentStore {
    fn drop(&mut self) {
        // Shutdown index system (non-blocking)
        let index_system = self.index_system.clone();
        tokio::spawn(async move {
            let _ = index_system.shutdown().await;
        });
    }
}