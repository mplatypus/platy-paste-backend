use serde::{Deserialize, Serialize};
use sqlx::PgTransaction;

use crate::app::database::Database;

use super::{error::AppError, snowflake::Snowflake};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Paste {
    /// The ID of the paste.
    pub id: Snowflake,
    /// Whether the paste has been edited.
    pub edited: bool,
}

impl Paste {
    pub const fn new(id: Snowflake, edited: bool) -> Self {
        Self {
            id,
            edited,
        }
    }

    pub fn set_edited(&mut self) {
        self.edited = true;
    }

    /// Fetch.
    ///
    /// Fetch the pastes, via their ID.
    ///
    /// - [id]: The ID to look for.
    pub async fn fetch(db: &Database, id: Snowflake) -> Result<Option<Self>, AppError> {
        let paste_id: i64 = id.into();
        let query = sqlx::query!(
            "SELECT id, edited FROM pastes WHERE id = $1",
            paste_id
        )
        .fetch_optional(db.pool())
        .await?;

        if let Some(q) = query {
            return Ok(Some(Self::new(
                q.id.into(),
                q.edited,
            )));
        }

        Ok(None)
    }

    /// Update.
    ///
    /// Update a existing paste.
    pub async fn update(&self, transaction: &mut PgTransaction<'_>) -> Result<(), AppError> {
        let paste_id: i64 = self.id.into();

        sqlx::query!(
            "INSERT INTO pastes(id, edited) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET edited = $2",
            paste_id,
            self.edited,
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
