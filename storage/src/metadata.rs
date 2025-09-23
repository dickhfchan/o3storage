use arrow::array::{StringArray, UInt64Array, BooleanArray, TimestampMillisecondArray};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use datafusion::prelude::*;
use datafusion::execution::context::SessionContext;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json;
use tokio::fs;

use crate::{Result, StorageError};
use crate::object::{Object, ObjectReference};
use crate::versioning::{VersionedObject, Version};

pub struct MetadataStore {
    storage_path: PathBuf,
    ctx: SessionContext,
    objects_schema: Arc<Schema>,
    buckets_schema: Arc<Schema>,
    replication_schema: Arc<Schema>,
}

impl MetadataStore {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let storage_path = path.as_ref().to_path_buf();
        
        // Create metadata directory
        fs::create_dir_all(&storage_path).await?;
        
        let ctx = SessionContext::new();
        
        // Define schemas for our parquet files
        let objects_schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("bucket", DataType::Utf8, false),
            Field::new("key", DataType::Utf8, false),
            Field::new("version_id", DataType::Utf8, false),
            Field::new("size", DataType::UInt64, false),
            Field::new("etag", DataType::Utf8, false),
            Field::new("content_type", DataType::Utf8, false),
            Field::new("created_at", DataType::Timestamp(TimeUnit::Millisecond, None), false),
            Field::new("custom_metadata", DataType::Utf8, false),
            Field::new("checksum_sha256", DataType::Utf8, false),
            Field::new("checksum_blake3", DataType::Utf8, false),
            Field::new("is_delete_marker", DataType::Boolean, false),
        ]));

        let buckets_schema = Arc::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("created_at", DataType::Timestamp(TimeUnit::Millisecond, None), false),
            Field::new("region", DataType::Utf8, false),
        ]));

        let replication_schema = Arc::new(Schema::new(vec![
            Field::new("object_id", DataType::Utf8, false),
            Field::new("replicas", DataType::Utf8, false), // JSON encoded
            Field::new("target_replicas", DataType::UInt64, false),
            Field::new("is_fully_replicated", DataType::Boolean, false),
        ]));

        let store = Self {
            storage_path,
            ctx,
            objects_schema,
            buckets_schema,
            replication_schema,
        };

        // Initialize parquet files if they don't exist
        store.init_parquet_files().await?;
        
        Ok(store)
    }

    async fn init_parquet_files(&self) -> Result<()> {
        let objects_path = self.storage_path.join("objects.parquet");
        let buckets_path = self.storage_path.join("buckets.parquet");
        let replication_path = self.storage_path.join("replication.parquet");

        // Create empty parquet files if they don't exist
        if !objects_path.exists() {
            self.create_empty_parquet(&objects_path, &self.objects_schema).await?;
        }
        
        if !buckets_path.exists() {
            self.create_empty_parquet(&buckets_path, &self.buckets_schema).await?;
        }
        
        if !replication_path.exists() {
            self.create_empty_parquet(&replication_path, &self.replication_schema).await?;
        }

        // Register parquet files with DataFusion
        self.ctx.register_parquet("objects", objects_path.to_str().unwrap(), ParquetReadOptions::default()).await
            .map_err(|e| StorageError::Database(format!("Failed to register objects table: {}", e)))?;
            
        self.ctx.register_parquet("buckets", buckets_path.to_str().unwrap(), ParquetReadOptions::default()).await
            .map_err(|e| StorageError::Database(format!("Failed to register buckets table: {}", e)))?;
            
        self.ctx.register_parquet("replication", replication_path.to_str().unwrap(), ParquetReadOptions::default()).await
            .map_err(|e| StorageError::Database(format!("Failed to register replication table: {}", e)))?;

        Ok(())
    }

    async fn create_empty_parquet(&self, path: &Path, schema: &Schema) -> Result<()> {
        let file = std::fs::File::create(path)
            .map_err(|e| StorageError::Io(e))?;
        
        let props = WriterProperties::builder().build();
        let mut writer = ArrowWriter::try_new(file, schema.clone().into(), Some(props))
            .map_err(|e| StorageError::Database(format!("Failed to create parquet writer: {}", e)))?;

        // Write empty batch to create the file structure
        let empty_batch = RecordBatch::new_empty(schema.clone().into());
        writer.write(&empty_batch)
            .map_err(|e| StorageError::Database(format!("Failed to write empty batch: {}", e)))?;
            
        writer.close()
            .map_err(|e| StorageError::Database(format!("Failed to close parquet writer: {}", e)))?;

        Ok(())
    }

    pub async fn store_object(&self, object: &Object) -> Result<()> {
        let custom_metadata_json = serde_json::to_string(&object.metadata.custom_metadata)
            .map_err(|e| StorageError::Serialization(format!("JSON error: {:?}", e)))?;

        // Create arrays for the new record
        let ids = StringArray::from(vec![object.id.as_str()]);
        let buckets = StringArray::from(vec![object.metadata.bucket.as_str()]);
        let keys = StringArray::from(vec![object.metadata.key.as_str()]);
        let version_ids = StringArray::from(vec![object.metadata.version_id.to_string()]);
        let sizes = UInt64Array::from(vec![object.metadata.size]);
        let etags = StringArray::from(vec![object.metadata.etag.as_str()]);
        let content_types = StringArray::from(vec![object.metadata.content_type.as_str()]);
        let created_ats = TimestampMillisecondArray::from(vec![object.metadata.created_at.timestamp_millis()]);
        let custom_metadatas = StringArray::from(vec![custom_metadata_json.as_str()]);
        let checksum_sha256s = StringArray::from(vec![object.checksum.sha256.as_str()]);
        let checksum_blake3s = StringArray::from(vec![object.checksum.blake3.as_str()]);
        let is_delete_markers = BooleanArray::from(vec![false]);

        let batch = RecordBatch::try_new(
            self.objects_schema.clone(),
            vec![
                Arc::new(ids),
                Arc::new(buckets),
                Arc::new(keys),
                Arc::new(version_ids),
                Arc::new(sizes),
                Arc::new(etags),
                Arc::new(content_types),
                Arc::new(created_ats),
                Arc::new(custom_metadatas),
                Arc::new(checksum_sha256s),
                Arc::new(checksum_blake3s),
                Arc::new(is_delete_markers),
            ],
        ).map_err(|e| StorageError::Database(format!("Failed to create record batch: {}", e)))?;

        self.append_to_parquet("objects.parquet", batch).await?;
        Ok(())
    }

    async fn append_to_parquet(&self, filename: &str, batch: RecordBatch) -> Result<()> {
        let path = self.storage_path.join(filename);
        
        // Read existing data
        let existing_df = self.ctx.read_parquet(path.to_str().unwrap(), ParquetReadOptions::default()).await
            .map_err(|e| StorageError::Database(format!("Failed to read existing parquet: {}", e)))?;
            
        let existing_batches = existing_df.collect().await
            .map_err(|e| StorageError::Database(format!("Failed to collect existing data: {}", e)))?;

        // Combine with new data
        let mut all_batches = existing_batches;
        all_batches.push(batch);

        // Write all data back
        let file = std::fs::File::create(&path)
            .map_err(|e| StorageError::Io(e))?;
            
        let props = WriterProperties::builder().build();
        let mut writer = ArrowWriter::try_new(file, all_batches[0].schema(), Some(props))
            .map_err(|e| StorageError::Database(format!("Failed to create parquet writer: {}", e)))?;

        for batch in all_batches {
            writer.write(&batch)
                .map_err(|e| StorageError::Database(format!("Failed to write batch: {}", e)))?;
        }

        writer.close()
            .map_err(|e| StorageError::Database(format!("Failed to close parquet writer: {}", e)))?;

        // Re-register the updated file
        self.ctx.deregister_table(filename.strip_suffix(".parquet").unwrap())
            .map_err(|e| StorageError::Database(format!("Failed to deregister table: {}", e)))?;
            
        self.ctx.register_parquet(filename.strip_suffix(".parquet").unwrap(), path.to_str().unwrap(), ParquetReadOptions::default()).await
            .map_err(|e| StorageError::Database(format!("Failed to re-register table: {}", e)))?;

        Ok(())
    }

    pub async fn get_object_metadata(&self, bucket: &str, key: &str, version_id: Option<Version>) -> Result<Option<ObjectReference>> {
        let sql = if let Some(vid) = version_id {
            format!(
                "SELECT id, bucket, key, version_id, size, etag, created_at 
                 FROM objects 
                 WHERE bucket = '{}' AND key = '{}' AND version_id = '{}' AND is_delete_marker = false",
                bucket, key, vid
            )
        } else {
            format!(
                "SELECT id, bucket, key, version_id, size, etag, created_at 
                 FROM objects 
                 WHERE bucket = '{}' AND key = '{}' AND is_delete_marker = false
                 ORDER BY created_at DESC 
                 LIMIT 1",
                bucket, key
            )
        };

        let df = self.ctx.sql(&sql).await
            .map_err(|e| StorageError::Database(format!("Query failed: {}", e)))?;
            
        let batches = df.collect().await
            .map_err(|e| StorageError::Database(format!("Failed to collect results: {}", e)))?;

        if batches.is_empty() || batches[0].num_rows() == 0 {
            return Ok(None);
        }

        let batch = &batches[0];
        let id_array = batch.column(0).as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| StorageError::Database("Failed to cast id column".to_string()))?;
        let bucket_array = batch.column(1).as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| StorageError::Database("Failed to cast bucket column".to_string()))?;
        let key_array = batch.column(2).as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| StorageError::Database("Failed to cast key column".to_string()))?;
        let version_id_array = batch.column(3).as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| StorageError::Database("Failed to cast version_id column".to_string()))?;
        let size_array = batch.column(4).as_any().downcast_ref::<UInt64Array>()
            .ok_or_else(|| StorageError::Database("Failed to cast size column".to_string()))?;
        let etag_array = batch.column(5).as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| StorageError::Database("Failed to cast etag column".to_string()))?;
        let created_at_array = batch.column(6).as_any().downcast_ref::<TimestampMillisecondArray>()
            .ok_or_else(|| StorageError::Database("Failed to cast created_at column".to_string()))?;

        let obj_ref = ObjectReference {
            id: id_array.value(0).to_string(),
            bucket: bucket_array.value(0).to_string(),
            key: key_array.value(0).to_string(),
            version_id: Uuid::parse_str(version_id_array.value(0))
                .map_err(|e| StorageError::Database(format!("Invalid UUID: {}", e)))?,
            size: size_array.value(0),
            etag: etag_array.value(0).to_string(),
            last_modified: DateTime::from_timestamp_millis(created_at_array.value(0))
                .ok_or_else(|| StorageError::Database("Invalid timestamp".to_string()))?
                .with_timezone(&Utc),
            storage_class: crate::object::StorageClass::Standard,
        };

        Ok(Some(obj_ref))
    }

    pub async fn get_versioned_object(&self, bucket: &str, key: &str) -> Result<Option<VersionedObject>> {
        let sql = format!(
            "SELECT version_id, id, size, etag, created_at, is_delete_marker 
             FROM objects 
             WHERE bucket = '{}' AND key = '{}' 
             ORDER BY created_at ASC",
            bucket, key
        );

        let df = self.ctx.sql(&sql).await
            .map_err(|e| StorageError::Database(format!("Query failed: {}", e)))?;
            
        let batches = df.collect().await
            .map_err(|e| StorageError::Database(format!("Failed to collect results: {}", e)))?;

        if batches.is_empty() || batches[0].num_rows() == 0 {
            return Ok(None);
        }

        let mut versioned_obj = VersionedObject::new(bucket.to_string(), key.to_string());

        for batch in batches {
            for row in 0..batch.num_rows() {
                let version_id_array = batch.column(0).as_any().downcast_ref::<StringArray>()
                    .ok_or_else(|| StorageError::Database("Failed to cast version_id column".to_string()))?;
                let id_array = batch.column(1).as_any().downcast_ref::<StringArray>()
                    .ok_or_else(|| StorageError::Database("Failed to cast id column".to_string()))?;
                let size_array = batch.column(2).as_any().downcast_ref::<UInt64Array>()
                    .ok_or_else(|| StorageError::Database("Failed to cast size column".to_string()))?;
                let etag_array = batch.column(3).as_any().downcast_ref::<StringArray>()
                    .ok_or_else(|| StorageError::Database("Failed to cast etag column".to_string()))?;
                let created_at_array = batch.column(4).as_any().downcast_ref::<TimestampMillisecondArray>()
                    .ok_or_else(|| StorageError::Database("Failed to cast created_at column".to_string()))?;
                let is_delete_marker_array = batch.column(5).as_any().downcast_ref::<BooleanArray>()
                    .ok_or_else(|| StorageError::Database("Failed to cast is_delete_marker column".to_string()))?;

                let version_id = Uuid::parse_str(version_id_array.value(row))
                    .map_err(|e| StorageError::Database(format!("Invalid UUID: {}", e)))?;
                let object_id = id_array.value(row).to_string();
                let size = size_array.value(row);
                let etag = etag_array.value(row).to_string();
                let created_at = DateTime::from_timestamp_millis(created_at_array.value(row))
                    .ok_or_else(|| StorageError::Database("Invalid timestamp".to_string()))?
                    .with_timezone(&Utc);
                let is_delete_marker = is_delete_marker_array.value(row);

                let version_info = crate::versioning::VersionInfo {
                    version_id,
                    object_id,
                    size,
                    etag,
                    created_at,
                    is_delete_marker,
                };

                versioned_obj.versions.insert(created_at, version_info);
                
                if !is_delete_marker {
                    versioned_obj.latest_version = Some(version_id);
                    versioned_obj.is_deleted = false;
                } else {
                    versioned_obj.latest_version = Some(version_id);
                    versioned_obj.is_deleted = true;
                }
            }
        }

        Ok(Some(versioned_obj))
    }

    pub async fn list_objects(&self, bucket: &str, prefix: Option<&str>, max_keys: usize) -> Result<Vec<ObjectReference>> {
        let sql = if let Some(p) = prefix {
            format!(
                "SELECT DISTINCT bucket, key, 
                        FIRST_VALUE(id) OVER (PARTITION BY bucket, key ORDER BY created_at DESC) as id,
                        FIRST_VALUE(version_id) OVER (PARTITION BY bucket, key ORDER BY created_at DESC) as version_id,
                        FIRST_VALUE(size) OVER (PARTITION BY bucket, key ORDER BY created_at DESC) as size,
                        FIRST_VALUE(etag) OVER (PARTITION BY bucket, key ORDER BY created_at DESC) as etag,
                        FIRST_VALUE(created_at) OVER (PARTITION BY bucket, key ORDER BY created_at DESC) as created_at
                 FROM objects 
                 WHERE bucket = '{}' AND key LIKE '{}%' AND is_delete_marker = false
                 ORDER BY key 
                 LIMIT {}",
                bucket, p, max_keys
            )
        } else {
            format!(
                "SELECT DISTINCT bucket, key, 
                        FIRST_VALUE(id) OVER (PARTITION BY bucket, key ORDER BY created_at DESC) as id,
                        FIRST_VALUE(version_id) OVER (PARTITION BY bucket, key ORDER BY created_at DESC) as version_id,
                        FIRST_VALUE(size) OVER (PARTITION BY bucket, key ORDER BY created_at DESC) as size,
                        FIRST_VALUE(etag) OVER (PARTITION BY bucket, key ORDER BY created_at DESC) as etag,
                        FIRST_VALUE(created_at) OVER (PARTITION BY bucket, key ORDER BY created_at DESC) as created_at
                 FROM objects 
                 WHERE bucket = '{}' AND is_delete_marker = false
                 ORDER BY key 
                 LIMIT {}",
                bucket, max_keys
            )
        };

        let df = self.ctx.sql(&sql).await
            .map_err(|e| StorageError::Database(format!("Query failed: {}", e)))?;
            
        let batches = df.collect().await
            .map_err(|e| StorageError::Database(format!("Failed to collect results: {}", e)))?;

        let mut objects = Vec::new();
        for batch in batches {
            for row in 0..batch.num_rows() {
                let bucket_array = batch.column(0).as_any().downcast_ref::<StringArray>()
                    .ok_or_else(|| StorageError::Database("Failed to cast bucket column".to_string()))?;
                let key_array = batch.column(1).as_any().downcast_ref::<StringArray>()
                    .ok_or_else(|| StorageError::Database("Failed to cast key column".to_string()))?;
                let id_array = batch.column(2).as_any().downcast_ref::<StringArray>()
                    .ok_or_else(|| StorageError::Database("Failed to cast id column".to_string()))?;
                let version_id_array = batch.column(3).as_any().downcast_ref::<StringArray>()
                    .ok_or_else(|| StorageError::Database("Failed to cast version_id column".to_string()))?;
                let size_array = batch.column(4).as_any().downcast_ref::<UInt64Array>()
                    .ok_or_else(|| StorageError::Database("Failed to cast size column".to_string()))?;
                let etag_array = batch.column(5).as_any().downcast_ref::<StringArray>()
                    .ok_or_else(|| StorageError::Database("Failed to cast etag column".to_string()))?;
                let created_at_array = batch.column(6).as_any().downcast_ref::<TimestampMillisecondArray>()
                    .ok_or_else(|| StorageError::Database("Failed to cast created_at column".to_string()))?;

                let obj_ref = ObjectReference {
                    id: id_array.value(row).to_string(),
                    bucket: bucket_array.value(row).to_string(),
                    key: key_array.value(row).to_string(),
                    version_id: Uuid::parse_str(version_id_array.value(row))
                        .map_err(|e| StorageError::Database(format!("Invalid UUID: {}", e)))?,
                    size: size_array.value(row),
                    etag: etag_array.value(row).to_string(),
                    last_modified: DateTime::from_timestamp_millis(created_at_array.value(row))
                        .ok_or_else(|| StorageError::Database("Invalid timestamp".to_string()))?
                        .with_timezone(&Utc),
                    storage_class: crate::object::StorageClass::Standard,
                };

                objects.push(obj_ref);
            }
        }

        Ok(objects)
    }

    pub async fn delete_object(&self, bucket: &str, key: &str, version_id: Option<Version>) -> Result<bool> {
        if let Some(vid) = version_id {
            // Delete specific version - we need to rewrite the parquet file without this record
            let sql = format!(
                "SELECT * FROM objects WHERE NOT (bucket = '{}' AND key = '{}' AND version_id = '{}')",
                bucket, key, vid
            );
            
            let df = self.ctx.sql(&sql).await
                .map_err(|e| StorageError::Database(format!("Query failed: {}", e)))?;
                
            let batches = df.collect().await
                .map_err(|e| StorageError::Database(format!("Failed to collect results: {}", e)))?;

            // Rewrite the objects parquet file
            let path = self.storage_path.join("objects.parquet");
            let file = std::fs::File::create(&path)
                .map_err(|e| StorageError::Io(e))?;
                
            let props = WriterProperties::builder().build();
            let mut writer = ArrowWriter::try_new(file, self.objects_schema.clone(), Some(props))
                .map_err(|e| StorageError::Database(format!("Failed to create parquet writer: {}", e)))?;

            for batch in batches {
                writer.write(&batch)
                    .map_err(|e| StorageError::Database(format!("Failed to write batch: {}", e)))?;
            }

            writer.close()
                .map_err(|e| StorageError::Database(format!("Failed to close parquet writer: {}", e)))?;

            // Re-register the updated file
            self.ctx.deregister_table("objects")
                .map_err(|e| StorageError::Database(format!("Failed to deregister table: {}", e)))?;
                
            self.ctx.register_parquet("objects", path.to_str().unwrap(), ParquetReadOptions::default()).await
                .map_err(|e| StorageError::Database(format!("Failed to re-register table: {}", e)))?;

            Ok(true)
        } else {
            // Add delete marker
            let new_version_id = Uuid::new_v4();
            
            let ids = StringArray::from(vec![format!("{}:{}:delete-marker", bucket, key)]);
            let buckets = StringArray::from(vec![bucket]);
            let keys = StringArray::from(vec![key]);
            let version_ids = StringArray::from(vec![new_version_id.to_string()]);
            let sizes = UInt64Array::from(vec![0u64]);
            let etags = StringArray::from(vec![""]);
            let content_types = StringArray::from(vec![""]);
            let created_ats = TimestampMillisecondArray::from(vec![Utc::now().timestamp_millis()]);
            let custom_metadatas = StringArray::from(vec!["{}"]);
            let checksum_sha256s = StringArray::from(vec![""]);
            let checksum_blake3s = StringArray::from(vec![""]);
            let is_delete_markers = BooleanArray::from(vec![true]);

            let batch = RecordBatch::try_new(
                self.objects_schema.clone(),
                vec![
                    Arc::new(ids),
                    Arc::new(buckets),
                    Arc::new(keys),
                    Arc::new(version_ids),
                    Arc::new(sizes),
                    Arc::new(etags),
                    Arc::new(content_types),
                    Arc::new(created_ats),
                    Arc::new(custom_metadatas),
                    Arc::new(checksum_sha256s),
                    Arc::new(checksum_blake3s),
                    Arc::new(is_delete_markers),
                ],
            ).map_err(|e| StorageError::Database(format!("Failed to create record batch: {}", e)))?;

            self.append_to_parquet("objects.parquet", batch).await?;
            Ok(true)
        }
    }

    pub async fn create_bucket(&self, name: &str, region: Option<&str>) -> Result<()> {
        let names = StringArray::from(vec![name]);
        let created_ats = TimestampMillisecondArray::from(vec![Utc::now().timestamp_millis()]);
        let regions = StringArray::from(vec![region.unwrap_or("us-east-1")]);

        let batch = RecordBatch::try_new(
            self.buckets_schema.clone(),
            vec![
                Arc::new(names),
                Arc::new(created_ats),
                Arc::new(regions),
            ],
        ).map_err(|e| StorageError::Database(format!("Failed to create record batch: {}", e)))?;

        self.append_to_parquet("buckets.parquet", batch).await?;
        Ok(())
    }

    pub async fn bucket_exists(&self, name: &str) -> Result<bool> {
        let sql = format!("SELECT COUNT(*) as count FROM buckets WHERE name = '{}'", name);
        
        let df = self.ctx.sql(&sql).await
            .map_err(|e| StorageError::Database(format!("Query failed: {}", e)))?;
            
        let batches = df.collect().await
            .map_err(|e| StorageError::Database(format!("Failed to collect results: {}", e)))?;

        if batches.is_empty() || batches[0].num_rows() == 0 {
            return Ok(false);
        }

        let batch = &batches[0];
        let count_array = batch.column(0).as_any().downcast_ref::<UInt64Array>()
            .ok_or_else(|| StorageError::Database("Failed to cast count column".to_string()))?;

        Ok(count_array.value(0) > 0)
    }
}