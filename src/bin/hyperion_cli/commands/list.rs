// src/bin/hyperion_cli/commands/list.rs (modifié)
use anyhow::Result;
use crate::context::Context;

/// Exécute la commande de listage des chemins
pub fn execute(context: &mut Context, prefix: Option<&str>) -> Result<()> {
    // Vérifier que le contexte est connecté
    let client = context.client()?;
    
    // Créer le préfixe (chaîne vide si non spécifié)
    let prefix_str = prefix.unwrap_or("");
    
    // Lister les chemins de manière asynchrone via le runtime
    let paths: Vec<String> = context.runtime().block_on(async {
        client.list_paths(prefix_str).await
    })?;
    
    // Formater et afficher les chemins
    let formatted = context.formatter().format_paths(&paths)?;
    println!("{}", formatted);
    
    Ok(())
}