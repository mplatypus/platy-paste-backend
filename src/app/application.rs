use std::sync::Arc;

use crate::{
    app::object_store::{ObjectStore, ObjectStoreExt as _},
    models::error::AppError,
};

use super::{config::Config, database::Database};

pub type App = Arc<ApplicationState>;

pub struct ApplicationState {
    config: Config,
    database: Database,
    object_store: ObjectStore,
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

        let mut state = Self {
            config: config.clone(),
            database: Database::new(),
            object_store: ObjectStore::from_config(config.object_store())?,
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
    pub const fn object_store(&self) -> &ObjectStore {
        &self.object_store
    }

    async fn init(&mut self) -> Result<(), AppError> {
        self.database.connect(self.config.database_url()).await?;

        self.object_store.create_buckets().await?;

        Ok(())
    }
}
