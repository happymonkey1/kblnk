use std::path::PathBuf;
use std::sync::Arc;
use crate::platform::env::Env;
use crate::platform::filesystem::{get_home_directory, Filesystem};
use crate::platform::os::OsPlatform;
use crate::platform::error::{PlatformError, Result};

mod filesystem;
mod env;
mod os;
pub mod error;

const KBLNK_ENV_APP_CONFIG_DIR_NAME: &str = "KBLNK_APP_CONFIG_DIR_NAME";
pub const KBLNK_DEFAULT_APP_CONFIG_DIR_NAME: &str = ".kblnk";
const KBLNK_ENV_GLOBAL_CONTEXT_DIR_NAME: &str = "KBLNK_GLOBAL_CONTEXT_DIR_NAME";
pub const KBLNK_DEFAULT_GLOBAL_CONTEXT_DIR_NAME: &str = "context";

#[derive(Clone, Debug)]
pub struct PlatformContext {
    fs: Filesystem,
    env: Env,
    platform: OsPlatform,
}

impl PlatformContext {
    pub fn new() -> Arc<Self> {
        Arc::new_cyclic(|_| Self {
            fs: Filesystem::new(),
            env: Env::new(),
            platform: OsPlatform::new(),
        })
    }
    
    pub fn fs(&self) -> &Filesystem {
        &self.fs
    }
    
    pub fn env(&self) -> &Env {
        &self.env
    }
    
    pub fn platform(&self) -> &OsPlatform {
        &self.platform
    }
    
    pub fn get_home_directory(&self) -> Result<PathBuf> {
        get_home_directory().ok_or(PlatformError::HomeDirNotExistError)
    }
    
    pub fn get_app_config_directory(&self) -> Result<PathBuf> {
        Ok(self.get_home_directory()?.join(KBLNK_DEFAULT_APP_CONFIG_DIR_NAME))
    }

    pub fn get_global_context_path(&self) -> Result<PathBuf> {
        Ok(self.get_app_config_directory()?.join(KBLNK_DEFAULT_GLOBAL_CONTEXT_DIR_NAME))
    }
}