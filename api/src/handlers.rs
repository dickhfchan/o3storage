use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    body::Bytes,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::{ApiError, ApiResult};
use crate::auth::{extract_auth_info, AuthContext};
use crate::xml;
use crate::{
    ListBucketsResponse, ListObjectsV2Response, BucketInfo, ObjectInfo, Owner, CommonPrefix,
    PutObjectResponse, GetObjectResponse, DeleteObjectResponse, HeadObjectResponse,
};

pub struct AppState {
    pub storage_engine: Arc<storage::StorageEngine>,
    pub consensus_manager: Arc<consensus::ConsensusManager>,
    pub cluster_state: Arc<tokio::sync::RwLock<crate::ClusterState>>,
}

#[derive(serde::Deserialize)]
pub struct ListObjectsV2Query {
    #[serde(rename = "list-type")]
    list_type: Option<String>,
    #[serde(rename = "continuation-token")]
    continuation_token: Option<String>,
    #[serde(rename = "max-keys")]
    max_keys: Option<u32>,
    prefix: Option<String>,
    delimiter: Option<String>,
    #[serde(rename = "start-after")]
    start_after: Option<String>,
    #[serde(rename = "encoding-type")]
    encoding_type: Option<String>,
}

pub async fn list_buckets(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let _auth = extract_auth_info(&headers)?;
    
    // For now, return a simple bucket list
    // In a real implementation, this would query the storage engine
    let response = ListBucketsResponse {
        buckets: vec![
            BucketInfo {
                name: "default".to_string(),
                creation_date: chrono::Utc::now(),
            }
        ],
        owner: Owner {
            id: "o3storage".to_string(),
            display_name: "O3Storage System".to_string(),
        },
    };

    let xml = xml::serialize_list_buckets(&response);
    
    Ok((
        StatusCode::OK,
        [("content-type", "application/xml")],
        xml,
    ).into_response())
}

pub async fn create_bucket(
    State(state): State<Arc<AppState>>,
    Path(bucket): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let _auth = extract_auth_info(&headers)?;
    
    // Check if writes are enabled
    let cluster_state = state.cluster_state.read().await;
    if !cluster_state.is_write_enabled {
        return Err(ApiError::InsufficientReplicas);
    }
    drop(cluster_state);
    
    state.storage_engine.create_bucket(&bucket, None).await
        .map_err(|e| ApiError::Storage(e.to_string()))?;
    
    let xml = xml::serialize_create_bucket();
    
    Ok((
        StatusCode::OK,
        [("content-type", "application/xml")],
        xml,
    ).into_response())
}

pub async fn list_objects_v2(
    State(state): State<Arc<AppState>>,
    Path(bucket): Path<String>,
    Query(query): Query<ListObjectsV2Query>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let _auth = extract_auth_info(&headers)?;
    
    if !state.storage_engine.bucket_exists(&bucket).await
        .map_err(|e| ApiError::Storage(e.to_string()))? {
        return Err(ApiError::NoSuchBucket(bucket));
    }
    
    let max_keys = query.max_keys.unwrap_or(1000).min(1000) as usize;
    let prefix = query.prefix.as_deref();
    
    let objects = state.storage_engine.list_objects(&bucket, prefix, max_keys).await
        .map_err(|e| ApiError::Storage(e.to_string()))?;
    
    let contents: Vec<ObjectInfo> = objects.into_iter().map(|obj| {
        ObjectInfo {
            key: obj.key,
            last_modified: obj.last_modified,
            etag: obj.etag,
            size: obj.size,
            storage_class: "STANDARD".to_string(),
            owner: Some(Owner {
                id: "o3storage".to_string(),
                display_name: "O3Storage System".to_string(),
            }),
        }
    }).collect();
    
    let response = ListObjectsV2Response {
        is_truncated: false, // TODO: Implement pagination
        contents,
        name: bucket,
        prefix: query.prefix,
        delimiter: query.delimiter,
        max_keys: query.max_keys.unwrap_or(1000),
        common_prefixes: vec![], // TODO: Implement common prefixes
        encoding_type: query.encoding_type,
        key_count: contents.len() as u32,
        continuation_token: query.continuation_token,
        next_continuation_token: None, // TODO: Implement pagination
        start_after: query.start_after,
    };
    
    let xml = xml::serialize_list_objects_v2(&response);
    
    Ok((
        StatusCode::OK,
        [("content-type", "application/xml")],
        xml,
    ).into_response())
}

