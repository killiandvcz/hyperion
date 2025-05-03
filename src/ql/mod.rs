//! Query Language for Hyperion
//!
//! This module provides a query language interface to the endpoint-first database,
//! allowing users to express complex operations with a concise syntax.

pub mod ast;
pub mod parser;
pub mod evaluator;
pub mod executor;

use crate::errors::Result;
use crate::persistent_store::PersistentStore;
use crate::value::Value;

/// Execute a query string on the given store
pub fn execute_query(store: &PersistentStore, query_str: &str) -> Result<Value> {
    // Parse the query
    let query = parser::parse_query(query_str)?;
    
    // Execute the query
    executor::execute_query(store, &query)
}