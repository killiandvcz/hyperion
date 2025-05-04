//! Query executor for HyperionQL

use crate::core::errors::Result;
use crate::core::store::Store;
use crate::core::value::Value;
use crate::ql::ast::{Query, Operation};
use crate::ql::evaluator::EvaluationContext;

/// Execute a parsed query against the store
pub fn execute_query<S: Store + ?Sized>(store: &mut S, query: &Query) -> Result<Value> {
    // Create context (no store reference)
    let context = EvaluationContext::new();
    
    // Execute all operations in order
    for operation in &query.operations {
        execute_operation(store, &context, operation)?;
    }
    
    // Evaluate and return the return expression, or true if no return
    match &query.return_expr {
        Some(expr) => {
            // Pass store explicitly to evaluate
            context.evaluate(store, expr)
        },
        None => Ok(Value::Boolean(true)),
    }
}

/// Execute a single operation
fn execute_operation<S: Store + ?Sized>(
    store: &mut S,
    context: &EvaluationContext,
    operation: &Operation
) -> Result<()> {
    match operation {
        Operation::Assignment { path, expression } => {
            // Evaluate the expression
            let value = context.evaluate(store, expression)?;
            
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