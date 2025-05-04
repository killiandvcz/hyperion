//! Hyperion: An endpoint-first database
//!
//! This crate provides a database system that treats endpoints
//! (path-value pairs) as the fundamental unit of storage.

// Exporter les modules
pub mod core;
pub mod storage;
// Module temporaires - à déplacer dans core ou à supprimer
pub mod ql;

use std::path::{Path as StdPath, PathBuf};
use tokio::runtime::Runtime;
use core::store::Store;
use core::entity::reconstruct_entity;
use storage::{MemoryStore, PersistentStore};

/// Main API for Hyperion database
pub struct Hyperion {
    store: Box<dyn Store>,
    runtime: Option<Runtime>,
}

impl Hyperion {
    /// Create a new in-memory database instance
    pub fn new_in_memory() -> Self {
        Hyperion {
            store: Box::new(MemoryStore::new()),
            runtime: None,
        }
    }
    
    /// Create a new persistent database instance at the given path
    pub fn new_persistent<P: AsRef<StdPath>>(path: P) -> Result<Self> {
        let persistent_store = PersistentStore::open(PathBuf::from(path.as_ref()))?;
        Ok(Hyperion {
            store: Box::new(persistent_store),
            runtime: None,
        })
    }
    
    /// Get a value at the given path
    pub fn get(&self, path: &Path) -> Result<Value> {
        self.store.get(path)
    }
    
    /// Set a value at the given path
    pub fn set(&mut self, path: Path, value: Value) -> Result<()> {
        self.store.set(path, value)
    }
    
    /// Delete a value at the given path
    pub fn delete(&mut self, path: &Path) -> Result<()> {
        self.store.delete(path)
    }
    
    /// Check if a path exists
    pub fn exists(&self, path: &Path) -> Result<bool> {
        self.store.exists(path)
    }
    
    /// List all paths with the given prefix
    pub fn list_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        self.store.list_prefix(prefix)
    }
    
    /// Query paths that match a pattern
    pub fn query(&self, pattern: &Path) -> Result<Vec<(Path, Value)>> {
        self.store.query(pattern)
    }
    
    /// Reconstruct an entity from its endpoints
    pub fn get_entity(&self, prefix: &Path) -> Result<Entity> {
        reconstruct_entity(&*self.store, prefix)
    }
    
    /// Flush changes to disk (no-op for in-memory store)
    pub fn flush(&self) -> Result<()> {
        self.store.flush()
    }
    
    /// Count the number of paths in the database
    pub fn count(&self) -> Result<usize> {
        self.store.count()
    }
    
    /// Count the number of paths with the given prefix
    pub fn count_prefix(&self, prefix: &Path) -> Result<usize> {
        self.store.count_prefix(prefix)
    }
    
    /// Get index statistics (only available for persistent store)
    pub fn index_stats(&self) -> Result<Option<IndexStats>> {
        // Try to downcast to PersistentStore to access store-specific methods
        if let Some(persistent) = self.store.as_any().downcast_ref::<PersistentStore>() {
            Ok(Some(persistent.index_stats()?))
        } else {
            Ok(None)
        }
    }
}

// Ré-exporter les types principaux pour faciliter l'utilisation
pub use core::path::Path;
pub use core::value::Value;
pub use core::entity::Entity;
pub use core::errors::{Result, StoreError};
pub use core::index::IndexStats;