//! Entity reconstruction for Persistent Store
//!
//! This module provides functionality to reconstruct entities from
//! individual endpoints that share a common path prefix in a persistent store.

use std::collections::HashMap;
use crate::path::Path;
use crate::value::Value;
use crate::entity::Entity;
use crate::errors::{Result, StoreError};
use crate::persistent_store::PersistentStore;

/// Insert a value into the appropriate place in the entity
fn insert_into_entity(
    entity: &mut HashMap<String, Entity>,
    segments: &[String],
    value: Value
) -> Result<()> {
    if segments.is_empty() {
        return Err(StoreError::InvalidOperation("Empty segments".to_string()));
    }
    
    let segment = &segments[0];
    
    // Check if this is an array index
    if let Some(index_str) = segment.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        // Parse the index
        let index = index_str.parse::<usize>().map_err(|_| {
            StoreError::InvalidOperation(format!("Invalid array index: {}", index_str))
        })?;
        
        // Get or create the array
        let array = entity
            .entry("".to_string())
            .or_insert_with(|| Entity::Array(Vec::new()));
        
        // Ensure we have an array
        if let Entity::Array(items) = array {
            // Ensure the array is large enough
            while items.len() <= index {
                items.push(Entity::Null);
            }
            
            if segments.len() == 1 {
                // This is the last segment, set the value directly
                items[index] = Entity::from(value);
            } else {
                // More segments to process
                let next_segments = &segments[1..];
                
                // Get or create an object at this index
                if let Entity::Null = items[index] {
                    items[index] = Entity::Object(HashMap::new());
                }
                
                if let Entity::Object(ref mut obj) = items[index] {
                    insert_into_entity(obj, next_segments, value)?;
                } else {
                    return Err(StoreError::InvalidOperation(
                        format!("Cannot insert at path: expected object, found {}", segment)
                    ));
                }
            }
        } else {
            return Err(StoreError::InvalidOperation(
                format!("Cannot insert at path: expected array, found {}", segment)
            ));
        }
        
        return Ok(());
    }
    
    if segments.len() == 1 {
        // This is the last segment, set the value directly
        entity.insert(segment.clone(), Entity::from(value));
    } else {
        // More segments to process
        let next_segments = &segments[1..];
        
        // Get or create an object at this key
        let nested = entity
            .entry(segment.clone())
            .or_insert_with(|| Entity::Object(HashMap::new()));
        
        if let Entity::Object(ref mut obj) = nested {
            insert_into_entity(obj, next_segments, value)?;
        } else {
            return Err(StoreError::InvalidOperation(
                format!("Cannot insert at path: expected object, found {}", segment)
            ));
        }
    }
    
    Ok(())
}

/// Get the remaining path segments after the prefix
fn get_remaining_segments(path: &Path, prefix: &Path) -> Vec<String> {
    let path_segments = path.segments();
    let prefix_segments = prefix.segments();
    
    path_segments[prefix_segments.len()..]
        .iter()
        .map(|s| s.as_str())
        .collect()
}

/// Reconstruct an entity from a collection of endpoints in a persistent store
pub fn reconstruct_entity(store: &PersistentStore, prefix: &Path) -> Result<Entity> {
    // Get all endpoints under the prefix
    let endpoints = store.get_prefix(prefix)?;
    
    if endpoints.is_empty() {
        return Err(StoreError::NotFound(prefix.clone()));
    }
    
    // If there's only one endpoint and it's exactly the prefix,
    // return it directly as an entity
    if endpoints.len() == 1 {
        let (path, value) = &endpoints[0];
        if path == prefix {
            return Ok(Entity::from(value.clone()));
        }
    }
    
    // Start with an empty object
    let mut result = HashMap::new();
    
    // Process each endpoint
    for (path, value) in endpoints {
        // Skip paths that don't start with the prefix
        if !path.starts_with(prefix) {
            continue;
        }
        
        // Get the remaining segments after the prefix
        let remaining_segments = get_remaining_segments(&path, prefix);
        
        // Insert the value into the appropriate place in the result
        insert_into_entity(&mut result, &remaining_segments, value)?;
    }
    
    Ok(Entity::Object(result))
}