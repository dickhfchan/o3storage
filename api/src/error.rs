use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Consensus error: {0}")]
    Consensus(String),
    
    #[error("Bucket not found: {0}")]
    NoSuchBucket(String),
    
    #[error("Object not found: {0}")]
    NoSuchKey(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Insufficient replicas")]
    InsufficientReplicas,
    
    #[error("XML parsing error: {0}")]
    XmlError(String),
    
    #[error("Authentication error: {0}")]
    AuthError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            ApiError::Storage(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "InternalError", msg),
            ApiError::Consensus(msg) => (StatusCode::SERVICE_UNAVAILABLE, "ServiceUnavailable", msg),
            ApiError::NoSuchBucket(msg) => (StatusCode::NOT_FOUND, "NoSuchBucket", msg),
            ApiError::NoSuchKey(msg) => (StatusCode::NOT_FOUND, "NoSuchKey", msg),
            ApiError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, "InvalidRequest", msg),
            ApiError::AccessDenied(msg) => (StatusCode::FORBIDDEN, "AccessDenied", msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "InternalError", msg),
            ApiError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, "ServiceUnavailable", msg),
            ApiError::InsufficientReplicas => (StatusCode::SERVICE_UNAVAILABLE, "ServiceUnavailable", "Insufficient replicas for write operation".to_string()),
            ApiError::XmlError(msg) => (StatusCode::BAD_REQUEST, "MalformedXML", msg),
            ApiError::AuthError(msg) => (StatusCode::UNAUTHORIZED, "InvalidAccessKeyId", msg),
        };

        let error_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>{}</Code>
    <Message>{}</Message>
    <RequestId>{}</RequestId>
</Error>"#,
            error_code,
            escape_xml(&message),
            uuid::Uuid::new_v4()
        );

        (
            status,
            [("content-type", "application/xml")],
            error_xml,
        ).into_response()
    }
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}