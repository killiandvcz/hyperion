// src/core/index/mod.rs
pub mod types;
pub mod worker;
pub mod prefix_index;
pub mod wildcard_index;

use std::sync::{Arc, Mutex};

use crate::core::path::Path;
use crate::core::errors::{Result, StoreError};
pub use types::{IndexImplementation, IndexOp, IndexStats};
use worker::IndexWorker;
use prefix_index::PrefixIndex;
use wildcard_index::WildcardIndex;

/// Système d'indexation unifié
pub struct IndexSystem {
    /// Index par préfixe
    prefix_index: Arc<Mutex<PrefixIndex>>,
    /// Index par wildcard
    wildcard_index: Arc<Mutex<WildcardIndex>>,
    /// Worker pour les opérations asynchrones
    worker: IndexWorker,
}

impl IndexSystem {
    /// Crée un nouveau système d'indexation
    pub fn new(db: Arc<sled::Db>) -> Result<Self> {
        // Créer les implémentations d'index
        let prefix_index = Arc::new(Mutex::new(PrefixIndex::new(db.clone(), "prefix_index")?));
        let wildcard_index = Arc::new(Mutex::new(WildcardIndex::new(db.clone(), "wildcard_index")?));
        
        // Créer et configurer le worker
        let mut worker = IndexWorker::new();
        worker.add_index(prefix_index.clone())?;
        worker.add_index(wildcard_index.clone())?;
        
        // Démarrer le worker une fois que tous les index sont ajoutés
        worker.start()?;
        
        Ok(IndexSystem {
            prefix_index,
            wildcard_index,
            worker,
        })
    }
    
    /// Ajoute un chemin aux index de manière asynchrone
    pub async fn add_path(&self, path: Path) -> Result<()> {
        println!("\nIndexSystem: Adding path: {:?}", path);
        
        // Envoyer l'opération au worker
        self.worker.submit_operation(IndexOp::Add(path.clone())).await?;
        
        Ok(())
    }
    
    /// Supprime un chemin des index de manière asynchrone
    pub async fn remove_path(&self, path: Path) -> Result<()> {
        println!("IndexSystem: Removing path: {:?}", path);
        
        // Envoyer l'opération au worker
        self.worker.submit_operation(IndexOp::Remove(path.clone())).await?;
        
        Ok(())
    }
    
    /// Force le traitement de toutes les opérations en attente
    pub async fn flush(&self) -> Result<()> {
        println!("IndexSystem: Flushing");
        
        // Envoyer l'opération de flush au worker
        self.worker.submit_operation(IndexOp::Flush).await?;
        
        Ok(())
    }
    
    /// Trouve tous les chemins correspondant à un préfixe
    pub fn find_by_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        println!("IndexSystem: Finding by prefix: {:?}", prefix);
        
        // Rechercher dans l'index par préfixe
        let prefix_results = if let Ok(index) = self.prefix_index.lock() {
            index.find_by_prefix(prefix)?
        } else {
            Vec::new()
        };
        
        println!("IndexSystem: Found {} paths by prefix", prefix_results.len());
        Ok(prefix_results)
    }
    
    /// Trouve tous les chemins correspondant à un motif (avec wildcards)
    pub fn find_by_pattern(&self, pattern: &Path) -> Result<Vec<Path>> {
        println!("IndexSystem: Finding by pattern: {:?}", pattern);
        
        // Si le motif n'a pas de wildcards, utiliser la recherche par préfixe
        if !pattern.has_wildcards() {
            return self.find_by_prefix(pattern);
        }
        
        // Rechercher dans l'index par wildcard
        let wildcard_results = if let Ok(index) = self.wildcard_index.lock() {
            index.find_by_pattern(pattern)?
        } else {
            Vec::new()
        };
        
        println!("IndexSystem: Found {} paths by pattern", wildcard_results.len());
        Ok(wildcard_results)
    }
    
    /// Obtient les statistiques d'indexation
    pub fn stats(&self) -> IndexStats {
        self.worker.get_stats()
    }
    
    /// Arrête le system d'indexation
    pub async fn shutdown(&self) -> Result<()> {
        println!("IndexSystem: Shutting down");
        
        // Arrêter le worker
        self.worker.shutdown().await?;
        
        Ok(())
    }
}

impl Clone for IndexSystem {
    fn clone(&self) -> Self {
        IndexSystem {
            prefix_index: Arc::clone(&self.prefix_index),
            wildcard_index: Arc::clone(&self.wildcard_index),
            worker: self.worker.clone(),
        }
    }
}