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
}

impl Paste {
    pub const fn new(id: Snowflake, edited: bool, expiry: Option<OffsetDateTime>) -> Self {
        Self { id, edited, expiry }
    }

    pub fn set_edited(&mut self) {
        self.edited = true;
    }

    pub fn set_expiry(&mut self, expiry: Option<OffsetDateTime>) {
        self.expiry = expiry;
    }

    /// Fetch.
    ///
    /// Fetch the pastes, via their ID.
    ///
    /// - [id]: The ID to look for.
    pub async fn fetch(db: &Database, id: Snowflake) -> Result<Option<Self>, AppError> {
        let paste_id: i64 = id.into();
        let query = sqlx::query!(
            "SELECT id, edited, expiry FROM pastes WHERE id = $1",
            paste_id
        )
        .fetch_optional(db.pool())
        .await?;

        if let Some(q) = query {
            return Ok(Some(Self::new(q.id.into(), q.edited, q.expiry)));
        }

        Ok(None)
    }

    pub async fn fetch_between(
        db: &Database,
        start: OffsetDateTime,
        end: OffsetDateTime,
    ) -> Result<Vec<Self>, AppError> {
        let records = sqlx::query!(
            "SELECT id, edited, expiry FROM pastes WHERE expiry >= $1 AND expiry <= $2",
            start,
            end
        )
        .fetch_all(db.pool())
        .await?;

        let mut pastes = Vec::new();
        for record in records {
            let paste = Self::new(record.id.into(), record.edited, record.expiry);

            pastes.push(paste);
        }

        Ok(pastes)
    }

    /// Update.
    ///
    /// Update a existing paste.
    pub async fn update(&self, transaction: &mut PgTransaction<'_>) -> Result<(), AppError> {
        let paste_id: i64 = self.id.into();

        sqlx::query!(
            "INSERT INTO pastes(id, edited, expiry) VALUES ($1, $2, $3) ON CONFLICT (id) DO UPDATE SET edited = $2, expiry = $3",
            paste_id,
            self.edited,
            self.expiry
        ).execute(transaction.as_mut()).await?;

        Ok(())
    }

    /// Delete.
    ///
    /// Delete an existing paste with the provided ID.
    ///
    /// - [id]: The ID to delete from.
    pub async fn delete_with_id(db: &Database, id: Snowflake) -> Result<(), AppError> {
        let paste_id: i64 = id.into();
        sqlx::query!("DELETE FROM pastes WHERE id = $1", paste_id,)
            .execute(db.pool())
            .await?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum ExpiryTaskMessage {
    /// Cancel the expiry runners.
    Cancel,
}

pub async fn expiry_tasks(app: App, mut rx: Receiver<ExpiryTaskMessage>) {
    const MINUTES: u64 = 50;

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

        match Paste::delete_with_id(&app.database, paste.id).await {
            Ok(()) => tracing::trace!("Successfully deleted paste: {}", paste.id),
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

                    match Paste::delete_with_id(&app.database, paste.id).await {
                        Ok(()) => tracing::trace!("Successfully deleted paste: {}", paste.id),
                        Err(e) => tracing::warn!("Failure to delete paste: {}. Reason: {}", paste.id, e)
                    }
                }
            }
        }
    }
}

async fn collect_nearby_expired_tasks(db: &Database) -> Result<Vec<Paste>, AppError> {
    let start = OffsetDateTime::from_unix_timestamp(0)
        .expect("Failed to make a timestamp with the time of 0.");
    let end = OffsetDateTime::now_utc();

    Paste::fetch_between(db, start, end).await
}
