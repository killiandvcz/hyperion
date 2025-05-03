//! Hyperion: An endpoint-first database
//!
//! This crate provides a database system that treats endpoints
//! (path-value pairs) as the fundamental unit of storage.

pub mod path;
pub mod value;
pub mod store;
pub mod errors;
pub mod entity;
pub mod persistent_entity;
pub mod persistent_store;
pub mod index;
pub mod bench;
pub mod wildcard_index;
pub mod index_batcher;
pub mod ql;

// Re-export commonly used items for convenience
pub use path::Path;
pub use value::Value;
pub use store::MemoryStore;
pub use persistent_store::PersistentStore;
pub use errors::{Result, StoreError};
pub use index_batcher::{BatcherConfig, BatcherStats};