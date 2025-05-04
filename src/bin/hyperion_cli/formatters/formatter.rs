use anyhow::Result;
use hyperion::value::Value;
use hyperion::entity::Entity;

/// Trait définissant un formateur de sortie
pub trait Formatter {
    /// Formate une valeur pour l'affichage
    fn format_value(&self, value: &Value) -> Result<String>;
    
    /// Formate une entité pour l'affichage
    fn format_entity(&self, entity: &Entity) -> Result<String>;
    
    /// Formate une liste de chemins pour l'affichage
    fn format_paths(&self, paths: &[String]) -> Result<String>;
    
    /// Formate un message d'erreur
    fn format_error(&self, error: &str) -> String;
    
    /// Formate un message d'information
    fn format_info(&self, info: &str) -> String;
    
    /// Formate un message de succès
    fn format_success(&self, success: &str) -> String;
}