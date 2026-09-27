//! Storage adapters for attachments.
//!
//! Production points the port at a private R2 bucket. Until those credentials
//! exist, development can point it at a directory on this machine. Either way
//! the adapter keeps every object under its root: a key must be relative and
//! must not contain traversal components.

use std::path::{Component, Path, PathBuf};

use school_collect_application::storage::{ObjectStorage, StorageError, StorageFuture};

pub struct FileStorage {
    root: PathBuf,
}

impl FileStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path_for(&self, key: &str) -> Result<PathBuf, StorageError> {
        let relative = Path::new(key);
        if relative.as_os_str().is_empty() || relative.is_absolute() {
            return Err(StorageError::Failed(
                "object key must be a relative path".to_owned(),
            ));
        }
        for component in relative.components() {
            if !matches!(component, Component::Normal(_)) {
                return Err(StorageError::Failed(
                    "object key must not contain traversal".to_owned(),
                ));
            }
        }
        Ok(self.root.join(relative))
    }
}

impl ObjectStorage for FileStorage {
    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        _content_type: &'a str,
    ) -> StorageFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let path = self.path_for(key)?;
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|error| StorageError::Failed(error.to_string()))?;
            }
            tokio::fs::write(&path, bytes)
                .await
                .map_err(|error| StorageError::Failed(error.to_string()))?;
            Ok(())
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> StorageFuture<'a, Result<Option<Vec<u8>>, StorageError>> {
        Box::pin(async move {
            let path = self.path_for(key)?;
            match tokio::fs::read(&path).await {
                Ok(bytes) => Ok(Some(bytes)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(StorageError::Failed(error.to_string())),
            }
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> StorageFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let path = self.path_for(key)?;
            match tokio::fs::remove_file(&path).await {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(StorageError::Failed(error.to_string())),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::FileStorage;
    use school_collect_application::storage::ObjectStorage;

    #[tokio::test]
    async fn file_storage_round_trips_and_refuses_traversal() {
        let root = std::env::temp_dir().join(format!("storage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let storage = FileStorage::new(root.clone());

        storage
            .put("tenants/a/attachments/1", b"hello".to_vec(), "text/plain")
            .await
            .expect("put");
        assert_eq!(
            storage.get("tenants/a/attachments/1").await.expect("get"),
            Some(b"hello".to_vec())
        );
        assert_eq!(storage.get("tenants/a/missing").await.expect("get"), None);

        storage
            .delete("tenants/a/attachments/1")
            .await
            .expect("delete");
        assert_eq!(
            storage.get("tenants/a/attachments/1").await.expect("get"),
            None
        );
        // Removing something twice is not an error.
        storage
            .delete("tenants/a/attachments/1")
            .await
            .expect("delete again");

        assert!(
            storage
                .put("../escape", b"x".to_vec(), "text/plain")
                .await
                .is_err()
        );
        assert!(
            storage
                .put("/absolute", b"x".to_vec(), "text/plain")
                .await
                .is_err()
        );
        assert!(storage.get("tenants/../escape").await.is_err());

        let _ = std::fs::remove_dir_all(&root);
    }
}
