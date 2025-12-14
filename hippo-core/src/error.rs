//! Error types for Hippo

use thiserror::Error;

/// Result type for Hippo operations
pub type Result<T> = std::result::Result<T, HippoError>;

/// Errors that can occur in Hippo
#[derive(Error, Debug)]
pub enum HippoError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Vector database error: {0}")]
    VectorDb(String),
    
    #[error("Embedding error: {0}")]
    Embedding(String),
    
    #[error("Indexing error: {0}")]
    Indexing(String),
    
    #[error("Source error: {0}")]
    Source(String),
    
    #[error("Search error: {0}")]
    Search(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Invalid configuration: {0}")]
    Config(String),
    
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for HippoError {
    fn from(err: anyhow::Error) -> Self {
        HippoError::Other(err.to_string())
    }
}
