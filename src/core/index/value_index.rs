// src/core/index/value_index.rs
use std::sync::Arc;
use sled::Db;
use bincode::{serialize, deserialize};
use std::collections::HashMap;

use crate::core::path::Path;
use crate::core::value::Value;
use crate::core::errors::{Result, StoreError};
use super::types::IndexImplementation;

/// Index optimisé pour les recherches par valeur
pub struct ValueIndex {
    /// La base de données sled
    db: Arc<Db>,
    /// Nom de l'arbre pour stocker l'index
    index_tree_name: String,
    /// Nom de l'arbre pour stocker les métadonnées (patterns indexés)
    metadata_tree_name: String,
    /// Cache mémoire des patterns indexés (pour les vérifications rapides)
    indexed_patterns: HashMap<Path, bool>,
}

impl ValueIndex {
    // (Méthodes d'initialisation, getters d'arbres, etc.)
    
    /// Crée un nouvel index de valeurs
    pub fn new(db: Arc<Db>, base_name: &str) -> Result<Self> {
        let index_tree_name = format!("{}_values", base_name);
        let metadata_tree_name = format!("{}_metadata", base_name);
        
        // Initialiser l'index
        let mut index = ValueIndex {
            db,
            index_tree_name,
            metadata_tree_name,
            indexed_patterns: HashMap::new(),
        };
        
        // Charger les patterns indexés depuis le stockage
        index.load_indexed_patterns()?;
        
        Ok(index)
    }
    
    /// Charge les patterns indexés depuis le stockage
    fn load_indexed_patterns(&mut self) -> Result<()> {
        let tree = self.get_metadata_tree()?;
        
        for item in tree.iter() {
            let (key, _) = item.map_err(|e| StoreError::Internal(format!("Failed to iterate metadata tree: {}", e)))?;
            let pattern: Path = deserialize(&key).map_err(|e| StoreError::Internal(format!("Failed to deserialize key: {}", e)))?;
            self.indexed_patterns.insert(pattern, true);
        }
        
        Ok(())
    }
    
    /// Obtient l'arbre pour l'index des valeurs
    fn get_index_tree(&self) -> Result<sled::Tree> {
        self.db.open_tree(&self.index_tree_name)
            .map_err(|e| StoreError::Internal(format!("Failed to open value index tree: {}", e)))
    }
    
    /// Obtient l'arbre pour les métadonnées
    fn get_metadata_tree(&self) -> Result<sled::Tree> {
        self.db.open_tree(&self.metadata_tree_name)
            .map_err(|e| StoreError::Internal(format!("Failed to open metadata tree: {}", e)))
    }
    
    /// Crée une clé d'index à partir d'une valeur
    fn create_value_key(value: &Value) -> Result<Vec<u8>> {
        // (Même implémentation qu'avant)
        let mut key_bytes = Vec::new();
        
        match value {
            Value::Null => {
                key_bytes.push(0x00); // Code pour null
            },
            Value::Boolean(b) => {
                key_bytes.push(0x01); // Code pour boolean
                key_bytes.push(if *b { 1 } else { 0 });
            },
            Value::Integer(i) => {
                key_bytes.push(0x02); // Code pour integer
                key_bytes.extend_from_slice(&i.to_be_bytes());
            },
            Value::Float(f) => {
                key_bytes.push(0x03); // Code pour float
                let bits = f.to_bits().to_be_bytes();
                if *f >= 0.0 {
                    key_bytes.extend_from_slice(&bits);
                } else {
                    let mut inverted = bits;
                    inverted[0] ^= 0x80;
                    key_bytes.extend_from_slice(&inverted);
                }
            },
            Value::String(s) => {
                key_bytes.push(0x04); // Code pour string
                key_bytes.extend_from_slice(s.as_bytes());
            },
            Value::Binary(_, _) => {
                return Err(StoreError::InvalidOperation(
                    "Binary values cannot be indexed".to_string()
                ));
            },
            Value::Reference(path) => {
                key_bytes.push(0x05); // Code pour reference
                let path_str = path.to_string();
                key_bytes.extend_from_slice(path_str.as_bytes());
            },
        }
        
        Ok(key_bytes)
    }
    
    /// Ajoute un pattern à indexer
    pub fn add_indexed_pattern(&mut self, pattern: &Path) -> Result<()> {
        let tree = self.get_metadata_tree()?;
        let pattern_key = serialize(&pattern).map_err(|e| StoreError::Internal(format!("Failed to serialize pattern: {}", e)))?;
        
        // Stocker le pattern dans l'arbre de métadonnées
        tree.insert(pattern_key, vec![1]).map_err(|e| StoreError::Internal(format!("Failed to insert pattern into metadata tree: {}", e)))?;
        
        // Mettre à jour le cache
        self.indexed_patterns.insert(pattern.clone(), true);
        
        Ok(())
    }
    
