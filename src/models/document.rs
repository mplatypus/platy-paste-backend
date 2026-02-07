use regex::Regex;
use serde::Serialize;
use sqlx::{PgExecutor, PgTransaction};

#[cfg(test)]
use serde::Deserialize;

use crate::{
    app::config::Config,
    models::{errors::RESTError, snowflake::PartialSnowflake, undefined::Undefined},
};

use super::{errors::DatabaseError, snowflake::Snowflake};

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

#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize, Clone, Debug)]
pub struct Document {
    /// The ID of the document.
    id: Snowflake,
    /// The paste that owns the document.
    paste_id: Snowflake,
    /// The type of document.
    #[serde(rename = "type")]
    doc_type: String,
    /// The name of the document.
    name: String,
    /// The size of the document.
    size: usize,
}

impl Document {
    /// New.
    ///
    /// Create a new [`Document`] object.
    pub fn new(
        id: Snowflake,
        paste_id: Snowflake,
        doc_type: &str,
        name: &str,
        size: usize,
    ) -> Self {
        Self {
            id,
            paste_id,
            doc_type: doc_type.to_string(),
            name: name.to_string(),
            size,
        }
    }

    /// The documents ID.
    #[inline]
    pub const fn id(&self) -> &Snowflake {
        &self.id
    }

    /// The paste ID this document belongs too.
    #[inline]
    pub const fn paste_id(&self) -> &Snowflake {
        &self.paste_id
    }

    /// The documents type.
    #[inline]
    pub fn doc_type(&self) -> &str {
        &self.doc_type
    }

    /// The documents name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The documents size.
    #[inline]
    pub const fn size(&self) -> usize {
        self.size
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
    #[inline]
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
    #[inline]
    pub fn generate_path(&self) -> String {
        format!("{}/{}/{}", self.paste_id, self.id, self.name)
    }

    /// Set Document Type.
    ///
    /// Set the document type.
    #[inline]
    pub fn set_doc_type(&mut self, document_type: &str) {
        self.doc_type = document_type.to_string();
    }

    /// Set Name.
    ///
    /// Set the document name.
    #[inline]
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    /// Set Size.
    ///
    /// Set the document size.
    #[inline]
    pub const fn set_size(&mut self, size: usize) {
        self.size = size;
    }

    /// Fetch.
    ///
    /// Fetch a document via its ID.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
    /// - `id` - The ID of the document.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error.
    ///
    /// ## Returns
    ///
    /// - [`Option::Some`] - The [`Document`] object.
    /// - [`Option::None`] - No document was found.
    pub async fn fetch<'e, 'c: 'e, E>(
        executor: E,
        id: &Snowflake,
    ) -> Result<Option<Self>, DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let paste_id: i64 = (*id).into();
        let query = sqlx::query!(
            "SELECT id, paste_id, type, name, size FROM documents WHERE id = $1",
            paste_id
        )
        .fetch_optional(executor)
        .await?;

        if let Some(q) = query {
            return Ok(Some(Self::new(
                q.id.into(),
                q.paste_id.into(),
                &q.r#type,
                &q.name,
                q.size as usize,
            )));
        }

