//! Wildcard index for Hyperion
//!
//! This module provides specialized indexing capabilities for wildcard
//! pattern matching, significantly optimizing queries with * and ** patterns.

use std::collections::{HashMap, HashSet, BTreeMap};
use std::sync::{Arc, RwLock};
use sled::{Db, Tree};
use bincode::{serialize, deserialize};

use crate::path::Path;
use crate::errors::{Result, StoreError};

/// A specialized index for optimizing wildcard queries
pub struct WildcardIndex {
    /// Sled tree for single-level wildcard patterns
    single_wildcard_tree: Arc<Tree>,
    /// Sled tree for multi-level wildcard patterns 
    multi_wildcard_tree: Arc<Tree>,
    
    /// In-memory cache for frequently accessed patterns
    pattern_cache: RwLock<HashMap<String, HashSet<Path>>>,
}

/// A structural pattern for indexing
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
struct StructuralPattern {
    /// Total number of segments
    segment_count: usize,
    /// Positions of non-wildcard segments with their values
    fixed_segments: BTreeMap<usize, String>,
}

impl WildcardIndex {
    /// Create a new wildcard index
    pub fn new(db: &Db) -> Result<Self> {
        let single_tree = db.open_tree("wildcard_single")
            .map_err(|e| StoreError::Internal(format!("Failed to open single wildcard tree: {}", e)))?;
            
        let multi_tree = db.open_tree("wildcard_multi")
            .map_err(|e| StoreError::Internal(format!("Failed to open multi wildcard tree: {}", e)))?;
        
        Ok(WildcardIndex {
            single_wildcard_tree: Arc::new(single_tree),
            multi_wildcard_tree: Arc::new(multi_tree),
            pattern_cache: RwLock::new(HashMap::new()),
        })
    }
    
    /// Add a path to the index
    pub fn add_path(&mut self, path: &Path) -> Result<()> {
        // Index for single-level wildcards
        self.index_for_single_wildcards(path)?;
        
        // Index for multi-level wildcards
        self.index_for_multi_wildcards(path)?;
        
        // Clear cache since the index has changed
        let mut cache = self.pattern_cache.write().unwrap();
        cache.clear();
        
        Ok(())
    }
    
    /// Remove a path from the index
    pub fn remove_path(&mut self, path: &Path) -> Result<()> {
        // Remove from single-level wildcard index
        self.remove_from_single_wildcards(path)?;
        
        // Remove from multi-level wildcard index
        self.remove_from_multi_wildcards(path)?;
        
        // Clear cache since the index has changed
        let mut cache = self.pattern_cache.write().unwrap();
        cache.clear();
        
        Ok(())
    }
    
    /// Find all paths that match the given wildcard pattern
    pub fn find_matches(&self, pattern: &Path) -> Result<Vec<Path>> {
        // Check cache first
        {
            let cache = self.pattern_cache.read().unwrap();
            if let Some(paths) = cache.get(&pattern.to_string()) {
                return Ok(paths.iter().cloned().collect());
            }
        }
        
        let mut results = HashSet::new();
        
        // Handle single-wildcard patterns
        if self.is_single_wildcard_pattern(pattern) {
            let single_matches = self.find_single_wildcard_matches(pattern)?;
            results.extend(single_matches);
        }
        
        // Handle multi-wildcard patterns
        if self.is_multi_wildcard_pattern(pattern) {
            let multi_matches = self.find_multi_wildcard_matches(pattern)?;
            results.extend(multi_matches);
        }
        
        // If pattern has no wildcards, check if the path exists directly
        if !self.is_single_wildcard_pattern(pattern) && !self.is_multi_wildcard_pattern(pattern) {
            // Just add the pattern itself if it exists
            results.insert(pattern.clone());
        }
        
        // Cache the results for future queries
        {
            let mut cache = self.pattern_cache.write().unwrap();
            cache.insert(pattern.to_string(), results.clone());
        }
        
        Ok(results.into_iter().collect())
    }
    
    /// Clear the entire index
    pub fn clear(&mut self) -> Result<()> {
        self.single_wildcard_tree.clear()
            .map_err(|e| StoreError::Internal(format!("Failed to clear single wildcard tree: {}", e)))?;
        
        self.multi_wildcard_tree.clear()
            .map_err(|e| StoreError::Internal(format!("Failed to clear multi wildcard tree: {}", e)))?;
        
        let mut cache = self.pattern_cache.write().unwrap();
        cache.clear();
        
        Ok(())
    }
    