pub async fn put_object(
    State(state): State<Arc<AppState>>,
    Path((bucket, key)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Response> {
    let _auth = extract_auth_info(&headers)?;
    
    // Check if writes are enabled
    let cluster_state = state.cluster_state.read().await;
    if !cluster_state.is_write_enabled {
        return Err(ApiError::InsufficientReplicas);
    }
    drop(cluster_state);
    
    let content_type = headers.get("content-type")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    
    // Extract custom metadata from x-amz-meta-* headers
    let mut custom_metadata = HashMap::new();
    for (name, value) in headers.iter() {
        if let Some(meta_key) = name.as_str().strip_prefix("x-amz-meta-") {
            if let Ok(meta_value) = value.to_str() {
                custom_metadata.insert(meta_key.to_string(), meta_value.to_string());
            }
        }
    }
    
    let object_ref = state.storage_engine
        .put_object(&bucket, &key, body, content_type, custom_metadata).await
        .map_err(|e| ApiError::Storage(e.to_string()))?;
    
    // Trigger replication via consensus
    let metadata = consensus::ObjectReplicationMetadata {
        bucket: bucket.clone(),
        key: key.clone(),
        version_id: object_ref.version_id,
        size: object_ref.size,
        checksum: object_ref.etag.clone(),
        target_nodes: vec![], // Will be filled by consensus manager
    };
    
    if let Err(e) = state.consensus_manager.request_replication(
        object_ref.id.clone(),
        consensus::ReplicationOperation::Store,
        metadata,
        None, // Data will be read from storage by other nodes
    ).await {
        tracing::warn!("Failed to initiate replication for {}: {}", object_ref.id, e);
        // Continue anyway - the object is stored locally
    }
    
    Ok((
        StatusCode::OK,
        [
            ("etag", object_ref.etag.as_str()),
            ("x-amz-version-id", &object_ref.version_id.to_string()),
        ],
    ).into_response())
}

pub async fn get_object(
    State(state): State<Arc<AppState>>,
    Path((bucket, key)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let _auth = extract_auth_info(&headers)?;
    
    let version_id = query.get("versionId")
        .and_then(|v| Uuid::parse_str(v).ok());
    
    let object = state.storage_engine
        .get_object(&bucket, &key, version_id).await
        .map_err(|e| ApiError::Storage(e.to_string()))?;
    
    let object = object.ok_or_else(|| ApiError::NoSuchKey(format!("{}:{}", bucket, key)))?;
    
    let mut response_headers = vec![
        ("content-type", object.metadata.content_type.clone()),
        ("content-length", object.metadata.size.to_string()),
        ("etag", object.metadata.etag.clone()),
        ("last-modified", object.metadata.created_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string()),
        ("x-amz-version-id", object.metadata.version_id.to_string()),
    ];
    
    // Add custom metadata headers
    for (key, value) in &object.metadata.custom_metadata {
        response_headers.push((&format!("x-amz-meta-{}", key), value.clone()));
    }
    
    Ok((
        StatusCode::OK,
        response_headers,
        object.data,
    ).into_response())
}

pub async fn head_object(
    State(state): State<Arc<AppState>>,
    Path((bucket, key)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let _auth = extract_auth_info(&headers)?;
    
    let version_id = query.get("versionId")
        .and_then(|v| Uuid::parse_str(v).ok());
    
    let object_ref = state.storage_engine
        .get_object_metadata(&bucket, &key, version_id).await
        .map_err(|e| ApiError::Storage(e.to_string()))?;
    
    let object_ref = object_ref.ok_or_else(|| ApiError::NoSuchKey(format!("{}:{}", bucket, key)))?;
    
    Ok((
        StatusCode::OK,
        [
            ("content-length", object_ref.size.to_string()),
            ("etag", object_ref.etag),
            ("last-modified", object_ref.last_modified.format("%a, %d %b %Y %H:%M:%S GMT").to_string()),
            ("x-amz-version-id", object_ref.version_id.to_string()),
        ],
    ).into_response())
}

pub async fn delete_object(
    State(state): State<Arc<AppState>>,
    Path((bucket, key)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let _auth = extract_auth_info(&headers)?;
    
    // Check if writes are enabled
    let cluster_state = state.cluster_state.read().await;
    if !cluster_state.is_write_enabled {
        return Err(ApiError::InsufficientReplicas);
    }
    drop(cluster_state);
    
    let version_id = query.get("versionId")
        .and_then(|v| Uuid::parse_str(v).ok());
    
    let deleted = state.storage_engine
        .delete_object(&bucket, &key, version_id).await
        .map_err(|e| ApiError::Storage(e.to_string()))?;
    
    if !deleted {
        return Err(ApiError::NoSuchKey(format!("{}:{}", bucket, key)));
    }
    
    // Trigger replication via consensus
    let metadata = consensus::ObjectReplicationMetadata {
        bucket: bucket.clone(),
        key: key.clone(),
        version_id: version_id.unwrap_or_else(|| Uuid::new_v4()),
        size: 0,
        checksum: String::new(),
        target_nodes: vec![],
    };
    
    if let Err(e) = state.consensus_manager.request_replication(
        format!("{}:{}", bucket, key),
        consensus::ReplicationOperation::Delete,
        metadata,
        None,
    ).await {
        tracing::warn!("Failed to initiate delete replication for {}:{}: {}", bucket, key, e);
    }
    
    let delete_marker = version_id.is_none(); // Delete marker created when no version specified
    let xml = xml::serialize_delete_result(&key, version_id.as_ref().map(|v| v.to_string().as_str()), delete_marker);
    
    Ok((
        StatusCode::NO_CONTENT,
        [("content-type", "application/xml")],
        xml,
    ).into_response())
}

// Health check endpoint
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Response> {
    let cluster_state = state.cluster_state.read().await;
    let stats = state.storage_engine.get_stats().await;
    
    let health_info = serde_json::json!({
        "status": "healthy",
        "cluster": {
            "active_nodes": cluster_state.active_nodes.len(),
            "total_replicas": cluster_state.total_replicas,
            "write_enabled": cluster_state.is_write_enabled,
        },
        "storage": {
            "total_objects": stats.total_objects,
            "used_space_bytes": stats.used_space_bytes,
            "available_space_bytes": stats.available_space_bytes,
        }
    });
    
    Ok((
        StatusCode::OK,
        [("content-type", "application/json")],
        health_info.to_string(),
    ).into_response())
}