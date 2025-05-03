//! Example of using HyperionQL
//!
//! This example demonstrates how to use the query language
//! to interact with the database.

use hyperion::persistent_store::PersistentStore;
use hyperion::path::Path;
use hyperion::value::Value;
use hyperion::ql;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a persistent store in a temporary directory
    let db_path = "hyperion_ql_test";
    let store = PersistentStore::open(db_path)?;
    
    // Populate the store with some sample data
    println!("Populating the database with sample data...");
    
    store.set(Path::from_str("users.u-123456.username")?, 
               Value::String("alice".to_string()))?;
               
    store.set(Path::from_str("users.u-123456.email")?, 
               Value::String("alice@example.com".to_string()))?;
               
    store.set(Path::from_str("users.u-123456.active")?, 
               Value::Boolean(true))?;
    
    store.set(Path::from_str("users.u-789012.username")?, 
               Value::String("bob".to_string()))?;
               
    store.set(Path::from_str("users.u-789012.email")?, 
               Value::String("bob@example.com".to_string()))?;
               
    store.set(Path::from_str("users.u-789012.active")?, 
               Value::Boolean(false))?;
    
    // Flush changes to disk
    store.flush()?;
    
    println!("Database populated successfully!");
    
    // Run a simple query to retrieve a value
    println!("\nRunning a simple query to retrieve a value...");
    
    let query1 = r#"{
        return users.u-123456.username
    }"#;
    
    let result1 = ql::execute_query(&store, query1)?;
    println!("Result of query 1: {}", result1);
    
    // Run a more complex query with entity reconstruction
    println!("\nRunning a query with entity reconstruction...");
    
    let query2 = r#"{
        return entity(users.u-123456)
    }"#;
    
    let result2 = ql::execute_query(&store, query2)?;
    println!("Result of query 2: {}", result2);
    
    // Run a query with data modification
    println!("\nRunning a query with data modification...");
    
    let query3 = r#"{
        users.u-123456.last_login = now()
        return users.u-123456.last_login
    }"#;
    
    let result3 = ql::execute_query(&store, query3)?;
    println!("Result of query 3: {}", result3);
    
    // Clean up
    std::fs::remove_dir_all(db_path)?;
    
    Ok(())
}