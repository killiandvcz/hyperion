use anyhow::Result;
use std::path::Path;
use hyperion::persistent_store::PersistentStore;
use crate::context::Context;

/// Exécute la commande de connexion
pub fn execute(context: &mut Context, path: &Path) -> Result<()> {
    // Ouvrir le store
    let store = PersistentStore::open(path)?;
    
    // Définir le store dans le contexte
    context.set_store(store);
    
    // Afficher un message de succès
    println!("{}", context.formatter().format_success(&format!("Connecté à la base de données: {}", path.display())));
    
    Ok(())
}