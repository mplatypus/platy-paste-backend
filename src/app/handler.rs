//! The handler for dealing with cleaning up pastes and documents.

use std::{collections::HashMap, time::Duration};

use chrono::{TimeDelta, Utc};
use tokio::sync::{mpsc, oneshot};

use crate::{
    app::{
        config::Config,
        database::Database,
        object_store::{ObjectStore, ObjectStoreExt as _},
    },
    models::{DtUtc, document::Document, errors::HandlerError, paste::Paste, snowflake::Snowflake},
};

/// ## Default Timeout
///
/// The default amount of time to wait for a message response from the handler actor.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// ## Collection Offset
///
/// The amount of time from now into the future to collect pastes for deletion.
/// The more pastes collected, the more memory that can be consumed.
const COLLECTION_OFFSET: TimeDelta = TimeDelta::hours(1);

#[derive(Debug)]
enum HandlerMessage {
    /// ## Get
    ///
    /// Get all the nearby tasks from the handler.
    #[cfg(test)]
    Get(oneshot::Sender<Result<HashMap<Snowflake, DtUtc>, HandlerError>>),
    /// ## Add
    ///
    /// Add a paste to the handler.
    ///
    /// This is not forced. All this does, it tells the handler that the paste is new,
    /// and should check if its in the current time scope, and should/shouldn't be added to the list of results.
    Add {
        id: Snowflake,
        expiry: DtUtc,
        sender: oneshot::Sender<Result<(), HandlerError>>,
    },
    /// ## Remove
    ///
    /// Remove a paste from the handler.
    ///
    /// Attempts to remove the paste from the handler, as its already been deleted to stop errors from occuring.
    Remove {
        id: Snowflake,
        sender: oneshot::Sender<Result<(), HandlerError>>,
    },
    /// ## Close.
    ///
    /// Close the entire handler down.
    Close(oneshot::Sender<Result<(), HandlerError>>),
}

/// ## Handler
///
/// The handler for dealing with expired notes.
#[derive(Debug, Clone)]
pub struct Handler {
    sender: Option<mpsc::Sender<HandlerMessage>>,
}

impl Handler {
    /// ## New
    ///
    /// Create a new [`handler`] object.
    #[expect(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self { sender: None }
    }

    /// ## Start
    ///
    /// Start up the handlers actor.
    ///
    /// ## Errors
    ///
    /// Errors if the handler has already been started.
    pub fn start(
        &mut self,
        database: Database,
        object_store: ObjectStore,
        config: Config,
    ) -> Result<(), HandlerError> {
        if self.sender.is_some() {
            return Err(HandlerError::AlreadyStarted);
        }

        let (sender, receiver) = mpsc::channel(10);

        let actor = HandlerActor::new(database, object_store, config, receiver);

        tokio::spawn(actor.run());

        self.sender = Some(sender);

        Ok(())
    }

    async fn send(&self, message: HandlerMessage) -> Result<(), HandlerError> {
        let Some(sender) = &self.sender else {
            return Err(HandlerError::NotStarted);
        };

        if sender.is_closed() {
            return Err(HandlerError::Closed);
        }

        sender.send(message).await?;

        Ok(())
    }

    /// ## Add
    ///
    /// Add a paste to the handler.
    ///
    /// This is not forced. All this does, it tells the handler that the paste is new,
    /// and should check if its in the current time scope, and should/shouldn't be added to the list of results.
    ///
    /// ## Arguments
    /// - `id` - The paste ID that has been created.
    ///
    /// ## Errors
    /// Errors if it times out on a response, or the handler was never started.
    pub async fn add(&self, id: &Snowflake, expiry: DtUtc) -> Result<(), HandlerError> {
        let (sender, receiver) = oneshot::channel();

        let message = HandlerMessage::Add {
            id: *id,
            expiry,
            sender,
        };

        self.send(message).await?;

        tokio::time::timeout(DEFAULT_TIMEOUT, receiver).await??
    }

    /// ## Remove
    ///
    /// Remove a paste from the handler.
    ///
    /// Attempts to remove the paste from the handler, as its already been deleted to stop errors from occuring.
    ///
    /// ## Arguments
    /// - `id` - The paste ID that has been created.
    ///
    /// ## Errors
    /// Errors if it times out on a response, or the handler was never started.
    pub async fn remove(&self, id: &Snowflake) -> Result<(), HandlerError> {
        let (sender, receiver) = oneshot::channel();

        let message = HandlerMessage::Remove { id: *id, sender };

        self.send(message).await?;

        tokio::time::timeout(DEFAULT_TIMEOUT, receiver).await??
    }

    /// ## Close
    ///
    /// Close the handler down.
    ///
    /// ## Errors
    /// Errors if it times out on a response, or the handler was never started.
    pub async fn close(&self) -> Result<(), HandlerError> {
        let (sender, receiver) = oneshot::channel();

        let message = HandlerMessage::Close(sender);

        self.send(message).await?;

        tokio::time::timeout(DEFAULT_TIMEOUT, receiver).await??
    }
}

