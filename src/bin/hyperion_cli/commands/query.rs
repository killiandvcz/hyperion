use anyhow::Result;
use hyperion::ql;
use crate::context::Context;

/// Exécute la commande d'exécution de requête
pub fn execute(context: &mut Context, query: &str) -> Result<()> {
    // Vérifier que le contexte est connecté
    let store = context.store()?;
    
    // Exécuter la requête
    let result = ql::execute_query(&store, query)?;
    
    // Formater et afficher le résultat
    let formatted = context.formatter().format_value(&result)?;
    println!("{}", formatted);
    
    Ok(())
}