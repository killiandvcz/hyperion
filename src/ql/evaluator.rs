//! Expression evaluator for HyperionQL
//!
//! This module provides functionality to evaluate expressions in the context
//! of a database store.

use std::str::FromStr;

use crate::errors::{Result, StoreError};
use crate::persistent_store::PersistentStore;
use crate::value::Value;
use crate::path::Path;
use crate::persistent_entity::reconstruct_entity;
use crate::ql::ast::Expression;

/// Context for expression evaluation
pub struct EvaluationContext<'a> {
    /// The database store
    pub store: &'a PersistentStore,
}

impl<'a> EvaluationContext<'a> {
    /// Create a new evaluation context
    pub fn new(store: &'a PersistentStore) -> Self {
        EvaluationContext { store }
    }
    
    /// Evaluate an expression in this context
    pub fn evaluate(&self, expr: &Expression) -> Result<Value> {
        match expr {
            Expression::Literal(val) => Ok(val.clone()),
            Expression::Path(path) => self.evaluate_path(path),
            Expression::FunctionCall { name, arguments } => {
                self.evaluate_function_call(name, arguments)
            }
        }
    }
    
    /// Evaluate a path expression by fetching its value from the store
    fn evaluate_path(&self, path: &Path) -> Result<Value> {
        // Essayer d'abord de récupérer une valeur directe
        match self.store.get(path) {
            Ok(value) => Ok(value),
            Err(StoreError::NotFound(_)) => {
                // Si la valeur directe n'existe pas, essayer de reconstruire une entité
                match reconstruct_entity(self.store, path) {
                    Ok(entity) => entity_to_value(&entity),
                    Err(_) => Err(StoreError::NotFound(path.clone()))
                }
            },
            Err(e) => Err(e)
        }
    }
    
    /// Evaluate an entity expression by reconstructing the entity
    fn evaluate_entity(&self, path: &Path) -> Result<Value> {
        let entity = reconstruct_entity(self.store, path)?;
        entity_to_value(&entity)
    }
    
    /// Evaluate a function call
    fn evaluate_function_call(&self, name: &str, arguments: &[Expression]) -> Result<Value> {
        // Évaluer les arguments
        let mut evaluated_args = Vec::with_capacity(arguments.len());
        for arg in arguments {
            let value = self.evaluate(arg)?;
            evaluated_args.push(value);
        }
        
        // Exécuter la fonction en fonction de son nom
        match name {
            "count" => self.function_count(&evaluated_args),
            "now" => self.function_now(),
            "uuid" => self.function_uuid(),
            // Ajouter d'autres fonctions ici...
            _ => Err(StoreError::InvalidOperation(
                format!("Unknown function: {}", name)
            )),
        }
    }
    
    // Implémentations de fonctions intégrées
    
    fn function_count(&self, args: &[Value]) -> Result<Value> {
        if args.len() != 1 {
            return Err(StoreError::InvalidOperation(
                "count() function requires exactly one argument".to_string()
            ));
        }
        
        match &args[0] {
            Value::String(path_str) => {
                // Utiliser FromStr correctement
                let path = Path::from_str(path_str)?;
                
                // Compter les éléments sous ce chemin
                let count = self.store.count_prefix(&path)?;
                Ok(Value::Integer(count as i64))
            },
            _ => Err(StoreError::InvalidOperation(
                "count() function requires a path string argument".to_string()
            )),
        }
    }
    
    fn function_now(&self) -> Result<Value> {
        // Retourne la date et l'heure actuelles au format ISO 8601
        let now = chrono::Utc::now();
        let now_str = now.to_rfc3339();
        Ok(Value::String(now_str))
    }
    
    fn function_uuid(&self) -> Result<Value> {
        // Générer un UUID v4
        let uuid = uuid::Uuid::new_v4();
        Ok(Value::String(uuid.to_string()))
    }
}

// Helper function to convert Entity to Value
fn entity_to_value(entity: &crate::entity::Entity) -> Result<Value> {
    use crate::entity::Entity;
    
    match entity {
        Entity::Null => Ok(Value::Null),
        Entity::Boolean(b) => Ok(Value::Boolean(*b)),
        Entity::Integer(i) => Ok(Value::Integer(*i)),
        Entity::Float(f) => Ok(Value::Float(*f)),
        Entity::String(s) => Ok(Value::String(s.clone())),
        Entity::Binary(data, mime) => Ok(Value::Binary(data.clone(), mime.clone())),
        Entity::Reference(path) => Ok(Value::Reference(path.clone())),
        Entity::Object(_) | Entity::Array(_) => {
            // Pour les objets et tableaux, nous devons les sérialiser en JSON
            // puis les convertir en chaîne de caractères
            let json = serde_json::to_string(entity)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;
            Ok(Value::String(json))
        }
    }
}