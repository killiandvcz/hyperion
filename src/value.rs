//! Value module for NanoDB
//!
//! This module defines the Value enum, representing different types
//! of values that can be stored at database endpoints.

use std::fmt;
use crate::path::Path;

/// The different types of values that can be stored in the database
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
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
}

impl Value {
    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
    
    /// Check if the value is a boolean
    pub fn is_boolean(&self) -> bool {
        matches!(self, Value::Boolean(_))
    }
    
    /// Check if the value is an integer
    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }
    
    /// Check if the value is a float
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }
    
    /// Check if the value is a number (integer or float)
    pub fn is_number(&self) -> bool {
        self.is_integer() || self.is_float()
    }
    
    /// Check if the value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }
    
    /// Check if the value is binary data
    pub fn is_binary(&self) -> bool {
        matches!(self, Value::Binary(_, _))
    }
    
    /// Check if the value is a reference
    pub fn is_reference(&self) -> bool {
        matches!(self, Value::Reference(_))
    }
    
    /// Get a string representation of the value's type
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Boolean(_) => "boolean",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Binary(_, _) => "binary",
            Value::Reference(_) => "reference",
        }
    }
}

/// Format a Value as a string
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Binary(_, mime) => {
                if let Some(m) = mime {
                    write!(f, "[binary data: {}]", m)
                } else {
                    write!(f, "[binary data]")
                }
            },
            Value::Reference(path) => write!(f, "@{}", path),
        }
    }
}

/// Convert from common types to Value
impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Integer(i64::from(i))
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Integer(i)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Boolean(b)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[test]
    fn test_value_types() {
        let null = Value::Null;
        let boolean = Value::Boolean(true);
        let integer = Value::Integer(42);
        let float = Value::Float(3.14);
        let string = Value::String("Hello".to_string());
        let binary = Value::Binary(vec![1, 2, 3], Some("image/jpeg".to_string()));
        let reference = Value::Reference(Path::from_str("users.u-123456").unwrap());
        
        assert!(null.is_null());
        assert!(boolean.is_boolean());
        assert!(integer.is_integer());
        assert!(float.is_float());
        assert!(integer.is_number());
        assert!(float.is_number());
        assert!(string.is_string());
        assert!(binary.is_binary());
        assert!(reference.is_reference());
    }
    
    #[test]
    fn test_value_conversion() {
        let int_value: Value = 42.into();
        let bool_value: Value = true.into();
        let string_value: Value = "Hello".into();
        
        assert_eq!(int_value, Value::Integer(42));
        assert_eq!(bool_value, Value::Boolean(true));
        assert_eq!(string_value, Value::String("Hello".to_string()));
    }
    
    #[test]
    fn test_value_display() {
        let null = Value::Null;
        let boolean = Value::Boolean(true);
        let integer = Value::Integer(42);
        let float = Value::Float(3.14);
        let string = Value::String("Hello".to_string());
        
        assert_eq!(null.to_string(), "null");
        assert_eq!(boolean.to_string(), "true");
        assert_eq!(integer.to_string(), "42");
        assert_eq!(float.to_string(), "3.14");
        assert_eq!(string.to_string(), "\"Hello\"");
    }
}