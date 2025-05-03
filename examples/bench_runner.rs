//! Benchmark runner for Hyperion
//!
//! This example demonstrates how to use the benchmark module
//! to measure performance of the database.

use std::fs;
use hyperion::persistent_store::PersistentStore;
use hyperion::bench::Benchmark;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hyperion Database Benchmark Suite");
    println!("=================================");
    
    // Create a temporary directory for benchmarking
    let bench_dir = "hyperion_benchmark";
    
    // Remove any existing benchmark directory
    let _ = fs::remove_dir_all(bench_dir);
    
    // Create a fresh database for benchmarking
    let store = PersistentStore::open(bench_dir)?;
    
    // Create a benchmark instance
    let mut benchmark = Benchmark::new(store);
    
    // Run path benchmarks
    benchmark.run_path_benchmarks()?;
    
    // Run storage benchmarks with 10,000 operations
    benchmark.run_storage_benchmarks(10_000)?;
    
    // Run batcher benchmarks with 5,000 operations
    benchmark.run_batcher_benchmarks(5_000)?;
    
    // Print results
    benchmark.print_results();
    
    println!("\nBenchmark completed successfully.");
    
    // Clean up benchmark directory
    let _ = fs::remove_dir_all(bench_dir);
    
    Ok(())
}