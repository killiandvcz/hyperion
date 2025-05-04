use anyhow::{Result, anyhow};
use colored::*;
use prettytable::{Table, Row, Cell};
use hyperion::Value;
use hyperion::Entity;
use crate::formatters::Formatter;

/// Formateur au format tableau
pub struct TableFormatter {
    /// Indique si les couleurs sont activées
    colored: bool,
}

impl TableFormatter {
    /// Crée un nouveau formateur tableau
    pub fn new() -> Self {
        TableFormatter {
            colored: true,
        }
    }
    
    /// Désactive les couleurs
    pub fn without_colors() -> Self {
        TableFormatter {
            colored: false,
        }
    }
}

impl Formatter for TableFormatter {
    // Ajouter cette méthode à l'implémentation de Formatter pour TableFormatter
    fn format_json(&self, json: &serde_json::Value) -> Result<String> {
        match json {
            // Pour les valeurs scalaires, utilisez un tableau simple
            serde_json::Value::Null | 
            serde_json::Value::Bool(_) | 
            serde_json::Value::Number(_) | 
            serde_json::Value::String(_) => {
                let mut table = Table::new();
                
                // En-têtes
                table.add_row(Row::new(vec![
                    Cell::new("Type"),
                    Cell::new("Valeur")
                    ]));
                    
                    // Valeur
                    let type_name = match json {
                        serde_json::Value::Null => "null",
                        serde_json::Value::Bool(_) => "boolean",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::String(_) => "string",
                        _ => unreachable!(),
                    };
                    
                    let value_str = match json {
                        serde_json::Value::Null => "null".to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::String(s) => format!("\"{}\"", s),
                        _ => unreachable!(),
                    };
                    
                    table.add_row(Row::new(vec![
                        Cell::new(type_name),
                        Cell::new(&value_str)
                        ]));
                        
                        Ok(table.to_string())
                    },
                    serde_json::Value::Array(items) => {
                        let mut table = Table::new();
                        
                        // En-têtes
                        table.add_row(Row::new(vec![
                            Cell::new("Index"),
                            Cell::new("Type"),
                            Cell::new("Valeur")
                            ]));
                            
                            // Ajouter chaque élément
                            for (i, item) in items.iter().enumerate() {
                                let type_name = match item {
                                    serde_json::Value::Null => "null",
                                    serde_json::Value::Bool(_) => "boolean",
                                    serde_json::Value::Number(_) => "number",
                                    serde_json::Value::String(_) => "string",
                                    serde_json::Value::Array(_) => "array",
                                    serde_json::Value::Object(_) => "object",
                                };
                                
                                let value_str = match item {
                                    serde_json::Value::Null => "null".to_string(),
                                    serde_json::Value::Bool(b) => b.to_string(),
                                    serde_json::Value::Number(n) => n.to_string(),
                                    serde_json::Value::String(s) => {
                                        // Tronquer les chaînes longues
                                        if s.len() > 50 {
                                            format!("\"{}...\"", &s[..47])
                                        } else {
                                            format!("\"{}\"", s)
                                        }
                                    },
                                    serde_json::Value::Array(_) => "[array]".to_string(),
                                    serde_json::Value::Object(_) => "{object}".to_string(),
                                };
                                
                                table.add_row(Row::new(vec![
                                    Cell::new(&i.to_string()),
                                    Cell::new(type_name),
                                    Cell::new(&value_str)
                                    ]));
                                }
                                
                                Ok(table.to_string())
                            },
                            serde_json::Value::Object(map) => {
                                let mut table = Table::new();
                                
                                // En-têtes
                                table.add_row(Row::new(vec![
                                    Cell::new("Clé"),
                                    Cell::new("Type"),
                                    Cell::new("Valeur")
                                    ]));
                                    
                                    // Ajouter chaque paire clé-valeur
                                    for (key, value) in map {
                                        let type_name = match value {
                                            serde_json::Value::Null => "null",
                                            serde_json::Value::Bool(_) => "boolean",
                                            serde_json::Value::Number(_) => "number",
                                            serde_json::Value::String(_) => "string",
                                            serde_json::Value::Array(_) => "array",
                                            serde_json::Value::Object(_) => "object",
                                        };
                                        
                                        let value_str = match value {
                                            serde_json::Value::Null => "null".to_string(),
                                            serde_json::Value::Bool(b) => b.to_string(),
                                            serde_json::Value::Number(n) => n.to_string(),
                                            serde_json::Value::String(s) => {
                                                // Tronquer les chaînes longues
                                                if s.len() > 50 {
                                                    format!("\"{}...\"", &s[..47])
                                                } else {
                                                    format!("\"{}\"", s)
                                                }
                                            },
                                            serde_json::Value::Array(_) => "[array]".to_string(),
                                            serde_json::Value::Object(_) => "{object}".to_string(),
                                        };
                                        
                                        table.add_row(Row::new(vec![
                                            Cell::new(key),
                                            Cell::new(type_name),
                                            Cell::new(&value_str)
                                            ]));
                                        }
                                        
                                        Ok(table.to_string())
                                    }
                                }
                            }
                            
