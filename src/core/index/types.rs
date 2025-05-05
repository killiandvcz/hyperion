// src/core/index/types.rs
use crate::core::path::Path;
use std::sync::Arc;

/// Type d'opération d'indexation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexOp {
    /// Ajouter un chemin à l'index
    Add(Path),
    /// Supprimer un chemin de l'index
    Remove(Path),
    /// Forcer un flush des opérations en attente
    Flush,
    /// Arrêter le worker
    Shutdown,
}

/// Statistiques des opérations d'indexation
#[derive(Debug, Default, Clone)]
pub struct IndexStats {
    /// Nombre total d'opérations traitées
    pub total_operations: usize,
    /// Nombre total d'opérations d'ajout
    pub total_adds: usize,
    /// Nombre total d'opérations de suppression
    pub total_removes: usize,
    /// Nombre d'opérations en attente
    pub pending_operations: usize,
}

/// Trait pour les implémentations d'index
pub trait IndexImplementation: Send + Sync {
    /// Ajouter un chemin à l'index (implémentation interne)
    fn add_path(&mut self, path: &Path) -> crate::core::errors::Result<()>;
    
    /// Supprimer un chemin de l'index (implémentation interne)
    fn remove_path(&mut self, path: &Path) -> crate::core::errors::Result<()>;
    
    /// Trouver tous les chemins qui correspondent à un préfixe
    fn find_by_prefix(&self, prefix: &Path) -> crate::core::errors::Result<Vec<Path>>;
    
    /// Trouver tous les chemins qui correspondent à un motif (avec wildcards)
    fn find_by_pattern(&self, pattern: &Path) -> crate::core::errors::Result<Vec<Path>>;
    
    /// Vider l'index
    fn clear(&mut self) -> crate::core::errors::Result<()>;
    
    /// Obtenir le nom de l'implémentation
    fn name(&self) -> &'static str;
}