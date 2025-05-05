//! Unified indexing system for Hyperion
//!
//! This module provides indexing capabilities with async updates
//! for path-based queries and pattern matching operations.

use std::collections::{HashMap, HashSet, BTreeMap};
use std::sync::{Arc, RwLock, Mutex};
use tokio::sync::mpsc::{channel, Sender, Receiver};
use tokio::task::JoinHandle;
use sled::Db;
use bincode::{serialize, deserialize};
use serde::Serialize;

use crate::core::path::Path;
use crate::core::errors::{Result, StoreError};

use super::path::PathSegment;

/// Operation type for index updates
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexOp {
    /// Add a path to the index
    Add(Path),
    /// Remove a path from the index
    Remove(Path),
    /// Special operation to flush pending operations
    Flush,
    /// Special operation to stop the worker
    Shutdown,
}

/// Statistics for index operations
#[derive(Debug, Default, Clone)]
pub struct IndexStats {
    /// Total number of operations processed
    pub total_operations: usize,
    /// Total number of add operations
    pub total_adds: usize,
    /// Total number of remove operations
    pub total_removes: usize,
    /// Total number of pending operations
    pub pending_operations: usize,
}

/// Base trait for all index implementations
pub trait BaseIndex: Send + Sync {
    /// Add a path to the index (internal implementation)
    fn add_path_internal(&mut self, path: &Path) -> Result<()>;
    
    /// Remove a path from the index (internal implementation)
    fn remove_path_internal(&mut self, path: &Path) -> Result<()>;
    
    /// Find all paths that match a prefix
    fn find_prefix(&self, prefix: &Path) -> Result<Vec<Path>>;
    
    /// Find all paths that match a pattern
    fn find_matches(&self, pattern: &Path) -> Result<Vec<Path>>;
    
    /// Clear the entire index
    fn clear(&mut self) -> Result<()>;
    
    /// Get the name of this index type
    fn name(&self) -> &'static str;
}

/// Main unified index interface
pub struct Index {
    /// The actual index implementation
    index_impl: Arc<RwLock<dyn BaseIndex>>,
    /// Sender for async operations
    tx: Option<Sender<IndexOp>>,
    /// Stats for this index
    stats: Arc<Mutex<IndexStats>>,
    /// Task handle for the background worker
    worker_handle: Option<JoinHandle<()>>,
}

impl Index {
    /// Create a new index with the given implementation
    pub fn new<I: BaseIndex + 'static>(impl_: I) -> Self {
        let index_impl = Arc::new(RwLock::new(impl_));
        let stats = Arc::new(Mutex::new(IndexStats::default()));
        
