//! Index batcher for Hyperion
//!
//! This module provides batching capabilities for index operations,
//! significantly improving write performance by accumulating index
//! updates and applying them in batches.

use std::collections::{HashSet, HashMap};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Instant, Duration};

use crate::path::Path;
use crate::errors::{Result, StoreError};
use crate::index::PathIndex;
use crate::wildcard_index::WildcardIndex;

/// Operation type for batched index operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchOperation {
    /// Add a path to the index
    Add,
    /// Remove a path from the index
    Remove,
}

/// Configuration for the index batcher
#[derive(Debug, Clone)]
pub struct BatcherConfig {
    /// Maximum number of operations to accumulate before flushing
    pub max_operations: usize,
    /// Maximum time to wait before flushing (in milliseconds)
    pub max_delay_ms: u64,
    /// Whether to flush automatically when thresholds are reached
    pub auto_flush: bool,
}

impl Default for BatcherConfig {
    fn default() -> Self {
        BatcherConfig {
            max_operations: 1500,
            max_delay_ms: 1500, // 5 second
            auto_flush: true,
        }
    }
}

/// A batcher for index operations
pub struct IndexBatcher<I: PathIndex, L> {
    /// Reference to the index
    index: Arc<L>,
    /// Batched operations (path -> operation)
    pending_operations: HashMap<Path, BatchOperation>,
    /// Configuration
    config: BatcherConfig,
    /// Time of the first pending operation
    first_op_time: Option<Instant>,
    /// Statistics
    stats: BatcherStats,
    /// Marker to indicate dependency on type parameter I
    _marker: std::marker::PhantomData<I>,
}

/// Statistics for the batcher
#[derive(Debug, Default, Clone)]
pub struct BatcherStats {
    /// Total number of operations added to the batcher
    pub total_operations: usize,
    /// Total number of batches flushed
    pub total_batches: usize,
    /// Total number of add operations
    pub total_adds: usize,
    /// Total number of remove operations
    pub total_removes: usize,
    /// Total number of operations eliminated by conflict resolution
    pub eliminated_operations: usize,
}

impl<I: PathIndex> IndexBatcher<I, RwLock<I>> {
    /// Create a new index batcher for RwLock
    pub fn new_rwlock(index: Arc<RwLock<I>>, config: BatcherConfig) -> Self {
        IndexBatcher {
            index,
            pending_operations: HashMap::new(),
            config,
            first_op_time: None,
            stats: BatcherStats::default(),
            _marker: std::marker::PhantomData,
        }
    }
    
    // Ajouter les mêmes méthodes, mais adaptées pour RwLock
    
    pub fn batch_add(&mut self, path: Path) -> Result<()> {
        self.record_operation_time();
        
        if let Some(BatchOperation::Remove) = self.pending_operations.get(&path) {
            self.pending_operations.remove(&path);
            self.stats.eliminated_operations += 1;
        } else {
            self.pending_operations.insert(path, BatchOperation::Add);
            self.stats.total_adds += 1;
        }
        
        self.stats.total_operations += 1;
        
        if self.should_flush() {
            self.flush()?;
        }
        
        Ok(())
    }
    
    pub fn batch_remove(&mut self, path: Path) -> Result<()> {
        self.record_operation_time();
        
        if let Some(BatchOperation::Add) = self.pending_operations.get(&path) {
            self.pending_operations.remove(&path);
            self.stats.eliminated_operations += 1;
        } else {
            self.pending_operations.insert(path, BatchOperation::Remove);
            self.stats.total_removes += 1;
        }
        
        self.stats.total_operations += 1;
        
        if self.should_flush() {
            self.flush()?;
        }
        
        Ok(())
    }
    
    pub fn flush(&mut self) -> Result<()> {
        if self.pending_operations.is_empty() {
            return Ok(());
        }
        
        // Group operations by type for bulk processing
        let mut to_add = Vec::new();
        let mut to_remove = Vec::new();
        
        for (path, op) in std::mem::take(&mut self.pending_operations) {
            match op {
                BatchOperation::Add => to_add.push(path),
                BatchOperation::Remove => to_remove.push(path),
            }
        }
        
        // Apply operations in bulk to the index
        {
            let mut index = self.index.write().unwrap();
            
            // First remove paths (to avoid potential conflicts)
            for path in to_remove {
                // Ignore errors here, as the path might not exist
                let _ = index.remove_path(&path);
            }
            
            // Then add paths
            for path in to_add {
                index.add_path(&path)?;
            }
        }
        
        // Reset the timer
        self.first_op_time = None;
        
        // Update stats
        self.stats.total_batches += 1;
        
        Ok(())
    }
    
    // Les autres méthodes restent identiques
    // Méthodes should_flush, record_operation_time, stats, pending_count, etc.
    
    /// Check if we should automatically flush based on thresholds
    fn should_flush(&self) -> bool {
        if !self.config.auto_flush || self.pending_operations.is_empty() {
            return false;
        }
        
        // Check if we've reached the max operations threshold
        if self.pending_operations.len() >= self.config.max_operations {
            return true;
        }
        
        // Check if we've reached the max delay threshold
        if let Some(first_time) = self.first_op_time {
            let elapsed = first_time.elapsed();
            let max_delay = Duration::from_millis(self.config.max_delay_ms);
            if elapsed >= max_delay {
                return true;
            }
        }
        
        false
    }
    
