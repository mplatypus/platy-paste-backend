use sqlx::PgExecutor;
use std::time::Duration;

use time::OffsetDateTime;
use tokio::{sync::mpsc::Receiver, time::sleep};

use crate::{
    app::{application::App, database::Database},
    models::document::Document,
};

use super::{
    authentication::Token,
    error::{AppError, AuthError},
    snowflake::Snowflake,
};

#[derive(Debug, Clone)]
pub struct Paste {
    /// The ID of the paste.
    pub id: Snowflake,
    /// When the paste was created.
    pub creation: OffsetDateTime,
    /// When the paste was last modified.
    pub edited: Option<OffsetDateTime>,
    /// The time at which the paste will expire.
    pub expiry: Option<OffsetDateTime>,
}

impl Paste {
    /// New.
    ///
    /// Create a new [`Paste`] object.
    pub const fn new(
        id: Snowflake,
        creation: OffsetDateTime,
        edited: Option<OffsetDateTime>,
        expiry: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            id,
            creation,
            edited,
            expiry,
        }
    }

    /// Set Edited.
    ///
    /// Update the edited timestamp to the current time.
    pub fn set_edited(&mut self) {
        self.edited = Some(OffsetDateTime::now_utc());
    }

    /// Set Expiry.
    ///
    /// Set or remove the expiry on the paste.
    pub fn set_expiry(&mut self, expiry: Option<OffsetDateTime>) {
        self.expiry = expiry;
    }

    /// Fetch.
    ///
    /// Fetch a paste via its ID.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
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
    pub async fn fetch<'e, 'c: 'e, E>(executor: E, id: Snowflake) -> Result<Option<Self>, AppError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let paste_id: i64 = id.into();
        let query = sqlx::query!(
            "SELECT id, creation, edited, expiry FROM pastes WHERE id = $1",
            paste_id
        )
        .fetch_optional(executor)
        .await?;

        if let Some(q) = query {
            return Ok(Some(Self::new(q.id.into(), q.creation, q.edited, q.expiry)));
        }

        Ok(None)
    }

    /// Fetch Between.
    ///
    /// Fetch all pastes between two times.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
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
    pub async fn fetch_between<'e, 'c: 'e, E>(
        executor: E,
        start: OffsetDateTime,
        end: OffsetDateTime,
    ) -> Result<Vec<Self>, AppError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let records = sqlx::query!(
            "SELECT id, creation, edited, expiry FROM pastes WHERE expiry >= $1 AND expiry <= $2",
            start,
            end
        )
        .fetch_all(executor)
        .await?;

        let mut pastes = Vec::new();
        for record in records {
            let paste = Self::new(
                record.id.into(),
                record.creation,
                record.edited,
                record.expiry,
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
    /// - `executor` - The database pool or transaction to use.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error, or the snowflake exists already.
    pub async fn insert<'e, 'c: 'e, E>(&self, executor: E) -> Result<(), AppError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let paste_id: i64 = self.id.into();

        sqlx::query!(
            "INSERT INTO pastes(id, creation, edited, expiry) VALUES ($1, $2, $3, $4)",
            paste_id,
            self.creation,
            self.edited,
            self.expiry
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Update.
    ///
    /// Create (or update) a document.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error.
    pub async fn update<'e, 'c: 'e, E>(&self, executor: E) -> Result<(), AppError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let paste_id: i64 = self.id.into();

        sqlx::query!(
            "INSERT INTO pastes(id, creation, edited, expiry) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET edited = $3, expiry = $4",
            paste_id,
            self.creation,
            self.edited,
            self.expiry
        ).execute(executor).await?;

        Ok(())
    }

    /// Delete.
    ///
    /// Delete a paste.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
    /// - `id` - The id of the paste.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error.
    pub async fn delete<'e, 'c: 'e, E>(executor: E, id: Snowflake) -> Result<bool, AppError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let paste_id: i64 = id.into();
        let result = sqlx::query!("DELETE FROM pastes WHERE id = $1", paste_id,)
            .execute(executor)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

/// Validate Paste.
///
/// Checks that a paste exists, and has not expired,
/// as well as supporting validating the token.
///
/// ## Arguments
///
/// - `db` - The database to use.
/// - `paste_id` - The ID of the paste.
/// - `token` - The token to validate (if required.)
///
/// ## Errors
///
/// - [`AppError`] - The database had an error.
///
/// ## Returns
///
/// The paste that was checked and found.
pub async fn validate_paste(
    db: &Database,
    paste_id: Snowflake,
    token: Option<Token>,
) -> Result<Paste, AppError> {
    let Some(paste) = Paste::fetch(db.pool(), paste_id).await? else {
        return Err(AppError::NotFound(
            "The paste requested could not be found".to_string(),
        ));
    };

    if let Some(expiry) = paste.expiry {
        if expiry < OffsetDateTime::now_utc() {
            Paste::delete(db.pool(), paste_id).await?;
            return Err(AppError::NotFound(
                "The paste requested could not be found".to_string(),
            ));
        }
    }

    if let Some(token) = token {
        if paste.id != token.paste_id() {
            return Err(AppError::Authentication(AuthError::ForbiddenPasteId));
        }
    }

    Ok(paste)
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
        let documents = match Document::fetch_all(app.database.pool(), paste.id).await {
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

        match Paste::delete(app.database.pool(), paste.id).await {
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
                    let documents = match Document::fetch_all(app.database.pool(), paste.id).await {
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

                    match Paste::delete(app.database.pool(), paste.id).await {
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

    Paste::fetch_between(db.pool(), start, end).await
}
