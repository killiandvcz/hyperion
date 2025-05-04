mod app;
mod context;
mod commands;
mod formatters;
mod repl;
mod utils;
mod client;

use anyhow::Result;
use log::info;

fn main() -> Result<()> {
    // Initialiser le logger
    env_logger::init();
    
    info!("Démarrage de l'application Hyperion CLI");
    
    // Exécuter l'application
    let result = app::run();
    
    info!("Fin de l'application Hyperion CLI");
    
    result
}