                            fn format_value(&self, value: &Value) -> Result<String> {
                                // Création d'un tableau simple pour les valeurs scalaires
                                let mut table = Table::new();
                                
                                table.add_row(Row::new(vec![
                                    Cell::new("Type"),
                                    Cell::new("Valeur")
                                    ]));
                                    
                                    table.add_row(Row::new(vec![
                                        Cell::new(value.type_name()),
                                        Cell::new(&format!("{}", value))
                                        ]));
                                        
                                        Ok(table.to_string())
                                    }
                                    
                                    fn format_entity(&self, entity: &Entity) -> Result<String> {
                                        match entity {
                                            Entity::Object(map) => {
                                                let mut table = Table::new();
                                                
                                                // En-têtes
                                                table.add_row(Row::new(vec![
                                                    Cell::new("Chemin"),
                                                    Cell::new("Type"),
                                                    Cell::new("Valeur")
                                                    ]));
                                                    
                                                    // Ajouter chaque propriété
                                                    for (key, value) in map {
                                                        table.add_row(Row::new(vec![
                                                            Cell::new(key),
                                                            Cell::new(&format!("{:?}", value)),
                                                            Cell::new(&entity_value_to_string(value))
                                                            ]));
                                                        }
                                                        
                                                        Ok(table.to_string())
                                                    },
                                                    Entity::Array(items) => {
                                                        let mut table = Table::new();
                                                        
                                                        // En-têtes
                                                        table.add_row(Row::new(vec![
                                                            Cell::new("Index"),
                                                            Cell::new("Type"),
                                                            Cell::new("Valeur")
                                                            ]));
                                                            
                                                            // Ajouter chaque élément
                                                            for (i, item) in items.iter().enumerate() {
                                                                table.add_row(Row::new(vec![
                                                                    Cell::new(&i.to_string()),
                                                                    Cell::new(&format!("{:?}", item)),
                                                                    Cell::new(&entity_value_to_string(item))
                                                                    ]));
                                                                }
                                                                
                                                                Ok(table.to_string())
                                                            },
                                                            _ => Err(anyhow!("Impossible d'afficher une entité de type scalaire sous forme de tableau")),
                                                        }
                                                    }
                                                    
                                                    fn format_paths(&self, paths: &[String]) -> Result<String> {
                                                        let mut table = Table::new();
                                                        
                                                        // En-têtes
                                                        table.add_row(Row::new(vec![
                                                            Cell::new("Index"),
                                                            Cell::new("Chemin")
                                                            ]));
                                                            
                                                            // Ajouter chaque chemin
                                                            for (i, path) in paths.iter().enumerate() {
                                                                table.add_row(Row::new(vec![
                                                                    Cell::new(&i.to_string()),
                                                                    Cell::new(path)
                                                                    ]));
                                                                }
                                                                
                                                                Ok(table.to_string())
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
                                                        
                                                        /// Convertit une entité en chaîne de caractères pour l'affichage
                                                        fn entity_value_to_string(entity: &Entity) -> String {
                                                            match entity {
                                                                Entity::Null => "null".to_string(),
                                                                Entity::Boolean(b) => b.to_string(),
                                                                Entity::Integer(i) => i.to_string(),
                                                                Entity::Float(f) => f.to_string(),
                                                                Entity::String(s) => s.clone(),
                                                                Entity::Binary(_, mime) => {
                                                                    if let Some(m) = mime {
                                                                        format!("[binary data: {}]", m)
                                                                    } else {
                                                                        "[binary data]".to_string()
                                                                    }
                                                                },
                                                                Entity::Reference(path) => format!("@{}", path),
                                                                Entity::Object(_) => "[object]".to_string(),
                                                                Entity::Array(_) => "[array]".to_string(),
                                                            }
                                                        }