    /// Vérifie si un chemin correspond à un pattern indexé
    pub fn is_path_indexed(&self, path: &Path) -> Result<bool> {
        // Vérification rapide dans le cache
        for (pattern, _) in &self.indexed_patterns {
            if path.matches(pattern) {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Supprime un pattern indexé
    pub fn remove_indexed_pattern(&mut self, pattern: &Path) -> Result<()> {
        let tree = self.get_metadata_tree()?;
        let pattern_key = serialize(&pattern).map_err(|e| StoreError::Internal(format!("Failed to serialize pattern: {}", e)))?;
        
        // Supprimer le pattern de l'arbre de métadonnées
        tree.remove(pattern_key).map_err(|e| StoreError::Internal(format!("Failed to remove pattern from metadata tree: {}", e)))?;
        
        // Mettre à jour le cache
        self.indexed_patterns.remove(pattern);
        
        Ok(())
    }
    
    /// Ajoute un chemin avec sa valeur à l'index
    pub fn add_with_value(&mut self, path: &Path, value: &Value) -> Result<()> {
        // Vérifier si ce chemin doit être indexé
        if !self.is_path_indexed(path)? {
            return Ok(());
        }
        
        let tree = self.get_index_tree()?;
        let value_key = Self::create_value_key(value)?;
        
        // Récupérer les chemins existants pour cette valeur
        let mut paths = if let Some(data) = tree.get(&value_key).map_err(|e| StoreError::Internal(format!("Failed to get value from index tree: {}", e)))? {
            deserialize::<Vec<Path>>(&data).map_err(|e| StoreError::Internal(format!("Failed to deserialize data: {}", e)))?
        } else {
            Vec::new()
        };
        
        // Ajouter le nouveau chemin s'il n'existe pas déjà
        if !paths.contains(path) {
            paths.push(path.clone());
            
            // Mettre à jour l'index
            let serialized = serialize(&paths).map_err(|e| StoreError::Internal(format!("Failed to serialize paths: {}", e)))?;
            tree.insert(value_key, serialized).map_err(|e| StoreError::Internal(format!("Failed to insert into index tree: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Trouve tous les chemins ayant une valeur spécifique
    pub fn find_by_value(&self, value: &Value) -> Result<Vec<Path>> {
        let tree = self.get_index_tree()?;
        let value_key = Self::create_value_key(value)?;
        
        if let Some(data) = tree.get(&value_key).map_err(|e| StoreError::Internal(format!("Failed to get value from index tree: {}", e)))? {
            let paths: Vec<Path> = deserialize(&data).map_err(|e| StoreError::Internal(format!("Failed to deserialize data: {}", e)))?;
            Ok(paths)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Trouve tous les chemins satisfaisant une condition de comparaison
    pub fn find_by_condition(&self, operator: &str, value: &Value) -> Result<Vec<Path>> {
        // (Même implémentation qu'avant)
        let tree = self.get_index_tree()?;
        let value_key = Self::create_value_key(value)?;
        let mut results = Vec::new();
        
        match operator {
            "==" => {
                // Égalité - recherche exacte
                if let Some(data) = tree.get(&value_key).map_err(|e| StoreError::Internal(format!("Failed to get value from index tree: {}", e)))? {
                    let paths: Vec<Path> = deserialize(&data).map_err(|e| StoreError::Internal(format!("Failed to deserialize data: {}", e)))?;
                    results.extend(paths);
                }
            },
            // (autres opérateurs...)
            _ => {
                return Err(StoreError::InvalidOperation(
                    format!("Unsupported operator: {}", operator)
                ));
            }
        }
        
        Ok(results)
    }
}

impl IndexImplementation for ValueIndex {
    // Ces méthodes sont toujours nécessaires pour le trait, mais elles restent basiques
    
    fn add_path(&mut self, path: &Path) -> Result<()> {
        // L'implementation standard ne fait rien car nous avons besoin de la valeur
        // pour indexer correctement
        Ok(())
    }
    
    fn remove_path(&mut self, path: &Path) -> Result<()> {
        // Parcourir tout l'index pour trouver et supprimer ce chemin
        let tree = self.get_index_tree()?;
        
        for item in tree.iter() {
            let (value_key, data) = item.map_err(|e| StoreError::Internal(format!("Failed to iterate index tree: {}", e)))?;
            let mut paths: Vec<Path> = deserialize(&data).map_err(|e| StoreError::Internal(format!("Failed to deserialize data: {}", e)))?;
            
            // Vérifier si ce chemin est présent
            if let Some(index) = paths.iter().position(|p| p == path) {
                // Supprimer le chemin
                paths.remove(index);
                
                // Mettre à jour ou supprimer l'entrée
                if paths.is_empty() {
                    tree.remove(value_key).map_err(|e| StoreError::Internal(format!("Failed to remove value from index tree: {}", e)))?;
                } else {
                    let serialized = serialize(&paths).map_err(|e| StoreError::Internal(format!("Failed to serialize paths: {}", e)))?;
                    tree.insert(value_key, serialized).map_err(|e| StoreError::Internal(format!("Failed to insert into index tree: {}", e)))?;
                }
            }
        }
        
        Ok(())
    }
    
    // (autres méthodes du trait...)
    
    fn find_by_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        // (...)
        Ok(Vec::new())
    }
    
    fn find_by_pattern(&self, pattern: &Path) -> Result<Vec<Path>> {
        // (...)
        Ok(Vec::new())
    }
    
    fn clear(&mut self) -> Result<()> {
        // Vider l'arbre d'index
        self.get_index_tree()?.clear().map_err(|e| StoreError::Internal(format!("Failed to clear index tree: {}", e)))?;
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "ValueIndex"
    }
}