mod path;
mod value;
mod store;
mod errors;
mod entity;

use std::str::FromStr;
use path::Path;
use value::Value;
use store::MemoryStore;
use entity::reconstruct_entity;

fn main() {
    // Create a new in-memory store
    let mut db = MemoryStore::new();
    
    println!("Creating users in our endpoint-first database...");
    
    // Store user data as individual endpoints
    db.set(Path::from_str("users.u-123456.username").unwrap(), 
           Value::String("alice".to_string())).unwrap();
           
    db.set(Path::from_str("users.u-123456.email").unwrap(), 
           Value::String("alice@example.com".to_string())).unwrap();
           
    db.set(Path::from_str("users.u-123456.profile.bio").unwrap(), 
           Value::String("Software developer and hobby photographer".to_string())).unwrap();
           
    db.set(Path::from_str("users.u-123456.active").unwrap(), 
           Value::Boolean(true)).unwrap();
    
    // Store another user
    db.set(Path::from_str("users.u-789012.username").unwrap(), 
           Value::String("bob".to_string())).unwrap();
           
    db.set(Path::from_str("users.u-789012.email").unwrap(), 
           Value::String("bob@example.com".to_string())).unwrap();
           
    db.set(Path::from_str("users.u-789012.profile.bio").unwrap(), 
           Value::String("Data scientist and machine learning enthusiast".to_string())).unwrap();
           
    db.set(Path::from_str("users.u-789012.active").unwrap(), 
           Value::Boolean(true)).unwrap();
    
    // Store a third user
    db.set(Path::from_str("users.u-345678.username").unwrap(), 
           Value::String("charlie".to_string())).unwrap();
           
    db.set(Path::from_str("users.u-345678.email").unwrap(), 
           Value::String("charlie@example.com".to_string())).unwrap();
           
    db.set(Path::from_str("users.u-345678.profile.details.bio").unwrap(), 
           Value::String("UX designer with a passion for user-centric design".to_string())).unwrap();
           
    db.set(Path::from_str("users.u-345678.active").unwrap(), 
           Value::Boolean(false)).unwrap();
    
    // Demonstrate single-level wildcard query
    println!("\nQuerying all user emails with single-level wildcard:");
    let email_pattern = Path::from_str("users.*.email").unwrap();
    let email_results = db.query(&email_pattern);
    
    for (path, value) in email_results {
        println!("{} = {}", path, value);
    }
    
    // Demonstrate multi-level wildcard query
    println!("\nQuerying all bios at any nesting level with multi-level wildcard:");
    let bio_pattern = Path::from_str("users.**.bio").unwrap();
    let bio_results = db.query(&bio_pattern);
    
    for (path, value) in bio_results {
        println!("{} = {}", path, value);
    }
    
    // Reconstruct entities from wildcard query
    println!("\nReconstructing all user entities:");
    let users_pattern = Path::from_str("users.*").unwrap();
    let user_paths = db.query_paths(&users_pattern);
    
    for user_path in user_paths {
        println!("\nUser: {}", user_path);
        
        // Reconstruct and print the user entity
        match reconstruct_entity(&db, user_path) {
            Ok(entity) => println!("{}", entity.to_string_pretty(2)),
            Err(e) => println!("Error reconstructing entity: {:?}", e),
        }
    }
    
    // Demonstrate combining wildcards with entity reconstruction
    println!("\nFinding all active users:");
    let active_pattern = Path::from_str("users.*.active").unwrap();
    let active_results = db.query(&active_pattern);
    
    for (path, value) in active_results {
        if let Value::Boolean(true) = value {
            // Extract the user prefix from the path
            let user_prefix = Path::from_str(&path.to_string().replace(".active", "")).unwrap();
            
            // Reconstruct and print the user entity
            println!("\nActive user: {}", user_prefix);
            match reconstruct_entity(&db, &user_prefix) {
                Ok(entity) => println!("{}", entity.to_string_pretty(2)),
                Err(e) => println!("Error reconstructing entity: {:?}", e),
            }
        }
    }
}