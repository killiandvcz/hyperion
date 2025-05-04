mod core;
mod storage;

use hyperion::core::path::Path;
use hyperion::core::value::Value;
use hyperion::core::entity::reconstruct_entity;
use hyperion::storage::PersistentStore;
use std::str::FromStr;

use hyperion::Hyperion;

fn main() -> Result<(), Box<dyn std::error::Error>> {
       // Créer une instance en mémoire
       let mut db = Hyperion::new_in_memory();
       
       // Définir quelques valeurs
       db.set(Path::from_str("users.u-123456.username")?, 
              Value::String("alice".to_string()))?;
              
       db.set(Path::from_str("users.u-123456.email")?, 
              Value::String("alice@example.com".to_string()))?;
              
       db.set(Path::from_str("users.u-123456.active")?, 
              Value::Boolean(true))?;
       
       // Récupérer une valeur spécifique
       let username = db.get(&Path::from_str("users.u-123456.username")?)?;
       println!("Username: {}", username);
       
       // Reconstruire une entité complète
       let user = db.get_entity(&Path::from_str("users.u-123456")?)?;
       println!("User entity: {}", user.to_string_pretty(0));
       
       // Exécuter une requête avec un wildcard
       let email_pattern = Path::from_str("users.*.email")?;
       let results = db.query(&email_pattern)?;
       
       println!("\nEmail query results:");
       for (path, value) in results {
           println!("{} = {}", path, value);
       }
       
       // Avec le stockage persistant, nous pourrions obtenir des statistiques d'index
       if let Some(stats) = db.index_stats()? {
           println!("\nIndex stats:");
           println!("  Total operations: {}", stats.total_operations);
           println!("  Total adds: {}", stats.total_adds);
           println!("  Total removes: {}", stats.total_removes);
           println!("  Pending operations: {}", stats.pending_operations);
       }
       
       Ok(())
   }