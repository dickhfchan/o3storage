// Phase 2: Custom Storage Format - Zero Dependencies, Maximum Security
#![no_std]

extern crate alloc;
use alloc::{collections::BTreeMap, vec::Vec, string::String};
use heapless::String as HeaplessString;
use core::mem::size_of;

use crate::crypto::CryptoEngine;
use crate::filesystem::FileSystem;

/// Custom O3Storage file format - replaces Parquet with zero dependencies
/// Format: [MAGIC][VERSION][HEADER][INDEX][DATA]
pub struct StorageManager {
    index: BTreeMap<String, ObjectMetadata>,
    filesystem: FileSystem,
    crypto: CryptoEngine,
    next_object_id: u64,
}

/// Magic bytes for O3Storage format
const O3_MAGIC: [u8; 8] = *b"O3STOR01";
const O3_VERSION: u32 = 1;

/// Object metadata stored in index
#[derive(Clone, Debug)]
pub struct ObjectMetadata {
    pub id: u64,
    pub bucket: String,
    pub key: String,
    pub version_id: [u8; 16],
    pub size: u64,
    pub offset: u64,           // Offset in data file
    pub checksum: [u8; 32],    // BLAKE3 hash
    pub created_at: u64,       // Unix timestamp
    pub content_type: String,
    pub is_deleted: bool,
}

/// File header structure
#[repr(C, packed)]
struct FileHeader {
    magic: [u8; 8],
    version: u32,
    index_offset: u64,
    index_size: u64,
    data_offset: u64,
    total_objects: u64,
    checksum: [u8; 32],
}

/// Index entry structure
#[repr(C, packed)]
struct IndexEntry {
    object_id: u64,
    bucket_hash: u64,
    key_hash: u64,
    version_id: [u8; 16],
    size: u64,
    offset: u64,
    checksum: [u8; 32],
    created_at: u64,
    flags: u32,            // is_deleted, etc.
    bucket_len: u16,
    key_len: u16,
    content_type_len: u16,
    // Variable length data follows: bucket, key, content_type
}

impl StorageManager {
    pub fn new() -> Self {
        Self {
            index: BTreeMap::new(),
            filesystem: FileSystem::new(),
            crypto: CryptoEngine::new(),
            next_object_id: 1,
        }
    }

    pub fn initialize(&mut self) -> Result<(), StorageError> {
        self.crypto.initialize_entropy();
        self.filesystem.initialize()?;
        
        // Try to load existing index
        if let Ok(data) = self.filesystem.read_file("metadata.o3s") {
            self.load_index_from_data(&data)?;
        } else {
            // Create new storage file
            self.create_new_storage_file()?;
        }
        
        Ok(())
    }

    pub fn put_object(
        &mut self,
        bucket: &str,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<ObjectMetadata, StorageError> {
        // Generate unique version ID
        let mut version_id = [0u8; 16];
        self.crypto.fill_random(&mut version_id);
        
        // Calculate checksum
        let checksum = self.crypto.blake3_hash(data);
        
        // Append data to storage file
        let offset = self.append_data_to_file(data)?;
        
        let metadata = ObjectMetadata {
            id: self.next_object_id,
            bucket: bucket.into(),
            key: key.into(),
            version_id,
            size: data.len() as u64,
            offset,
            checksum,
            created_at: self.get_timestamp(),
            content_type: content_type.into(),
            is_deleted: false,
        };
        
        // Update index
        let index_key = format!("{}:{}", bucket, key);
        self.index.insert(index_key, metadata.clone());
        self.next_object_id += 1;
        
        // Persist index
        self.save_index()?;
        
        Ok(metadata)
    }

    pub fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, StorageError> {
        let index_key = format!("{}:{}", bucket, key);
        
        let metadata = self.index.get(&index_key)
            .ok_or(StorageError::ObjectNotFound)?;
        
        if metadata.is_deleted {
            return Err(StorageError::ObjectNotFound);
        }
        
        // Read data from file
        let data = self.read_data_from_file(metadata.offset, metadata.size)?;
        
        // Verify checksum
        let computed_checksum = self.crypto.blake3_hash(&data);
        if computed_checksum != metadata.checksum {
            return Err(StorageError::CorruptedData);
        }
        
        Ok(data)
    }

    pub fn delete_object(&mut self, bucket: &str, key: &str) -> Result<(), StorageError> {
        let index_key = format!("{}:{}", bucket, key);
        
        if let Some(metadata) = self.index.get_mut(&index_key) {
            metadata.is_deleted = true;
            self.save_index()?;
            Ok(())
        } else {
            Err(StorageError::ObjectNotFound)
        }
    }

    pub fn list_objects(&self, bucket: &str, prefix: Option<&str>) -> Vec<&ObjectMetadata> {
        self.index
            .values()
            .filter(|meta| {
                !meta.is_deleted
                    && meta.bucket == bucket
                    && prefix.map_or(true, |p| meta.key.starts_with(p))
            })
            .collect()
    }

    fn create_new_storage_file(&self) -> Result<(), StorageError> {
        let header = FileHeader {
            magic: O3_MAGIC,
            version: O3_VERSION,
            index_offset: size_of::<FileHeader>() as u64,
            index_size: 0,
            data_offset: size_of::<FileHeader>() as u64,
            total_objects: 0,
            checksum: [0; 32],
        };
        
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                &header as *const FileHeader as *const u8,
                size_of::<FileHeader>(),
            )
        };
        
