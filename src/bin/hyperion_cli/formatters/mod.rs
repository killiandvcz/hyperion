pub mod formatter;
pub mod text;
pub mod json;
pub mod table;

pub use formatter::Formatter;
use clap::ValueEnum;

/// Formats de sortie disponibles
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Format texte
    Text,
    
    /// Format JSON
    Json,
    
    /// Format tableau
    Table,
}