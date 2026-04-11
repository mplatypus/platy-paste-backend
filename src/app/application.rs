//! The application state for holding references to all server related items.
use std::sync::Arc;

#[cfg(test)]
use sqlx::PgPool;

#[cfg(test)]
use crate::app::object_store::TestObjectStore;
use crate::{
    app::{
        handler::Handler,
        object_store::{ObjectStore, ObjectStoreExt as _},
    },
    models::errors::ApplicationError,
};

use super::{config::Config, database::Database};

/// A short hand for [`Arc<ApplicationState>`]
pub type App = Arc<ApplicationState>;

/// ## Application State
///
/// The application state used to share items within the server.
pub struct ApplicationState {
    config: Config,
    database: Database,
    object_store: ObjectStore,
    handler: Handler,
}

impl ApplicationState {
    /// New.
    ///
    /// Create a new [`ApplicationState`] object.
    ///
    /// ## Errors
    ///
    /// - [`ApplicationError`] - When it fails to create a client.
    ///
    /// ## Returns
    ///
    /// The created [`ApplicationState`] wrapped in [`Arc`].
    pub async fn new() -> Result<Arc<Self>, ApplicationError> {
        dotenvy::from_filename(".env").ok();

        let config = Config::from_env();

        let mut state = Self {
            config: config.clone(),
            database: Database::new(),
            object_store: ObjectStore::from_config(config.object_store())?,
            handler: Handler::new(),
        };

        state.init().await?;

        Ok(Arc::new_cyclic(|w| {
            state.database.bind_to(w.clone());
            state
        }))
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    #[expect(clippy::missing_errors_doc)]
    #[expect(clippy::unused_async)]
    #[cfg(test)]
    pub async fn new_tests(
        config: Config,
        pool: PgPool,
        object_store: TestObjectStore,
    ) -> Result<Arc<Self>, ApplicationError> {
        let database = Database::from_pool(pool);
        let object_store = ObjectStore::Test(object_store);

        let mut handler = Handler::new();

        handler.start(database.clone(), object_store.clone(), config.clone())?;

        Ok(Arc::new(Self {
            config,
            database,
            object_store,
            handler,
        }))
    }

    /// The configuration information about the server.
    #[inline]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// The database used by the server.
    #[inline]
    pub const fn database(&self) -> &Database {
        &self.database
    }

    /// The object storage used by the server.
    #[inline]
    pub const fn object_store(&self) -> &ObjectStore {
        &self.object_store
    }

    /// The handler used by the server.
    #[inline]
    pub const fn handler(&self) -> &Handler {
        &self.handler
    }

    async fn init(&mut self) -> Result<(), ApplicationError> {
        self.database.connect(self.config.database_url()).await?;

        self.object_store.create_buckets().await?;

        self.handler.start(
            self.database.clone(),
            self.object_store.clone(),
            self.config.clone(),
        )?;

        Ok(())
    }
}
