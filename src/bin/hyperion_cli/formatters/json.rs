use anyhow::Result;
use colored::*;
use serde_json::json;
use hyperion::value::Value;
use hyperion::entity::Entity;
use crate::formatters::Formatter;
use base64;

/// Formateur au format JSON
pub struct JsonFormatter {
    /// Indique si l'indentation est activée
    pretty: bool,
}

impl JsonFormatter {
    /// Crée un nouveau formateur JSON
    pub fn new() -> Self {
        JsonFormatter {
            pretty: true,
        }
    }
    
    /// Désactive l'indentation
    pub fn without_pretty() -> Self {
        JsonFormatter {
            pretty: false,
        }
    }
}

impl Formatter for JsonFormatter {
    fn format_value(&self, value: &Value) -> Result<String> {
        let json_value = match value {
            Value::Null => json!(null),
            Value::Boolean(b) => json!(b),
            Value::Integer(i) => json!(i),
            Value::Float(f) => json!(f),
            Value::String(s) => json!(s),
            Value::Binary(data, mime) => {
                let base64 = base64::encode(data);
                json!({
                    "type": "binary",
                    "mime": mime,
                    "data": base64
                })
            },
            Value::Reference(path) => {
                json!({
                    "type": "reference",
                    "path": path.to_string()
                })
            }
        };
        
        if self.pretty {
            Ok(serde_json::to_string_pretty(&json_value)?)
        } else {
            Ok(serde_json::to_string(&json_value)?)
        }
    }
    
    fn format_entity(&self, entity: &Entity) -> Result<String> {
        if self.pretty {
            Ok(serde_json::to_string_pretty(entity)?)
        } else {
            Ok(serde_json::to_string(entity)?)
        }
    }
    
    fn format_paths(&self, paths: &[String]) -> Result<String> {
        let json_value = json!(paths);
        
        if self.pretty {
            Ok(serde_json::to_string_pretty(&json_value)?)
        } else {
            Ok(serde_json::to_string(&json_value)?)
        }
    }
    
    fn format_error(&self, error: &str) -> String {
        let json_value = json!({
            "error": error
        });
        
        if self.pretty {
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| format!("{{\"error\":\"{}\"}}",error))
        } else {
            serde_json::to_string(&json_value).unwrap_or_else(|_| format!("{{\"error\":\"{}\"}}",error))
        }
    }
    
    fn format_info(&self, info: &str) -> String {
        let json_value = json!({
            "info": info
        });
        
        if self.pretty {
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| format!("{{\"info\":\"{}\"}}",info))
        } else {
            serde_json::to_string(&json_value).unwrap_or_else(|_| format!("{{\"info\":\"{}\"}}",info))
        }
    }
    
    fn format_success(&self, success: &str) -> String {
        let json_value = json!({
            "success": success
        });
        
        if self.pretty {
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| format!("{{\"success\":\"{}\"}}",success))
        } else {
            serde_json::to_string(&json_value).unwrap_or_else(|_| format!("{{\"success\":\"{}\"}}",success))
        }
    }
}