// src/server/mod.rs
//! Serveur HTTP pour Hyperion
//! 
//! Ce module fournit une API HTTP pour interagir avec une instance Hyperion.

pub mod routes;

use warp::Filter;
use crate::Hyperion;
use std::sync::{Arc, Mutex};

/// Configuration du serveur
pub struct ServerConfig {
    /// Port d'écoute
    pub port: u16,
    /// Adresse d'écoute
    pub host: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            port: 3000,
            host: "127.0.0.1".to_string(),
        }
    }
}

/// Serveur HTTP pour Hyperion
pub struct HyperionServer {
    /// Instance Hyperion
    hyperion: Arc<Mutex<Hyperion>>,
    /// Configuration du serveur
    config: ServerConfig,
}

impl HyperionServer {
    /// Crée un nouveau serveur avec l'instance Hyperion fournie
    pub fn new(hyperion: Hyperion, config: ServerConfig) -> Self {
        HyperionServer {
            hyperion: Arc::new(Mutex::new(hyperion)),
            config,
        }
    }
    
    /// Démarre le serveur HTTP
    pub async fn run(&self) {
        let hyperion = Arc::clone(&self.hyperion);
        
        // Route GET /health pour vérifier l'état du serveur
        let health_route = warp::path("health")
            .and(warp::get())
            .map(|| "Hyperion server is running");
        
        // Ajouter les routes spécifiques à l'API
        let api_routes = routes::api_routes(hyperion);
        
        // Combiner toutes les routes
        let routes = health_route.or(api_routes);
        
        // Démarrer le serveur
        println!("Hyperion server running at {}:{}", self.config.host, self.config.port);
        
        // Parse l'adresse IP à partir de la chaîne
        let host_parts: Vec<u8> = self.config.host
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        
        let addr = if host_parts.len() == 4 {
            [host_parts[0], host_parts[1], host_parts[2], host_parts[3]]
        } else {
            [127, 0, 0, 1] // Par défaut en cas d'erreur
        };
        
        warp::serve(routes)
            .run((addr, self.config.port))
            .await;
    }
}