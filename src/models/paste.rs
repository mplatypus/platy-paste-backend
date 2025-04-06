use time::OffsetDateTime;

use crate::app::database::Database;

use super::{error::AppError, snowflake::Snowflake};

#[derive(Debug, Clone)]
pub struct Paste {
    /// The ID of the paste.
    pub id: Snowflake,
    /// Whether the paste has been edited.
    pub edited: bool,
    /// The time when the paste expires.
    pub expiry: Option<OffsetDateTime>,
    /// The document ID's.
    pub document_ids: Vec<Snowflake>,
}

impl Paste {
    pub const fn new(
        id: Snowflake,
        edited: bool,
        expiry: Option<OffsetDateTime>,
        document_ids: Vec<Snowflake>,
    ) -> Self {
        Self {
            id,
            edited,
            expiry,
            document_ids,
        }
    }

    pub fn set_edited(&mut self) {
        self.edited = true;
    }

    pub fn set_expiry(&mut self, expiry: Option<OffsetDateTime>) {
        self.expiry = expiry;
    }

    pub fn add_document(&mut self, document_id: Snowflake) {
        self.document_ids.push(document_id);
    }

    pub fn remove_document(&mut self, index: usize) {
        self.document_ids.remove(index);
    }

    pub fn clear_documents(&mut self) {
        self.document_ids.clear();
    }

    /// Fetch.
    ///
    /// Fetch the pastes, via their ID.
    ///
    /// - [id]: The ID to look for.
    pub async fn fetch(db: &Database, id: Snowflake) -> Result<Option<Self>, AppError> {
        let paste_id: i64 = id.into();
        let query = sqlx::query!(
            "SELECT id, edited, expiry, document_ids FROM pastes WHERE id = $1",
            paste_id
        )
        .fetch_optional(db.pool())
        .await?;

        if let Some(q) = query {
            return Ok(Some(Self::new(
                q.id.into(),
                q.edited,
                q.expiry,
                Self::decode_document_ids(&q.document_ids)?,
            )));
        }

        Ok(None)
    }

    /// Update.
    ///
    /// Update a existing paste.
    pub async fn update(&self, db: &Database) -> Result<(), AppError> {
        let paste_id: i64 = self.id.into();

        sqlx::query!(
            "INSERT INTO pastes(id, edited, expiry, document_ids) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET edited = $2, expiry = $3, document_ids = $4",
            paste_id,
            self.edited,
            self.expiry,
            Self::encode_document_ids(&self.document_ids)
        ).execute(db.pool()).await?;

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

    fn decode_document_ids(document_ids_string: &str) -> Result<Vec<Snowflake>, AppError> {
        let mut document_ids = Vec::new();
        for document_id_string in document_ids_string.split("::") {
            document_ids.push(Snowflake::try_from(document_id_string)?);
        }
        Ok(document_ids)
    }

    fn encode_document_ids(document_ids: &[Snowflake]) -> String {
        let document_id_strings: Vec<String> =
            document_ids.iter().map(ToString::to_string).collect();
        document_id_strings.join("::")
    }
}
