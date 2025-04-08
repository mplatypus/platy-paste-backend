use serde::{Deserialize, Serialize};
use sqlx::PgTransaction;

use crate::app::database::Database;

use super::{error::AppError, snowflake::Snowflake};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Document {
    /// The ID of the document.
    pub id: Snowflake,
    /// The paste that owns the document.
    pub paste_id: Snowflake,
    /// The type of document.
    pub document_type: String,
    /// The name of the document.
    pub name: String,
}

impl Document {
    /// New.
    ///
    /// Create a new [`Document`] object.
    pub const fn new(
        id: Snowflake,
        paste_id: Snowflake,
        document_type: String,
        name: String,
    ) -> Self {
        Self {
            id,
            paste_id,
            document_type,
            name,
        }
    }

    /// Generate URL.
    ///
    /// Generate a URL to fetch the location of the document.
    ///
    /// ## Arguments
    ///
    /// - `base_url` - The base url to append.
    ///
    /// ## Returns
    ///
    /// The URL generated.
    pub fn generate_url(&self, base_url: &str) -> String {
        format!("{}/documents/{}", base_url, self.generate_path())
    }

    /// Generate Path.
    ///
    /// Generate the path to the resource.
    ///
    /// ## Returns
    ///
    /// The path generated.
    pub fn generate_path(&self) -> String {
        format!("{}/{}-{}", self.paste_id, self.id, self.name)
    }

    /// Fetch.
    ///
    /// Fetch a document via its ID.
    ///
    /// ## Arguments
    ///
    /// - `db` - The database to make the request to.
    /// - `id` - The ID of the document.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error.
    ///
    /// ## Returns
    ///
    /// - [`Option::Some`] - The [`Document`] object.
    /// - [`Option::None`] - No document was found.
    pub async fn fetch(db: &Database, id: Snowflake) -> Result<Option<Self>, AppError> {
        let paste_id: i64 = id.into();
        let query = sqlx::query!(
            "SELECT id, paste_id, type, name FROM documents WHERE id = $1",
            paste_id
        )
        .fetch_optional(db.pool())
        .await?;

        if let Some(q) = query {
            return Ok(Some(Self::new(
                q.id.into(),
                q.paste_id.into(),
                q.r#type,
                q.name,
            )));
        }

        Ok(None)
    }

    /// Fetch All.
    ///
    /// Fetch all documents attached to a paste.
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
    /// A [`Vec`] of [`Document`]'s.
    pub async fn fetch_all(db: &Database, id: Snowflake) -> Result<Vec<Self>, AppError> {
        let paste_id: i64 = id.into();
        let query = sqlx::query!(
            "SELECT id, paste_id, type, name FROM documents WHERE paste_id = $1",
            paste_id
        )
        .fetch_all(db.pool())
        .await?;

        let mut documents: Vec<Self> = Vec::new();
        for record in query {
            documents.push(Self::new(
                record.id.into(),
                record.paste_id.into(),
                record.r#type,
                record.name,
            ));
        }
        Ok(documents)
    }

    /// Insert.
    ///
    /// Insert (create) a document.
    ///
    /// ## Arguments
    ///
    /// - `transaction` The transaction to use.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error, or the snowflake exists already.
    pub async fn insert(
        &self, transaction: &mut PgTransaction<'_>
    ) -> Result<(), AppError> {
        let document_id: i64 = self.id.into();
        let paste_id: i64 = self.paste_id.into();

        sqlx::query!(
            "INSERT INTO documents(id, paste_id, type, name) VALUES ($1, $2, $3, $4)",
            document_id,
            paste_id,
            self.document_type,
            self.name
        ).execute(transaction.as_mut()).await?;

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
        let document_id: i64 = self.id.into();
        let paste_id: i64 = self.paste_id.into();

        sqlx::query!(
            "INSERT INTO documents(id, paste_id, type, name) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET type = $3, name = $4",
            document_id,
            paste_id,
            self.document_type,
            self.name
        ).execute(transaction.as_mut()).await?;

        Ok(())
    }

    /// Delete.
    ///
    /// Delete a document.
    ///
    /// ## Arguments
    ///
    /// - `db` - The database to make the request to.
    /// - `id` - The id of the document.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error.
    pub async fn delete(db: &Database, id: Snowflake) -> Result<(), AppError> {
        let paste_id: i64 = id.into();
        sqlx::query!("DELETE FROM documents WHERE id = $1", paste_id,)
            .execute(db.pool())
            .await?;

        Ok(())
    }
}
