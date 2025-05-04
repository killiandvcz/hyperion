use thiserror::Error;
use hyperion::errors::StoreError;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Erreur de base de données: {0}")]
    StoreError(#[from] StoreError),
    
    #[error("Erreur d'entrée/sortie: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Erreur de format: {0}")]
    FormatError(String),
    
    #[error("Commande inconnue: {0}")]
    UnknownCommand(String),
    
    #[error("Non connecté à une base de données")]
    NotConnected,
    
    #[error("Erreur: {0}")]
    Other(String),
}

impl From<String> for CliError {
    fn from(s: String) -> Self {
        CliError::Other(s)
    }
}

impl From<&str> for CliError {
    fn from(s: &str) -> Self {
        CliError::Other(s.to_string())
    }
}