use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use blake3;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashMap;

pub type ObjectId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub id: ObjectId,
    pub data: Bytes,
    pub metadata: ObjectMetadata,
    pub checksum: Checksum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub key: String,
    pub bucket: String,
    pub size: u64,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub etag: String,
    pub custom_metadata: HashMap<String, String>,
    pub version_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksum {
    pub sha256: String,
    pub blake3: String,
}

impl Object {
    pub fn new(
        bucket: String,
        key: String,
        data: Bytes,
        content_type: Option<String>,
        custom_metadata: HashMap<String, String>,
    ) -> Self {
        let size = data.len() as u64;
        let checksum = Self::calculate_checksum(&data);
        let version_id = Uuid::new_v4();
        
        let id = Self::generate_id(&bucket, &key, &checksum.blake3);
        let etag = format!("\"{}\"", &checksum.blake3[..32]);
        
        let metadata = ObjectMetadata {
            key,
            bucket,
            size,
            content_type: content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
            created_at: Utc::now(),
            etag,
            custom_metadata,
            version_id,
        };

        Self {
            id,
            data,
            metadata,
            checksum,
        }
    }

    pub fn verify_integrity(&self) -> bool {
        let calculated = Self::calculate_checksum(&self.data);
        calculated.sha256 == self.checksum.sha256 && calculated.blake3 == self.checksum.blake3
    }

    fn calculate_checksum(data: &[u8]) -> Checksum {
        let mut sha256_hasher = Sha256::new();
        sha256_hasher.update(data);
        let sha256 = format!("{:x}", sha256_hasher.finalize());

        let blake3_hash = blake3::hash(data);
        let blake3 = blake3_hash.to_hex().to_string();

        Checksum { sha256, blake3 }
    }

    fn generate_id(bucket: &str, key: &str, blake3_hash: &str) -> ObjectId {
        format!("{}:{}:{}", bucket, key, &blake3_hash[..16])
    }

    pub fn content_length(&self) -> u64 {
        self.metadata.size
    }

    pub fn last_modified(&self) -> DateTime<Utc> {
        self.metadata.created_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectReference {
    pub id: ObjectId,
    pub bucket: String,
    pub key: String,
    pub version_id: Uuid,
    pub size: u64,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
    pub storage_class: StorageClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageClass {
    Standard,
    Archive,
}

impl Default for StorageClass {
    fn default() -> Self {
        StorageClass::Standard
    }
}

impl ObjectReference {
    pub fn from_object(object: &Object) -> Self {
        Self {
            id: object.id.clone(),
            bucket: object.metadata.bucket.clone(),
            key: object.metadata.key.clone(),
            version_id: object.metadata.version_id,
            size: object.metadata.size,
            etag: object.metadata.etag.clone(),
            last_modified: object.metadata.created_at,
            storage_class: StorageClass::default(),
        }
    }
}