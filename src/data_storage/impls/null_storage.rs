use std::path::PathBuf;

use tokio::io::AsyncWriteExt;

use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use std::fs::DirBuilder;

use crate::{
    data_storage::base::Storage,
    errors::{RustusError, RustusResult},
    models::file_info::FileInfo,
    utils::{dir_struct::substr_now, headers::HeaderMapExt},
};

#[derive(Clone, Debug)]
pub struct NullStorage {
    data_dir: PathBuf,
    dir_struct: String,
}

impl NullStorage {
    #[must_use]
    pub fn new(data_dir: PathBuf, dir_struct: String) -> NullStorage {
        NullStorage {
            data_dir,
            dir_struct,
        }
    }

    /// Create path to file in a data directory.
    ///
    /// This function is using template from `dir_struct` field
    /// and based on it creates path to file.
    ///
    /// # Errors
    ///
    /// Might retur an error, if path is invalid, or directory cannot be created.
    pub fn data_file_path(&self, file_id: &str) -> RustusResult<PathBuf> {
        let dir = self
            .data_dir
            // We're working wit absolute paths, because tus.io says so.
            .canonicalize()?
            .join(substr_now(self.dir_struct.as_str()));
        DirBuilder::new().recursive(true).create(dir.as_path())?;
        Ok(dir.join(file_id))
    }
}

impl Storage for NullStorage {
    fn get_name(&self) -> &'static str {
        "file"
    }

    async fn prepare(&mut self) -> RustusResult<()> {
        // We're creating directory for new files
        // if it doesn't already exist.
        if !self.data_dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(self.data_dir.as_path())?;
        }
        Ok(())
    }

    async fn get_contents(&self, file_info: &FileInfo) -> RustusResult<Response> {
        if file_info.path.is_none() {
            return Err(RustusError::FileNotFound);
        };
        let mut resp = axum::body::Body::empty().into_response();
        resp.headers_mut()
            .generate_disposition(file_info.get_filename());
        Ok(resp)
    }

    async fn add_bytes(&self, _file_info: &FileInfo, mut bytes: Bytes) -> RustusResult<()> {
        bytes.clear();
        Ok(())
    }

    async fn create_file(&self, file_info: &FileInfo) -> RustusResult<String> {
        // New path to file.
        let file_path = self.data_file_path(file_info.id.as_str())?;
        let mut opened = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .create_new(true)
            .open(file_path.as_path())
            .await?;
        opened.shutdown().await?;
        Ok(file_path.display().to_string())
    }

    async fn concat_files(
        &self,
        file_info: &FileInfo,
        _parts_info: Vec<FileInfo>,
    ) -> RustusResult<()> {
        if file_info.path.is_none() {
            return Err(RustusError::FileNotFound);
        };
        Ok(())
    }

    async fn remove_file(&self, file_info: &FileInfo) -> RustusResult<()> {
        let Some(path) = &file_info.path else {
            return Err(RustusError::FileNotFound);
        };
        let data_path = PathBuf::from(path);
        if !data_path.exists() {
            return Err(RustusError::FileNotFound);
        }
        tokio::fs::remove_file(data_path).await.map_err(|err| {
            tracing::error!("{:?}", err);
            RustusError::UnableToRemove(String::from(path.as_str()))
        })?;
        Ok(())
    }
}
