mod path;
mod value;
mod store;
mod errors;
mod entity;
mod persistent_store;
mod index;  // <-- Ajout de cette ligne
mod wildcard_index;

use hyperion::path::Path;
use hyperion::value::Value;
use hyperion::entity::reconstruct_entity;
use hyperion::persistent_store::PersistentStore;
use std::str::FromStr;

fn main() {
    // Create a persistent store in the "hyperiondb" directory
    let db = PersistentStore::open("hyperiondb").unwrap();
    
    println!("Creating users in our persistent endpoint-first database...");
    
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
    
    // Flush changes to disk
    db.flush().unwrap();
    println!("Data has been persisted to disk!");
    
    // Demonstrate querying data from the persistent store
    println!("\nQuerying all user emails with single-level wildcard:");
    let email_pattern = Path::from_str("users.*.email").unwrap();
    let email_results = db.query(&email_pattern).unwrap();
    
    for (path, value) in email_results {
        println!("{} = {}", path, value);
    }
    
    // Count the number of entries
    let count = db.count().unwrap();
    println!("\nTotal number of endpoints in the database: {}", count);
    
    // Demonstrate retrieving and reconstructing entities
    let user_paths = db.list_prefix(&Path::from_str("users").unwrap()).unwrap();
    println!("\nUser paths in the database:");
    for path in &user_paths {
        println!("- {}", path);
    }
    
    // Show that data persists by closing and reopening the database
    println!("\nClosing and reopening the database to demonstrate persistence...");
    std::mem::drop(db);
    
    // Reopen the database
    let reopened_db = PersistentStore::open("hyperiondb").unwrap();
    
    // Verify the data is still there
    let username = reopened_db.get(&Path::from_str("users.u-123456.username").unwrap()).unwrap();
    println!("After reopening - Username: {}", username);
    
    // Query using wildcards on the reopened database
    println!("\nQuerying all bios with multi-level wildcard after reopening:");
    let bio_pattern = Path::from_str("users.**.bio").unwrap();
    let bio_results = reopened_db.query(&bio_pattern).unwrap();
    
    for (path, value) in bio_results {
        println!("{} = {}", path, value);
    }
    
    // Clean up by removing the database files (optional, comment out to keep the database)
//     println!("\nCleaning up database files...");
//     std::fs::remove_dir_all("hyperiondb").unwrap();
}