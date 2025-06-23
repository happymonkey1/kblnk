use std::sync::Arc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;
use crate::platform::PlatformContext;

const CONTEXT_FILES_MAX_SIZE: usize = 150 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum ContextError {
    
}

pub type Result<T> = std::result::Result<T, ContextError>;

#[derive(Debug, Serialize, Deserialize)]
pub struct ContextConfig {
    pub file_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContextManager {
    #[serde(skip)]
    #[serde(default = "default_context")]
    context: Arc<PlatformContext>,
    max_context_files_size: usize,
    pub global_config: ContextConfig,
}

/// Simple wrapper for a tuple of (filename, content)
#[derive(Clone, Debug)]
pub struct ContextFile {
    pub filename: String,
    pub content: String,
}

impl ContextManager {
    pub async fn new(context: Arc<PlatformContext>, max_context_files_size: Option<usize>) -> Result<Self> {
        let max_context_files_size = max_context_files_size.unwrap_or(CONTEXT_FILES_MAX_SIZE);

        let global_config = load_global_config(&context).await?;
        
        Ok(Self {
            context,
            max_context_files_size,
            global_config,
        })
    }
    
    async fn save_config(&self, global: bool) -> Result<()> {
        debug_assert!(global, "Only saving global context is supported for now");
        if global {
            todo!("save_config global is not finished!")
        } else {
            warn!("Saving non-global context is not supported yet")
        }
        
        Ok(())
    }
    
    pub async fn reload_config(&mut self) -> Result<()> {
        self.global_config = load_global_config(&self.context).await?;
        Ok(())
    }
    
    /// Add path(s) to context
    pub async fn add_paths(&mut self, paths: Vec<String>, global: bool, force: bool) -> Result<()> {
        debug_assert!(global, "Only adding to global context is supported for now");
        let global_paths = self.global_config.file_paths.clone();
        
        // Perform validation
        if !force {
            
        }
        
        for path in paths {
            if !global {
                continue;
            }
            
            if global_paths.contains(&path) {
                continue;
            }
            
            self.global_config.file_paths.push(path);
        }
        
        self.save_config(global).await?;
        
        Ok(())
    }
    
    /// Remove path(s) from context
    pub async fn remove_paths(&mut self, paths: Vec<String>, global: bool) -> Result<()> {
        debug_assert!(global, "Only removing from global context is supported for now");
        
        for path in paths {
            if !global {
                continue;
            }
            
            self.global_config.file_paths.retain(|p| p != &path);
        }
        
        self.save_config(global).await?;
        
        Ok(())
    }
    
    pub async fn clear(&mut self, global: bool) -> Result<()> {
        debug_assert!(global, "Only clearing global context is supported for now");
        
        if global {
            self.global_config.file_paths.clear();
        } else {
            warn!("Clearing non-global context is not supported yet");
        }
        
        self.save_config(global).await?;
        
        Ok(())
    }
    
    pub async fn get_context_files(&self) -> Result<Vec<ContextFile>> {
        let mut context_files = Vec::new();
        
        for path in &self.global_config.file_paths {
            let content = std::fs::read_to_string(path).await?;
            context_files.push(ContextFile{ filename: path.clone(), content });
        }
        
        // Sort by filename
        context_files.sort_by(|a, b| a.filename.cmp(&b.filename));
        
        Ok(context_files)
    }
    
    pub async fn collect_context_files_with_limit(&self) -> Result<(Vec<ContextFile>, Vec<ContextFile>)> {
        let mut context_files = self.get_context_files().await?;
        
        let dropped_files = drop_matched_context_files(&mut context_files, self.max_context_files_size)
            .unwrap_or_default();
        
        context_files.retain(|f| !dropped_files.iter().any(|dropped| dropped.filename == f.filename));
        
        Ok((context_files, dropped_files))
    }
}

async fn load_global_config(context: &Arc<PlatformContext>) -> Result<ContextConfig> {
    let global_path = 
}

fn default_context() -> Arc<PlatformContext> {
    PlatformContext::new()
}