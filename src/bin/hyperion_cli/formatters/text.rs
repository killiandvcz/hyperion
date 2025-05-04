use anyhow::Result;
use colored::*;
use hyperion::Value;
use hyperion::Entity;
use crate::formatters::Formatter;

/// Formateur au format texte
pub struct TextFormatter {
    /// Indique si les couleurs sont activées
    colored: bool,
}

impl TextFormatter {
    /// Crée un nouveau formateur texte
    pub fn new() -> Self {
        TextFormatter {
            colored: true,
        }
    }
    
    /// Désactive les couleurs
    pub fn without_colors() -> Self {
        TextFormatter {
            colored: false,
        }
    }
}

impl Formatter for TextFormatter {
    // Ajouter cette méthode à l'implémentation de Formatter pour TextFormatter
    fn format_json(&self, json: &serde_json::Value) -> Result<String> {
        // Pour le format texte, on peut utiliser une représentation lisible
        // avec indentation pour une meilleure lisibilité
        match json {
            serde_json::Value::Null => Ok("null".to_string()),
            serde_json::Value::Bool(b) => Ok(b.to_string()),
            serde_json::Value::Number(n) => Ok(n.to_string()),
            serde_json::Value::String(s) => Ok(format!("\"{}\"", s)),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                // Pour les structures complexes, utilisez pretty printing
                Ok(serde_json::to_string_pretty(json)?)
            }
        }
    }
    
    fn format_value(&self, value: &Value) -> Result<String> {
        Ok(format!("{}", value))
    }
    
    fn format_entity(&self, entity: &Entity) -> Result<String> {
        Ok(entity.to_string_pretty(0))
    }
    
    fn format_paths(&self, paths: &[String]) -> Result<String> {
        Ok(paths.join("\n"))
    }
    
    fn format_error(&self, error: &str) -> String {
        if self.colored {
            format!("{}", error.red().bold())
        } else {
            format!("Erreur: {}", error)
        }
    }
    
    fn format_info(&self, info: &str) -> String {
        if self.colored {
            format!("{}", info.blue())
        } else {
            format!("Info: {}", info)
        }
    }
    
    fn format_success(&self, success: &str) -> String {
        if self.colored {
            format!("{}", success.green().bold())
        } else {
            format!("Succès: {}", success)
        }
    }
}