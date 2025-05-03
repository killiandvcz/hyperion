//! Indexing system for Hyperion
//!
//! This module provides indexing capabilities to accelerate
//! path-based queries and pattern matching operations.

use crate::path::Path;
use crate::errors::{Result, StoreError};
use sled::{Tree, IVec};
use bincode::{serialize, deserialize};
use std::sync::Arc;

/// Trait defining the interface for path indexes
pub trait PathIndex {
    /// Add a path to the index
    fn add_path(&mut self, path: &Path) -> Result<()>;
    
    /// Remove a path from the index
    fn remove_path(&mut self, path: &Path) -> Result<()>;
    
    /// Find all paths that match a prefix
    fn find_prefix(&self, prefix: &Path) -> Result<Vec<Path>>;
    
    /// Clear the entire index
    fn clear(&mut self) -> Result<()>;
}

/// A persistent prefix index using Sled
pub struct PersistentPrefixIndex {
    /// The Sled tree for storing index data
    tree: Arc<Tree>,
}

impl PersistentPrefixIndex {
    /// Create a new persistent prefix index
    pub fn new(db: &sled::Db) -> Result<Self> {
        let tree = db.open_tree("prefix_index")
            .map_err(|e| StoreError::Internal(format!("Failed to open index tree: {}", e)))?;
        
        Ok(PersistentPrefixIndex {
            tree: Arc::new(tree),
        })
    }
    
    /// Create an index key from a path
    /// 
    /// The format is designed to preserve lexicographical ordering
    /// for efficient prefix searches:
    /// segment_count:[segment1]:[segment2]:...:[segmentN]
    fn create_index_key(path: &Path) -> Result<IVec> {
        let segments = path.segments();
        let segment_count = segments.len();
        
        // Start with segment count as a single byte (limits to 255 segments, which should be enough)
        if segment_count > 255 {
            return Err(StoreError::InvalidOperation(
                "Path has too many segments for indexing".to_string()
            ));
        }
        
        let mut key_parts = Vec::with_capacity(segment_count + 1);
        key_parts.push(segment_count.to_string());
        
        for segment in segments {
            key_parts.push(segment.as_str());
        }
        
        let key = key_parts.join(":");
        Ok(IVec::from(key.as_bytes()))
    }
    
    /// Create a range start for prefix search
    fn create_prefix_start(prefix: &Path) -> Result<IVec> {
        Self::create_index_key(prefix)
    }
    
    /// Create a range end for prefix search
    fn create_prefix_end(prefix: &Path) -> Result<IVec> {
        let segments = prefix.segments();
        let segment_count = segments.len();
        
        let mut key_parts = Vec::with_capacity(segment_count + 1);
        key_parts.push(segment_count.to_string());
        
        for (i, segment) in segments.iter().enumerate() {
            let segment_str = if i == segment_count - 1 {
                // For the last segment, we want the next lexicographical string
                // This gives us a range that includes all paths with this prefix
                format!("{}:", segment.as_str())
            } else {
                segment.as_str()
            };
            key_parts.push(segment_str);
        }
        
        let key = key_parts.join(":");
        Ok(IVec::from(key.as_bytes()))
    }
}

impl PathIndex for PersistentPrefixIndex {
    fn add_path(&mut self, path: &Path) -> Result<()> {
        let key = Self::create_index_key(path)?;
        let path_bytes = serialize(path)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
        
        self.tree.insert(key, path_bytes)
            .map_err(|e| StoreError::Internal(format!("Failed to insert into index: {}", e)))?;
        
        Ok(())
    }
    
    fn remove_path(&mut self, path: &Path) -> Result<()> {
        let key = Self::create_index_key(path)?;
        
        let result = self.tree.remove(key)
            .map_err(|e| StoreError::Internal(format!("Failed to remove from index: {}", e)))?;
        
        if result.is_none() {
            return Err(StoreError::NotFound(path.clone()));
        }
        
        Ok(())
    }
    
    fn find_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        let start = Self::create_prefix_start(prefix)?;
        let end = Self::create_prefix_end(prefix)?;
        
        let mut results = Vec::new();
        
        for item in self.tree.range(start..end) {
            let (_, value_bytes) = item
                .map_err(|e| StoreError::Internal(format!("Failed to iterate index: {}", e)))?;
            
            let path: Path = deserialize(&value_bytes)
                .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
            
            results.push(path);
        }
        
        Ok(results)
    }
    
    fn clear(&mut self) -> Result<()> {
        self.tree.clear()
            .map_err(|e| StoreError::Internal(format!("Failed to clear index: {}", e)))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tempfile::tempdir;
    
    #[test]
    fn test_persistent_prefix_index() {
        // Create a temporary directory for the test database
        let dir = tempdir().unwrap();
        let db_path = dir.path();
        
        // Open a temporary database
        let db = sled::open(db_path).unwrap();
        
        {
            let mut index = PersistentPrefixIndex::new(&db).unwrap();
            
            // Add some paths
            let path1 = Path::from_str("users.u-123456.username").unwrap();
            let path2 = Path::from_str("users.u-123456.email").unwrap();
            let path3 = Path::from_str("users.u-123456.profile.bio").unwrap();
            let path4 = Path::from_str("posts.p-789012.title").unwrap();
            
            index.add_path(&path1).unwrap();
            index.add_path(&path2).unwrap();
            index.add_path(&path3).unwrap();
            index.add_path(&path4).unwrap();
            
            // Test finding by exact prefix
            let user_prefix = Path::from_str("users.u-123456").unwrap();
            let user_results = index.find_prefix(&user_prefix).unwrap();
            
            assert_eq!(user_results.len(), 3);
            assert!(user_results.iter().any(|p| p == &path1));
            assert!(user_results.iter().any(|p| p == &path2));
            assert!(user_results.iter().any(|p| p == &path3));
            
            // Test finding by another prefix
            let posts_prefix = Path::from_str("posts").unwrap();
            let posts_results = index.find_prefix(&posts_prefix).unwrap();
            
            assert_eq!(posts_results.len(), 1);
            assert!(posts_results.iter().any(|p| p == &path4));
            
            // Test removing a path
            index.remove_path(&path1).unwrap();
            
            let user_results_after = index.find_prefix(&user_prefix).unwrap();
            assert_eq!(user_results_after.len(), 2);
            assert!(!user_results_after.iter().any(|p| p == &path1));
        }
        
        // Reopen the database to test persistence
        {
            let db_reopened = sled::open(db_path).unwrap();
            let index = PersistentPrefixIndex::new(&db_reopened).unwrap();
            
            // Check if index was persisted
            let user_prefix = Path::from_str("users.u-123456").unwrap();
            let user_results = index.find_prefix(&user_prefix).unwrap();
            
            // We removed one path, so should have 2 results
            assert_eq!(user_results.len(), 2);
            
            let posts_prefix = Path::from_str("posts").unwrap();
            let posts_results = index.find_prefix(&posts_prefix).unwrap();
            
            assert_eq!(posts_results.len(), 1);
        }
    }
}