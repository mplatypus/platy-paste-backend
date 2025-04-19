use sqlx::PgTransaction;
use std::time::Duration;

use time::OffsetDateTime;
use tokio::{sync::mpsc::Receiver, time::sleep};

use crate::{
    app::{application::App, database::Database},
    models::document::Document,
};

use super::{error::AppError, snowflake::Snowflake};

#[derive(Debug, Clone)]
pub struct Paste {
    /// The ID of the paste.
    pub id: Snowflake,
    /// Whether the paste has been edited.
    pub edited: bool,
    /// The time when the paste expires.
    pub expiry: Option<OffsetDateTime>,
    /// The amount of views a paste has.
    pub views: usize,
    /// The maximum allowed views for a paste.
    pub max_views: Option<usize>,
}

impl Paste {
    /// New.
    ///
    /// Create a new [`Paste`] object.
    pub const fn new(
        id: Snowflake,
        edited: bool,
        expiry: Option<OffsetDateTime>,
        views: usize,
        max_views: Option<usize>,
    ) -> Self {
        Self {
            id,
            edited,
            expiry,
            views,
            max_views,
        }
    }

    /// Set Edited.
    ///
    /// Update the paste so it shows as edited.
    pub fn set_edited(&mut self) {
        self.edited = true;
    }

    /// Set Expiry.
    ///
    /// Set or remove the expiry on the paste.
    pub fn set_expiry(&mut self, expiry: Option<OffsetDateTime>) {
        self.expiry = expiry;
    }

    /// Set Max Views.
    ///
    /// Set or remove the maximum amount of views for a paste.
    pub fn set_max_views(&mut self, max_views: Option<usize>) {
        self.max_views = max_views;
    }

    /// Fetch.
    ///
    /// Fetch a paste via its ID.
    ///
    /// ## Arguments
    ///
    /// - `db` - The database to make the request to.
    /// - `id` - The ID of the paste.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error.
    ///
    /// ## Returns
    ///
    /// - [`Option::Some`] - The [`Paste`] object.
    /// - [`Option::None`] - No paste was found.
    pub async fn fetch(db: &Database, id: Snowflake) -> Result<Option<Self>, AppError> {
        let paste_id: i64 = id.into();
        let query = sqlx::query!(
            "SELECT id, edited, expiry, views, max_views FROM pastes WHERE id = $1",
            paste_id
        )
        .fetch_optional(db.pool())
        .await?;

        if let Some(q) = query {
            return Ok(Some(Self::new(
                q.id.into(),
                q.edited,
                q.expiry,
                q.views as usize,
                q.max_views.map(|v| v as usize),
            )));
        }

        Ok(None)
    }

    /// Fetch Between.
    ///
    /// Fetch all pastes between two times.
    ///
    /// ## Arguments
    ///
    /// - `db` - The database to make the request to.
    /// - `start` - The start [`OffsetDateTime`] (inclusive).
    /// - `end` - The end [`OffsetDateTime`] (inclusive).
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error.
    ///
    /// ## Returns
    ///
    /// A [`Vec`] of [`Paste`]'s.
    pub async fn fetch_between(
        db: &Database,
        start: OffsetDateTime,
        end: OffsetDateTime,
    ) -> Result<Vec<Self>, AppError> {
        let records = sqlx::query!(
            "SELECT id, edited, expiry, views, max_views FROM pastes WHERE expiry >= $1 AND expiry <= $2",
            start,
            end
        )
        .fetch_all(db.pool())
        .await?;

        let mut pastes = Vec::new();
        for record in records {
            let paste = Self::new(
                record.id.into(),
                record.edited,
                record.expiry,
                record.views as usize,
                record.max_views.map(|v| v as usize),
            );

            pastes.push(paste);
        }

        Ok(pastes)
    }

    /// Insert.
    ///
    /// Insert (create) a paste.
    ///
    /// ## Arguments
    ///
    /// - `transaction` The transaction to use.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error, or the snowflake exists already.
    pub async fn insert(&self, transaction: &mut PgTransaction<'_>) -> Result<(), AppError> {
        let paste_id: i64 = self.id.into();

        sqlx::query!(
            "INSERT INTO pastes(id, edited, expiry, views, max_views) VALUES ($1, $2, $3, $4, $5)",
            paste_id,
            self.edited,
            self.expiry,
            self.views as i64,
            self.max_views.map(|v| v as i64)
        )
        .execute(transaction.as_mut())
        .await?;

        Ok(())
    }

    /// Update.
    ///
    /// Create (or update) a document.
    ///
    /// ## Arguments
    ///
    /// - `transaction` The transaction to use.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error.
    pub async fn update(&self, transaction: &mut PgTransaction<'_>) -> Result<(), AppError> {
        let paste_id: i64 = self.id.into();

        sqlx::query!(
            "INSERT INTO pastes(id, edited, expiry, views, max_views) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO UPDATE SET edited = $2, expiry = $3, max_views = $5",
            paste_id,
            self.edited,
            self.expiry,
            self.views as i64,
            self.max_views.map(|v|v as i64)
        ).execute(transaction.as_mut()).await?;

        Ok(())
    }

