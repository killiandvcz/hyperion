//! Benchmarking system for Hyperion
//!
//! This module provides tools to measure performance of various operations
//! and identify bottlenecks in the database system.

use std::time::{Duration, Instant};
use std::str::FromStr;
use std::fmt;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use crate::path::Path;
use crate::value::Value;
use crate::persistent_store::PersistentStore;
use crate::errors::Result;
use crate::BatcherConfig;

/// A benchmark result for a single operation
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Name of the operation
    pub name: String,
    /// Number of operations performed
    pub operations: usize,
    /// Total time taken
    pub duration: Duration,
    /// Operations per second
    pub ops_per_second: f64,
    /// Time per operation in microseconds
    pub time_per_op_micros: f64,
}

impl fmt::Display for BenchmarkResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} ops in {:?} ({:.2} ops/sec, {:.2} µs/op)",
            self.name, self.operations, self.duration, 
            self.ops_per_second, self.time_per_op_micros)
    }
}

/// A benchmark suite for the database
pub struct Benchmark {
    /// The database to benchmark
    store: Rc<RefCell<PersistentStore>>,
    /// Results of benchmarks
    results: HashMap<String, BenchmarkResult>,
}

impl Benchmark {
    /// Create a new benchmark for the given store
    pub fn new(store: PersistentStore) -> Self {
        Benchmark {
            store: Rc::new(RefCell::new(store)),
            results: HashMap::new(),
        }
    }
    
    /// Run a benchmark function and record the result
    pub fn run<F>(&mut self, name: &str, operations: usize, mut f: F) -> Result<&BenchmarkResult>
    where
        F: FnMut() -> Result<()>,
    {
        let start = Instant::now();
        
        // Run the benchmark function repeatedly
        for _ in 0..operations {
            f()?;
        }
        
        let duration = start.elapsed();
        let ops_per_second = operations as f64 / duration.as_secs_f64();
        let time_per_op_micros = duration.as_micros() as f64 / operations as f64;
        
        let result = BenchmarkResult {
            name: name.to_string(),
            operations,
            duration,
            ops_per_second,
            time_per_op_micros,
        };
        
        self.results.insert(name.to_string(), result.clone());
        
        Ok(&self.results[name])
    }

    /// Run benchmarks comparing batched vs non-batched index updates
    pub fn run_batcher_benchmarks(&mut self, count: usize) -> Result<()> {
        println!("Running batcher benchmarks with {} operations...", count);
        
        // Créer un Rc<RefCell<>> pour partager le store avec les closures
        let store_ref = Rc::clone(&self.store);
        
        // Benchmark standard set operations (with default batching)
        let store_ref_set_batched = Rc::clone(&store_ref);
        let mut set_counter = 0;
        self.run("set_with_batching", count, || {
            let user_id = format!("u-{}", set_counter % 1000);
            let field = match set_counter % 5 {
                0 => "username",
                1 => "email",
                2 => "profile.bio",
                3 => "profile.location",
                _ => "active",
            };
            
            let path_str = format!("users.{}.{}", user_id, field);
            let path = Path::from_str(&path_str)?;
            
            let value = match field {
                "username" => Value::String(format!("user{}", set_counter)),
                "email" => Value::String(format!("user{}@example.com", set_counter)),
                "profile.bio" => Value::String(format!("Bio for user {}", set_counter)),
                "profile.location" => Value::String("San Francisco, CA".to_string()),
                _ => Value::Boolean(true),
            };
            
            // Emprunter de façon mutable uniquement ici
            store_ref_set_batched.borrow_mut().set(path, value)?;
            set_counter += 1;
            
            Ok(())
        })?;
        
        // Get the batcher stats
        let (prefix_stats, wildcard_stats) = store_ref.borrow().batcher_stats()?;
        println!("Prefix index batcher: {:?}", prefix_stats);
        println!("Wildcard index batcher: {:?}", wildcard_stats);
        
        // Force flush after the benchmark
        store_ref.borrow_mut().flush()?;
        
        // Disable batching for comparison
        {
            let mut store = store_ref.borrow_mut();
            let no_batch_config = BatcherConfig {
                max_operations: 1,  // Flush after each operation
                max_delay_ms: 0,    // No delay
                auto_flush: true,   // Always flush
            };
            store.configure_batcher(no_batch_config)?;
        }
        
        // Benchmark set operations without batching
        let store_ref_set_no_batch = Rc::clone(&store_ref);
        set_counter = 0;
        self.run("set_without_batching", count / 10, || {  // Fewer operations as it will be slower
            let user_id = format!("u-{}", set_counter % 1000);
            let field = match set_counter % 5 {
                0 => "username",
                1 => "email",
                2 => "profile.bio",
                3 => "profile.location",
                _ => "active",
            };
            
            let path_str = format!("users.{}.{}", user_id, field);
            let path = Path::from_str(&path_str)?;
            
            let value = match field {
                "username" => Value::String(format!("user{}", set_counter)),
                "email" => Value::String(format!("user{}@example.com", set_counter)),
                "profile.bio" => Value::String(format!("Bio for user {}", set_counter)),
                "profile.location" => Value::String("San Francisco, CA".to_string()),
                _ => Value::Boolean(true),
            };
            
            store_ref_set_no_batch.borrow_mut().set(path, value)?;
            set_counter += 1;
            
            Ok(())
        })?;
        
        // Reset to default batching configuration
        {
            let mut store = store_ref.borrow_mut();
            store.configure_batcher(BatcherConfig::default())?;
        }
        
        println!("Batcher benchmarks completed.");
        
        Ok(())
    }
    
