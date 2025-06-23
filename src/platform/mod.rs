use std::path::PathBuf;
use std::sync::Arc;
use crate::platform::env::Env;
use crate::platform::filesystem::{get_app_config_directory, get_home_directory, Filesystem};
use crate::platform::os::OsPlatform;

mod filesystem;
mod env;
mod os;

const KBLNK_ENV_APP_CONFIG_DIR_NAME: &str = "KBLNK_APP_CONFIG_DIR_NAME";
const KBLNK_DEFAULT_APP_CONFIG_DIR_NAME: &str = ".kblnk";

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
    
    pub fn get_home_directory(&self) -> Option<PathBuf> {
        get_home_directory()
    }
    
    pub fn get_app_config_directory(&self) -> Option<PathBuf> {
        get_app_config_directory()
    }
}