        Ok(None)
    }

    /// Fetch With Paste.
    ///
    /// Fetch a document via its ID, along with a paste ID.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
    /// - `paste_id` - The ID of the paste.
    /// - `id` - The ID of the document.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error.
    ///
    /// ## Returns
    ///
    /// - [`Option::Some`] - The [`Document`] object.
    /// - [`Option::None`] - No document was found.
    pub async fn fetch_with_paste<'e, 'c: 'e, E>(
        executor: E,
        paste_id: &Snowflake,
        id: &Snowflake,
    ) -> Result<Option<Self>, DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let paste_id: i64 = (*paste_id).into();
        let id: i64 = (*id).into();
        let query = sqlx::query!(
            "SELECT id, paste_id, type, name, size FROM documents WHERE paste_id = $1 AND id = $2",
            paste_id,
            id
        )
        .fetch_optional(executor)
        .await?;

        if let Some(q) = query {
            return Ok(Some(Self::new(
                q.id.into(),
                q.paste_id.into(),
                &q.r#type,
                &q.name,
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
    /// - `executor` - The database pool or transaction to use.
    /// - `id` - The ID of the paste.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error.
    ///
    /// ## Returns
    ///
    /// A [`Vec`] of [`Document`]'s.
    pub async fn fetch_all<'e, 'c: 'e, E>(
        executor: E,
        id: &Snowflake,
    ) -> Result<Vec<Self>, DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let paste_id: i64 = (*id).into();
        let query = sqlx::query!(
            "SELECT id, paste_id, type, name, size FROM documents WHERE paste_id = $1",
            paste_id
        )
        .fetch_all(executor)
        .await?;

        let mut documents: Vec<Self> = Vec::new();
        for record in query {
            documents.push(Self::new(
                record.id.into(),
                record.paste_id.into(),
                &record.r#type,
                &record.name,
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
    /// - `executor` - The database pool or transaction to use.
    /// - `id` - The ID of the paste.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error.
    ///
    /// ## Returns
    ///
    /// The size of the total documents.
    pub async fn fetch_total_document_size<'e, 'c: 'e, E>(
        executor: E,
        id: &Snowflake,
    ) -> Result<usize, DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let id: i64 = (*id).into();
        let size = sqlx::query_scalar!(
            "SELECT SUM(size)::BIGINT FROM documents WHERE paste_id = $1",
            id
        )
        .fetch_one(executor)
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
    /// - `executor` - The database pool or transaction to use.
    /// - `id` - The ID of the paste.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error.
    ///
    /// ## Returns
    ///
    /// The total count of documents.
    pub async fn fetch_total_document_count<'e, 'c: 'e, E>(
        executor: E,
        id: &Snowflake,
    ) -> Result<usize, DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let id: i64 = (*id).into();
        let size = sqlx::query_scalar!("SELECT COUNT(*) FROM documents WHERE paste_id = $1", id)
            .fetch_one(executor)
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
    /// - `executor` - The database pool or transaction to use.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error, or the snowflake exists already.
    pub async fn insert<'e, 'c: 'e, E>(&self, executor: E) -> Result<(), DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let document_id: i64 = self.id.into();
        let paste_id: i64 = self.paste_id.into();

        sqlx::query!(
            "INSERT INTO documents(id, paste_id, type, name, size) VALUES ($1, $2, $3, $4, $5)",
            document_id,
            paste_id,
            self.doc_type,
            self.name,
            self.size as i64
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
    /// - [`DatabaseError`] - The database had an error.
    pub async fn update<'e, 'c: 'e, E>(&self, executor: E) -> Result<(), DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let document_id: i64 = self.id.into();
        let paste_id: i64 = self.paste_id.into();

        sqlx::query!(
            "INSERT INTO documents(id, paste_id, type, name, size) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO UPDATE SET type = $3, name = $4, size = $5",
            document_id,
            paste_id,
            self.doc_type,
            self.name,
            self.size as i64
        ).execute(executor).await?;

        Ok(())
    }

    /// Delete.
    ///
    /// Delete a document.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
    /// - `id` - The id of the document.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error.
    pub async fn delete<'e, 'c: 'e, E>(executor: E, id: &Snowflake) -> Result<bool, DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let paste_id: i64 = (*id).into();
        let result = sqlx::query!("DELETE FROM documents WHERE id = $1", paste_id,)
            .execute(executor)
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
            } else if let Some(capture) = match_all_mime.captures(mime)
                && let (Some(mime_value_left), Some(capture_value_left)) =
                    (split_mime_value.name("left"), capture.name("left"))
                && mime_value_left.as_str() == capture_value_left.as_str()
            {
                return true;
            }
        }
    }

    false
}

