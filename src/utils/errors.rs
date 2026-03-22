use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProvenanceError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Unsupported file format: {format}")]
    UnsupportedFormat { format: String },

    #[error("File integrity check failed: {reason}")]
    IntegrityFailure { reason: String },

    #[error("Metadata extraction failed: {reason}")]
    MetadataError { reason: String },

    #[error("Text extraction failed: {reason}")]
    ExtractionError { reason: String },

    #[error("Analysis failed: {reason}")]
    AnalysisError { reason: String },

    #[error("Profile error: {reason}")]
    ProfileError { reason: String },

    #[error("Insufficient text for analysis (need at least {min_words} words, got {actual_words})")]
    InsufficientText { min_words: usize, actual_words: usize },

    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ProvenanceError>;