    /// Index a path for single-level wildcard queries
    fn index_for_single_wildcards(&self, path: &Path) -> Result<()> {
        let segments = path.segments();
        let segment_count = segments.len();
        
        // Generate all possible single-wildcard patterns for this path
        for wildcard_pos in 0..segment_count {
            // Create a structural pattern with one wildcard
            let mut fixed_segments = HashMap::new();
            for (i, segment) in segments.iter().enumerate() {
                if i != wildcard_pos {
                    fixed_segments.insert(i, segment.as_str());
                }
            }
            
            let pattern = StructuralPattern {
                segment_count,
                fixed_segments: fixed_segments.into_iter().collect(),
            };
            
            // Serialize the pattern as key
            let key = serialize(&pattern)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                
            // Get existing paths for this pattern
            let mut paths = if let Some(data) = self.single_wildcard_tree.get(&key)
                .map_err(|e| StoreError::Internal(format!("Failed to read from index: {}", e)))? {
                deserialize::<HashSet<Path>>(&data)
                    .map_err(|e| StoreError::DeserializationError(e.to_string()))?
            } else {
                HashSet::new()
            };
            
            // Add this path to the set
            paths.insert(path.clone());
            
            // Store the updated set
            let value = serialize(&paths)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                
            self.single_wildcard_tree.insert(&key, value)
                .map_err(|e| StoreError::Internal(format!("Failed to update index: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Index a path for multi-level wildcard queries
    fn index_for_multi_wildcards(&self, path: &Path) -> Result<()> {
        let segments = path.segments();
        
        // For each suffix of the path, add an entry to the multi_wildcard_tree
        for start_pos in 0..segments.len() {
            // Create a key based on the suffix segments
            let suffix: Vec<String> = segments[start_pos..]
                .iter()
                .map(|s| s.as_str())
                .collect();
                
            let key = serialize(&suffix)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                
            // Get existing paths for this suffix
            let mut paths = if let Some(data) = self.multi_wildcard_tree.get(&key)
                .map_err(|e| StoreError::Internal(format!("Failed to read from index: {}", e)))? {
                deserialize::<HashSet<Path>>(&data)
                    .map_err(|e| StoreError::DeserializationError(e.to_string()))?
            } else {
                HashSet::new()
            };
            
            // Add this path to the set
            paths.insert(path.clone());
            
            // Store the updated set
            let value = serialize(&paths)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                
            self.multi_wildcard_tree.insert(&key, value)
                .map_err(|e| StoreError::Internal(format!("Failed to update index: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Remove a path from the single-level wildcard index
    fn remove_from_single_wildcards(&self, path: &Path) -> Result<()> {
        let segments = path.segments();
        let segment_count = segments.len();
        
        // Remove from all possible single-wildcard patterns for this path
        for wildcard_pos in 0..segment_count {
            // Create a structural pattern with one wildcard
            let mut fixed_segments = HashMap::new();
            for (i, segment) in segments.iter().enumerate() {
                if i != wildcard_pos {
                    fixed_segments.insert(i, segment.as_str());
                }
            }
            
            let pattern = StructuralPattern {
                segment_count,
                fixed_segments: fixed_segments.into_iter().collect(),
            };
            
            // Serialize the pattern as key
            let key = serialize(&pattern)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                
            // Get existing paths for this pattern
            if let Some(data) = self.single_wildcard_tree.get(&key)
                .map_err(|e| StoreError::Internal(format!("Failed to read from index: {}", e)))? {
                let mut paths = deserialize::<HashSet<Path>>(&data)
                    .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                
                // Remove this path from the set
                paths.remove(path);
                
                if paths.is_empty() {
                    // If no paths left, remove the entry
                    self.single_wildcard_tree.remove(&key)
                        .map_err(|e| StoreError::Internal(format!("Failed to remove from index: {}", e)))?;
                } else {
                    // Store the updated set
                    let value = serialize(&paths)
                        .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                        
                    self.single_wildcard_tree.insert(&key, value)
                        .map_err(|e| StoreError::Internal(format!("Failed to update index: {}", e)))?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Remove a path from the multi-level wildcard index
    fn remove_from_multi_wildcards(&self, path: &Path) -> Result<()> {
        let segments = path.segments();
        
        // Remove for each suffix of the path
        for start_pos in 0..segments.len() {
            // Create a key based on the suffix segments
            let suffix: Vec<String> = segments[start_pos..]
                .iter()
                .map(|s| s.as_str())
                .collect();
                
            let key = serialize(&suffix)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                
            // Get existing paths for this suffix
            if let Some(data) = self.multi_wildcard_tree.get(&key)
                .map_err(|e| StoreError::Internal(format!("Failed to read from index: {}", e)))? {
                let mut paths = deserialize::<HashSet<Path>>(&data)
                    .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                
                // Remove this path from the set
                paths.remove(path);
                
                if paths.is_empty() {
                    // If no paths left, remove the entry
                    self.multi_wildcard_tree.remove(&key)
                        .map_err(|e| StoreError::Internal(format!("Failed to remove from index: {}", e)))?;
                } else {
                    // Store the updated set
                    let value = serialize(&paths)
                        .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                        
                    self.multi_wildcard_tree.insert(&key, value)
                        .map_err(|e| StoreError::Internal(format!("Failed to update index: {}", e)))?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if a pattern contains single-level wildcards
    fn is_single_wildcard_pattern(&self, pattern: &Path) -> bool {
        pattern.segments().iter().any(|s| s.is_single_wildcard())
    }
    
    /// Check if a pattern contains multi-level wildcards
    fn is_multi_wildcard_pattern(&self, pattern: &Path) -> bool {
        pattern.segments().iter().any(|s| s.is_multi_wildcard())
    }
    
    /// Find matches for a single-level wildcard pattern
    fn find_single_wildcard_matches(&self, pattern: &Path) -> Result<HashSet<Path>> {
        let segments = pattern.segments();
        let segment_count = segments.len();
        
        // Find all wildcard positions
        let mut wildcard_positions = Vec::new();
        for (i, segment) in segments.iter().enumerate() {
            if segment.is_single_wildcard() {
                wildcard_positions.push(i);
            }
        }
        
        if wildcard_positions.is_empty() {
            // No wildcards, just check if the exact path exists
            return Ok(HashSet::new());
        }
        
        // Create a structural pattern with wildcards
        let mut fixed_segments = HashMap::new();
        for (i, segment) in segments.iter().enumerate() {
            if !segment.is_single_wildcard() {
                fixed_segments.insert(i, segment.as_str());
            }
        }
        
        let pattern_struct = StructuralPattern {
            segment_count,
            fixed_segments: fixed_segments.into_iter().collect(),
        };
        
        // Serialize the pattern as key
        let key = serialize(&pattern_struct)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
            
        // Get paths for this pattern
        if let Some(data) = self.single_wildcard_tree.get(&key)
            .map_err(|e| StoreError::Internal(format!("Failed to read from index: {}", e)))? {
            let paths = deserialize::<HashSet<Path>>(&data)
                .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
            
            return Ok(paths);
        }
        
        Ok(HashSet::new())
    }
    
    /// Find matches for a multi-level wildcard pattern
    fn find_multi_wildcard_matches(&self, pattern: &Path) -> Result<HashSet<Path>> {
        let segments = pattern.segments();
        
        // Find the position of the first ** wildcard
        let mut multi_wildcard_pos = None;
        for (i, segment) in segments.iter().enumerate() {
            if segment.is_multi_wildcard() {
                multi_wildcard_pos = Some(i);
                break;
            }
        }
        
        if multi_wildcard_pos.is_none() {
            // No multi-level wildcards
            return Ok(HashSet::new());
        }
        
        let multi_pos = multi_wildcard_pos.unwrap();
        
        // Get the prefix before the wildcard
        let prefix: Vec<String> = segments[0..multi_pos]
            .iter()
            .map(|s| s.as_str())
            .collect();
            
        // Get the suffix after the wildcard
        let suffix: Vec<String> = if multi_pos + 1 < segments.len() {
            segments[multi_pos + 1..]
                .iter()
                .map(|s| s.as_str())
                .collect()
        } else {
            Vec::new()
        };
        
        let mut matches = HashSet::new();
        
        // If there's a suffix, use it to find candidate paths
        if !suffix.is_empty() {
            let suffix_key = serialize(&suffix)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                
            if let Some(data) = self.multi_wildcard_tree.get(&suffix_key)
                .map_err(|e| StoreError::Internal(format!("Failed to read from index: {}", e)))? {
                let paths = deserialize::<HashSet<Path>>(&data)
                    .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                
                // Filter paths that have the correct prefix
                for path in paths {
                    if self.path_matches_pattern(&path, pattern) {
                        matches.insert(path);
                    }
                }
            }
        } else {
            // If there's no suffix, we need to check all paths
            // For efficiency, we'll scan through all keys in the multi-tree
            for item in self.multi_wildcard_tree.iter() {
                let (_, value_bytes) = item
                    .map_err(|e| StoreError::Internal(format!("Failed to iterate index: {}", e)))?;
                
                let paths = deserialize::<HashSet<Path>>(&value_bytes)
                    .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                
                // Filter paths that have the correct prefix
                for path in paths {
                    if self.path_matches_pattern(&path, pattern) {
                        matches.insert(path);
                    }
                }
            }
        }
        
        Ok(matches)
    }
    
    /// Check if a path matches a pattern (used for verification)
    fn path_matches_pattern(&self, path: &Path, pattern: &Path) -> bool {
        path.matches(pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tempfile::tempdir;
    
    #[test]
    fn test_wildcard_index_single() {
        // Create a temporary directory for the test database
        let dir = tempdir().unwrap();
        let db_path = dir.path();
        
        // Open a temporary database
        let db = sled::open(db_path).unwrap();
        
        let mut index = WildcardIndex::new(&db).unwrap();
        
        // Add paths to the index
        let path1 = Path::from_str("users.u-123456.username").unwrap();
        let path2 = Path::from_str("users.u-789012.username").unwrap();
        let path3 = Path::from_str("users.u-123456.email").unwrap();
        let path4 = Path::from_str("posts.p-789012.title").unwrap();
        
        index.add_path(&path1).unwrap();
        index.add_path(&path2).unwrap();
        index.add_path(&path3).unwrap();
        index.add_path(&path4).unwrap();
        
        // Test single wildcard patterns
        let pattern1 = Path::from_str("users.*.username").unwrap();
        let results1 = index.find_matches(&pattern1).unwrap();
        
        assert_eq!(results1.len(), 2);
        assert!(results1.iter().any(|p| p == &path1));
        assert!(results1.iter().any(|p| p == &path2));
        
        let pattern2 = Path::from_str("users.u-123456.*").unwrap();
        let results2 = index.find_matches(&pattern2).unwrap();
        
        assert_eq!(results2.len(), 2);
        assert!(results2.iter().any(|p| p == &path1));
        assert!(results2.iter().any(|p| p == &path3));
    }
    
    #[test]
    fn test_wildcard_index_multi() {
        // Create a temporary directory for the test database
        let dir = tempdir().unwrap();
        let db_path = dir.path();
        
        // Open a temporary database
        let db = sled::open(db_path).unwrap();
        
        let mut index = WildcardIndex::new(&db).unwrap();
        
        // Add paths to the index
        let path1 = Path::from_str("users.u-123456.profile.bio").unwrap();
        let path2 = Path::from_str("users.u-789012.profile.bio").unwrap();
        let path3 = Path::from_str("users.u-123456.bio").unwrap();
        let path4 = Path::from_str("posts.p-789012.content.bio").unwrap();
        
        index.add_path(&path1).unwrap();
        index.add_path(&path2).unwrap();
        index.add_path(&path3).unwrap();
        index.add_path(&path4).unwrap();
        
        // Test multi-level wildcard patterns
        let pattern1 = Path::from_str("users.**.bio").unwrap();
        let results1 = index.find_matches(&pattern1).unwrap();
        
        assert_eq!(results1.len(), 3);
        assert!(results1.iter().any(|p| p == &path1));
        assert!(results1.iter().any(|p| p == &path2));
        assert!(results1.iter().any(|p| p == &path3));
        
        let pattern2 = Path::from_str("**.content.bio").unwrap();
        let results2 = index.find_matches(&pattern2).unwrap();
        
        assert_eq!(results2.len(), 1);
        assert!(results2.iter().any(|p| p == &path4));
    }
    
    #[test]
    fn test_wildcard_index_remove() {
        // Create a temporary directory for the test database
        let dir = tempdir().unwrap();
        let db_path = dir.path();
        
        // Open a temporary database
        let db = sled::open(db_path).unwrap();
        
        let mut index = WildcardIndex::new(&db).unwrap();
        
        // Add paths to the index
        let path1 = Path::from_str("users.u-123456.username").unwrap();
        let path2 = Path::from_str("users.u-789012.username").unwrap();
        
        index.add_path(&path1).unwrap();
        index.add_path(&path2).unwrap();
        
        // Test before removal
        let pattern = Path::from_str("users.*.username").unwrap();
        let results_before = index.find_matches(&pattern).unwrap();
        assert_eq!(results_before.len(), 2);
        
        // Remove one path
        index.remove_path(&path1).unwrap();
        
        // Test after removal
        let results_after = index.find_matches(&pattern).unwrap();
        assert_eq!(results_after.len(), 1);
        assert!(results_after.iter().any(|p| p == &path2));
    }
}