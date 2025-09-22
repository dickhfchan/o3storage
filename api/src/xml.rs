use crate::{ListBucketsResponse, ListObjectsV2Response, BucketInfo, ObjectInfo, Owner, CommonPrefix};
use chrono::{DateTime, Utc};

pub fn serialize_list_buckets(response: &ListBucketsResponse) -> String {
    let buckets_xml = response.buckets
        .iter()
        .map(|bucket| format!(
            r#"    <Bucket>
        <Name>{}</Name>
        <CreationDate>{}</CreationDate>
    </Bucket>"#,
            escape_xml(&bucket.name),
            bucket.creation_date.format("%Y-%m-%dT%H:%M:%S%.3fZ")
        ))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Owner>
    <ID>{}</ID>
    <DisplayName>{}</DisplayName>
  </Owner>
  <Buckets>
{}
  </Buckets>
</ListAllMyBucketsResult>"#,
        escape_xml(&response.owner.id),
        escape_xml(&response.owner.display_name),
        buckets_xml
    )
}

pub fn serialize_list_objects_v2(response: &ListObjectsV2Response) -> String {
    let contents_xml = response.contents
        .iter()
        .map(|obj| {
            let owner_xml = if let Some(owner) = &obj.owner {
                format!(
                    r#"    <Owner>
      <ID>{}</ID>
      <DisplayName>{}</DisplayName>
    </Owner>"#,
                    escape_xml(&owner.id),
                    escape_xml(&owner.display_name)
                )
            } else {
                String::new()
            };

            format!(
                r#"  <Contents>
    <Key>{}</Key>
    <LastModified>{}</LastModified>
    <ETag>{}</ETag>
    <Size>{}</Size>
    <StorageClass>{}</StorageClass>
{}
  </Contents>"#,
                escape_xml(&obj.key),
                obj.last_modified.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                escape_xml(&obj.etag),
                obj.size,
                escape_xml(&obj.storage_class),
                owner_xml
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let common_prefixes_xml = response.common_prefixes
        .iter()
        .map(|cp| format!(
            r#"  <CommonPrefixes>
    <Prefix>{}</Prefix>
  </CommonPrefixes>"#,
            escape_xml(&cp.prefix)
        ))
        .collect::<Vec<_>>()
        .join("\n");

    let prefix_xml = response.prefix.as_ref()
        .map(|p| format!("  <Prefix>{}</Prefix>", escape_xml(p)))
        .unwrap_or_default();

    let delimiter_xml = response.delimiter.as_ref()
        .map(|d| format!("  <Delimiter>{}</Delimiter>", escape_xml(d)))
        .unwrap_or_default();

    let encoding_type_xml = response.encoding_type.as_ref()
        .map(|e| format!("  <EncodingType>{}</EncodingType>", escape_xml(e)))
        .unwrap_or_default();

    let continuation_token_xml = response.continuation_token.as_ref()
        .map(|t| format!("  <ContinuationToken>{}</ContinuationToken>", escape_xml(t)))
        .unwrap_or_default();

    let next_continuation_token_xml = response.next_continuation_token.as_ref()
        .map(|t| format!("  <NextContinuationToken>{}</NextContinuationToken>", escape_xml(t)))
        .unwrap_or_default();

    let start_after_xml = response.start_after.as_ref()
        .map(|s| format!("  <StartAfter>{}</StartAfter>", escape_xml(s)))
        .unwrap_or_default();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Name>{}</Name>
{}
{}
  <MaxKeys>{}</MaxKeys>
  <IsTruncated>{}</IsTruncated>
  <KeyCount>{}</KeyCount>
{}
{}
{}
{}
{}
{}
</ListBucketResult>"#,
        escape_xml(&response.name),
        prefix_xml,
        delimiter_xml,
        response.max_keys,
        response.is_truncated,
        response.key_count,
        encoding_type_xml,
        continuation_token_xml,
        next_continuation_token_xml,
        start_after_xml,
        contents_xml,
        common_prefixes_xml
    )
}

pub fn serialize_create_bucket() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<CreateBucketConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
</CreateBucketConfiguration>"#.to_string()
}

pub fn serialize_delete_result(key: &str, version_id: Option<&str>, delete_marker: bool) -> String {
    let version_xml = version_id
        .map(|v| format!("  <VersionId>{}</VersionId>", escape_xml(v)))
        .unwrap_or_default();

    let delete_marker_xml = if delete_marker {
        "  <DeleteMarker>true</DeleteMarker>".to_string()
    } else {
        String::new()
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<DeleteResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Deleted>
    <Key>{}</Key>
{}
{}
  </Deleted>
</DeleteResult>"#,
        escape_xml(key),
        version_xml,
        delete_marker_xml
    )
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}