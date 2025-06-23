use std::sync::Arc;
use crate::platform::env::Env;
use crate::platform::filesystem::Filesystem;
use crate::platform::os::OsPlatform;

mod filesystem;
mod env;
mod os;

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
}