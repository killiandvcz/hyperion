//! Error types for NanoDB
//!
//! This module defines the various error types that can occur
//! during database operations.

use thiserror::Error;
use crate::path::{Path, PathError};

/// Errors that can occur during database operations
#[derive(Error, Debug)]
pub enum StoreError {
    #[error("Path error: {0}")]
    PathError(#[from] PathError),
    
    #[error("Value not found at path: {0}")]
    NotFound(Path),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for database operations
pub type Result<T> = std::result::Result<T, StoreError>;