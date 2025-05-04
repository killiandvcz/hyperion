// src/bin/hyperion_cli/commands/connect.rs (modifié)
use anyhow::Result;
use crate::context::Context;

/// Exécute la commande de connexion
pub fn execute(context: &mut Context, server_url: &str) -> Result<()> {
    // Se connecter au serveur
    context.connect(server_url)?;
    
    // Afficher un message de succès
    println!("{}", context.formatter().format_success(&format!("Connecté au serveur: {}", server_url)));
    
    Ok(())
}