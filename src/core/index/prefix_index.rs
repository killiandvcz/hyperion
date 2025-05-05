// src/core/index/prefix_index.rs
use std::sync::Arc;
use sled::Db;
use bincode::{serialize, deserialize};

use crate::core::path::Path;
use crate::core::errors::{Result, StoreError};
use super::types::IndexImplementation;

/// Index optimisé pour les recherches par préfixe
pub struct PrefixIndex {
    /// La base de données sled
    db: Arc<Db>,
    /// Nom de l'arbre où les données d'index sont stockées
    tree_name: String,
}

impl PrefixIndex {
    /// Crée un nouvel index de préfixe
    pub fn new(db: Arc<Db>, tree_name: &str) -> Result<Self> {
        Ok(PrefixIndex {
            db,
            tree_name: tree_name.to_string(),
        })
    }
    
    /// Obtient l'arbre sled pour cet index
    fn get_tree(&self) -> Result<sled::Tree> {
        self.db.open_tree(&self.tree_name)
        .map_err(|e| StoreError::Internal(format!("Failed to open index tree: {}", e)))
    }
    
    /// Crée une clé d'index à partir d'un chemin
    /// Crée une clé d'index à partir d'un chemin
    fn create_index_key(path: &Path) -> Result<Vec<u8>> {
        let segments = path.segments();
        
        // Format simple: segment1:segment2:segment3...
        // Sans compteur de segments au début
        let mut key_parts = Vec::with_capacity(segments.len());
        
        for segment in segments {
            key_parts.push(segment.as_str());
        }
        
        let key = key_parts.join(":");
        println!("Created key: {}", key);
        Ok(key.as_bytes().to_vec())
    }
}

impl IndexImplementation for PrefixIndex {
    fn add_path(&mut self, path: &Path) -> Result<()> {
        println!("PrefixIndex: Adding path: {:?}", path);
        let tree = self.get_tree()?;
        
        // Créer la clé avec notre format textuel
        let key = Self::create_index_key(path)?;
        
        // La valeur reste le chemin sérialisé
        let value = serialize(path).map_err(|e| 
            StoreError::SerializationError(e.to_string())
        )?;
        
        println!("PrefixIndex: Inserting key: {}", String::from_utf8_lossy(&key));
        
        tree.insert(key, value).map_err(|e| 
            StoreError::Internal(format!("Failed to insert into index: {}", e))
        )?;
        
        // Assurer la persistance
        tree.flush().map_err(|e| 
            StoreError::Internal(format!("Failed to flush tree: {}", e))
        )?;
        
        Ok(())
    }


    fn remove_path(&mut self, path: &Path) -> Result<()> {
        let tree = self.get_tree()?;
        let key = Self::create_index_key(path)?;
        
        tree.remove(key)
        .map_err(|e| StoreError::Internal(format!("Failed to remove from index: {}", e)))?;
        
        // Assurons-nous que les modifications sont persistées
        tree.flush()
        .map_err(|e| StoreError::Internal(format!("Failed to flush tree: {}", e)))?;
        
        Ok(())
    }
    
    fn find_by_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        println!("PrefixIndex: Finding by prefix: {:?}", prefix);
        let tree = self.get_tree()?;
        
        // Créer la clé de début
        let start_key = Self::create_index_key(prefix)?;
        println!("Start key: {}", String::from_utf8_lossy(&start_key));
        
        // Pour la recherche par plage, on ajoute un séparateur à la fin
        let mut end_key_bound = start_key.clone();
        end_key_bound.push(b':');  // Ajouter le séparateur ':'
        end_key_bound.push(0xFF);  // Ajouter un byte qui est après tous les caractères normaux
        
        println!("End key bound: {:?}", end_key_bound);
        
        let mut results = Vec::new();
        
        // Scan toutes les clés dans la plage
        for item in tree.range(start_key.clone()..end_key_bound) {
            let (key, value) = item.map_err(|e| 
                StoreError::Internal(format!("Failed to scan index: {}", e))
            )?;
            
            println!("Found key in range: {}", String::from_utf8_lossy(&key));
            
            // Désérialiser la valeur pour obtenir le chemin
            let path = deserialize(&value).map_err(|e| 
                StoreError::DeserializationError(e.to_string())
            )?;
            
            results.push(path);
        }
        
        // Vérifier aussi une correspondance exacte
        if let Some(value) = tree.get(&start_key).map_err(|e| StoreError::Internal(format!("Failed to get from index: {}", e)))? {
            let path = deserialize(&value).map_err(|e| StoreError::DeserializationError(e.to_string()))?;
            if !results.contains(&path) {
                results.push(path);
            }
        }
        
        println!("PrefixIndex: Found {} paths", results.len());
        Ok(results)
    }
    

    fn find_by_pattern(&self, pattern: &Path) -> Result<Vec<Path>> {
        // Si pas de wildcards, c'est une recherche par préfixe classique
        if !pattern.has_wildcards() {
            return self.find_by_prefix(pattern);
        }
        
        // Pour les recherches avec wildcards, on récupère tous les chemins et on les filtre
        let all_paths = self.find_by_prefix(&Path::new())?;
        
        let mut results = Vec::new();
        for path in all_paths {
            if path.matches(pattern) {
                results.push(path);
            }
        }
        
        Ok(results)
    }
    
    fn clear(&mut self) -> Result<()> {
        let tree = self.get_tree()?;
        tree.clear()
        .map_err(|e| StoreError::Internal(format!("Failed to clear index: {}", e)))?;
        
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "PrefixIndex"
    }
}