        Index {
            index_impl,
            tx: None,
            stats,
            worker_handle: None,
        }
    }
    
    /// Start the async worker to process operations
    pub fn start_worker(&mut self) -> Result<()> {
        if self.tx.is_some() {
            return Ok(()); // Already started
        }
        
        let (tx, rx) = channel(1000); // Buffer size of 1000 operations
        let index_impl = Arc::clone(&self.index_impl);
        let stats = Arc::clone(&self.stats);
        
        // Spawn a Tokio task to process operations
        let handle = tokio::spawn(async move {
            Self::process_operations(rx, index_impl, stats).await;
        });
        
        self.tx = Some(tx);
        self.worker_handle = Some(handle);
        
        Ok(())
    }
    
    /// Process operations in background
    async fn process_operations(
        mut rx: Receiver<IndexOp>,
        index_impl: Arc<RwLock<dyn BaseIndex>>,
        stats: Arc<Mutex<IndexStats>>
    ) {
        while let Some(op) = rx.recv().await {
            match op {
                IndexOp::Add(path) => {
                    let mut index = index_impl.write().unwrap();
                    if let Ok(()) = index.add_path_internal(&path) {
                        println!("ADDED PATH: {:?}", path);
                        let mut stats = stats.lock().unwrap();
                        stats.total_operations += 1;
                        stats.total_adds += 1;
                    }
                },
                IndexOp::Remove(path) => {
                    let mut index = index_impl.write().unwrap();
                    if let Ok(()) = index.remove_path_internal(&path) {
                        let mut stats = stats.lock().unwrap();
                        stats.total_operations += 1;
                        stats.total_removes += 1;
                    }
                },
                IndexOp::Flush => {
                    // Just a signal to process all queued operations
                },
                IndexOp::Shutdown => {
                    break; // Exit the loop to shutdown
                }
            }
        }
    }
    
    /// Asynchronously update the index by adding or removing a path
    pub async fn update(&self, path: Path, value: Option<()>) -> Result<()> {
        println!("Updating index with path: {:?}", path);
        let tx = self.tx.as_ref().ok_or_else(|| 
            StoreError::Internal("Index worker not started".to_string())
        )?;
        
        let op = if value.is_some() {
            IndexOp::Add(path)
        } else {
            IndexOp::Remove(path)
        };
        
        // Increment pending operations count
        {
            let mut stats = self.stats.lock().unwrap();
            stats.pending_operations += 1;
        }
        
        // Send the operation to the worker
        tx.send(op).await.map_err(|_| 
            StoreError::Internal("Failed to send operation to index worker".to_string())
        )?;
        
        Ok(())
    }
    
    /// Synchronously find all paths with the given prefix
    pub fn find_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        println!("Finding prefix: {:?}", prefix);
        let index = self.index_impl.read().unwrap();
        println!("Index type: {}", index.name());
        index.find_prefix(prefix)
    }
    
    /// Synchronously find all paths matching the given pattern
    pub fn find_matches(&self, pattern: &Path) -> Result<Vec<Path>> {
        let index = self.index_impl.read().unwrap();
        index.find_matches(pattern)
    }
    
    /// Asynchronously flush all pending operations
    pub async fn flush(&self) -> Result<()> {
        let tx = self.tx.as_ref().ok_or_else(|| 
            StoreError::Internal("Index worker not started".to_string())
        )?;
        
        tx.send(IndexOp::Flush).await.map_err(|_| 
            StoreError::Internal("Failed to send flush operation to index worker".to_string())
        )?;
        
        Ok(())
    }
    
    /// Get current statistics
    pub fn stats(&self) -> IndexStats {
        let stats = self.stats.lock().unwrap();
        stats.clone()
    }
    
    /// Stop the worker (should be called on shutdown)
    pub async fn shutdown(&self) -> Result<()> {
        if let Some(tx) = &self.tx {
            let _ = tx.send(IndexOp::Shutdown).await;
        }
        
        Ok(())
    }

    pub async fn clear(&self) -> Result<()> {
        let mut index = self.index_impl.write().unwrap();
        index.clear()?;
        Ok(())
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        // Try to shutdown gracefully on drop
        if let Some(tx) = self.tx.take() {
            let _ = tokio::spawn(async move {
                let _ = tx.send(IndexOp::Shutdown).await;
            });
        }
    }
}

// Maintenant nous allons implémenter les deux types d'index principaux

/// Index for prefix search operations
pub struct PrefixIndex {
    /// The Sled database
    db: Arc<Db>,
    /// Name of the tree where index data is stored
    tree_name: String,
}

impl PrefixIndex {
    /// Create a new prefix index
    pub fn new(db: Arc<Db>, tree_name: &str) -> Result<Self> {
        let tree_name = tree_name.to_string();
        
        Ok(PrefixIndex {
            db,
            tree_name,
        })
    }
    
    /// Get the Sled tree for this index
    fn get_tree(&self) -> Result<sled::Tree> {
        self.db.open_tree(&self.tree_name)
        .map_err(|e| StoreError::Internal(format!("Failed to open index tree: {}", e)))
    }
    
    /// Create an index key from a path
    fn create_index_key(path: &Path) -> Result<Vec<u8>> {
        let segments = path.segments();
        let segment_count = segments.len();
        
        // Start with segment count as a single byte
        if segment_count > 255 {
            return Err(StoreError::InvalidOperation(
                "Path has too many segments for indexing".to_string()
            ));
        }
        
        let key_bytes = serialize(path)
        .map_err(|e| StoreError::SerializationError(e.to_string()))?;
        
        Ok(key_bytes)
    }
    
