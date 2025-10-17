use std::sync::Arc;

use crate::models::error::AppError;

use super::{config::Config, database::Database, s3::S3Service};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{Client, Config as S3Config, config::Credentials};
use secrecy::ExposeSecret;

pub type App = Arc<ApplicationState>;
pub type S3Client = Client;

pub struct ApplicationState {
    config: Config,
    database: Database,
    s3: S3Service,
}

impl ApplicationState {
    /// New.
    ///
    /// Create a new [`ApplicationState`] object.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - When it fails to create a client.
    ///
    /// ## Returns
    ///
    /// The created [`ApplicationState`] wrapped in [`Arc`].
    pub async fn new() -> Result<Arc<Self>, AppError> {
        let config = Config::from_env();

        let s3creds = Credentials::new(
            config.s3_access_key().expose_secret(),
            config.s3_secret_key().expose_secret(),
            None,
            None,
            "paste",
        );

        let s3conf = S3Config::builder()
            //.region(Region::new("vault"))
            .endpoint_url(config.s3_url())
            .credentials_provider(s3creds)
            .region(Region::new("direct"))
            .force_path_style(true) // MinIO does not support virtual hosts
            .behavior_version(BehaviorVersion::v2025_08_07())
            .build();

        let s3 = S3Service::new(S3Client::from_conf(s3conf));

        let mut state = Self {
            config,
            database: Database::new(),
            s3,
        };

        state.init().await?;

        Ok(Arc::new_cyclic(|w| {
            state.database.bind_to(w.clone());
            state
        }))
    }

    #[inline]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    #[inline]
    pub const fn database(&self) -> &Database {
        &self.database
    }

    #[inline]
    pub const fn s3(&self) -> &S3Service {
        &self.s3
    }

    async fn init(&mut self) -> Result<(), AppError> {
        self.database.connect(self.config.database_url()).await?;

        self.s3.create_buckets().await?;

        Ok(())
    }
}
