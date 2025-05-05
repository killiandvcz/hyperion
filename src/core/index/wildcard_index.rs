// src/core/index/wildcard_index.rs
use std::collections::{HashSet, BTreeMap};
use std::sync::Arc;
use sled::Db;
use bincode::{serialize, deserialize};

use crate::core::path::{Path, PathSegment};
use crate::core::errors::{Result, StoreError};
use super::types::IndexImplementation;

/// Index optimisé pour les recherches avec wildcards
pub struct WildcardIndex {
    /// La base de données sled
    db: Arc<Db>,
    /// Nom de l'arbre pour les wildcards à un niveau
    single_tree_name: String,
    /// Nom de l'arbre pour les wildcards multi-niveaux
    multi_tree_name: String,
}

impl WildcardIndex {
    /// Crée un nouvel index de wildcards
    pub fn new(db: Arc<Db>, base_name: &str) -> Result<Self> {
        Ok(WildcardIndex {
            db,
            single_tree_name: format!("{}_single", base_name),
            multi_tree_name: format!("{}_multi", base_name),
        })
    }
    
    /// Obtient l'arbre pour les wildcards à un niveau
    fn get_single_tree(&self) -> Result<sled::Tree> {
        self.db.open_tree(&self.single_tree_name)
        .map_err(|e| StoreError::Internal(format!("Failed to open single wildcard tree: {}", e)))
    }
    
    /// Obtient l'arbre pour les wildcards multi-niveaux
    fn get_multi_tree(&self) -> Result<sled::Tree> {
        self.db.open_tree(&self.multi_tree_name)
        .map_err(|e| StoreError::Internal(format!("Failed to open multi wildcard tree: {}", e)))
    }
    
    /// Crée un motif structurel pour l'indexation des wildcards à un niveau
    fn create_structural_pattern(path: &Path) -> Result<Vec<u8>> {
        let segments = path.segments();
        let segment_count = segments.len();
        
        // Format: "seg_count:pos1=val1:pos2=val2:..." (format texte au lieu de bincode)
        let mut key_parts = Vec::new();
        key_parts.push(format!("len={}", segment_count));
        
        for (i, segment) in segments.iter().enumerate() {
            if !segment.is_single_wildcard() && !segment.is_multi_wildcard() {
                key_parts.push(format!("{}={}", i, segment.as_str()));
            } else if segment.is_single_wildcard() {
                key_parts.push(format!("{}=*", i));
            } else if segment.is_multi_wildcard() {
                key_parts.push(format!("{}=**", i));
            }
        }
        
        let key = key_parts.join(":");
        println!("Created structural pattern key: {}", key);
        Ok(key.as_bytes().to_vec())
    }
    
    /// Crée une clé de suffixe pour l'indexation des wildcards multi-niveaux
    fn create_suffix_key(segments: &[String]) -> Result<Vec<u8>> {
        // Format texte: "seg1:seg2:seg3:..."
        let key = segments.join(":");
        println!("Created suffix key: {}", key);
        Ok(key.as_bytes().to_vec())
    }
    