        self.filesystem.write_file("metadata.o3s", header_bytes)
            .map_err(|_| StorageError::IoError)
    }

    fn append_data_to_file(&self, data: &[u8]) -> Result<u64, StorageError> {
        // In a real implementation, this would append to the data section
        // For now, we'll use a simple approach
        let offset = self.filesystem.get_file_size("data.o3s")
            .unwrap_or(0);
        
        self.filesystem.append_file("data.o3s", data)
            .map_err(|_| StorageError::IoError)?;
        
        Ok(offset)
    }

    fn read_data_from_file(&self, offset: u64, size: u64) -> Result<Vec<u8>, StorageError> {
        self.filesystem.read_file_range("data.o3s", offset, size)
            .map_err(|_| StorageError::IoError)
    }

    fn save_index(&self) -> Result<(), StorageError> {
        let mut index_data = Vec::new();
        
        for (_, metadata) in &self.index {
            let entry = self.metadata_to_index_entry(metadata);
            let entry_bytes = unsafe {
                core::slice::from_raw_parts(
                    &entry as *const IndexEntry as *const u8,
                    size_of::<IndexEntry>(),
                )
            };
            index_data.extend_from_slice(entry_bytes);
            
            // Append variable length data
            index_data.extend_from_slice(metadata.bucket.as_bytes());
            index_data.extend_from_slice(metadata.key.as_bytes());
            index_data.extend_from_slice(metadata.content_type.as_bytes());
        }
        
        self.filesystem.write_file("index.o3s", &index_data)
            .map_err(|_| StorageError::IoError)
    }

    fn load_index_from_data(&mut self, data: &[u8]) -> Result<(), StorageError> {
        if data.len() < size_of::<FileHeader>() {
            return Err(StorageError::CorruptedData);
        }
        
        let header = unsafe {
            &*(data.as_ptr() as *const FileHeader)
        };
        
        if header.magic != O3_MAGIC {
            return Err(StorageError::InvalidFormat);
        }
        
        if header.version != O3_VERSION {
            return Err(StorageError::UnsupportedVersion);
        }
        
        // Load index data
        if let Ok(index_data) = self.filesystem.read_file("index.o3s") {
            self.parse_index_data(&index_data)?;
        }
        
        Ok(())
    }

    fn parse_index_data(&mut self, data: &[u8]) -> Result<(), StorageError> {
        let mut offset = 0;
        
        while offset + size_of::<IndexEntry>() <= data.len() {
            let entry = unsafe {
                &*(data[offset..].as_ptr() as *const IndexEntry)
            };
            
            offset += size_of::<IndexEntry>();
            
            // Read variable length data
            if offset + entry.bucket_len as usize + entry.key_len as usize + entry.content_type_len as usize > data.len() {
                break;
            }
            
            let bucket = String::from_utf8_lossy(&data[offset..offset + entry.bucket_len as usize]).into_owned();
            offset += entry.bucket_len as usize;
            
            let key = String::from_utf8_lossy(&data[offset..offset + entry.key_len as usize]).into_owned();
            offset += entry.key_len as usize;
            
            let content_type = String::from_utf8_lossy(&data[offset..offset + entry.content_type_len as usize]).into_owned();
            offset += entry.content_type_len as usize;
            
            let metadata = ObjectMetadata {
                id: entry.object_id,
                bucket,
                key: key.clone(),
                version_id: entry.version_id,
                size: entry.size,
                offset: entry.offset,
                checksum: entry.checksum,
                created_at: entry.created_at,
                content_type,
                is_deleted: (entry.flags & 1) != 0,
            };
            
            let index_key = format!("{}:{}", metadata.bucket, key);
            self.index.insert(index_key, metadata);
        }
        
        Ok(())
    }

    fn metadata_to_index_entry(&self, metadata: &ObjectMetadata) -> IndexEntry {
        IndexEntry {
            object_id: metadata.id,
            bucket_hash: self.hash_string(&metadata.bucket),
            key_hash: self.hash_string(&metadata.key),
            version_id: metadata.version_id,
            size: metadata.size,
            offset: metadata.offset,
            checksum: metadata.checksum,
            created_at: metadata.created_at,
            flags: if metadata.is_deleted { 1 } else { 0 },
            bucket_len: metadata.bucket.len() as u16,
            key_len: metadata.key.len() as u16,
            content_type_len: metadata.content_type.len() as u16,
        }
    }

    fn hash_string(&self, s: &str) -> u64 {
        let hash = self.crypto.blake3_hash(s.as_bytes());
        u64::from_le_bytes([
            hash[0], hash[1], hash[2], hash[3],
            hash[4], hash[5], hash[6], hash[7],
        ])
    }

    fn get_timestamp(&self) -> u64 {
        // In a real OS, this would use the RTC
        // For now, return a simple counter
        0
    }

    /// Process storage requests from the main loop
    pub fn process_requests(&mut self) {
        // This would handle incoming storage requests
        // For now, just a placeholder
    }
}

