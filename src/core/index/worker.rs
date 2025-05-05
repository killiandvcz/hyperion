// src/core/index/worker.rs
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::task::JoinHandle;

use crate::core::path::Path;
use crate::core::errors::{Result, StoreError};
use super::types::{IndexOp, IndexStats, IndexImplementation};

/// Gestionnaire des opérations d'indexation asynchrones
pub struct IndexWorker {
    /// Émetteur pour les opérations d'indexation
    tx: Option<Sender<IndexOp>>,
    /// Handle de la tâche worker
    worker_handle: Option<JoinHandle<()>>,
    /// Statistiques d'indexation
    stats: Arc<Mutex<IndexStats>>,
}

impl IndexWorker {
    /// Crée un nouveau worker d'indexation
    pub fn new() -> Self {
        IndexWorker {
            tx: None,
            worker_handle: None,
            stats: Arc::new(Mutex::new(IndexStats::default())),
        }
    }
    
    /// Démarre le worker asynchrone
    pub fn start<T: IndexImplementation + 'static>(&mut self, index_impl: Arc<Mutex<T>>) -> Result<()> {
        if self.tx.is_some() {
            return Ok(());  // Déjà démarré
        }
        
        let (tx, rx) = mpsc::channel(1000);
        let stats = Arc::clone(&self.stats);
        
        // Démarre la tâche de traitement
        let handle = tokio::spawn(async move {
            Self::process_operations(rx, index_impl, stats).await;
        });
        
        self.tx = Some(tx);
        self.worker_handle = Some(handle);
        
        Ok(())
    }
    
    /// Traite les opérations en arrière-plan
    async fn process_operations<T: IndexImplementation + 'static>(
        mut rx: Receiver<IndexOp>,
        index_impl: Arc<Mutex<T>>,
        stats: Arc<Mutex<IndexStats>>,
    ) {
        while let Some(op) = rx.recv().await {
            match op {
                IndexOp::Add(path) => {
                    println!("Worker: Processing add operation for path: {:?}", path);
                    let result = if let Ok(mut index) = index_impl.lock() {
                        index.add_path(&path)
                    } else {
                        Err(StoreError::Internal("Failed to lock index implementation".to_string()))
                    };
                    
                    match result {
                        Ok(()) => {
                            println!("Worker: Successfully added path: {:?}", path);
                            let mut stats = stats.lock().unwrap();
                            stats.total_operations += 1;
                            stats.total_adds += 1;
                            stats.pending_operations -= 1;
                        },
                        Err(e) => {
                            println!("Worker: Failed to add path: {:?} - Error: {:?}", path, e);
                        }
                    }
                },
                IndexOp::Remove(path) => {
                    if let Ok(mut index) = index_impl.lock() {
                        if let Ok(()) = index.remove_path(&path) {
                            let mut stats = stats.lock().unwrap();
                            stats.total_operations += 1;
                            stats.total_removes += 1;
                            stats.pending_operations -= 1;
                        }
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
        }
    }
}