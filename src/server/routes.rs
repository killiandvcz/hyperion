// src/server/routes.rs
//! Routes API pour le serveur Hyperion

use serde_json::Error;
use warp::{Filter, Rejection, Reply};
use warp::filters::body::json;
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use crate::Hyperion;
use crate::core::path::Path;
use crate::core::value::Value;
use std::str::FromStr;

/// Requête pour définir une valeur
#[derive(Debug, Deserialize)]
struct SetRequest {
    /// Chemin à définir
    path: String,
    /// Valeur à stocker
    value: serde_json::Value,
}

/// Requête pour récupérer une valeur
#[derive(Debug, Deserialize)]
struct GetRequest {
    /// Chemin à récupérer
    path: String,
}

/// Requête pour exécuter une requête HyperionQL
#[derive(Debug, Deserialize)]
struct QueryRequest {
    /// Requête à exécuter
    query: String,
}

/// Réponse générique pour l'API
#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    /// Statut de la réponse
    success: bool,
    /// Message d'erreur éventuel
    error: Option<String>,
    /// Données de la réponse
    data: Option<T>,
}

/// Crée les routes pour l'API Hyperion
pub fn api_routes(
    hyperion: Arc<Mutex<Hyperion>>
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Route GET /api/get?path=...
    let get_route = warp::path!("api" / "get")
        .and(warp::get())
        .and(warp::query::<GetRequest>())
        .and(with_hyperion(hyperion.clone()))
        .and_then(handle_get);
    
    // Route POST /api/set
    let set_route = warp::path!("api" / "set")
        .and(warp::post())
        .and(json::<SetRequest>())
        .and(with_hyperion(hyperion.clone()))
        .and_then(handle_set);
    
    // Route POST /api/query
    let query_route = warp::path!("api" / "query")
        .and(warp::post())
        .and(json::<QueryRequest>())
        .and(with_hyperion(hyperion.clone()))
        .and_then(handle_query);
    
    // Route GET /api/list?prefix=...
    let list_route = warp::path!("api" / "list")
        .and(warp::get())
        .and(warp::query::<GetRequest>())
        .and(with_hyperion(hyperion))
        .and_then(handle_list);
    
    // Combiner toutes les routes
    get_route.or(set_route).or(query_route).or(list_route)
}

/// Fonction utilitaire pour partager l'instance Hyperion avec les gestionnaires
fn with_hyperion(
    hyperion: Arc<Mutex<Hyperion>>
) -> impl Filter<Extract = (Arc<Mutex<Hyperion>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || hyperion.clone())
}

/// Gestionnaire pour GET /api/get
async fn handle_get(
    req: GetRequest,
    hyperion: Arc<Mutex<Hyperion>>
) -> Result<impl Reply, Rejection> {
    let path = match Path::from_str(&req.path) {
        Ok(p) => p,
        Err(e) => {
            return Ok(warp::reply::json(&ApiResponse {
                success: false,
                error: Some(format!("Invalid path: {}", e)),
                data: None::<()>,
            }));
        }
    };
    
    let response = {
        let db = hyperion.lock().unwrap();
        match db.get(&path) {
            Ok(value) => ApiResponse {
                success: true,
                error: None,
                data: Some(value_to_json(&value)),
            },
            Err(e) => ApiResponse {
                success: false,
                error: Some(format!("Error: {}", e)),
                data: None::<serde_json::Value>,
            },
        }
    };
    
    Ok(warp::reply::json(&response))
}

/// Gestionnaire pour POST /api/set
async fn handle_set(
    req: SetRequest,
    hyperion: Arc<Mutex<Hyperion>>
) -> Result<impl Reply, Rejection> {
    let path = match Path::from_str(&req.path) {
        Ok(p) => p,
        Err(e) => {
            return Ok(warp::reply::json(&ApiResponse {
                success: false,
                error: Some(format!("Invalid path: {}", e)),
                data: None::<()>,
            }));
        }
    };
    
    let value = match json_to_value(req.value) {
        Ok(v) => v,
        Err(e) => {
            return Ok(warp::reply::json(&ApiResponse {
                success: false,
                error: Some(format!("Invalid value: {}", e)),
                data: None::<()>,
            }));
        }
    };
    
    let response = {
        let mut db = hyperion.lock().unwrap();
        match db.set(path, value) {
            Ok(_) => ApiResponse {
                success: true,
                error: None,
                data: None::<()>,
            },
            Err(e) => ApiResponse {
                success: false,
                error: Some(format!("Error: {}", e)),
                data: None::<()>,
            },
        }
    };
    
    Ok(warp::reply::json(&response))
}