#[derive(Debug)]
struct HandlerActor {
    receiver: mpsc::Receiver<HandlerMessage>,
    database: Database,
    object_store: ObjectStore,
    #[expect(unused)]
    config: Config,
    last_checked: DtUtc,
    nearby: HashMap<Snowflake, DtUtc>,
    attempts: usize,
}

impl HandlerActor {
    pub fn new(
        database: Database,
        object_store: ObjectStore,
        config: Config,
        receiver: mpsc::Receiver<HandlerMessage>,
    ) -> Self {
        Self {
            receiver,
            database,
            object_store,
            config,
            last_checked: Utc::now(),
            nearby: HashMap::new(),
            attempts: 0,
        }
    }

    /// ## Update Expired Pastes
    ///
    /// Update the current list of expired pastes.
    async fn update_expired_pastes(&mut self) -> Result<(), HandlerError> {
        let start = chrono::DateTime::from_timestamp(0, 0)
            .expect("Failed to make a timestamp with the time of 0.");
        let end = Utc::now() + COLLECTION_OFFSET;

        let pastes = Paste::fetch_between(self.database.pool(), &start, &end).await?;

        for paste in pastes {
            if let Some(expiry) = paste.expiry() {
                self.nearby.insert(*paste.id(), *expiry);
            }
        }

        self.last_checked = end;

        Ok(())
    }

    /// ## Delete Paste
    ///
    /// Completely delete a paste from its ID.
    async fn delete_paste(&self, id: &Snowflake) -> Result<(), HandlerError> {
        let documents = match Document::fetch_all(self.database.pool(), id).await {
            Ok(documents) => documents,
            Err(err) => {
                tracing::warn!("Failed to fetch documents for the paste of: {id}. Error: {err}");
                Paste::delete(self.database.pool(), id).await?;
                return Ok(());
            }
        };

        Paste::delete(self.database.pool(), id).await?;

        for document in documents {
            self.object_store.delete_document(&document).await?;
        }

        Ok(())
    }

    /// ## Load Pastes
    ///
    /// Loads new pastes via [`HandlerActor::update_expired_tasks`] or backs off.
    async fn load_pastes(&mut self) {
        if let Err(err) = self.update_expired_pastes().await {
            self.attempts += 1;

            tracing::warn!("Failed to update expired pastes. Error: {err}");

            let backing_off = self.attempts * 5;
            tracing::warn!("Backing off for {backing_off}s");

            tokio::time::sleep(Duration::from_secs(backing_off as u64)).await;
        } else {
            self.attempts = 0;
        }
    }

