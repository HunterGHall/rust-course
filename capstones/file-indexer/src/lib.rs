//! A parallel full-text indexer: scoped threads read and tokenize files,
//! one thread owns the index.

pub mod builder;
pub mod model;

pub use builder::{BuildResult, build_index, discover, index_file};
pub use model::{FileIndex, Hit, SearchIndex, tokenize};
