use anyhow::Result;
use colored::*;
use hyperion::value::Value;
use hyperion::entity::Entity;
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