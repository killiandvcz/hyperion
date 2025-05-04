// src/storage/persistent.rs - Modification complète

use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;
use sled::Db;
use bincode::{serialize, deserialize};
use tokio::sync::OnceCell;

use crate::core::path::Path;
use crate::core::value::Value;
use crate::core::errors::{Result, StoreError};
use crate::core::store::Store;
use crate::core::index::{Index, PrefixIndex, WildcardIndex, IndexStats};

/// A persistent store for the database using sled
pub struct PersistentStore {
    /// The underlying sled database
    db: Arc<Db>,
    /// Index for prefix operations
    prefix_index: Index,
    /// Index for wildcard operations
    wildcard_index: Index,
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
        
        // Create the indexes
        let prefix_impl = PrefixIndex::new(Arc::clone(&db_arc), "prefix_index")?;
        let mut prefix_index = Index::new(prefix_impl);
        prefix_index.start_worker()?;
        
        let wildcard_impl = WildcardIndex::new(Arc::clone(&db_arc), "wildcard_index")?;
        let mut wildcard_index = Index::new(wildcard_impl);
        wildcard_index.start_worker()?;
        
        let store = PersistentStore {
            db: db_arc,
            prefix_index,
            wildcard_index,
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
        // Clear the indexes
        self.prefix_index.clear().await?;
        self.wildcard_index.clear().await?;
        
        // Iterate through all paths and add them to indexes
        for item in self.db.iter() {
            let (key_bytes, _) = item
                .map_err(|e| StoreError::Internal(format!("Failed to iterate database: {}", e)))?;
            
            // Deserialize the path
            let path: Path = deserialize(&key_bytes)
                .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
            
            // Add to indexes asynchronously
            self.prefix_index.update(path.clone(), Some(())).await?;
            self.wildcard_index.update(path, Some(())).await?;
        }
        
        // Flush the indexes to ensure all operations are complete
        self.prefix_index.flush().await?;
        self.wildcard_index.flush().await?;
        
        Ok(())
    }
    
    /// Get index statistics
    pub async fn index_stats_async(&self) -> Result<IndexStats> {
        // Get stats from both indexes
        let prefix_stats = self.prefix_index.stats();
        let wildcard_stats = self.wildcard_index.stats();
        
        // Combine stats
        let combined_stats = IndexStats {
            total_operations: prefix_stats.total_operations + wildcard_stats.total_operations,
            total_adds: prefix_stats.total_adds + wildcard_stats.total_adds,
            total_removes: prefix_stats.total_removes + wildcard_stats.total_removes,
            pending_operations: prefix_stats.pending_operations + wildcard_stats.pending_operations,
        };
        
        // Cache the stats
        let _ = self.cached_stats.set(combined_stats.clone());
        
        Ok(combined_stats)
    }
    
    /// Get index statistics (sync version)
    pub fn index_stats(&self) -> Result<IndexStats> {
        // If we have cached stats, return them
        if let Some(stats) = self.cached_stats.get() {
            return Ok(stats.clone());
        }
        
        // Otherwise, just return the current stats without async calls
        let prefix_stats = self.prefix_index.stats();
        let wildcard_stats = self.wildcard_index.stats();
        
        let combined_stats = IndexStats {
            total_operations: prefix_stats.total_operations + wildcard_stats.total_operations,
            total_adds: prefix_stats.total_adds + wildcard_stats.total_adds,
            total_removes: prefix_stats.total_removes + wildcard_stats.total_removes,
            pending_operations: prefix_stats.pending_operations + wildcard_stats.pending_operations,
        };
        
        Ok(combined_stats)
    }
}

// Implémentation des méthodes Store pour PersistentStore
impl Store for PersistentStore {
    // Toutes les méthodes synchrones qui utilisaient runtime.block_on() doivent être
    // réécrites pour être purement synchrones ou utiliser des fonctions/méthodes auxiliaires

    fn set(&mut self, path: Path, value: Value) -> Result<()> {
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
        
        // Update indexes (non-blocking, fire and forget)
        let _ = self.prefix_index.update(path.clone(), Some(()));
        let _ = self.wildcard_index.update(path, Some(()));
        
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
        
        // Update indexes (non-blocking, fire and forget)
        let _ = self.prefix_index.update(path.clone(), None);
        let _ = self.wildcard_index.update(path.clone(), None);
        
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
        // Utiliser l'index synchrone sans block_on
        self.prefix_index.find_prefix(prefix)
    }
    
    fn get_prefix(&self, prefix: &Path) -> Result<Vec<(Path, Value)>> {
        let mut results = Vec::new();
        
        // Find all paths with this prefix
        let paths = self.list_prefix(prefix)?;
        
        // Get the value for each path
        for path in paths {
            if let Ok(value) = self.get(&path) {
                results.push((path, value));
            }
        }
        
        Ok(results)
    }
    
    fn query(&self, pattern: &Path) -> Result<Vec<(Path, Value)>> {
        let mut results = Vec::new();
        
        // If there are no wildcards, we can do a simple get
        if !pattern.has_wildcards() {
            if let Ok(value) = self.get(pattern) {
                results.push((pattern.clone(), value));
            }
            return Ok(results);
        }
        
        // Use the wildcard index without block_on
        let matching_paths = self.wildcard_index.find_matches(pattern)?;
        
        // Get the values for all matching paths
        for path in matching_paths {
            if let Ok(value) = self.get(&path) {
                results.push((path, value));
            }
        }
        
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
        
        // Flush indexes without block_on (non-blocking)
        let _ = self.prefix_index.flush();
        let _ = self.wildcard_index.flush();
        
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for PersistentStore {
    fn drop(&mut self) {
        // Shutdown indexes (non-blocking)
        let _ = self.prefix_index.shutdown();
        let _ = self.wildcard_index.shutdown();
    }
}