use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::PgTransaction;

use crate::app::database::Database;

use super::{error::AppError, snowflake::Snowflake};

/* FIXME: Unsure if this is actually needed.
/// Supported mimes are the ones that will be supported by the website.
const SUPPORTED_MIMES: &[&str] = &[
    // Text mimes
    "text/x-asm",
    "text/x-c",
    "text/plain",
    "text/markdown",
    "text/css",
    "text/csv",
    "text/html",
    "text/x-java-source",
    "text/javascript",
    "text/x-pascal",
    "text/x-python",
    // Application mimes
    "application/json"
];
*/

/// Unsupported mimes, are ones that will be declined.
pub const UNSUPPORTED_MIMES: &[&str] =
    &["image/*", "video/*", "audio/*", "font/*", "application/pdf"];

pub const DEFAULT_MIME: &str = "text/plain";

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
    /// The size of the document.
    pub size: usize,
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
        size: usize,
    ) -> Self {
        Self {
            id,
            paste_id,
            document_type,
            name,
            size,
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

    /// Set Document Type.
    ///
    /// Set the document type.
    pub fn set_document_type(&mut self, document_type: String) {
        self.document_type = document_type;
    }

    /// Set Name.
    ///
    /// Set the document name.
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Set Size.
    ///
    /// Set the document size.
    pub fn set_size(&mut self, size: usize) {
        self.size = size;
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
            "SELECT id, paste_id, type, name, size FROM documents WHERE id = $1",
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
                q.size as usize,
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
            "SELECT id, paste_id, type, name, size FROM documents WHERE paste_id = $1",
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
                record.size as usize,
            ));
        }
        Ok(documents)
    }

    /// Fetch Total Document Size.
    ///
    /// Fetch the total size of all documents attached to a paste.
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
    /// The size of the total documents.
    pub async fn fetch_total_document_size(
        db: &Database,
        id: Snowflake,
    ) -> Result<usize, AppError> {
        let id: i64 = id.into();
        let size = sqlx::query_scalar!(
            "SELECT SUM(size)::BIGINT FROM documents WHERE paste_id = $1",
            id
        )
        .fetch_one(db.pool())
        .await?
        .unwrap_or(0);

        Ok(size as usize)
    }

    /// Fetch Total Document Count.
    ///
    /// Fetch the total amount of documents attached to a paste.
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
    /// The total count of documents.
    pub async fn fetch_total_document_count(
        db: &Database,
        id: Snowflake,
    ) -> Result<usize, AppError> {
        let id: i64 = id.into();
        let size = sqlx::query_scalar!("SELECT COUNT(*) FROM documents WHERE paste_id = $1", id)
            .fetch_one(db.pool())
            .await?
            .unwrap_or(0);

        Ok(size as usize)
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
    pub async fn insert(&self, transaction: &mut PgTransaction<'_>) -> Result<(), AppError> {
        let document_id: i64 = self.id.into();
        let paste_id: i64 = self.paste_id.into();

        sqlx::query!(
            "INSERT INTO documents(id, paste_id, type, name, size) VALUES ($1, $2, $3, $4, $5)",
            document_id,
            paste_id,
            self.document_type,
            self.name,
            self.size as i64
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
        let document_id: i64 = self.id.into();
        let paste_id: i64 = self.paste_id.into();

        sqlx::query!(
            "INSERT INTO documents(id, paste_id, type, name, size) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO UPDATE SET type = $3, name = $4, size = $5",
            document_id,
            paste_id,
            self.document_type,
            self.name,
            self.size as i64
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
    pub async fn delete(db: &Database, id: Snowflake) -> Result<bool, AppError> {
        let paste_id: i64 = id.into();
        let result = sqlx::query!("DELETE FROM documents WHERE id = $1", paste_id,)
            .execute(db.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

// FIXME: This whole function needs rebuilding. I do not like the way its made.
// For example, the regex values. Can I have them as constants in any way? or are they super light when unwrapping?
// Any way to shrink the `.capture` call so that its not being called each time?
/// Contains Mime.
///
/// Checks if the mime is in the list of mimes.
///
/// If a mime in the mimes list ends with an asterisk "*",
/// at the end like `images/*` it will become a catch all,
/// making all mimes that start with `images` return true.
///
/// ## Arguments
///
/// - `mimes` - The array of mimes to check in.
/// - `value` - The value to look for.
///
/// ## Returns
///
/// True if mime was found, otherwise False.
pub fn contains_mime(mimes: &[&str], value: &str) -> bool {
    let match_all_mime =
        Regex::new(r"^(?P<left>[a-zA-Z0-9]+)/\*$").expect("Failed to build match all mime regex."); // checks if the mime ends with /* which indicates any of the mime type.
    let split_mime = Regex::new(r"^(?P<left>[a-zA-Z0-9]+)/(?P<right>[a-zA-Z0-9\*]+)$")
        .expect("Failed to build split mime regex."); // extracts the left and right parts of the mime.

    if let Some(split_mime_value) = split_mime.captures(value) {
        for mime in mimes {
            if mime == &value {
                return true;
            } else if let Some(capture) = match_all_mime.captures(mime) {
                if let (Some(mime_value_left), Some(capture_value_left)) =
                    (split_mime_value.name("left"), capture.name("left"))
                {
                    if mime_value_left.as_str() == capture_value_left.as_str() {
                        return true;
                    }
                }
            }
        }
    }

    false
}