    /// Indexe un chemin pour les requêtes avec wildcards à un niveau
    fn index_for_single_wildcards(&self, path: &Path) -> Result<()> {
        let tree = self.get_single_tree()?;
        let segments = path.segments();
        
        // Générer tous les motifs possibles avec un wildcard
        for wildcard_pos in 0..segments.len() {
            // Créer une copie du chemin avec une position en wildcard
            let mut pattern_segments = Vec::with_capacity(segments.len());
            
            for (i, segment) in segments.iter().enumerate() {
                if i == wildcard_pos {
                    pattern_segments.push(PathSegment::new("*")); // Wildcard ici
                } else {
                    pattern_segments.push(segment.clone()); // Segment normal
                }
            }
            
            // Créer le chemin avec un wildcard
            let pattern_path = Path::from_segments(pattern_segments);
            println!("Creating pattern for indexing: {:?}", pattern_path);
            
            // Créer la clé du motif
            let pattern_key = Self::create_structural_pattern(&pattern_path)?;
            
            // Créer/mettre à jour le HashSet des chemins pour ce motif
            let mut paths = if let Some(data) = tree.get(&pattern_key).map_err(|e| StoreError::Internal(format!("Failed to get pattern key: {}", e)))? {
                let existing: HashSet<Path> = deserialize(&data).map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                existing
            } else {
                HashSet::new()
            };
            
            // Ajouter le chemin actuel à l'ensemble
            paths.insert(path.clone());
            
            // Stocker l'ensemble mis à jour
            let serialized = serialize(&paths).map_err(|e| StoreError::SerializationError(e.to_string()))?;
            println!("Storing path {:?} under pattern key: {}", 
            path, String::from_utf8_lossy(&pattern_key));
            tree.insert(pattern_key, serialized).map_err(|e| StoreError::Internal(format!("Failed to insert into single tree: {}", e)))?;
        }
        
        Ok(())
    }
    
    
    /// Indexe un chemin pour les requêtes avec wildcards multi-niveaux
    fn index_for_multi_wildcards(&self, path: &Path) -> Result<()> {
        let tree = self.get_multi_tree()?;
        let segments = path.segments()
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();
        
        // Pour chaque suffixe du chemin
        for start_pos in 0..segments.len() {
            let suffix = &segments[start_pos..];
            let suffix_key = Self::create_suffix_key(suffix)?;
            
            // Stocker le chemin dans l'entrée du suffixe
            let serialized_path = serialize(path).map_err(|e| 
                StoreError::SerializationError(e.to_string())
            )?;
            
            println!("Storing suffix: {:?} -> path: {:?}", 
            String::from_utf8_lossy(&suffix_key), path);
            
            tree.insert(suffix_key, serialized_path)
            .map_err(|e| StoreError::Internal(format!("Failed to update index: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Pour déboguer: lister toutes les clés dans l'arbre des wildcards à un niveau
    fn debug_dump_single_tree(&self) -> Result<()> {
        println!("=== DUMP SINGLE WILDCARD TREE ===");
        let tree = self.get_single_tree()?;
        
        for item in tree.iter() {
            let (key, value) = item.map_err(|e| 
                StoreError::Internal(format!("Failed to iterate tree: {}", e))
            )?;
            
            let path: Path = deserialize(&value).map_err(|e|
                StoreError::DeserializationError(e.to_string())
            )?;
            
            println!("Key: {} => Path: {:?}", 
            String::from_utf8_lossy(&key), path);
        }
        
        println!("=== END DUMP ===");
        Ok(())
    }
}

impl IndexImplementation for WildcardIndex {
    fn add_path(&mut self, path: &Path) -> Result<()> {
        println!("WildcardIndex: Adding path: {:?}", path);
        
        // Indexer pour les deux types de wildcards
        self.index_for_single_wildcards(path)?;
        self.index_for_multi_wildcards(path)?;
        
        // Assurer la persistance
        self.get_single_tree()?.flush().map_err(|e| 
            StoreError::Internal(format!("Failed to flush single tree: {}", e))
        )?;
        self.get_multi_tree()?.flush().map_err(|e| 
            StoreError::Internal(format!("Failed to flush multi tree: {}", e))
        )?;
        
        Ok(())
    }
    
    fn remove_path(&mut self, path: &Path) -> Result<()> {
        // Supprimer pour les deux types de wildcards
        
        // Pour les wildcards à un niveau
        let single_tree = self.get_single_tree()?;
        let segments = path.segments();
        
        for wildcard_pos in 0..segments.len() {
            let pattern_segments = segments.iter()
            .enumerate()
            .map(|(i, s)| {
                if i == wildcard_pos {
                    PathSegment::new("*")
                } else {
                    s.clone()
                }
            })
            .collect::<Vec<_>>();
            
            let pattern_path = Path::from_segments(pattern_segments);
            let pattern_key = Self::create_structural_pattern(&pattern_path)?;
            
            single_tree.remove(pattern_key).map_err(|e| 
                StoreError::Internal(format!("Failed to remove from single tree: {}", e))
            )?;
        }
        
        // Pour les wildcards multi-niveaux
        let multi_tree = self.get_multi_tree()?;
        let segments_str = path.segments()
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();
        
        for start_pos in 0..segments_str.len() {
            let suffix = &segments_str[start_pos..];
            let suffix_key = Self::create_suffix_key(suffix)?;
            
            multi_tree.remove(suffix_key).map_err(|e| 
                StoreError::Internal(format!("Failed to remove from multi tree: {}", e))
            )?;
        }
        
        // Assurer la persistance
        single_tree.flush().map_err(|e| 
            StoreError::Internal(format!("Failed to flush single tree: {}", e))
        )?;
        multi_tree.flush().map_err(|e| 
            StoreError::Internal(format!("Failed to flush multi tree: {}", e))
        )?;
        
        Ok(())
    }
    
    fn find_by_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        // Pour la recherche par préfixe, on utilise l'approche la plus simple
        // On parcourt tous les chemins dans l'index des wildcards à un niveau
        println!("WildcardIndex: Finding by prefix: {:?}", prefix);
        
        let tree = self.get_single_tree()?;
        let mut results = HashSet::new();
        
        for item in tree.iter() {
            let (_, value_bytes) = item
            .map_err(|e| StoreError::Internal(format!("Failed to iterate index: {}", e)))?;
            
            let path: Path = deserialize(&value_bytes)
            .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
            
            if path.starts_with(prefix) {
                results.insert(path);
            }
        }
        
        println!("WildcardIndex: Found {} paths", results.len());
        Ok(results.into_iter().collect())
    }
    
    fn find_by_pattern(&self, pattern: &Path) -> Result<Vec<Path>> {
        println!("WildcardIndex: Finding by pattern: {:?}", pattern);
        let mut results = HashSet::new();
        
        // Dump l'arbre pour déboguer
        self.debug_dump_single_tree()?;
        
        // Vérifier si c'est un motif avec wildcard à un niveau
        if pattern.segments().iter().any(|s| s.is_single_wildcard()) {
            let pattern_key = Self::create_structural_pattern(pattern)?;
            println!("Looking for pattern key: {}", String::from_utf8_lossy(&pattern_key));
            
            let tree = self.get_single_tree()?;
            
            // Recherche exacte pour le motif
            println!("Looking for exact match with key: {}", String::from_utf8_lossy(&pattern_key));
            if let Some(data) = tree.get(&pattern_key).map_err(|e| StoreError::Internal(format!("Failed to get pattern key: {}", e)))? {
                // Désérialiser l'ensemble des chemins pour ce motif
                let paths: HashSet<Path> = deserialize(&data).map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                println!("Found {} paths for this pattern", paths.len());
                results.extend(paths);
            }
            
            // Chercher tous les motifs qui pourraient correspondre si le format de clé n'est pas exact
            println!("Scanning all keys for potential matches");
            for item in tree.iter() {
                let (key, value_bytes) = item.map_err(|e| 
                    StoreError::Internal(format!("Failed to iterate index: {}", e))
                )?;
                
                let path: Path = deserialize(&value_bytes)
                .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                
                // Vérifier si le chemin correspond au motif
                if path.matches(pattern) {
                    println!("Found matching path via scan: {:?}", path);
                    results.insert(path);
                }
            }
        }
        
        // Vérifier si c'est un motif avec wildcard multi-niveaux
        if pattern.segments().iter().any(|s| s.is_multi_wildcard()) {
            println!("Pattern contains multi-level wildcards");
            // Trouver la position du premier wildcard multi-niveaux
            let pos = pattern.segments().iter()
            .position(|s| s.is_multi_wildcard())
            .unwrap();
            
            // Obtenir le suffixe après le wildcard
            let suffix: Vec<String> = if pos + 1 < pattern.segments().len() {
                pattern.segments()[pos + 1..]
                .iter()
                .map(|s| s.as_str())
                .collect()
            } else {
                Vec::new()
            };
            
            // Trouver les chemins avec ce suffixe
            if !suffix.is_empty() {
                let suffix_key = Self::create_suffix_key(&suffix)?;
                println!("Looking for suffix: {}", String::from_utf8_lossy(&suffix_key));
                
                let tree = self.get_multi_tree()?;
                
                if let Some(value_bytes) = tree.get(&suffix_key).map_err(|e| 
                    StoreError::Internal(format!("Failed to get from multi tree: {}", e))
                )? {
                    let path: Path = deserialize(&value_bytes)
                    .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                    
                    // Vérifier que le chemin correspond au motif complet
                    if path.matches(pattern) {
                        println!("Found match with suffix: {:?}", path);
                        results.insert(path);
                    }
                }
                
                // Recherche par préfixe pour attraper les suffixes partiels
                for item in tree.scan_prefix(suffix_key) {
                    let (_, value_bytes) = item.map_err(|e| 
                        StoreError::Internal(format!("Failed to scan multi tree: {}", e))
                    )?;
                    
                    let path: Path = deserialize(&value_bytes)
                    .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                    
                    if path.matches(pattern) {
                        println!("Found match with suffix prefix: {:?}", path);
                        results.insert(path);
                    }
                }
            } else {
                // S'il n'y a pas de suffixe, on doit scanner tous les chemins
                println!("No suffix after **, scanning all paths");
                for item in self.get_single_tree()?.iter() {
                    let (_, value_bytes) = item.map_err(|e| 
                        StoreError::Internal(format!("Failed to iterate index: {}", e))
                    )?;
                    
                    let path: Path = deserialize(&value_bytes)
                    .map_err(|e| StoreError::DeserializationError(e.to_string()))?;
                    
                    if path.matches(pattern) {
                        println!("Found match via full scan: {:?}", path);
                        results.insert(path);
                    }
                }
            }
        }
        
        // Si le motif n'a pas de wildcards, chercher le chemin exact
        if !pattern.segments().iter().any(|s| s.is_wildcard()) {
            // Utiliser find_by_prefix pour trouver le chemin exact
            let exact_paths = self.find_by_prefix(pattern)?;
            for path in exact_paths {
                results.insert(path);
            }
        }
        
        println!("WildcardIndex: Found {} paths matching pattern", results.len());
        Ok(results.into_iter().collect())
    }
    
    fn clear(&mut self) -> Result<()> {
        // Vider les deux arbres
        self.get_single_tree()?.clear().map_err(|e| 
            StoreError::Internal(format!("Failed to clear single tree: {}", e))
        )?;
        self.get_multi_tree()?.clear().map_err(|e| 
            StoreError::Internal(format!("Failed to clear multi tree: {}", e))
        )?;
        
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "WildcardIndex"
    }
}