    /// Record the time of an operation (for auto-flush timing)
    fn record_operation_time(&mut self) {
        if self.first_op_time.is_none() {
            self.first_op_time = Some(Instant::now());
        }
    }
    
    /// Get the current statistics
    pub fn stats(&self) -> BatcherStats {
        self.stats.clone()
    }
    
    /// Get the number of pending operations
    pub fn pending_count(&self) -> usize {
        self.pending_operations.len()
    }
    
    /// Check if a specific path has a pending operation
    pub fn has_pending(&self, path: &Path) -> bool {
        self.pending_operations.contains_key(path)
    }
    
    /// Get the type of pending operation for a path (if any)
    pub fn pending_operation(&self, path: &Path) -> Option<&BatchOperation> {
        self.pending_operations.get(path)
    }
}


impl<I: PathIndex> IndexBatcher<I, Mutex<I>> {
    /// Create a new index batcher
    pub fn new(index: Arc<Mutex<I>>, config: BatcherConfig) -> Self {
        IndexBatcher {
            index,
            pending_operations: HashMap::new(),
            config,
            first_op_time: None,
            stats: BatcherStats::default(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn new_mutex(index: Arc<Mutex<I>>, config: BatcherConfig) -> Self {
        IndexBatcher {
            index,
            pending_operations: HashMap::new(),
            config,
            first_op_time: None,
            stats: BatcherStats::default(),
            _marker: std::marker::PhantomData,
        }
    }
    
    /// Add a path to the batch for addition to the index
    pub fn batch_add(&mut self, path: Path) -> Result<()> {
        self.record_operation_time();
        
        // If we already have a remove operation for this path,
        // they cancel each other out, so just remove the pending remove
        if let Some(BatchOperation::Remove) = self.pending_operations.get(&path) {
            self.pending_operations.remove(&path);
            self.stats.eliminated_operations += 1;
        } else {
            // Otherwise, queue the add operation
            self.pending_operations.insert(path, BatchOperation::Add);
            self.stats.total_adds += 1;
        }
        
        self.stats.total_operations += 1;
        
        // Check if we should auto-flush
        if self.should_flush() {
            self.flush()?;
        }
        
        Ok(())
    }
    
    /// Add a path to the batch for removal from the index
    pub fn batch_remove(&mut self, path: Path) -> Result<()> {
        self.record_operation_time();
        
        // If we already have an add operation for this path,
        // they cancel each other out, so just remove the pending add
        if let Some(BatchOperation::Add) = self.pending_operations.get(&path) {
            self.pending_operations.remove(&path);
            self.stats.eliminated_operations += 1;
        } else {
            // Otherwise, queue the remove operation
            self.pending_operations.insert(path, BatchOperation::Remove);
            self.stats.total_removes += 1;
        }
        
        self.stats.total_operations += 1;
        
        // Check if we should auto-flush
        if self.should_flush() {
            self.flush()?;
        }
        
        Ok(())
    }
    
    /// Apply all pending operations to the index
    pub fn flush(&mut self) -> Result<()> {
        if self.pending_operations.is_empty() {
            return Ok(());
        }
        
        // Group operations by type for bulk processing
        let mut to_add = Vec::new();
        let mut to_remove = Vec::new();
        
        for (path, op) in std::mem::take(&mut self.pending_operations) {
            match op {
                BatchOperation::Add => to_add.push(path),
                BatchOperation::Remove => to_remove.push(path),
            }
        }
        
        // Apply operations in bulk to the index
        let mut index = self.index.lock().unwrap();
        
        // First remove paths (to avoid potential conflicts)
        for path in to_remove {
            // Ignore errors here, as the path might not exist
            let _ = index.remove_path(&path);
        }
        
        // Then add paths
        for path in to_add {
            index.add_path(&path)?;
        }
        
        // Reset the timer
        self.first_op_time = None;
        
        // Update stats
        self.stats.total_batches += 1;
        
        Ok(())
    }
    
    /// Check if we should automatically flush based on thresholds
    fn should_flush(&self) -> bool {
        if !self.config.auto_flush || self.pending_operations.is_empty() {
            return false;
        }
        
        // Check if we've reached the max operations threshold
        if self.pending_operations.len() >= self.config.max_operations {
            return true;
        }
        
        // Check if we've reached the max delay threshold
        if let Some(first_time) = self.first_op_time {
            let elapsed = first_time.elapsed();
            let max_delay = Duration::from_millis(self.config.max_delay_ms);
            if elapsed >= max_delay {
                return true;
            }
        }
        
        false
    }
    
    /// Record the time of an operation (for auto-flush timing)
    fn record_operation_time(&mut self) {
        if self.first_op_time.is_none() {
            self.first_op_time = Some(Instant::now());
        }
    }
    
    /// Get the current statistics
    pub fn stats(&self) -> BatcherStats {
        self.stats.clone()
    }
    
    /// Get the number of pending operations
    pub fn pending_count(&self) -> usize {
        self.pending_operations.len()
    }
    
    /// Check if a specific path has a pending operation
    pub fn has_pending(&self, path: &Path) -> bool {
        self.pending_operations.contains_key(path)
    }
    
    /// Get the type of pending operation for a path (if any)
    pub fn pending_operation(&self, path: &Path) -> Option<&BatchOperation> {
        self.pending_operations.get(path)
    }
}

/// A specialized batcher for the wildcard index
pub type WildcardIndexBatcher = IndexBatcher<WildcardIndex, RwLock<WildcardIndex>>;