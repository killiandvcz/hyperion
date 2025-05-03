//! Entity reconstruction for NanoDB
//!
//! This module provides functionality to reconstruct entities from
//! individual endpoints that share a common path prefix.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::path::Path;
use crate::value::Value;
use crate::errors::{Result, StoreError};
use crate::store::MemoryStore;

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

/// Reconstruct an entity from a collection of endpoints
pub fn reconstruct_entity(store: &MemoryStore, prefix: &Path) -> Result<Entity> {
    // Get all endpoints under the prefix
    let endpoints = store.get_prefix(prefix);
    
    if endpoints.is_empty() {
        return Err(StoreError::NotFound(prefix.clone()));
    }
    
    // If there's only one endpoint and it's exactly the prefix,
    // return it directly as an entity
    if endpoints.len() == 1 {
        if let Some((path, value)) = endpoints.iter().next() {
            if *path == prefix {
                return Ok(Entity::from((*value).clone()));
            }
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
        let remaining_segments = get_remaining_segments(path, prefix);
        
        // Insert the value into the appropriate place in the result
        insert_into_entity(&mut result, &remaining_segments, (*value).clone())?;
    }
    
    Ok(Entity::Object(result))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[test]
    fn test_basic_entity_reconstruction() {
        let mut store = MemoryStore::new();
        
        // Add several paths under the same prefix
        store.set(Path::from_str("users.u-123456.username").unwrap(), 
                 Value::String("alice".to_string())).unwrap();
        store.set(Path::from_str("users.u-123456.email").unwrap(), 
                 Value::String("alice@example.com".to_string())).unwrap();
        store.set(Path::from_str("users.u-123456.active").unwrap(), 
                 Value::Boolean(true)).unwrap();
        
        let prefix = Path::from_str("users.u-123456").unwrap();
        let entity = reconstruct_entity(&store, &prefix).unwrap();
        
        if let Entity::Object(map) = entity {
            assert_eq!(map.len(), 3);
            
            if let Some(Entity::String(username)) = map.get("username") {
                assert_eq!(username, "alice");
            } else {
                panic!("Username not found or wrong type");
            }
            
            if let Some(Entity::String(email)) = map.get("email") {
                assert_eq!(email, "alice@example.com");
            } else {
                panic!("Email not found or wrong type");
            }
            
            if let Some(Entity::Boolean(active)) = map.get("active") {
                assert!(active);
            } else {
                panic!("Active flag not found or wrong type");
            }
        } else {
            panic!("Expected object entity");
        }
    }
    
    #[test]
    fn test_nested_entity_reconstruction() {
        let mut store = MemoryStore::new();
        
        store.set(Path::from_str("users.u-123456.username").unwrap(), 
                 Value::String("alice".to_string())).unwrap();
        store.set(Path::from_str("users.u-123456.profile.bio").unwrap(), 
                 Value::String("Software developer".to_string())).unwrap();
        store.set(Path::from_str("users.u-123456.profile.location").unwrap(), 
                 Value::String("San Francisco, CA".to_string())).unwrap();
        
        let prefix = Path::from_str("users.u-123456").unwrap();
        let entity = reconstruct_entity(&store, &prefix).unwrap();
        
        if let Entity::Object(map) = entity {
            assert_eq!(map.len(), 2); // username and profile
            
            if let Some(Entity::Object(profile)) = map.get("profile") {
                assert_eq!(profile.len(), 2); // bio and location
                
                if let Some(Entity::String(bio)) = profile.get("bio") {
                    assert_eq!(bio, "Software developer");
                } else {
                    panic!("Bio not found or wrong type");
                }
                
                if let Some(Entity::String(location)) = profile.get("location") {
                    assert_eq!(location, "San Francisco, CA");
                } else {
                    panic!("Location not found or wrong type");
                }
            } else {
                panic!("Profile not found or wrong type");
            }
        } else {
            panic!("Expected object entity");
        }
    }
    
    #[test]
    fn test_array_entity_reconstruction() {
        let mut store = MemoryStore::new();
        
        store.set(Path::from_str("users.u-123456.username").unwrap(), 
                 Value::String("alice".to_string())).unwrap();
        store.set(Path::from_str("users.u-123456.tags[0]").unwrap(), 
                 Value::String("developer".to_string())).unwrap();
        store.set(Path::from_str("users.u-123456.tags[1]").unwrap(), 
                 Value::String("rust".to_string())).unwrap();
        store.set(Path::from_str("users.u-123456.tags[2]").unwrap(), 
                 Value::String("databases".to_string())).unwrap();
        
        let prefix = Path::from_str("users.u-123456").unwrap();
        let entity = reconstruct_entity(&store, &prefix).unwrap();
        
        if let Entity::Object(map) = entity {
            assert_eq!(map.len(), 2); // username and tags
            
            if let Some(Entity::Array(tags)) = map.get("tags") {
                assert_eq!(tags.len(), 3);
                
                if let Entity::String(tag0) = &tags[0] {
                    assert_eq!(tag0, "developer");
                } else {
                    panic!("First tag not found or wrong type");
                }
                
                if let Entity::String(tag1) = &tags[1] {
                    assert_eq!(tag1, "rust");
                } else {
                    panic!("Second tag not found or wrong type");
                }
                
                if let Entity::String(tag2) = &tags[2] {
                    assert_eq!(tag2, "databases");
                } else {
                    panic!("Third tag not found or wrong type");
                }
            } else {
                panic!("Tags not found or wrong type");
            }
        } else {
            panic!("Expected object entity");
        }
    }
}