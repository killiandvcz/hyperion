//! Expression evaluator for HyperionQL
//!
//! This module provides functionality to evaluate expressions in the context
//! of a database store.

use std::str::FromStr;
use std::collections::HashSet;

use crate::errors::{Result, StoreError};
use crate::persistent_store::PersistentStore;
use crate::value::Value;
use crate::path::Path;
use crate::entity::Entity;
use crate::persistent_entity::reconstruct_entity;
use crate::ql::ast::{Expression, ComparisonOperator, LogicalOperator, Condition, WhereClause};


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
            Expression::TheirPath(_) => Err(StoreError::InvalidOperation(
                "'their' can only be used in a 'where' clause".to_string()
            )),
            Expression::FunctionCall { name, arguments } => {
                self.evaluate_function_call(name, arguments)
            },
            Expression::Filtered { base, where_clause } => {
                self.evaluate_filtered_expression(base, where_clause)
            }
        }
    }
    
    /// Evaluate a path expression by fetching its value from the store
    fn evaluate_path(&self, path: &Path) -> Result<Value> {
        // Try to get a direct value first
        match self.store.get(path) {
            Ok(value) => Ok(value),
            Err(StoreError::NotFound(_)) => {
                // If direct value doesn't exist, try to reconstruct an entity
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
    
    
    fn evaluate_filtered_expression(&self, base: &Expression, where_clause: &WhereClause) -> Result<Value> {
        // Currently we only support filtering on path expressions
        let base_path = match base {
            Expression::Path(path) => path,
            _ => return Err(StoreError::InvalidOperation(
                "Filtering is currently only supported on path expressions".to_string()
            )),
        };
        
        // Process the where clause to extract conditions on 'their' paths
        let their_conditions = self.extract_their_conditions(where_clause)?;
        
        if their_conditions.is_empty() {
            return Err(StoreError::InvalidOperation(
                "Where clause must contain conditions on 'their' paths".to_string()
            ));
        }
        
        // Find matching entity IDs for each condition
        let mut all_matching_ids = HashSet::new();
        let mut is_first_condition = true;
        
        for (their_path, operator, value) in &their_conditions {
            // Construct wildcard path for searching
            // e.g., users.*.active for "their.active"
            let search_path_str = format!("{}.*.{}", base_path, their_path.join("."));
            let search_path = Path::from_str(&search_path_str)?;
            
            // Find all paths matching this pattern
            let matching_paths = self.store.query(&search_path)?;
            
            // Filter paths by the condition value
            let mut matching_ids_for_condition = HashSet::new();
            
            for (path, actual_value) in matching_paths {
                // Check if the value matches our condition
                if self.compare_values(&actual_value, operator, value)? {
                    // Extract the entity ID from the path
                    // e.g., "u-123456" from "users.u-123456.active"
                    let path_segments = path.segments();
                    if path_segments.len() >= 2 {
                        let entity_id = path_segments[1].as_str();
                        matching_ids_for_condition.insert(entity_id.to_string());
                    }
                }
            }
            
            // Combine with previous conditions using appropriate logical operation
            if is_first_condition {
                all_matching_ids = matching_ids_for_condition;
                is_first_condition = false;
            } else {
                // For simplicity, we're assuming AND logic between conditions
                // In a more complete implementation, we'd handle the logical operators
                all_matching_ids = all_matching_ids
                .intersection(&matching_ids_for_condition)
                .cloned()
                .collect();
            }
        }
        
        // Reconstruct matching entities
        let mut result_entities = Vec::new();
        
        for entity_id in all_matching_ids {
            let entity_path_str = format!("{}.{}", base_path, entity_id);
            let entity_path = Path::from_str(&entity_path_str)?;
            
            match reconstruct_entity(self.store, &entity_path) {
                Ok(entity) => {
                    let entity_value = entity_to_value(&entity)?;
                    result_entities.push(entity_value);
                },
                Err(_) => continue, // Skip entities that can't be reconstructed
            }
        }
        
        // Return as a JSON array
        let json_array = format!("[{}]", result_entities
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", "));
        
        Ok(Value::String(json_array))
    }
    
    fn extract_their_conditions(&self, where_clause: &WhereClause) 
    -> Result<Vec<(Vec<String>, ComparisonOperator, Value)>> {
        
        let mut their_conditions = Vec::new();
        
        // Process the first condition
        self.extract_condition_if_their(&where_clause.first_condition, &mut their_conditions)?;
        
        // Process additional conditions
        for (_, condition) in &where_clause.additional_conditions {
            self.extract_condition_if_their(condition, &mut their_conditions)?;
        }
        
        Ok(their_conditions)
    }
    
    
    fn extract_condition_if_their(&self, condition: &Condition, 
        result: &mut Vec<(Vec<String>, ComparisonOperator, Value)>) -> Result<()> {
            
            println!("Extracting condition: {:?}", condition);
            // Check if left side is a TheirPath and right side is a literal
            if let (Expression::TheirPath(path), Expression::Literal(value)) = (&*condition.left, &*condition.right) {
                result.push((path.clone(), condition.operator.clone(), value.clone()));
                return Ok(());
            }
            
            // Check if right side is a TheirPath and left side is a literal (reversed condition)
            if let (Expression::Literal(value), Expression::TheirPath(path)) = (&*condition.left, &*condition.right) {
                // Reverse the operator for correct comparison
                let reversed_operator = match condition.operator {
                    ComparisonOperator::Equal => ComparisonOperator::Equal,
                    ComparisonOperator::NotEqual => ComparisonOperator::NotEqual,
                    ComparisonOperator::LessThan => ComparisonOperator::GreaterThan,
                    ComparisonOperator::LessThanOrEqual => ComparisonOperator::GreaterThanOrEqual,
                    ComparisonOperator::GreaterThan => ComparisonOperator::LessThan,
                    ComparisonOperator::GreaterThanOrEqual => ComparisonOperator::LessThanOrEqual,
                };
                
                result.push((path.clone(), reversed_operator, value.clone()));
                return Ok(());
            }
            
            Err(StoreError::InvalidOperation(
                "Where conditions must compare 'their' paths with literal values".to_string()
            ))
        }
        
        fn compare_values(&self, left: &Value, operator: &ComparisonOperator, right: &Value) -> Result<bool> {
            match operator {
                ComparisonOperator::Equal => Ok(left == right),
                ComparisonOperator::NotEqual => Ok(left != right),
                ComparisonOperator::LessThan => {
                    match (left, right) {
                        (Value::Integer(l), Value::Integer(r)) => Ok(l < r),
                        (Value::Float(l), Value::Float(r)) => Ok(l < r),
                        (Value::Integer(l), Value::Float(r)) => Ok((*l as f64) < *r),
                        (Value::Float(l), Value::Integer(r)) => Ok(*l < (*r as f64)),
                        (Value::String(l), Value::String(r)) => Ok(l < r),
                        _ => Err(StoreError::InvalidOperation(
                            format!("Cannot compare {:?} and {:?} with <", left, right)
                        )),
                    }
                },
                ComparisonOperator::LessThanOrEqual => {
                    match (left, right) {
                        (Value::Integer(l), Value::Integer(r)) => Ok(l <= r),
                        (Value::Float(l), Value::Float(r)) => Ok(l <= r),
                        (Value::Integer(l), Value::Float(r)) => Ok((*l as f64) <= *r),
                        (Value::Float(l), Value::Integer(r)) => Ok(*l <= (*r as f64)),
                        (Value::String(l), Value::String(r)) => Ok(l <= r),
                        _ => Err(StoreError::InvalidOperation(
                            format!("Cannot compare {:?} and {:?} with <=", left, right)
                        )),
                    }
                },
                ComparisonOperator::GreaterThan => {
                    match (left, right) {
                        (Value::Integer(l), Value::Integer(r)) => Ok(l > r),
                        (Value::Float(l), Value::Float(r)) => Ok(l > r),
                        (Value::Integer(l), Value::Float(r)) => Ok((*l as f64) > *r),
                        (Value::Float(l), Value::Integer(r)) => Ok(*l > (*r as f64)),
                        (Value::String(l), Value::String(r)) => Ok(l > r),
                        _ => Err(StoreError::InvalidOperation(
                            format!("Cannot compare {:?} and {:?} with >", left, right)
                        )),
                    }
                },
                ComparisonOperator::GreaterThanOrEqual => {
                    match (left, right) {
                        (Value::Integer(l), Value::Integer(r)) => Ok(l >= r),
                        (Value::Float(l), Value::Float(r)) => Ok(l >= r),
                        (Value::Integer(l), Value::Float(r)) => Ok((*l as f64) >= *r),
                        (Value::Float(l), Value::Integer(r)) => Ok(*l >= (*r as f64)),
                        (Value::String(l), Value::String(r)) => Ok(l >= r),
                        _ => Err(StoreError::InvalidOperation(
                            format!("Cannot compare {:?} and {:?} with >=", left, right)
                        )),
                    }
                },
            }
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