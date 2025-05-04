// src/bin/hyperion_cli/context.rs (modifié)
use std::sync::Arc;
use tokio::runtime::Runtime;
use crate::client::{HyperionClient, ClientConfig};
use crate::formatters::{OutputFormat, Formatter};
use crate::formatters::text::TextFormatter;
use crate::formatters::json::JsonFormatter;
use crate::formatters::table::TableFormatter;
use anyhow::{Result, anyhow};

/// Contexte d'exécution du CLI
pub struct Context {
    /// Client Hyperion
    client: Option<HyperionClient>,
    
    /// Format de sortie
    format: OutputFormat,
    
    /// Niveau de verbosité
    verbosity: u8,
    
    /// Formateur actuel
    formatter: Box<dyn Formatter>,
    
    /// Runtime Tokio pour les appels asynchrones
    runtime: Runtime,
}

impl Context {
    /// Crée un nouveau contexte
    pub fn new(verbosity: u8, format: OutputFormat) -> Result<Self> {
        let formatter: Box<dyn Formatter> = match format {
            OutputFormat::Text => Box::new(TextFormatter::new()),
            OutputFormat::Json => Box::new(JsonFormatter::new()),
            OutputFormat::Table => Box::new(TableFormatter::new()),
        };
        
        // Créer un runtime Tokio
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| anyhow!("Failed to create Tokio runtime: {}", e))?;
        
        Ok(Context {
            client: None,
            format,
            verbosity,
            formatter,
            runtime,
        })
    }
    
    /// Vérifie si le contexte est connecté à un serveur
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }
    
    /// Obtient une référence au client
    pub fn client(&self) -> Result<&HyperionClient> {
        self.client.as_ref().ok_or_else(|| anyhow!("Non connecté à un serveur Hyperion"))
    }
    
    /// Définit le client
    pub fn connect(&mut self, server_url: &str) -> Result<()> {
        let config = ClientConfig {
            server_url: server_url.to_string(),
        };
        
        let client = HyperionClient::with_config(config);
        
        // Vérifier la connexion
        self.runtime.block_on(async {
            client.check_connection().await
        })?;
        
        self.client = Some(client);
        Ok(())
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
    
    /// Obtient le runtime Tokio
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }
}