/// Document Limits.
///
/// Validate that a document is within the requirements.
///
/// ## Arguments
///
/// - `id` - The documents relavant ID.
/// - `config` - The config to check again.
/// - `document` - The document to check.
///
/// ## Errors
///
/// - [`RESTError`] - Returned when the documents are outside of the limits.
pub fn document_limits(
    config: &Config,
    id: &PartialSnowflake,
    name: Undefined<&str>,
    content: Undefined<&str>,
) -> Result<(), RESTError> {
    let size_limits = config.size_limits();

    if let Undefined::Some(content) = content {
        let content_length = content.len();

        if size_limits.minimum_document_size() > content_length {
            return Err(RESTError::BadRequest(format!(
                "Document `{}` is too small.",
                id
            )));
        }

        if size_limits.maximum_document_size() < content_length {
            return Err(RESTError::BadRequest(format!(
                "Document `{}` is too large.",
                id
            )));
        }
    }

    if let Undefined::Some(name) = name {
        let name_length = name.len();

        if size_limits.minimum_document_name_size() > name_length {
            return Err(RESTError::BadRequest(format!(
                "Document `{}`'s name: `{}` is too small.",
                id, name
            )));
        }

        if size_limits.maximum_document_name_size() < name_length {
            if name_length > 50 {
                return Err(RESTError::BadRequest(format!(
                    "Document `{}`'s name: `{}`... is too large.",
                    id,
                    &name[..name
                        .char_indices()
                        .nth(47)
                        .map(|(i, _)| i)
                        .unwrap_or(name.len())]
                )));
            }
            return Err(RESTError::BadRequest(format!(
                "Document `{}`'s name: `{}` is too large.",
                id, name
            )));
        }
    }

    Ok(())
}

