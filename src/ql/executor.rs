//! Query executor for HyperionQL
//!
//! This module provides functionality to execute parsed queries
//! against a database store.

use crate::errors::Result;
use crate::persistent_store::PersistentStore;
use crate::value::Value;
use crate::ql::ast::{Query, Operation};
use crate::ql::evaluator::EvaluationContext;

/// Execute a parsed query against the store
pub fn execute_query(store: &PersistentStore, query: &Query) -> Result<Value> {
    // Créer un contexte d'évaluation
    let context = EvaluationContext::new(store);
    
    // Exécuter toutes les opérations dans l'ordre
    for operation in &query.operations {
        execute_operation(store, &context, operation)?;
    }
    
    // Évaluer et retourner l'expression de retour, ou true si pas de return
    match &query.return_expr {
        Some(expr) => context.evaluate(expr),
        None => Ok(Value::Boolean(true)), // Retourne true si toutes les opérations ont réussi
    }
}

/// Execute a single operation
fn execute_operation(
    store: &PersistentStore,
    context: &EvaluationContext,
    operation: &Operation
) -> Result<()> {
    match operation {
        Operation::Assignment { path, expression } => {
            // Évaluer l'expression
            let value = context.evaluate(expression)?;
            
            // Stocker la valeur à l'emplacement spécifié
            store.set(path.clone(), value)?;
            
            Ok(())
        },
        Operation::Delete { path } => {
            // Supprimer la valeur à l'emplacement spécifié
            store.delete(path)?;
            
            Ok(())
        },
    }
}