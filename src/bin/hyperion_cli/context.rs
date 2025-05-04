use std::sync::Arc;
use hyperion::persistent_store::PersistentStore;
use crate::formatters::{OutputFormat, Formatter};
use crate::formatters::text::TextFormatter;
use crate::formatters::json::JsonFormatter;
use crate::formatters::table::TableFormatter;
use anyhow::{Result, anyhow};

/// Contexte d'exécution du CLI
pub struct Context {
    /// Magasin persistant Hyperion
    store: Option<Arc<PersistentStore>>,
    
    /// Format de sortie
    format: OutputFormat,
    
    /// Niveau de verbosité
    verbosity: u8,
    
    /// Formateur actuel
    formatter: Box<dyn Formatter>,
}

impl Context {
    /// Crée un nouveau contexte
    pub fn new(verbosity: u8, format: OutputFormat) -> Self {
        let formatter: Box<dyn Formatter> = match format {
            OutputFormat::Text => Box::new(TextFormatter::new()),
            OutputFormat::Json => Box::new(JsonFormatter::new()),
            OutputFormat::Table => Box::new(TableFormatter::new()),
        };
        
        Context {
            store: None,
            format,
            verbosity,
            formatter,
        }
    }
    
    /// Vérifie si le contexte est connecté à une base de données
    pub fn is_connected(&self) -> bool {
        self.store.is_some()
    }
    
    /// Obtient une référence au store
    pub fn store(&self) -> Result<Arc<PersistentStore>> {
        self.store.clone().ok_or_else(|| anyhow!("Non connecté à une base de données"))
    }
    
    /// Définit le store
    pub fn set_store(&mut self, store: PersistentStore) {
        self.store = Some(Arc::new(store));
    }
    
    /// Obtient le formateur actuel
    pub fn formatter(&self) -> &dyn Formatter {
        self.formatter.as_ref()
    }
    
    /// Définit le format de sortie
    pub fn set_format(&mut self, format: OutputFormat) {
        if format != self.format {
            self.format = format;
            self.formatter = match format {
                OutputFormat::Text => Box::new(TextFormatter::new()),
                OutputFormat::Json => Box::new(JsonFormatter::new()),
                OutputFormat::Table => Box::new(TableFormatter::new()),
            };
        }
    }
    
    /// Obtient le niveau de verbosité
    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }
    
    /// Définit le niveau de verbosité
    pub fn set_verbosity(&mut self, verbosity: u8) {
        self.verbosity = verbosity;
    }
}