    /// Run a benchmarking suite for path operations
    pub fn run_path_benchmarks(&mut self) -> Result<()> {
        println!("Running path benchmarks...");
        
        // Benchmark path creation
        self.run("path_creation", 100_000, || {
            let _path = Path::from_str("users.u-123456.profile.bio")?;
            Ok(())
        })?;
        
        // Benchmark path matching (no wildcards)
        let path1 = Path::from_str("users.u-123456.profile.bio")?;
        let path2 = Path::from_str("users.u-123456.profile")?;
        
        self.run("path_starts_with", 100_000, || {
            let _result = path1.starts_with(&path2);
            Ok(())
        })?;
        
        // Benchmark wildcard matching (single wildcard)
        let path = Path::from_str("users.u-123456.profile.bio")?;
        let pattern = Path::from_str("users.*.profile.bio")?;
        
        self.run("path_match_single_wildcard", 50_000, || {
            let _result = path.matches(&pattern);
            Ok(())
        })?;
        
        // Benchmark wildcard matching (multi wildcard)
        let path = Path::from_str("users.u-123456.profile.bio")?;
        let pattern = Path::from_str("users.**.bio")?;
        
        self.run("path_match_multi_wildcard", 50_000, || {
            let _result = path.matches(&pattern);
            Ok(())
        })?;
        
        println!("Path benchmarks completed.");
        
        Ok(())
    }
    
    /// Run benchmarks for storage operations
    pub fn run_storage_benchmarks(&mut self, count: usize) -> Result<()> {
        println!("Running storage benchmarks with {} operations...", count);
        
        // Créer un Rc<RefCell<>> pour partager le store avec les closures
        let store_ref = Rc::clone(&self.store);
        
        // Clear any existing data
        self.run("clear", 1, || {
            // This is a simplification - we should properly clear the database
            Ok(())
        })?;
        
        // Benchmark set operations
        let store_ref_set = Rc::clone(&store_ref);
        let mut set_counter = 0;
        self.run("set", count, || {
            let user_id = format!("u-{}", set_counter % 1000);
            let field = match set_counter % 5 {
                0 => "username",
                1 => "email",
                2 => "profile.bio",
                3 => "profile.location",
                _ => "active",
            };
            
            let path_str = format!("users.{}.{}", user_id, field);
            let path = Path::from_str(&path_str)?;
            
            let value = match field {
                "username" => Value::String(format!("user{}", set_counter)),
                "email" => Value::String(format!("user{}@example.com", set_counter)),
                "profile.bio" => Value::String(format!("Bio for user {}", set_counter)),
                "profile.location" => Value::String("San Francisco, CA".to_string()),
                _ => Value::Boolean(true),
            };
            
            // Emprunter de façon mutable uniquement ici
            store_ref_set.borrow_mut().set(path, value)?;
            set_counter += 1;
            
            Ok(())
        })?;
        
        // Benchmark get operations
        let store_ref_get = Rc::clone(&store_ref);
        let mut get_counter = 0;
        self.run("get", count, || {
            let user_id = format!("u-{}", get_counter % 1000);
            let field = match get_counter % 5 {
                0 => "username",
                1 => "email",
                2 => "profile.bio",
                3 => "profile.location",
                _ => "active",
            };
            
            let path_str = format!("users.{}.{}", user_id, field);
            let path = Path::from_str(&path_str)?;
            
            // Emprunter immutablement
            let _value = store_ref_get.borrow().get(&path)?;
            get_counter += 1;
            
            Ok(())
        })?;
        
        // Benchmark prefix search
        let store_ref_prefix = Rc::clone(&store_ref);
        let mut prefix_counter = 0;
        self.run("list_prefix", 1000, || {
            let user_id = format!("u-{}", prefix_counter % 1000);
            let path_str = format!("users.{}", user_id);
            let path = Path::from_str(&path_str)?;
            
            let _paths = store_ref_prefix.borrow().list_prefix(&path)?;
            prefix_counter += 1;
            
            Ok(())
        })?;
        
        // Benchmark wildcard queries
        let store_ref_wildcard1 = Rc::clone(&store_ref);
        self.run("query_wildcard_single", 100, || {
            let path = Path::from_str("users.*.email")?;
            let _results = store_ref_wildcard1.borrow().query(&path)?;
            
            Ok(())
        })?;
        
        let store_ref_wildcard2 = Rc::clone(&store_ref);
        self.run("query_wildcard_multi", 100, || {
            let path = Path::from_str("users.**.bio")?;
            let _results = store_ref_wildcard2.borrow().query(&path)?;
            
            Ok(())
        })?;
        
        println!("Storage benchmarks completed.");
        
        Ok(())
    }
    
    /// Print all benchmark results
    pub fn print_results(&self) {
        println!("\nBenchmark Results:");
        println!("==================");
        
        for (_, result) in &self.results {
            println!("{}", result);
        }
    }
    
    /// Get a specific benchmark result
    pub fn get_result(&self, name: &str) -> Option<&BenchmarkResult> {
        self.results.get(name)
    }
    
    /// Get all benchmark results
    pub fn get_all_results(&self) -> &HashMap<String, BenchmarkResult> {
        &self.results
    }
}