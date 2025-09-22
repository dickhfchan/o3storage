use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs;
use bytes::Bytes;
use uuid::Uuid;

use crate::{Result, StorageError, StorageStats, ReplicationStatus};
use crate::object::{Object, ObjectId, ObjectReference};
use crate::metadata::MetadataStore;
use crate::versioning::{VersionedObject, Version};

pub struct StorageEngine {
    storage_path: PathBuf,
    max_storage_size: u64,
    metadata_store: Arc<MetadataStore>,
    stats: Arc<RwLock<StorageStats>>,
}

impl StorageEngine {
    pub async fn new(storage_path: &str, max_storage_size: u64) -> Result<Self> {
        let storage_path = PathBuf::from(storage_path);
        
        fs::create_dir_all(&storage_path).await?;
        fs::create_dir_all(storage_path.join("objects")).await?;
        
        let metadata_path = storage_path.join("metadata.db");
        let metadata_store = Arc::new(MetadataStore::new(metadata_path).await?);
        
        let stats = Arc::new(RwLock::new(StorageStats {
            total_objects: 0,
            total_size_bytes: 0,
            used_space_bytes: 0,
            available_space_bytes: max_storage_size,
            replication_status: std::collections::HashMap::new(),
        }));

        let engine = Self {
            storage_path,
            max_storage_size,
            metadata_store,
            stats,
        };

        engine.update_stats().await?;
        
        Ok(engine)
    }

    pub async fn start(&self) -> Result<()> {
        tracing::info!("Storage engine started at {:?}", self.storage_path);
        
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            if let Err(e) = self.update_stats().await {
                tracing::error!("Failed to update storage stats: {}", e);
            }
        }
    }

    pub async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        data: Bytes,
        content_type: Option<String>,
        custom_metadata: std::collections::HashMap<String, String>,
    ) -> Result<ObjectReference> {
        if !self.metadata_store.bucket_exists(bucket).await? {
            self.metadata_store.create_bucket(bucket, None).await?;
        }

        let object = Object::new(
            bucket.to_string(),
            key.to_string(),
            data,
            content_type,
            custom_metadata,
        );

        if !object.verify_integrity() {
            return Err(StorageError::Corruption("Object failed integrity check".to_string()));
        }

        {
            let stats = self.stats.read().await;
            if stats.used_space_bytes + object.metadata.size > self.max_storage_size {
                return Err(StorageError::InsufficientSpace(
                    format!("Not enough space: {} + {} > {}", 
                           stats.used_space_bytes, object.metadata.size, self.max_storage_size)
                ));
            }
        }

        self.store_object_data(&object).await?;
        self.metadata_store.store_object(&object).await?;

        let object_ref = ObjectReference::from_object(&object);
        
        {
            let mut stats = self.stats.write().await;
            stats.total_objects += 1;
            stats.total_size_bytes += object.metadata.size;
            stats.used_space_bytes += object.metadata.size;
            stats.available_space_bytes = self.max_storage_size.saturating_sub(stats.used_space_bytes);
        }

        tracing::info!("Stored object: {} ({})", object.id, object.metadata.size);
        
        Ok(object_ref)
    }

    pub async fn get_object(&self, bucket: &str, key: &str, version_id: Option<Version>) -> Result<Option<Object>> {
        let metadata = self.metadata_store.get_object_metadata(bucket, key, version_id).await?;
        
        if let Some(obj_ref) = metadata {
            let data = self.load_object_data(&obj_ref.id).await?;
            
            let object = Object {
                id: obj_ref.id,
                data,
                metadata: crate::object::ObjectMetadata {
                    key: obj_ref.key,
                    bucket: obj_ref.bucket,
                    size: obj_ref.size,
                    content_type: "application/octet-stream".to_string(), 
                    created_at: obj_ref.last_modified,
                    etag: obj_ref.etag,
                    custom_metadata: std::collections::HashMap::new(),
                    version_id: obj_ref.version_id,
                },
                checksum: crate::object::Checksum {
                    sha256: String::new(),
                    blake3: String::new(),
                },
            };

            if !object.verify_integrity() {
                return Err(StorageError::Corruption(
                    format!("Object {} failed integrity check", object.id)
                ));
            }

            Ok(Some(object))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_object(&self, bucket: &str, key: &str, version_id: Option<Version>) -> Result<bool> {
        let result = self.metadata_store.delete_object(bucket, key, version_id).await?;
        
        if result {
            tracing::info!("Deleted object: {}:{}", bucket, key);
        }
        
        Ok(result)
    }

    pub async fn list_objects(&self, bucket: &str, prefix: Option<&str>, max_keys: usize) -> Result<Vec<ObjectReference>> {
        self.metadata_store.list_objects(bucket, prefix, max_keys).await
    }

    pub async fn get_versioned_object(&self, bucket: &str, key: &str) -> Result<Option<VersionedObject>> {
        self.metadata_store.get_versioned_object(bucket, key).await
    }

    pub async fn create_bucket(&self, name: &str, region: Option<&str>) -> Result<()> {
        self.metadata_store.create_bucket(name, region).await
    }

    pub async fn bucket_exists(&self, name: &str) -> Result<bool> {
        self.metadata_store.bucket_exists(name).await
    }

    pub async fn get_stats(&self) -> StorageStats {
        self.stats.read().await.clone()
    }

    pub async fn get_object_metadata(&self, bucket: &str, key: &str, version_id: Option<Version>) -> Result<Option<ObjectReference>> {
        self.metadata_store.get_object_metadata(bucket, key, version_id).await
    }

    async fn store_object_data(&self, object: &Object) -> Result<()> {
        let object_dir = self.storage_path.join("objects").join(&object.id[..2]);
        fs::create_dir_all(&object_dir).await?;
        
        let file_path = object_dir.join(&object.id);
        fs::write(&file_path, &object.data).await?;
        
        Ok(())
    }

    async fn load_object_data(&self, object_id: &str) -> Result<Bytes> {
        let object_dir = self.storage_path.join("objects").join(&object_id[..2]);
        let file_path = object_dir.join(object_id);
        
        match fs::read(&file_path).await {
            Ok(data) => Ok(Bytes::from(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(StorageError::ObjectNotFound(object_id.to_string()))
            }
            Err(e) => Err(StorageError::Io(e)),
        }
    }

    async fn update_stats(&self) -> Result<()> {
        let mut total_size = 0u64;
        let objects_dir = self.storage_path.join("objects");
        
        if objects_dir.exists() {
            let mut entries = fs::read_dir(&objects_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_dir() {
                    let mut sub_entries = fs::read_dir(entry.path()).await?;
                    while let Some(sub_entry) = sub_entries.next_entry().await? {
                        if let Ok(metadata) = sub_entry.metadata().await {
                            total_size += metadata.len();
                        }
                    }
                }
            }
        }

        {
            let mut stats = self.stats.write().await;
            stats.used_space_bytes = total_size;
            stats.available_space_bytes = self.max_storage_size.saturating_sub(total_size);
        }

        Ok(())
    }
}