/// Total Document Size Limit.
///
/// Validate that all documents attached to a paste are within the limits.
///
/// ## Arguments
///
/// - `transaction` - The transaction to use.
/// - `config` - The config to check again.
/// - `paste_id` - The Paste ID the document(s) are attached to.
///
/// ## Errors
///
/// - [`RESTError`] - Returned when the documents are outside of the limits.
pub async fn total_document_limits(
    transaction: &mut PgTransaction<'_>,
    config: &Config,
    paste_id: &Snowflake,
) -> Result<(), RESTError> {
    let size_limits = config.size_limits();

    let total_document_count =
        Document::fetch_total_document_count(transaction.as_mut(), paste_id).await?;

    if size_limits.minimum_total_document_count() > total_document_count {
        return Err(RESTError::BadRequest(format!(
            "Not enough documents were provided. Expected: {}, Received: {}",
            size_limits.minimum_total_document_count(),
            total_document_count,
        )));
    }

    if size_limits.maximum_total_document_count() < total_document_count {
        return Err(RESTError::BadRequest(format!(
            "Too many documents were provided. Expected: {}, Received: {}",
            size_limits.maximum_total_document_count(),
            total_document_count,
        )));
    }

    let total_document_size =
        Document::fetch_total_document_size(transaction.as_mut(), paste_id).await?;

    if size_limits.minimum_total_document_size() > total_document_size {
        return Err(RESTError::BadRequest(
            "One or more documents is below the minimum individual document size.".to_string(),
        ));
    }

    if size_limits.maximum_total_document_size() < total_document_size {
        return Err(RESTError::BadRequest(
            "One or more documents exceed the maximum individual document size.".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use sqlx::PgPool;

    use crate::app::{
        config::{Config, SizeLimitConfig},
        database::Database,
    };

    fn make_document_limits_config(
        minimum_document_size: usize,
        minimum_document_name_size: usize,
        maximum_document_size: usize,
        maximum_document_name_size: usize,
    ) -> Config {
        Config::test_builder()
            .size_limits(
                SizeLimitConfig::test_builder()
                    .minimum_document_size(minimum_document_size)
                    .minimum_document_name_size(minimum_document_name_size)
                    .maximum_document_size(maximum_document_size)
                    .maximum_document_name_size(maximum_document_name_size)
                    .build()
                    .expect("Failed to build rate limits"),
            )
            .build()
            .expect("Failed to build config.")
    }

    #[test]
    fn test_document_limits() {
        document_limits(
            &make_document_limits_config(1, 3, 1_000_000, 50),
            &PartialSnowflake::new(123),
            Undefined::Some("text/plain"),
            Undefined::Some("some random content."),
        )
        .expect("An error occurred.");
    }

    #[rstest]
    #[case(
        make_document_limits_config(1, 50, 1_000_000, 50),
        "test_doc.txt",
        "Document `123`'s name: `test_doc.txt` is too small."
    )]
    #[case(
        make_document_limits_config(1, 3, 1_000_000, 10),
        "test_doc.txt",
        "Document `123`'s name: `test_doc.txt` is too large."
    )]
    #[case(
        make_document_limits_config(1, 3, 1_000_000, 10),
        "this_is_a_really_long_document_name_that_is_far_too_long_for_use.txt",
        "Document `123`'s name: `this_is_a_really_long_document_name_that_is_far`... is too large."
    )]
    #[case(
        make_document_limits_config(500, 3, 1_000_000, 50),
        "test_doc.txt",
        "Document `123` is too small."
    )]
    #[case(
        make_document_limits_config(1, 3, 250, 50),
        "test_doc.txt",
        "Document `123` is too large."
    )]
    fn test_document_limits_errors(
        #[case] config: Config,
        #[case] title: &str,
        #[case] expected: &str,
    ) {
        let content: String = (0..=489).map(|_| 'a').collect();

        let error = document_limits(
            &config,
            &PartialSnowflake::new(123),
            Undefined::Some(title),
            Undefined::Some(&content),
        )
        .expect_err("No error received.");

        if let RESTError::BadRequest(bad_request) = error {
            assert_eq!(
                bad_request, expected,
                "The bad request message received was unexpected."
            );
        } else {
            panic!("The error received, was not expected.");
        }
    }

    fn make_total_document_limits_config(
        minimum_total_document_count: usize,
        minimum_total_document_size: usize,
        maximum_total_document_count: usize,
        maximum_total_document_size: usize,
    ) -> Config {
        Config::test_builder()
            .size_limits(
                SizeLimitConfig::test_builder()
                    .minimum_total_document_count(minimum_total_document_count)
                    .minimum_total_document_size(minimum_total_document_size)
                    .maximum_total_document_count(maximum_total_document_count)
                    .maximum_total_document_size(maximum_total_document_size)
                    .build()
                    .expect("Failed to build rate limits"),
            )
            .build()
            .expect("Failed to build config.")
    }

    #[sqlx::test(fixtures(path = "../../tests/fixtures/", scripts("pastes", "documents")))]
    async fn test_total_document_limits(pool: PgPool) {
        let db = Database::from_pool(pool);

        let mut transaction = db
            .pool()
            .begin()
            .await
            .expect("Failed to generate a transaction.");

        total_document_limits(
            &mut transaction,
            &make_total_document_limits_config(1, 1, 10, 10_000_000),
            &Snowflake::new(517_815_304_354_284_601),
        )
        .await
        .expect("An error occurred.");
    }

    #[rstest]
    #[case(
        make_total_document_limits_config(5, 1, 5, 5000),
        "Not enough documents were provided. Expected: 5, Received: 2"
    )]
    #[case(
        make_total_document_limits_config(1, 1, 1, 5000),
        "Too many documents were provided. Expected: 1, Received: 2"
    )]
    #[case(
        make_total_document_limits_config(1, 2500, 5, 5000),
        "One or more documents is below the minimum individual document size."
    )]
    #[case(
        make_total_document_limits_config(1, 1, 5, 2000),
        "One or more documents exceed the maximum individual document size."
    )]
    #[sqlx::test(fixtures(path = "../../tests/fixtures/", scripts("pastes", "documents")))]
    async fn test_total_document_limits_errors(
        #[ignore] pool: PgPool,
        #[case] config: Config,
        #[case] expected: &str,
    ) {
        use crate::app::database::Database;

        let db = Database::from_pool(pool);

        let mut transaction = db
            .pool()
            .begin()
            .await
            .expect("Failed to generate a transaction.");

        let error = total_document_limits(
            &mut transaction,
            &config,
            &Snowflake::new(517_815_304_354_284_602),
        )
        .await
        .expect_err("No error received.");

        if let RESTError::BadRequest(bad_request) = error {
            assert_eq!(
                bad_request, expected,
                "The bad request message received was unexpected."
            );
        } else {
            panic!("The error received, was not expected.");
        }
    }
}
