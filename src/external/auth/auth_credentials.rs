use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use crate::external::auth::error::{AuthCredentialsError, Result};
use crate::platform::error::PlatformError;
use crate::platform::PlatformContext;

pub const CREDENTIALS_FILE_NAME: &str = "credentials.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthCredentials {
    pub google_api_key: Option<String>,
}

impl AuthCredentials {

    pub async fn build_config(context: Arc<PlatformContext>) -> Result<AuthCredentials> {
        let config_filepath = Self::get_auth_credentials_file_path(Arc::clone(&context))?;

        // try to read existing config
        info!("Trying to read config from '{:?}'", &config_filepath);
        let config = if context.fs().exists(&config_filepath).await {
            serde_json::from_slice(&*context.fs().read(&config_filepath).await?)
        } else {
            info!("Config file was not found, building default config instead");
            Ok(AuthCredentials::default())
        }?;

        // TODO: read environment variables

        config.save_config(context, config_filepath).await?;
        
        Ok(config)
    }

    // TODO: config abstraction
    pub fn get_auth_credentials_file_path(context: Arc<PlatformContext>) -> Result<PathBuf> {
        let app_dir = context.get_app_config_directory()?;
        Ok(app_dir.join(CREDENTIALS_FILE_NAME))
    }

    // TODO: config abstraction
    pub async fn save_config(&self, context: Arc<PlatformContext>, path: impl AsRef<Path>) -> Result<()> {
        let config_string = serde_json::to_string_pretty(self)?;
        
        let parent_path = if let Some(path) = path.as_ref().parent() {
            path
        } else {
            error!("AuthCredentials parent config path is not valid: '{:?}'", path.as_ref());
            return Err(AuthCredentialsError::PlatformError(PlatformError::InvalidDirectoryError))
        };

        if !context.fs().exists(parent_path).await {
            info!("Creating directories for non-existent parent path: '{:?}'", parent_path);
            context.fs().create_dir_all(parent_path).await?;
        }
        
        context.fs().write(path, config_string).await?;
        
        Ok(())
    }

    // TODO: config abstraction
    pub async fn save_config_to_default_location(&self, context: Arc<PlatformContext>) -> Result<()> {
        let config_filepath = Self::get_auth_credentials_file_path(Arc::clone(&context))?;
        self.save_config(context, config_filepath).await
    }
}

impl Default for AuthCredentials {
    fn default() -> Self {
        Self {
            google_api_key: None,
        } 
    }
}