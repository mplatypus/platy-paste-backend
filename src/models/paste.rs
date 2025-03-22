use serde::{Deserialize, Serialize};

use crate::app::database::Database;

use super::{error::AppError, snowflake::Snowflake};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Paste {
    /// The ID of the paste.
    pub id: Snowflake,
    /// The owner ID that owns the paste.
    pub owner_id: Option<Snowflake>,
    /// The bot token that owns the paste.
    pub owner_token: Option<String>,
    /// The document ID's.
    pub document_ids: Vec<Snowflake>,
}

impl Paste {
    pub const fn new(
        id: Snowflake,
        owner_id: Option<Snowflake>,
        owner_token: Option<String>,
        document_ids: Vec<Snowflake>,
    ) -> Self {
        Self {
            id,
            owner_id,
            owner_token,
            document_ids,
        }
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
            "SELECT id, owner_id, owner_token, document_ids FROM pastes WHERE id = $1",
            paste_id
        )
        .fetch_optional(db.pool())
        .await?;

        if let Some(q) = query {
            return Ok(Some(Self::new(
                q.id.into(),
                q.owner_id.map(std::convert::Into::into),
                q.owner_token,
                Self::decode_document_ids(&q.document_ids)?,
            )));
        }

        Ok(None)
    }

    /// Fetch All.
    ///
    /// Fetch all pastes owned by a token.
    ///
    /// - [token]: The Token to look for.
    pub async fn fetch_all(db: &Database, token: String) -> Result<Vec<Self>, AppError> {
        let query = sqlx::query!(
            "SELECT id, owner_id, owner_token, document_ids FROM pastes WHERE owner_token = $1",
            token
        )
        .fetch_all(db.pool())
        .await?;

        let mut pastes: Vec<Self> = Vec::new();
        for record in query {
            pastes.push(Self::new(
                record.id.into(),
                record.owner_id.map(std::convert::Into::into),
                record.owner_token,
                Self::decode_document_ids(&record.document_ids)?,
            ));
        }
        Ok(pastes)
    }

    /// Update.
    ///
    /// Update a existing paste.
    pub async fn update(&self, db: &Database) -> Result<(), AppError> {
        let paste_id: i64 = self.id.into();
        let owner_id: Option<i64> = self.owner_id.map(std::convert::Into::into);

        sqlx::query!(
            "INSERT INTO pastes(id, owner_id, owner_token, document_ids) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET document_ids = $4",
            paste_id,
            owner_id,
            self.owner_token,
            Self::encode_document_ids(&self.document_ids)
        ).execute(db.pool()).await?;

        Ok(())
    }

    /// Delete.
    ///
    /// Delete an existing paste.
    ///
    /// - [id]: The ID to delete from.
    pub async fn delete(db: &Database, id: Snowflake) -> Result<(), AppError> {
        let paste_id: i64 = id.into();
        sqlx::query!("DELETE FROM pastes WHERE id = $1", paste_id,)
            .execute(db.pool())
            .await?;

        Ok(())
    }

    /// Delete All.
    ///
    /// Delete all existing pastes owned by a token.
    ///
    ///  - [token]: The Token to delete from.
    pub async fn delete_all(db: &Database, token: String) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM pastes WHERE owner_token = $1", token,)
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
