//! Abstract Syntax Tree for HyperionQL
//!
//! This module defines the structures that represent the parsed query.

use crate::path::Path;
use crate::value::Value;

/// A complete query
#[derive(Debug, Clone)]
pub struct Query {
    /// Operations to perform
    pub operations: Vec<Operation>,
    /// Expression to return
    pub return_expr: Option<Expression>,
}

/// Types of operations
#[derive(Debug, Clone)]
pub enum Operation {
    /// Assign a value to a path
    Assignment {
        /// The path to assign to
        path: Path,
        /// The value to assign
        expression: Expression,
    },
    /// Delete a path
    Delete {
        /// The path to delete
        path: Path,
    },
}

/// Types of expressions
#[derive(Debug, Clone)]
pub enum Expression {
    /// A literal value
    Literal(Value),
    /// A path reference
    Path(Path),
    /// A function call
    FunctionCall {
        /// The function name
        name: String,
        /// The arguments to the function
        arguments: Vec<Expression>,
    },
}