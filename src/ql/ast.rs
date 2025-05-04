//! Abstract Syntax Tree for HyperionQL
//!
//! This module defines the structures that represent the parsed query.

use crate::core::path::Path;
use crate::core::value::Value;

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

/// Comparison operators for conditions
#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOperator {
    /// Equal (==)
    Equal,
    /// Not equal (!=)
    NotEqual,
    /// Less than (<)
    LessThan,
    /// Less than or equal (<=)
    LessThanOrEqual,
    /// Greater than (>)
    GreaterThan,
    /// Greater than or equal (>=)
    GreaterThanOrEqual,
}

/// Logical operators for combining conditions
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOperator {
    /// AND (&&)
    And,
    /// OR (||)
    Or,
}

/// A condition in a where clause
#[derive(Debug, Clone)]
pub struct Condition {
    /// Left side of the condition
    pub left: Box<Expression>,
    /// Comparison operator
    pub operator: ComparisonOperator,
    /// Right side of the condition
    pub right: Box<Expression>,
}

/// A where clause with conditions
#[derive(Debug, Clone)]
pub struct WhereClause {
    /// First condition
    pub first_condition: Condition,
    /// Additional conditions with logical operators
    pub additional_conditions: Vec<(LogicalOperator, Condition)>,
}

/// Types of expressions
#[derive(Debug, Clone)]
pub enum Expression {
    /// A literal value
    Literal(Value),
    /// A path reference
    Path(Path),
    /// A 'their' path reference
    TheirPath(Vec<String>),
    /// A function call
    FunctionCall {
        /// The function name
        name: String,
        /// The arguments to the function
        arguments: Vec<Expression>,
    },
    /// A filtered expression (with where clause)
    Filtered {
        /// The base expression
        base: Box<Expression>,
        /// The where clause
        where_clause: WhereClause,
    },
}