    /// Create a range start for prefix search
    fn create_prefix_start(prefix: &Path) -> Result<Vec<u8>> {
        Self::create_index_key(prefix)
    }
}

impl BaseIndex for PrefixIndex {
    fn add_path_internal(&mut self, path: &Path) -> Result<()> {
        let tree = self.get_tree()?;
        let key = Self::create_index_key(path)?;
        let value = vec![1]; // Just a marker

        println!("Inserting into PrefixIndex: {:?}", path);
        
        tree.insert(key, value)
        .map_err(|e| StoreError::Internal(format!("Failed to insert into index: {}", e)))?;
        
        Ok(())
    }
    
    fn remove_path_internal(&mut self, path: &Path) -> Result<()> {
        let tree = self.get_tree()?;
        let key = Self::create_index_key(path)?;
        
        tree.remove(key)
        .map_err(|e| StoreError::Internal(format!("Failed to remove from index: {}", e)))?;
        
        Ok(())
    }
    
    fn find_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        println!("\n");
        println!("Finding prefix in PrefixIndex: {:?}", prefix);
        let tree = self.get_tree()?;
        let prefix_bytes = Self::create_prefix_start(prefix)?;
        
        let mut results = Vec::new();
        println!("Prefix bytes: {:?}", prefix_bytes);
        
        println!("Iterating through index tree for prefix: {:?}", prefix);
        for item in tree.scan_prefix(prefix_bytes) {
            println!("Scanning item: {:?}", item);
            let (key, _) = item
            .map_err(|e| StoreError::Internal(format!("Failed to scan index: {}", e)))?;
            
            let path: Path = deserialize(&key)
            .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
            
            println!("Found path: {:?}", path);
            results.push(path);
        }
        
        Ok(results)
    }
    
    fn find_matches(&self, pattern: &Path) -> Result<Vec<Path>> {
        if !pattern.has_wildcards() {
            return self.find_prefix(pattern);
        }
        
        // For wildcard searches, we need to get all paths and filter them
        let mut results = Vec::new();
        
        let all_paths = self.find_prefix(&Path::new())?;
        for path in all_paths {
            if path.matches(pattern) {
                results.push(path);
            }
        }
        
        Ok(results)
    }
    
    fn clear(&mut self) -> Result<()> {
        let tree = self.get_tree()?;
        tree.clear()
        .map_err(|e| StoreError::Internal(format!("Failed to clear index: {}", e)))?;
        
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "PrefixIndex"
    }
}

/// Index optimized for wildcard pattern searches
pub struct WildcardIndex {
    /// The Sled database
    db: Arc<Db>,
    /// Name of the tree for single-level wildcards
    single_tree_name: String,
    /// Name of the tree for multi-level wildcards
    multi_tree_name: String,
}

impl WildcardIndex {
    /// Create a new wildcard index
    pub fn new(db: Arc<Db>, base_name: &str) -> Result<Self> {
        let single_tree_name = format!("{}_single", base_name);
        let multi_tree_name = format!("{}_multi", base_name);
        
        Ok(WildcardIndex {
            db,
            single_tree_name,
            multi_tree_name,
        })
    }
    
    /// Get the tree for single-level wildcards
    fn get_single_tree(&self) -> Result<sled::Tree> {
        self.db.open_tree(&self.single_tree_name)
            .map_err(|e| StoreError::Internal(format!("Failed to open single wildcard tree: {}", e)))
    }
    
    /// Get the tree for multi-level wildcards
    fn get_multi_tree(&self) -> Result<sled::Tree> {
        self.db.open_tree(&self.multi_tree_name)
            .map_err(|e| StoreError::Internal(format!("Failed to open multi wildcard tree: {}", e)))
    }
    
    /// Create a structural pattern for single-level wildcard indexing
    fn create_structural_pattern(path: &Path) -> Result<Vec<u8>> {
        #[derive(Serialize)]
        struct Pattern {
            segment_count: usize,
            fixed_segments: BTreeMap<usize, String>,
        }
        
        let segments = path.segments();
        let segment_count = segments.len();
        
        let mut fixed_segments = BTreeMap::new();
        for (i, segment) in segments.iter().enumerate() {
            if !segment.is_single_wildcard() && !segment.is_multi_wildcard() {
                fixed_segments.insert(i, segment.as_str());
            }
        }
        
        let pattern = Pattern {
            segment_count,
            fixed_segments,
        };
        
        serialize(&pattern)
            .map_err(|e| StoreError::SerializationError(e.to_string()))
    }
    
