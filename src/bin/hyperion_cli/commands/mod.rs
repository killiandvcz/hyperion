pub mod connect;
pub mod query;
pub mod list;

use anyhow::Result;
use crate::context::Context;

/// Trait définissant une commande
pub trait Command {
    /// Exécute la commande
    fn execute(&self, context: &mut Context) -> Result<()>;
}