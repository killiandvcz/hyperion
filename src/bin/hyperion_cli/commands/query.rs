// src/bin/hyperion_cli/commands/query.rs (modifié)
use anyhow::Result;
use crate::context::Context;

/// Exécute la commande d'exécution de requête
pub fn execute(context: &mut Context, query: &str) -> Result<()> {
    // Vérifier que le contexte est connecté
    let client = context.client()?;
    
    // Exécuter la requête de manière asynchrone via le runtime
    let result = context.runtime().block_on(async {
        client.execute_query(query).await
    })?;
    
    // Formater et afficher le résultat
    let formatted = context.formatter().format_json(&result)?;
    println!("{}", formatted);
    
    Ok(())
}