#[derive(Debug)]
pub enum StorageError {
    ObjectNotFound,
    CorruptedData,
    InvalidFormat,
    UnsupportedVersion,
    IoError,
    InsufficientSpace,
}

/// Custom bucket management
impl StorageManager {
    pub fn create_bucket(&mut self, name: &str) -> Result<(), StorageError> {
        // For simplicity, buckets are just prefixes in our index
        // In a real implementation, we'd have a separate bucket metadata
        Ok(())
    }

    pub fn list_buckets(&self) -> Vec<String> {
        let mut buckets = Vec::new();
        for metadata in self.index.values() {
            if !metadata.is_deleted && !buckets.contains(&metadata.bucket) {
                buckets.push(metadata.bucket.clone());
            }
        }
        buckets
    }

    pub fn delete_bucket(&mut self, name: &str) -> Result<(), StorageError> {
        // Mark all objects in bucket as deleted
        for metadata in self.index.values_mut() {
            if metadata.bucket == name {
                metadata.is_deleted = true;
            }
        }
        self.save_index()
    }
}

/// Storage statistics and monitoring
impl StorageManager {
    pub fn get_stats(&self) -> StorageStats {
        let mut total_objects = 0;
        let mut total_size = 0;
        let mut buckets = Vec::new();

        for metadata in self.index.values() {
            if !metadata.is_deleted {
                total_objects += 1;
                total_size += metadata.size;
                
                if !buckets.contains(&metadata.bucket) {
                    buckets.push(metadata.bucket.clone());
                }
            }
        }

        StorageStats {
            total_objects,
            total_size,
            bucket_count: buckets.len(),
            index_size: self.index.len(),
        }
    }
}

pub struct StorageStats {
    pub total_objects: usize,
    pub total_size: u64,
    pub bucket_count: usize,
    pub index_size: usize,
}