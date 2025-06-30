use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::platform::filesystem::inner::Inner;
use crate::platform::KBLNK_DEFAULT_APP_CONFIG_DIR_NAME;

/// Wrapper for filesystem utilities for easier testing
#[derive(Clone, Debug, Default)]
pub struct Filesystem(inner::Inner);

mod inner {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, Default)]
    pub(super) enum Inner {
        #[default]
        Real,
        Chroot(Arc<tempfile::TempDir>),
        Mock(Arc<Mutex<HashMap<PathBuf, Vec<u8>>>>)
    }
}

impl Filesystem {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn new_chroot() -> Self {
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        Self(Inner::Chroot(Arc::new(temp_dir)))
    }

    /// Pass through to [tokio::fs::read] in production mode
    pub async fn read(&self, path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
        match &self.0 {
            Inner::Real => tokio::fs::read(path).await,
            Inner::Chroot(root) => tokio::fs::read(root.path().join(path)).await,
            Inner::Mock(map) => {
                let Ok(lock) = map.lock() else {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "lock poisoned"));
                };

                let Some(data) = lock.get(path.as_ref()) else {
                    return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "not found"))
                };

                Ok(data.clone())
            },
        }
    }

    /// Pass through to [tokio::fs::read_to_string] in production mode
    pub async fn read_to_string(&self, path: impl AsRef<Path>) -> std::io::Result<String> {
        match &self.0 {
            Inner::Real => tokio::fs::read_to_string(path).await,
            Inner::Chroot(root) => tokio::fs::read_to_string(root.path().join(path)).await,
            Inner::Mock(map) => {
                let Ok(lock) = map.lock() else {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "lock poisoned"));
                };

                let Some(data) = lock.get(path.as_ref()) else {
                    return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "not found"))
                };
                
                let Ok(str) = String::from_utf8(data.clone()) else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid data"))
                };

                Ok(str)
            }
        }
    }

    /// Pass through to [std::path::Path::exists] in production mode
    pub async fn exists(&self, path: impl AsRef<Path>) -> bool {
        match &self.0 {
            Inner::Real => path.as_ref().exists(),
            Inner::Chroot(root) => root.path().join(path).exists(),
            Inner::Mock(map) => {
                let Ok(lock) = map.lock() else {
                    panic!("lock poisoned")
                };
                
                lock.contains_key(path.as_ref())
            }
        }
    }

    /// Pass through to [tokio::fs::write] in production mode
    pub async fn write(&self, path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> std::io::Result<()> {
        match &self.0 {
            Inner::Real => tokio::fs::write(path, content).await,
            Inner::Chroot(root) => tokio::fs::write(root.path().join(path), content).await,
            Inner::Mock(map) => {
                let Ok(mut lock) = map.lock() else {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "lock poisoned"))
                };
                
                lock.insert(path.as_ref().to_owned(), content.as_ref().to_owned());
                Ok(())
            }
        }
    }
    
    pub fn is_dir(&self, path: impl AsRef<Path>) -> bool {
        match &self.0 {
            Inner::Real => path.as_ref().is_dir(),
            Inner::Chroot(root) => root.path().join(path.as_ref()).is_dir(),
            Inner::Mock(_) => todo!("is_dir is not implemented for Mock filesystem"),
        }
    }
    
    pub fn is_file(&self, path: impl AsRef<Path>) -> bool {
        match &self.0 {
            Inner::Real => path.as_ref().is_file(),
            Inner::Chroot(root) => root.path().join(path.as_ref()).is_file(),
            Inner::Mock(_) => todo!("is_file is not implemented for Mock filesystem"),
        } 
    }
    
    pub async fn create_dir_all(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        if self.is_file(&path) {
            return Err(std::io::Error::new(std::io::ErrorKind::NotADirectory, "not a directory"))
        }
        
        match &self.0 {
            Inner::Real => tokio::fs::create_dir_all(path.as_ref()).await,
            Inner::Chroot(root) => {
                let path = root.path().join(path.as_ref());
                tokio::fs::create_dir_all(path).await
            }
            Inner::Mock(_) => todo!("create_dir_all is not implemented for Mock filesystem"),
        }
    }
}

pub fn get_home_directory() -> Option<PathBuf> {
    dirs::home_dir()
}