    /// Add view.
    ///
    /// Create (or update) a document.
    ///
    /// ## Arguments
    ///
    /// - `transaction` The transaction to use.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error.
    pub async fn add_view(&mut self, transaction: &mut PgTransaction<'_>) -> Result<(), AppError> {
        let id: i64 = self.id.into();

        let views = sqlx::query_scalar!(
            "UPDATE pastes SET views = views + 1 WHERE id = $1 RETURNING views",
            id,
        )
        .fetch_one(transaction.as_mut())
        .await?;

        self.views = views as usize;
        Ok(())
    }

    /// Delete.
    ///
    /// Delete a paste.
    ///
    /// ## Arguments
    ///
    /// - `db` - The database to make the request to.
    /// - `id` - The id of the paste.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error.
    pub async fn delete(db: &Database, id: Snowflake) -> Result<bool, AppError> {
        let paste_id: i64 = id.into();
        let result = sqlx::query!("DELETE FROM pastes WHERE id = $1", paste_id,)
            .execute(db.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

#[derive(Clone, Debug)]
pub enum ExpiryTaskMessage {
    /// Cancel the expiry runners.
    Cancel,
}

/// Expiry Tasks.
///
/// A task that deletes pastes (and their documents) when required.
///
/// ## Arguments
///
/// - `app` - The application to use.
/// - `rx` - The [`Receiver`] to listen for messages.
pub async fn expiry_tasks(app: App, mut rx: Receiver<ExpiryTaskMessage>) {
    const MINUTES: u64 = 50;

    let pastes = match collect_nearby_expired_tasks(&app.database).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Failed to collect all pastes to expire. Reason: {e}");
            panic!("Failed to collect all pastes to expire. Reason: {e}")
        }
    };

    for paste in pastes {
        let documents = match Document::fetch_all(&app.database, paste.id).await {
            Ok(documents) => documents,
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch documents for paste {}. Reason: {}",
                    paste.id,
                    e
                );
                continue;
            }
        };

        for document in documents {
            match app.s3.delete_document(document.generate_path()).await {
                Ok(()) => tracing::trace!(
                    "Successfully deleted paste document (minio): {}",
                    document.id
                ),
                Err(e) => tracing::trace!(
                    "Failed to delete paste document: {} (minio). Reason: {}",
                    document.id,
                    e
                ),
            }
        }

        match Paste::delete(&app.database, paste.id).await {
            Ok(_) => tracing::trace!("Successfully deleted paste: {}", paste.id),
            Err(e) => tracing::warn!("Failure to delete paste: {}. Reason: {}", paste.id, e),
        }
    }

    loop {
        let sleep = sleep(Duration::from_secs(MINUTES * 60));
        tokio::pin!(sleep);
        tokio::select! {
            biased;

            msg = rx.recv() => {
                match msg {
                    Some(ExpiryTaskMessage::Cancel) => {
                        println!("Received cancel message, shutting down.");
                        break;
                    }
                    None => {
                        println!("Channel closed, shutting down.");
                        break;
                    }
                }
            }
            () = &mut sleep => {
                let pastes = match collect_nearby_expired_tasks(&app.database).await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("Failed to collect all pastes to expire. Reason: {e}");
                        panic!("Failed to collect all pastes to expire. Reason: {e}")
                    }
                };

                // FIXME: Please tell me there is a cleaner way of doing this.
                for paste in pastes {
                    let documents = match Document::fetch_all(&app.database, paste.id).await {
                        Ok(documents) => documents,
                        Err(e) => {
                            tracing::warn!("Failed to fetch documents for paste {}. Reason: {}", paste.id, e);
                            continue
                        }
                    };

                    for document in documents {
                        match app.s3.delete_document(document.generate_path()).await {
                            Ok(()) => tracing::trace!("Successfully deleted paste document (minio): {}", document.id),
                            Err(e) => tracing::trace!("Failed to delete paste document: {} (minio). Reason: {}", document.id, e)
                        }
                    }

                    match Paste::delete(&app.database, paste.id).await {
                        Ok(_) => tracing::trace!("Successfully deleted paste: {}", paste.id),
                        Err(e) => tracing::warn!("Failure to delete paste: {}. Reason: {}", paste.id, e)
                    }
                }
            }
        }
    }
}

/// Collect Nearby Expired Tasks.
///
/// Fetch all the pastes, from EPOCH 0, to the current time.
///
/// ## Arguments
///
/// - `db` - The database to make the request to.
///
/// ## Errors
///
/// - [`AppError`] - The database had an error.
///
/// ## Returns
///
/// A [`Vec`] of [`Paste`]'s.
async fn collect_nearby_expired_tasks(db: &Database) -> Result<Vec<Paste>, AppError> {
    let start = OffsetDateTime::from_unix_timestamp(0)
        .expect("Failed to make a timestamp with the time of 0.");
    let end = OffsetDateTime::now_utc();

    Paste::fetch_between(db, start, end).await
}
