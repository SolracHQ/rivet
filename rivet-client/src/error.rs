//! Error types for the Rivet client

use thiserror::Error;

/// Result type alias for client operations
pub type Result<T> = std::result::Result<T, ClientError>;

/// Errors that can occur when using the Rivet client
#[derive(Debug, Error)]
pub enum ClientError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    /// API returned an error status code
    #[error("API error (status {status}): {message}")]
    ApiError {
        /// HTTP status code
        status: u16,
        /// Error message from the API
        message: String,
    },

    /// Failed to parse response
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl ClientError {
    /// Create an API error from status code and message
    pub fn api_error(status: u16, message: impl Into<String>) -> Self {
        Self::ApiError {
            status,
            message: message.into(),
        }
    }

    /// Check if this error is a "not found" error
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound(_)) || matches!(self, Self::ApiError { status: 404, .. })
    }

    /// Check if this error is a client error (4xx status)
    pub fn is_client_error(&self) -> bool {
        matches!(self, Self::ApiError { status, .. } if *status >= 400 && *status < 500)
    }

    /// Check if this error is a server error (5xx status)
    pub fn is_server_error(&self) -> bool {
        matches!(self, Self::ApiError { status, .. } if *status >= 500)
    }
}
