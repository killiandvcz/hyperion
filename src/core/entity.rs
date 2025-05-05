//! Entity reconstruction for Hyperion
//!
//! This module provides functionality to reconstruct entities from
//! individual endpoints that share a common path prefix.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use super::path::Path;
use super::value::Value;
use super::errors::{Result, StoreError};
use super::store::Store;

/// A reconstructed entity from the database
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Entity {
    /// Null value
    Null,
    /// Boolean value
    Boolean(bool),
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// String value
    String(String),
    /// Binary data with optional MIME type
    Binary(Vec<u8>, Option<String>),
    /// Reference to another path
    Reference(Path),
    /// Object with named fields
    Object(HashMap<String, Entity>),
    /// Array of values
    Array(Vec<Entity>),
}

impl From<Value> for Entity {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => Entity::Null,
            Value::Boolean(b) => Entity::Boolean(b),
            Value::Integer(i) => Entity::Integer(i),
            Value::Float(f) => Entity::Float(f),
            Value::String(s) => Entity::String(s),
            Value::Binary(data, mime) => Entity::Binary(data, mime),
            Value::Reference(path) => Entity::Reference(path),
        }
    }
}

impl Entity {
    /// Convert the entity to a debug string representation
    pub fn to_string_pretty(&self, indent: usize) -> String {
        match self {
            Entity::Null => "null".to_string(),
            Entity::Boolean(b) => b.to_string(),
            Entity::Integer(i) => i.to_string(),
            Entity::Float(f) => f.to_string(),
            Entity::String(s) => format!("\"{}\"", s),
            Entity::Binary(_, mime) => {
                if let Some(m) = mime {
                    format!("[binary data: {}]", m)
                } else {
                    "[binary data]".to_string()
                }
            },
            Entity::Reference(path) => format!("@{}", path),
            Entity::Object(map) => {
                if map.is_empty() {
                    return "{}".to_string();
                }
                
                let mut result = "{\n".to_string();
                
                for (key, value) in map {
                    let indentation = " ".repeat(indent + 2);
                    let value_str = value.to_string_pretty(indent + 2);
                    result.push_str(&format!("{}\"{}\": {},\n", indentation, key, value_str));
                }
                
                // Remove trailing comma and newline
                if !map.is_empty() {
                    result.pop();
                    result.pop();
                    result.push('\n');
                }
                
                result.push_str(&" ".repeat(indent));
                result.push('}');
                
                result
            },
            Entity::Array(items) => {
                if items.is_empty() {
                    return "[]".to_string();
                }
                
                let mut result = "[\n".to_string();
                
                for item in items {
                    let indentation = " ".repeat(indent + 2);
                    let item_str = item.to_string_pretty(indent + 2);
                    result.push_str(&format!("{}{},\n", indentation, item_str));
                }
                
                // Remove trailing comma and newline
                if !items.is_empty() {
                    result.pop();
                    result.pop();
                    result.push('\n');
                }
                
                result.push_str(&" ".repeat(indent));
                result.push(']');
                
                result
            },
        }
    }
}

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
        .map(|s| s.as_str().to_string())
        .collect()
}

/// Reconstruct an entity from a collection of endpoints in a store
pub fn reconstruct_entity<S: Store + ?Sized>(store: &S, prefix: &Path) -> Result<Entity> {
    // Get all endpoints under the prefix
    let endpoints = store.get_prefix(prefix)?;
    println!("Endpoints: {:?}", endpoints);
    
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