    /// Create a suffix key for multi-level wildcard indexing
    fn create_suffix_key(segments: &[String]) -> Result<Vec<u8>> {
        serialize(&segments)
            .map_err(|e| StoreError::SerializationError(e.to_string()))
    }
    
    /// Index a path for single-level wildcard queries
    fn index_for_single_wildcards(&self, path: &Path) -> Result<()> {
        let tree = self.get_single_tree()?;
        let segments = path.segments();
        
        // Generate all possible single-wildcard patterns
        for wildcard_pos in 0..segments.len() {
            // Create a copy of the path with one position as wildcard
            let mut pattern_segments = segments.iter()
                .enumerate()
                .map(|(i, s)| {
                    if i == wildcard_pos {
                        "*".to_string()
                    } else {
                        s.as_str()
                    }
                })
                .collect::<Vec<_>>();
            
            // Create a pattern key
            let pattern_segments = pattern_segments.into_iter().map(PathSegment::new).collect::<Vec<_>>();
            let pattern_key = Self::create_structural_pattern(&Path::from_segments(pattern_segments))?;
            
            // Store path in the pattern's entry
            tree.insert(pattern_key, serialize(path).map_err(|e| StoreError::SerializationError(e.to_string()))?)
                .map_err(|e| StoreError::Internal(format!("Failed to update index: {}", e)))?;
        }
        
        Ok(())
    }
    

