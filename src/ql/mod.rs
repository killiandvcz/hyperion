//! Query Language for Hyperion

pub mod ast;
pub mod parser;
pub mod evaluator;
pub mod executor;

use crate::core::errors::Result;
use crate::core::store::Store;
use crate::core::value::Value;

/// Execute a query string on the given store
pub fn execute_query<S: Store + ?Sized>(store: &mut S, query_str: &str) -> Result<Value> {
    // Parse the query
    let query = parser::parse_query(query_str)?;
    
    // Execute the query
    executor::execute_query(store, &query)
}