// src/bin/hyperion_cli/client.rs
//! Client HTTP pour communiquer avec le serveur Hyperion
//!
//! Ce module fournit une interface pour envoyer des requêtes au serveur.

use anyhow::{Result, anyhow};
use reqwest::Client as HttpClient;
use serde::{Serialize, Deserialize};
use crate::utils::error::CliError;

/// Configuration du client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// URL du serveur
    pub server_url: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            server_url: "http://localhost:3000".to_string(),
        }
    }
}

/// Réponse du serveur
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    /// Indique si la requête a réussi
    pub success: bool,
    /// Message d'erreur éventuel
    pub error: Option<String>,
    /// Données de la réponse
    pub data: Option<T>,
}

/// Client pour communiquer avec le serveur Hyperion
pub struct HyperionClient {
    /// Configuration du client
    config: ClientConfig,
    /// Client HTTP
    http_client: HttpClient,
}

impl HyperionClient {
    /// Crée un nouveau client avec la configuration par défaut
    pub fn new() -> Self {
        Self::with_config(ClientConfig::default())
    }
    
    /// Crée un nouveau client avec la configuration fournie
    pub fn with_config(config: ClientConfig) -> Self {
        HyperionClient {
            config,
            http_client: HttpClient::new(),
        }
    }
    
    /// Récupère une valeur du serveur
    pub async fn get_value(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/get", self.config.server_url);
        
        let response: ApiResponse<serde_json::Value> = self.http_client.get(&url)
            .query(&[("path", path)])
            .send()
            .await?
            .json()
            .await?;
        
        if response.success {
            response.data.ok_or_else(|| anyhow!("No data returned"))
        } else {
            Err(anyhow!(response.error.unwrap_or_else(|| "Unknown error".to_string())))
        }
    }
    
    /// Définit une valeur sur le serveur
    pub async fn set_value(&self, path: &str, value: serde_json::Value) -> Result<()> {
        let url = format!("{}/api/set", self.config.server_url);
        
        #[derive(Serialize)]
        struct SetRequest {
            path: String,
            value: serde_json::Value,
        }
        
        let request = SetRequest {
            path: path.to_string(),
            value,
        };
        
        let response: ApiResponse<()> = self.http_client.post(&url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;
        
        if response.success {
            Ok(())
        } else {
            Err(anyhow!(response.error.unwrap_or_else(|| "Unknown error".to_string())))
        }
    }
    
    /// Exécute une requête HyperionQL
    pub async fn execute_query(&self, query: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/query", self.config.server_url);
        
        #[derive(Serialize)]
        struct QueryRequest {
            query: String,
        }
        
        let request = QueryRequest {
            query: query.to_string(),
        };
        
        let response: ApiResponse<serde_json::Value> = self.http_client.post(&url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;
        
        if response.success {
            response.data.ok_or_else(|| anyhow!("No data returned"))
        } else {
            Err(anyhow!(response.error.unwrap_or_else(|| "Unknown error".to_string())))
        }
    }
    
    /// Liste les chemins avec un préfixe donné
    pub async fn list_paths(&self, prefix: &str) -> Result<Vec<String>> {
        let url = format!("{}/api/list", self.config.server_url);
        
        let response: ApiResponse<Vec<String>> = self.http_client.get(&url)
            .query(&[("path", prefix)])
            .send()
            .await?
            .json()
            .await?;
        
        if response.success {
            response.data.ok_or_else(|| anyhow!("No data returned"))
        } else {
            Err(anyhow!(response.error.unwrap_or_else(|| "Unknown error".to_string())))
        }
    }
    
    /// Vérifie la connexion au serveur
    pub async fn check_connection(&self) -> Result<bool> {
        let url = format!("{}/health", self.config.server_url);
        
        let response = self.http_client.get(&url)
            .send()
            .await?;
        
        Ok(response.status().is_success())
    }
}

/// Convertit les erreurs anyhow en CliError
impl From<anyhow::Error> for CliError {
    fn from(error: anyhow::Error) -> Self {
        CliError::Other(error.to_string())
    }
}