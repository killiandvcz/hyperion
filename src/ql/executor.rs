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
    // Create an evaluation context
    let context = EvaluationContext::new(store);
    
    // Execute all operations in order
    for operation in &query.operations {
        execute_operation(store, &context, operation)?;
    }
    
    // Evaluate and return the return expression, or true if no return
    match &query.return_expr {
        Some(expr) => {
            // The evaluator now handles filtered expressions (with where clauses)
            context.evaluate(expr)
        },
        None => Ok(Value::Boolean(true)), // Return true if all operations succeeded
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
            // Evaluate the expression
            let value = context.evaluate(expression)?;
            
            // Store the value at the specified path
            store.set(path.clone(), value)?;
            
            Ok(())
        },
        Operation::Delete { path } => {
            // Delete the value at the specified path
            store.delete(path)?;
            
            Ok(())
        },
    }
}