use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::external::auth::error;
use crate::external::auth::error::AuthCredentialsError;
use crate::platform::PlatformContext;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthCredentials {
    pub google_api_key: Option<String>,
}

impl AuthCredentials {

    pub async fn build_config(context: Arc<PlatformContext>) -> error::Result<AuthCredentials> {
        let app_dir = context.get_app_config_directory().ok_or_else(|| AuthCredentialsError::InitializationError)?;
        let config_filepath = app_dir.join("credentials.json");

        // try to read existing config
        info!("Trying to read config from '{:?}'", &config_filepath);
        let config = if context.fs().exists(&config_filepath).await {
            serde_json::from_slice(&*context.fs().read(config_filepath).await?)
        } else {
            info!("Config file was not found, building default config instead");
            Ok(AuthCredentials::default())
        }?;

        // read environment variables

        config.save_config();
        
        Ok(config)
    }

    pub fn save_config(&self) {

    }
}

impl Default for AuthCredentials {
    fn default() -> Self {
        Self {
            google_api_key: None,
        } 
    }
}