/// Gestionnaire pour POST /api/query
async fn handle_query(
    req: QueryRequest,
    hyperion: Arc<Mutex<Hyperion>>
) -> Result<impl Reply, Rejection> {
    let response = {
        let mut db = hyperion.lock().unwrap();
        
        // Accéder au store interne de Hyperion
        let store = db.store_mut();
        
        // Utiliser le store avec execute_query
        match crate::ql::execute_query(store, &req.query) {
            Ok(value) => ApiResponse {
                success: true,
                error: None,
                data: Some(value_to_json(&value)),
            },
            Err(e) => ApiResponse {
                success: false,
                error: Some(format!("Error: {}", e)),
                data: None::<serde_json::Value>,
            },
        }
    };
    
    Ok(warp::reply::json(&response))
}

/// Gestionnaire pour GET /api/list
async fn handle_list(
    req: GetRequest,
    hyperion: Arc<Mutex<Hyperion>>
) -> Result<impl Reply, Rejection> {
    let prefix = match Path::from_str(&req.path) {
        Ok(p) => p,
        Err(e) => {
            return Ok(warp::reply::json(&ApiResponse {
                success: false,
                error: Some(format!("Invalid path: {}", e)),
                data: None::<()>,
            }));
        }
    };
    
    let response = {
        let db = hyperion.lock().unwrap();
        match db.list_prefix(&prefix) {
            Ok(paths) => {
                let path_strings: Vec<String> = paths.iter().map(|p| p.to_string()).collect();
                ApiResponse {
                    success: true,
                    error: None,
                    data: Some(path_strings),
                }
            },
            Err(e) => ApiResponse {
                success: false,
                error: Some(format!("Error: {}", e)),
                data: None::<Vec<String>>,
            },
        }
    };
    
    Ok(warp::reply::json(&response))
}

/// Convertit une Value Hyperion en serde_json::Value
fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Integer(i) => serde_json::Value::Number((*i).into()),
        Value::Float(f) => {
            if let Some(n) = serde_json::Number::from_f64(*f) {
                serde_json::Value::Number(n)
            } else {
                // Fallback pour les nombres non représentables en JSON
                serde_json::Value::String(f.to_string())
            }
        },
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Binary(data, mime) => {
            // Encoder en base64
            let encoded = base64::encode(data);
            let mut obj = serde_json::Map::new();
            obj.insert("type".to_string(), serde_json::Value::String("binary".to_string()));
            obj.insert("data".to_string(), serde_json::Value::String(encoded));
            if let Some(m) = mime {
                obj.insert("mime".to_string(), serde_json::Value::String(m.clone()));
            }
            serde_json::Value::Object(obj)
        },
        Value::Reference(path) => {
            let mut obj = serde_json::Map::new();
            obj.insert("type".to_string(), serde_json::Value::String("reference".to_string()));
            obj.insert("path".to_string(), serde_json::Value::String(path.to_string()));
            serde_json::Value::Object(obj)
        },
    }
}
fn json_to_value(json: serde_json::Value) -> Result<Value, String> {
    match json {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(b)),
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                Ok(Value::Integer(n.as_i64().unwrap()))
            } else {
                Ok(Value::Float(n.as_f64().unwrap_or(0.0)))
            }
        },
        serde_json::Value::String(s) => Ok(Value::String(s)),
        serde_json::Value::Array(_) => {
            // Pour les tableaux, nous serialisons tout le JSON en chaîne
            let json_str = serde_json::to_string(&json)
                .map_err(|e| format!("Failed to serialize JSON array: {}", e))?;
            Ok(Value::String(json_str))
        },
        serde_json::Value::Object(ref obj) => { // Utilisez `ref` ici pour emprunter obj au lieu de le déplacer
            // Vérifier s'il s'agit d'un type spécial comme binary ou reference
            if let Some(serde_json::Value::String(t)) = obj.get("type") {
                match t.as_str() {
                    "binary" => {
                        let data = obj.get("data")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing data for binary type")?;
                        let mime = obj.get("mime").and_then(|v| v.as_str()).map(String::from);
                        let decoded = base64::decode(data)
                            .map_err(|e| format!("Invalid base64 data: {}", e))?;
                        Ok(Value::Binary(decoded, mime))
                    },
                    "reference" => {
                        let path_str = obj.get("path")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing path for reference type")?;
                        let path = Path::from_str(path_str)
                            .map_err(|e| format!("Invalid path: {}", e))?;
                        Ok(Value::Reference(path))
                    },
                    _ => {
                        // Type inconnu, sérialiser en JSON
                        let json_str = serde_json::to_string(&json)
                            .map_err(|e| format!("Failed to serialize JSON object: {}", e))?;
                        Ok(Value::String(json_str))
                    }
                }
            } else {
                // Objet ordinaire, sérialiser en JSON
                let json_str = serde_json::to_string(&json)
                    .map_err(|e| format!("Failed to serialize JSON object: {}", e))?;
                Ok(Value::String(json_str))
            }
        }
    }
}