    /// Index a path for multi-level wildcard queries
    fn index_for_multi_wildcards(&self, path: &Path) -> Result<()> {
        let tree = self.get_multi_tree()?;
        let segments = path.segments()
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>();
        
        // For each suffix of the path
        for start_pos in 0..segments.len() {
            let suffix = &segments[start_pos..];
            let suffix_key = Self::create_suffix_key(suffix)?;
            
            // Store path in the suffix's entry
            tree.insert(suffix_key, serialize(path).map_err(|e| StoreError::SerializationError(e.to_string()))?)
                .map_err(|e| StoreError::Internal(format!("Failed to update index: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Remove a path from single-level wildcard index
    fn remove_from_single_wildcards(&self, path: &Path) -> Result<()> {
        let tree = self.get_single_tree()?;
        let segments = path.segments();
        
        // Remove from all possible single-wildcard patterns
        for wildcard_pos in 0..segments.len() {
            // Create a copy of the path with one position as wildcard
            let mut pattern_segments = segments.iter()
                .enumerate()
                .map(|(i, s)| {
                    if i == wildcard_pos {
                        "*".to_string()
                    } else {
                        s.as_str()
                    }
                })
                .collect::<Vec<_>>();
            let pattern_segments = pattern_segments.into_iter().map(PathSegment::new).collect::<Vec<_>>();
            
            // Create a pattern key
            let pattern_key = Self::create_structural_pattern(&Path::from_segments(pattern_segments))?;
            
            // Remove path from the pattern's entry
            tree.remove(pattern_key)
                .map_err(|e| StoreError::Internal(format!("Failed to update index: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Remove a path from multi-level wildcard index
    fn remove_from_multi_wildcards(&self, path: &Path) -> Result<()> {
        let tree = self.get_multi_tree()?;
        let segments = path.segments()
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>();
        
        // Remove from all suffixes
        for start_pos in 0..segments.len() {
            let suffix = &segments[start_pos..];
            let suffix_key = Self::create_suffix_key(suffix)?;
            
            // Remove path from the suffix's entry
            tree.remove(suffix_key)
                .map_err(|e| StoreError::Internal(format!("Failed to update index: {}", e)))?;
        }
        
        Ok(())
    }
}



impl BaseIndex for WildcardIndex {
    fn add_path_internal(&mut self, path: &Path) -> Result<()> {
        // Index for both types of wildcards
        println!("Indexing path for wildcards: {:?}", path);
        self.index_for_single_wildcards(path)?;
        self.index_for_multi_wildcards(path)?;
        
        Ok(())
    }
    
    fn remove_path_internal(&mut self, path: &Path) -> Result<()> {
        // Remove from both indexes
        self.remove_from_single_wildcards(path)?;
        self.remove_from_multi_wildcards(path)?;
        
        Ok(())
    }
    
    fn find_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        // For prefix search, we use a simpler approach
        // We just iterate through all paths in the single wildcard index
        // This could be optimized in a real implementation
        println!("WILDCARD FIND: {:?}", prefix);
        let tree = self.get_single_tree()?;
        let mut results = HashSet::new();
        
        println!("Iterating through index tree for prefix: {:?}", prefix);
        for item in tree.iter() {
            let (_, value_bytes) = item
                .map_err(|e| StoreError::Internal(format!("Failed to iterate index: {}", e)))?;
            
            let path: Path = deserialize(&value_bytes)
                .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
            
            if path.starts_with(prefix) {
                results.insert(path);
            }
        }
        
        Ok(results.into_iter().collect())
    }
    
    fn find_matches(&self, pattern: &Path) -> Result<Vec<Path>> {
        let mut results = HashSet::new();
        
        // Check if it's a single-level wildcard pattern
        if pattern.segments().iter().any(|s| s.is_single_wildcard()) {
            // Use the structural pattern to find matches
            let pattern_key = Self::create_structural_pattern(pattern)?;
            let tree = self.get_single_tree()?;
            
            if let Some(value_bytes) = tree.get(pattern_key)
                .map_err(|e| StoreError::Internal(format!("Failed to query index: {}", e)))? {
                
                let path: Path = deserialize(&value_bytes)
                    .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                    
                results.insert(path);
            }
        }
        
        // Check if it's a multi-level wildcard pattern
        if pattern.segments().iter().any(|s| s.is_multi_wildcard()) {
            // Find the position of the first multi-wildcard
            let pos = pattern.segments().iter()
                .position(|s| s.is_multi_wildcard())
                .unwrap();
            
            // Get the suffix after the wildcard
            let suffix: Vec<String> = if pos + 1 < pattern.segments().len() {
                pattern.segments()[pos + 1..]
                    .iter()
                    .map(|s| s.as_str())
                    .collect()
            } else {
                Vec::new()
            };
            
            // Find paths with this suffix
            if !suffix.is_empty() {
                let suffix_key = Self::create_suffix_key(&suffix)?;
                let tree = self.get_multi_tree()?;
                
                if let Some(value_bytes) = tree.get(suffix_key)
                    .map_err(|e| StoreError::Internal(format!("Failed to query index: {}", e)))? {
                    
                    let path: Path = deserialize(&value_bytes)
                        .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                        
                    // Verify that the path matches the complete pattern
                    if path.matches(pattern) {
                        results.insert(path);
                    }
                }
            } else {
                // If there's no suffix, we need to scan all paths
                // This could be optimized in a real implementation
                for item in self.get_single_tree()?.iter() {
                    let (_, value_bytes) = item
                        .map_err(|e| StoreError::Internal(format!("Failed to iterate index: {}", e)))?;
                    
                    let path: Path = deserialize(&value_bytes)
                        .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                    
                    if path.matches(pattern) {
                        results.insert(path);
                    }
                }
            }
        }
        
        Ok(results.into_iter().collect())
    }
    
    fn clear(&mut self) -> Result<()> {
        // Clear both trees
        self.get_single_tree()?.clear()
            .map_err(|e| StoreError::Internal(format!("Failed to clear single wildcard index: {}", e)))?;
            
        self.get_multi_tree()?.clear()
            .map_err(|e| StoreError::Internal(format!("Failed to clear multi wildcard index: {}", e)))?;
            
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "WildcardIndex"
    }
}


impl Clone for Index {
    fn clone(&self) -> Self {
        Index {
            index_impl: Arc::clone(&self.index_impl),
            tx: self.tx.clone(),
            stats: Arc::clone(&self.stats),
            worker_handle: None, // Le handle ne se clone pas, mais ce n'est pas grave
        }
    }
}