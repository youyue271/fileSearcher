use once_cell::sync::Lazy;
use std::sync::RwLock;
use tantivy::{Index, IndexReader};

// Use Lazy to initialize the RwLock wrapping our optional Index and Reader.
// This will be our globally accessible, thread-safe index holder.
pub static INDEX: Lazy<RwLock<Option<(Index, IndexReader)>>> = Lazy::new(|| RwLock::new(None));
