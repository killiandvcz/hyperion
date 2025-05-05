// src/core/index/worker.rs (modifié)
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::task::JoinHandle;

use crate::core::path::Path;
use crate::core::errors::{Result, StoreError};
use super::types::{IndexOp, IndexStats, IndexImplementation};

// Trait pour effacer le type générique de l'index
trait AnyIndex: Send + Sync {
    fn add_path(&mut self, path: &Path) -> Result<()>;
    fn remove_path(&mut self, path: &Path) -> Result<()>;
    fn name(&self) -> &str;
}

// Implémentation de AnyIndex qui enveloppe un IndexImplementation
struct IndexWrapper<T: IndexImplementation + 'static> {
    index: Arc<Mutex<T>>,
    name: String,  // Stockage du nom
}

impl<T: IndexImplementation + 'static> AnyIndex for IndexWrapper<T> {
    fn add_path(&mut self, path: &Path) -> Result<()> {
        let mut index = self.index.lock().unwrap();
        index.add_path(path)
    }
    
    fn remove_path(&mut self, path: &Path) -> Result<()> {
        let mut index = self.index.lock().unwrap();
        index.remove_path(path)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Gestionnaire des opérations d'indexation asynchrones
pub struct IndexWorker {
    /// Émetteur pour les opérations d'indexation
    tx: Option<Sender<IndexOp>>,
    /// Handle de la tâche worker
    worker_handle: Option<JoinHandle<()>>,
    /// Statistiques d'indexation
    stats: Arc<Mutex<IndexStats>>,
    /// Liste des index (utilisée uniquement pour le démarrage)
    indexes: Vec<Box<dyn AnyIndex>>,
}

impl IndexWorker {
    /// Crée un nouveau worker d'indexation
    pub fn new() -> Self {
        IndexWorker {
            tx: None,
            worker_handle: None,
            stats: Arc::new(Mutex::new(IndexStats::default())),
            indexes: Vec::new(),
        }
    }

    pub fn add_index<T: IndexImplementation + 'static>(&mut self, index_impl: Arc<Mutex<T>>) -> Result<()> {
        // Obtenir le nom de l'index
        let name = {
            let index = index_impl.lock().unwrap();
            let name = index.name().to_string();
            println!("Worker: Adding index to list: {}", name);
            name
        };
        
        // Ajouter l'index à la liste
        let wrapper = IndexWrapper { 
            index: index_impl.clone(),
            name: name.clone()
        };
        self.indexes.push(Box::new(wrapper));
        println!("Worker: Index list now contains {} items", self.indexes.len());
        
        Ok(())
    }
    
    /// Démarre le worker asynchrone et ajoute un index
    pub fn start(&mut self) -> Result<()> {
        // Si le worker est déjà démarré, on n'a rien d'autre à faire
        if self.tx.is_some() {
            println!("Worker: Already started, not launching a new worker task");
            return Ok(());
        }
        
        println!("Worker: Starting worker task with {} indexes", self.indexes.len());
        
        // Création du canal pour la communication
        let (tx, rx) = mpsc::channel(1000);
        let stats = Arc::clone(&self.stats);
        
        // Conversion en liste de Box<dyn AnyIndex>
        let indexes = std::mem::take(&mut self.indexes);
        
        // Démarre la tâche de traitement
        let handle = tokio::spawn(async move {
            Self::process_operations(rx, indexes, stats).await;
        });
        
        self.tx = Some(tx);
        self.worker_handle = Some(handle);
        
        Ok(())
    }


    /// Traite les opérations en arrière-plan
    async fn process_operations(
        mut rx: Receiver<IndexOp>,
        mut indexes: Vec<Box<dyn AnyIndex>>,
        stats: Arc<Mutex<IndexStats>>,
    ) {
        
        println!("Worker: Started processing operations with {} indexes", indexes.len());
        for (i, index) in indexes.iter().enumerate() {
            println!("Worker: Index #{} is: {}", i, index.name());
        }
        
        while let Some(op) = rx.recv().await {
            match op {
                IndexOp::Add(path) => {
                    println!("Worker: Processing add operation for path: {:?}", path);
                    
                    let mut success = false;
                    // Appliquer l'opération à tous les index
                    for index in &mut indexes {
                        match index.add_path(&path) {
                            Ok(()) => {
                                println!("Worker: Successfully added path to {}: {:?}", 
                                index.name(), path);
                                success = true;
                            },
                            Err(e) => {
                                println!("Worker: Failed to add path to {}: {:?} - Error: {:?}", 
                                index.name(), path, e);
                            }
                        }
                    }
                    
                    if success {
                        let mut stats = stats.lock().unwrap();
                        stats.total_operations += 1;
                        stats.total_adds += 1;
                        stats.pending_operations = stats.pending_operations.saturating_sub(1);
                    }
                },
                IndexOp::Remove(path) => {
                    let mut success = false;
                    // Appliquer l'opération à tous les index
                    for index in &mut indexes {
                        if let Ok(()) = index.remove_path(&path) {
                            success = true;
                        }
                    }
                    
                    if success {
                        let mut stats = stats.lock().unwrap();
                        stats.total_operations += 1;
                        stats.total_removes += 1;
                        stats.pending_operations = stats.pending_operations.saturating_sub(1);
                    }
                },
                IndexOp::Flush => {
                    // Juste un signal pour traiter toutes les opérations en attente
                    println!("Worker: Flushing operations");
                },
                IndexOp::Shutdown => {
                    println!("Worker: Shutting down");
                    break; // Sortir de la boucle pour arrêter
                }
            }
        }
    }
    
    // Le reste des méthodes reste inchangé...
    
    /// Envoie une opération d'indexation au worker
    pub async fn submit_operation(&self, op: IndexOp) -> Result<()> {
        let tx = self.tx.as_ref().ok_or_else(|| 
            StoreError::Internal("Index worker not started".to_string())
        )?;
        
        // Incrémenter le compteur d'opérations en attente
        if matches!(op, IndexOp::Add(_) | IndexOp::Remove(_)) {
            let mut stats = self.stats.lock().unwrap();
            stats.pending_operations += 1;
        }
        
        // Envoyer l'opération au worker
        tx.send(op).await.map_err(|_| 
            StoreError::Internal("Failed to send operation to index worker".to_string())
        )?;
        
        Ok(())
    }
    
    /// Obtient les statistiques actuelles
    pub fn get_stats(&self) -> IndexStats {
        let stats = self.stats.lock().unwrap();
        stats.clone()
    }
    
    /// Arrête le worker
    pub async fn shutdown(&self) -> Result<()> {
        if let Some(tx) = &self.tx {
            let _ = tx.send(IndexOp::Shutdown).await;
        }
        
        Ok(())
    }
}

impl Clone for IndexWorker {
    fn clone(&self) -> Self {
        IndexWorker {
            tx: self.tx.clone(),
            worker_handle: None,
            stats: Arc::clone(&self.stats),
            indexes: Vec::new(), // Les index ne sont pas clonés, ils ne sont utilisés que lors du démarrage
        }
    }
}