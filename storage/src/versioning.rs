use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use crate::object::{Object, ObjectId, ObjectReference};

pub type Version = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedObject {
    pub bucket: String,
    pub key: String,
    pub versions: BTreeMap<DateTime<Utc>, VersionInfo>,
    pub latest_version: Option<Version>,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version_id: Version,
    pub object_id: ObjectId,
    pub size: u64,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub is_delete_marker: bool,
}

impl VersionedObject {
    pub fn new(bucket: String, key: String) -> Self {
        Self {
            bucket,
            key,
            versions: BTreeMap::new(),
            latest_version: None,
            is_deleted: false,
        }
    }

    pub fn add_version(&mut self, object: &Object) {
        let version_info = VersionInfo {
            version_id: object.metadata.version_id,
            object_id: object.id.clone(),
            size: object.metadata.size,
            etag: object.metadata.etag.clone(),
            created_at: object.metadata.created_at,
            is_delete_marker: false,
        };

        self.versions.insert(object.metadata.created_at, version_info);
        self.latest_version = Some(object.metadata.version_id);
        self.is_deleted = false;
    }

    pub fn add_delete_marker(&mut self, version_id: Version) -> DeleteMarker {
        let created_at = Utc::now();
        let version_info = VersionInfo {
            version_id,
            object_id: String::new(),
            size: 0,
            etag: String::new(),
            created_at,
            is_delete_marker: true,
        };

        self.versions.insert(created_at, version_info);
        self.latest_version = Some(version_id);
        self.is_deleted = true;

        DeleteMarker {
            bucket: self.bucket.clone(),
            key: self.key.clone(),
            version_id,
            created_at,
        }
    }

    pub fn get_version(&self, version_id: Option<Version>) -> Option<&VersionInfo> {
        match version_id {
            Some(id) => self.versions.values().find(|v| v.version_id == id),
            None => self.get_latest_version(),
        }
    }

    pub fn get_latest_version(&self) -> Option<&VersionInfo> {
        self.versions.values().last()
    }

    pub fn list_versions(&self) -> Vec<ObjectVersion> {
        self.versions
            .values()
            .map(|v| ObjectVersion {
                bucket: self.bucket.clone(),
                key: self.key.clone(),
                version_id: v.version_id,
                is_latest: Some(v.version_id) == self.latest_version,
                last_modified: v.created_at,
                etag: v.etag.clone(),
                size: v.size,
                storage_class: crate::object::StorageClass::Standard,
                is_delete_marker: v.is_delete_marker,
            })
            .collect()
    }

    pub fn delete_version(&mut self, version_id: Version) -> bool {
        let to_remove = self.versions
            .iter()
            .find(|(_, v)| v.version_id == version_id)
            .map(|(k, _)| *k);

        if let Some(timestamp) = to_remove {
            self.versions.remove(&timestamp);
            
            if Some(version_id) == self.latest_version {
                self.latest_version = self.versions.values().last().map(|v| v.version_id);
                self.is_deleted = self.versions.values().last()
                    .map(|v| v.is_delete_marker)
                    .unwrap_or(true);
            }
            
            true
        } else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectVersion {
    pub bucket: String,
    pub key: String,
    pub version_id: Version,
    pub is_latest: bool,
    pub last_modified: DateTime<Utc>,
    pub etag: String,
    pub size: u64,
    pub storage_class: crate::object::StorageClass,
    pub is_delete_marker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMarker {
    pub bucket: String,
    pub key: String,
    pub version_id: Version,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVersionsResponse {
    pub bucket: String,
    pub prefix: Option<String>,
    pub key_marker: Option<String>,
    pub version_id_marker: Option<Version>,
    pub max_keys: usize,
    pub is_truncated: bool,
    pub versions: Vec<ObjectVersion>,
    pub delete_markers: Vec<DeleteMarker>,
    pub next_key_marker: Option<String>,
    pub next_version_id_marker: Option<Version>,
}