    pub async fn run(mut self) {
        self.load_pastes().await;

        loop {
            let current = Utc::now();

            if current >= self.last_checked {
                self.load_pastes().await;
            }

            let mut deleted_paste_ids = Vec::new();
            for (paste_id, expiry) in &self.nearby {
                if current >= *expiry {
                    match self.delete_paste(paste_id).await {
                        Ok(()) => {
                            deleted_paste_ids.push(*paste_id);
                            tracing::debug!(
                                "The paste ({paste_id}) has successfully been deleted."
                            );
                        }
                        Err(err) => tracing::warn!(
                            "The paste ({paste_id}) could not be deleted. Error: {err}"
                        ),
                    }
                }
            }

            for paste_id in deleted_paste_ids {
                self.nearby.remove(&paste_id);
            }

            if let Some(message) = self.receiver.recv().await {
                match message {
                    #[cfg(test)]
                    HandlerMessage::Get(sender) => {
                        if let Err(err) = sender.send(Ok(self.nearby.clone())) {
                            tracing::warn!("Failed to send add message for handler. Err: {err:?}");
                        }
                    }
                    HandlerMessage::Add { id, expiry, sender } => {
                        if expiry <= current + COLLECTION_OFFSET {
                            self.nearby.insert(id, expiry);
                        }

                        if let Err(err) = sender.send(Ok(())) {
                            tracing::warn!("Failed to send add message for handler. Err: {err:?}");
                        }
                    }
                    HandlerMessage::Remove { id, sender } => {
                        self.nearby.remove(&id);

                        if let Err(err) = sender.send(Ok(())) {
                            tracing::warn!(
                                "Failed to send remove message for handler. Err: {err:?}"
                            );
                        }
                    }
                    HandlerMessage::Close(sender) => {
                        tracing::trace!("Handler message: `close`");

                        if let Err(err) = sender.send(Ok(())) {
                            tracing::warn!(
                                "Failed to send close message for handler. Err: {err:?}"
                            );
                        }

                        return;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use sqlx::PgPool;

    use crate::app::object_store::TestObjectStore;

    use super::*;

    #[sqlx::test]
    async fn test_expired(pool: PgPool) {
        let config = Config::test_builder()
            .build()
            .expect("Failed to build config.");
        let database = Database::from_pool(pool);
        let test_object_store = TestObjectStore::new();
        let object_store = ObjectStore::Test(test_object_store.clone());

        let now = Utc::now();
        let no_expiry_paste_id = Snowflake::new(1);
        let future_paste_id = Snowflake::new(2);
        let nearby_paste_id = Snowflake::new(3);
        let expired_paste_id = Snowflake::new(4);

        let no_expiry_paste = Paste::new(no_expiry_paste_id, None, now, None, None, 0, None);

        let future_paste = Paste::new(
            future_paste_id,
            None,
            now,
            None,
            Some(now + TimeDelta::hours(2)),
            0,
            None,
        );

        let nearby_paste = Paste::new(
            nearby_paste_id,
            None,
            now,
            None,
            Some(now + TimeDelta::minutes(30)),
            0,
            None,
        );

        let expired_paste = Paste::new(
            expired_paste_id,
            None,
            now,
            None,
            Some(now - TimeDelta::minutes(15)),
            0,
            None,
        );

        no_expiry_paste
            .insert(database.pool())
            .await
            .expect("Failed to insert paste.");
        future_paste
            .insert(database.pool())
            .await
            .expect("Failed to insert paste.");
        nearby_paste
            .insert(database.pool())
            .await
            .expect("Failed to insert paste.");
        expired_paste
            .insert(database.pool())
            .await
            .expect("Failed to insert paste.");

        let document_1 = Document::new(
            Snowflake::new(5),
            no_expiry_paste_id,
            "text/plain",
            "test.txt",
            2874,
        );

        let document_2 = Document::new(
            Snowflake::new(6),
            future_paste_id,
            "application/json",
            "cool.json",
            345,
        );

        let document_3 = Document::new(
            Snowflake::new(7),
            nearby_paste_id,
            "text/rust",
            "paste.rs",
            74211,
        );

        let document_4 = Document::new(
            Snowflake::new(8),
            expired_paste_id,
            "text/css",
            "example.css",
            94,
        );

        document_1
            .insert(database.pool())
            .await
            .expect("Failed to insert document 1.");
        object_store
            .create_document(&document_1, Bytes::from("Test Document 1"))
            .await
            .expect("Failed to create document 1");
        document_2
            .insert(database.pool())
            .await
            .expect("Failed to insert document 2.");
        object_store
            .create_document(&document_2, Bytes::from("Test Document 2"))
            .await
            .expect("Failed to create document 2");
        document_3
            .insert(database.pool())
            .await
            .expect("Failed to insert document 3.");
        object_store
            .create_document(&document_3, Bytes::from("Test Document 3"))
            .await
            .expect("Failed to create document 3");
        document_4
            .insert(database.pool())
            .await
            .expect("Failed to insert document 4.");
        object_store
            .create_document(&document_4, Bytes::from("Test Document 4"))
            .await
            .expect("Failed to create document 4");

        let mut handler = Handler::new();
        handler
            .start(database.clone(), object_store.clone(), config.clone())
            .expect("Failed to start handler.");

        handler
            .add(&future_paste_id, now + TimeDelta::hours(2))
            .await
            .expect("Failed to add paste.");
        handler
            .add(&nearby_paste_id, now + TimeDelta::minutes(30))
            .await
            .expect("Failed to add paste.");
        handler
            .add(&expired_paste_id, now - TimeDelta::minutes(15))
            .await
            .expect("Failed to add paste.");

        tokio::time::sleep(Duration::from_secs(10)).await;

        let (sender, receiver) = oneshot::channel();
        handler
            .send(HandlerMessage::Get(sender))
            .await
            .expect("Failed to send get message.");
        let result = tokio::time::timeout(DEFAULT_TIMEOUT, receiver)
            .await
            .expect("Timed out on request.")
            .expect("Failed to receive nearby tasks")
            .expect("Failed to get nearby tasks");
        assert_eq!(result.len(), 1);
        assert!(!result.contains_key(&no_expiry_paste_id));
        assert!(!result.contains_key(&future_paste_id));
        assert!(result.contains_key(&nearby_paste_id));
        assert!(!result.contains_key(&expired_paste_id));

        let expired_paste_db = Paste::fetch(database.pool(), &expired_paste_id)
            .await
            .expect("Failed to fetch paste.");
        assert!(expired_paste_db.is_none());

        let document_1_content = test_object_store
            .fetch_document(&document_1)
            .await
            .expect("Failed to retrieve document");
        let document_2_content = test_object_store
            .fetch_document(&document_2)
            .await
            .expect("Failed to retrieve document");
        let document_3_content = test_object_store
            .fetch_document(&document_3)
            .await
            .expect("Failed to retrieve document");
        let document_4_content = test_object_store
            .fetch_document(&document_4)
            .await
            .expect("Failed to retrieve document");

        assert!(document_1_content.is_some(), "Document 1 should exist.");
        assert!(document_2_content.is_some(), "Document 2 should exist.");
        assert!(document_3_content.is_some(), "Document 3 should exist.");
        assert!(document_4_content.is_none(), "Document 4 should not exist.");

        handler.close().await.expect("Failed to close handler.");
    }

    #[sqlx::test]
    async fn test_add(pool: PgPool) {
        let config = Config::test_builder()
            .build()
            .expect("Failed to build config.");
        let database = Database::from_pool(pool);
        let object_store = ObjectStore::Test(TestObjectStore::new());

        let now = Utc::now();
        let paste_id_1 = Snowflake::new(9);
        let paste_id_2 = Snowflake::new(10);

        let paste_1 = Paste::new(
            paste_id_1,
            None,
            now,
            None,
            Some(now + TimeDelta::minutes(30)),
            0,
            None,
        );

        paste_1
            .insert(database.pool())
            .await
            .expect("Failed to insert paste.");

        let mut handler = Handler::new();
        handler
            .start(database.clone(), object_store.clone(), config.clone())
            .expect("Failed to start handler.");

        handler
            .add(&paste_id_1, now + TimeDelta::minutes(30))
            .await
            .expect("Failed to add paste.");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let (sender, receiver) = oneshot::channel();
        handler
            .send(HandlerMessage::Get(sender))
            .await
            .expect("Failed to send get message.");
        let result = tokio::time::timeout(DEFAULT_TIMEOUT, receiver)
            .await
            .expect("Timed out on request.")
            .expect("Failed to receive nearby tasks")
            .expect("Failed to get nearby tasks");
        assert_eq!(result.len(), 1);
        assert!(result.contains_key(&paste_id_1));

        let paste_2 = Paste::new(
            paste_id_2,
            None,
            now,
            None,
            Some(now - TimeDelta::minutes(30)),
            0,
            None,
        );

        paste_2
            .insert(database.pool())
            .await
            .expect("Failed to insert paste.");

        handler
            .add(&paste_id_2, now - TimeDelta::minutes(30))
            .await
            .expect("Failed to add paste.");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let (sender, receiver) = oneshot::channel();
        handler
            .send(HandlerMessage::Get(sender))
            .await
            .expect("Failed to send get message.");
        let result = tokio::time::timeout(DEFAULT_TIMEOUT, receiver)
            .await
            .expect("Timed out on request.")
            .expect("Failed to receive nearby tasks")
            .expect("Failed to get nearby tasks");
        assert_eq!(result.len(), 1);
        assert!(!result.contains_key(&paste_id_2));

        handler.close().await.expect("Failed to close handler.");
    }

    #[sqlx::test]
    async fn test_remove(pool: PgPool) {
        let config = Config::test_builder()
            .build()
            .expect("Failed to build config.");
        let database = Database::from_pool(pool);
        let object_store = ObjectStore::Test(TestObjectStore::new());

        let now = Utc::now();
        let paste_id_1 = Snowflake::new(11);

        let paste_1 = Paste::new(
            paste_id_1,
            None,
            now,
            None,
            Some(now + TimeDelta::minutes(30)),
            0,
            None,
        );

        paste_1
            .insert(database.pool())
            .await
            .expect("Failed to insert paste.");

        let mut handler = Handler::new();
        handler
            .start(database.clone(), object_store.clone(), config.clone())
            .expect("Failed to start handler.");

        handler
            .add(&paste_id_1, now + TimeDelta::minutes(30))
            .await
            .expect("Failed to add paste.");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let (sender, receiver) = oneshot::channel();
        handler
            .send(HandlerMessage::Get(sender))
            .await
            .expect("Failed to send get message.");
        let result = tokio::time::timeout(DEFAULT_TIMEOUT, receiver)
            .await
            .expect("Timed out on request.")
            .expect("Failed to receive nearby tasks")
            .expect("Failed to get nearby tasks");
        assert_eq!(result.len(), 1);
        assert!(result.contains_key(&paste_id_1));

        handler
            .remove(&paste_id_1)
            .await
            .expect("Failed to remove paste.");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let (sender, receiver) = oneshot::channel();
        handler
            .send(HandlerMessage::Get(sender))
            .await
            .expect("Failed to send get message.");
        let result = tokio::time::timeout(DEFAULT_TIMEOUT, receiver)
            .await
            .expect("Timed out on request.")
            .expect("Failed to receive nearby tasks")
            .expect("Failed to get nearby tasks");
        assert_eq!(result.len(), 0);
        assert!(!result.contains_key(&paste_id_1));

        handler.close().await.expect("Failed to close handler.");
    }

    #[sqlx::test]
    async fn test_close(pool: PgPool) {
        let config = Config::test_builder()
            .build()
            .expect("Failed to build config.");
        let database = Database::from_pool(pool);
        let object_store = ObjectStore::Test(TestObjectStore::new());

        let mut handler = Handler::new();
        assert!(
            handler
                .start(database.clone(), object_store.clone(), config.clone())
                .is_ok()
        );

        let now = Utc::now();
        let paste_id_1 = Snowflake::new(12);

        let paste_1 = Paste::new(
            paste_id_1,
            None,
            now,
            None,
            Some(now + TimeDelta::minutes(30)),
            0,
            None,
        );

        paste_1
            .insert(database.pool())
            .await
            .expect("Failed to insert paste.");

        handler
            .add(&paste_id_1, now + TimeDelta::minutes(30))
            .await
            .expect("Failed to add paste.");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let (sender, receiver) = oneshot::channel();
        handler
            .send(HandlerMessage::Get(sender))
            .await
            .expect("Failed to send get message.");
        let result = tokio::time::timeout(DEFAULT_TIMEOUT, receiver)
            .await
            .expect("Timed out on request.")
            .expect("Failed to receive nearby tasks")
            .expect("Failed to get nearby tasks");
        assert_eq!(result.len(), 1);
        assert!(result.contains_key(&paste_id_1));

        assert!(
            handler.close().await.is_ok(),
            "The handler has already been closed before closure was requested."
        );

        let (sender, _) = oneshot::channel();
        assert!(handler.send(HandlerMessage::Get(sender)).await.is_err());

        tokio::time::sleep(Duration::from_millis(500)).await;

        handler.close().await.expect_err("Failed to close handler.");
    }
}
