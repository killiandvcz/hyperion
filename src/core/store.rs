use std::any::Any;

use crate::core::path::Path;
use crate::core::value::Value;
use crate::core::errors::Result;

/// Trait defining the core operations of a store
pub trait Store: Send + Sync {
    /// Set a value at the given path
    fn set(&mut self, path: Path, value: Value) -> Result<()>;
    
    /// Get a value at the given path
    fn get(&self, path: &Path) -> Result<Value>;
    
    /// Delete a value at the given path
    fn delete(&mut self, path: &Path) -> Result<()>;
    
    /// Check if a path exists in the store
    fn exists(&self, path: &Path) -> Result<bool>;
    
    /// List all paths that start with the given prefix
    fn list_prefix(&self, prefix: &Path) -> Result<Vec<Path>>;
    
    /// Get all values under a prefix (for entity reconstruction)
    fn get_prefix(&self, prefix: &Path) -> Result<Vec<(Path, Value)>>;
    
    /// Query paths that match a pattern (which may contain wildcards)
    fn query(&self, pattern: &Path) -> Result<Vec<(Path, Value)>>;
    
    /// Count the number of paths in the store
    fn count(&self) -> Result<usize>;
    
    /// Count the number of paths under a prefix
    fn count_prefix(&self, prefix: &Path) -> Result<usize>;
    
    /// Flush changes (for persistent stores)
    fn flush(&self) -> Result<()>;

    fn as_any(&self) -> &dyn Any;
}


