use anyhow::Result;
use hyperion::path::Path;
use std::str::FromStr;
use crate::context::Context;

/// Exécute la commande de listage des chemins
pub fn execute(context: &mut Context, prefix: Option<&str>) -> Result<()> {
    // Vérifier que le contexte est connecté
    let store = context.store()?;
    
    // Créer le préfixe
    let prefix_path = match prefix {
        Some(p) => Path::from_str(p)?,
        None => Path::from_str("")?,
    };
    
    // Lister les chemins
    let paths = store.list_prefix(&prefix_path)?;
    
    // Convertir les chemins en chaînes
    let path_strings: Vec<String> = paths.iter().map(|p| p.to_string()).collect();
    
    // Formater et afficher les chemins
    let formatted = context.formatter().format_paths(&path_strings)?;
    println!("{}", formatted);
    
    Ok(())
}