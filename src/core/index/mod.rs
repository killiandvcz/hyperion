// src/core/index/mod.rs
pub mod types;
pub mod worker;
pub mod prefix_index;
pub mod wildcard_index;
pub mod value_index;

use std::sync::{Arc, Mutex};

use crate::core::path::Path;
use crate::core::errors::{Result, StoreError};
pub use types::{IndexImplementation, IndexOp, IndexStats};
use value_index::ValueIndex;
use worker::IndexWorker;
use prefix_index::PrefixIndex;
use wildcard_index::WildcardIndex;

use crate::core::value::Value;

/// Système d'indexation unifié
pub struct IndexSystem {
    /// Index par préfixe
    prefix_index: Arc<Mutex<PrefixIndex>>,
    /// Index par wildcard
    wildcard_index: Arc<Mutex<WildcardIndex>>,
    /// Index par valeur
    value_index: Arc<Mutex<ValueIndex>>,
    /// Worker pour les opérations asynchrones,
    worker: IndexWorker,
}

impl IndexSystem {
    /// Crée un nouveau système d'indexation
    pub fn new(db: Arc<sled::Db>) -> Result<Self> {
        // Créer les implémentations d'index
        let prefix_index = Arc::new(Mutex::new(PrefixIndex::new(db.clone(), "prefix_index")?));
        let wildcard_index = Arc::new(Mutex::new(WildcardIndex::new(db.clone(), "wildcard_index")?));
        let value_index = Arc::new(Mutex::new(ValueIndex::new(db.clone(), "value_index")?));
        
        // Créer et configurer le worker
        let mut worker = IndexWorker::new();
        worker.add_index(prefix_index.clone())?;
        worker.add_index(wildcard_index.clone())?;
        worker.add_index(value_index.clone())?;
        
        // Démarrer le worker une fois que tous les index sont ajoutés
        worker.start()?;
        
        Ok(IndexSystem {
            prefix_index,
            wildcard_index,
            value_index,
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
    
    /// Ajoute un pattern à indexer par valeur
    pub fn add_value_indexed_pattern(&self, pattern: &Path) -> Result<()> {
        if let Ok(mut index) = self.value_index.lock() {
            index.add_indexed_pattern(pattern)
        } else {
            Err(StoreError::Internal("Failed to lock value index".to_string()))
        }
    }
    
    
    /// Supprime un pattern indexé par valeur
    pub async fn remove_value_indexed_pattern(&self, pattern: Path) -> Result<()> {
        // Supprimer le pattern des métadonnées
        if let Ok(mut index) = self.value_index.lock() {
            index.remove_indexed_pattern(&pattern)?;
        }
        
        Ok(())
    }
    
    /// Recherche par valeur
    pub fn find_by_value(&self, value: &Value) -> Result<Vec<Path>> {
        if let Ok(index) = self.value_index.lock() {
            index.find_by_value(value)
        } else {
            Err(StoreError::Internal("Failed to lock value index".to_string()))
        }
    }

    /// Recherche par condition
    pub fn find_by_condition(&self, operator: &str, value: &Value) -> Result<Vec<Path>> {
        if let Ok(index) = self.value_index.lock() {
            index.find_by_condition(operator, value)
        } else {
            Ok(Vec::new())
        }
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
    
     pub async fn add_path_with_value(&self, path: Path, value: Value) -> Result<()> {
        // Soumettre directement l'opération avec valeur
        self.worker.submit_operation(IndexOp::AddWithValue(path, value)).await
    }
}

impl Clone for IndexSystem {
    fn clone(&self) -> Self {
        IndexSystem {
            prefix_index: Arc::clone(&self.prefix_index),
            wildcard_index: Arc::clone(&self.wildcard_index),
            value_index: Arc::clone(&self.value_index),
            worker: self.worker.clone(),
        }
    }
}