use serde::{Deserialize, Serialize, Serializer};

use crate::app::database::Database;

use super::{error::AppError, paste::Paste, snowflake::Snowflake};

#[derive(Clone, Debug)]
pub enum DocumentType {
    /// Represents a text document.
    Text,
    /// Represents a python document.
    Python,
    /// Represents a rust document.
    Rust,
    /// Represents an sql document.
    Sql,
    /// Represents a markdown document.
    Markdown,
    /// Represents a document of an unknown type.
    ///
    /// This should always be displayed as the `Text` type.
    Unknown(String),
}

impl DocumentType {
    pub fn from_file_type(file_type: String) -> DocumentType {
        match file_type.to_lowercase().as_str() { // TODO: Is there more file types that should be matched?
            "txt" => DocumentType::Text,
            "py" => DocumentType::Python,
            "rs" => DocumentType::Rust,
            "sql" => DocumentType::Sql,
            "md" => DocumentType::Markdown,
            value => DocumentType::Unknown(value.to_string())
        }
    }
}

impl Serialize for DocumentType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DocumentType::Text => "text",
            DocumentType::Python => "python",
            DocumentType::Rust => "rust",
            DocumentType::Sql => "sql",
            DocumentType::Markdown => "markdown",
            DocumentType::Unknown(unknown_type) => unknown_type,
        }
        .serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for DocumentType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = String::deserialize(d)?;

        Ok(DocumentType::from(value))
    }
}

impl From<String> for DocumentType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "text" => DocumentType::Text,
            "python" => DocumentType::Python,
            "rust" => DocumentType::Rust,
            "sql" => DocumentType::Sql,
            "markdown" => DocumentType::Markdown,
            "unknown" => DocumentType::Unknown("unknown".to_string()), // the file type unknown is locked to unknown.
            unknown => DocumentType::Unknown(unknown.to_string()),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Document {
    /// The ID of the document.
    pub id: Snowflake,
    /// The token that owns the document.
    pub token: String,
    /// The paste that owns the document.
    pub paste_id: Snowflake,
    /// The type of document.
    pub doc_type: DocumentType,
}

impl Document {
    pub fn new(
        id: Snowflake,
        token: String,
        paste_id: Snowflake,
        doc_type: DocumentType,
    ) -> Document {
        Document {
            id,
            token,
            paste_id,
            doc_type,
        }
    }

    /// Generate URL.
    ///
    /// Generate a URL to fetch the location of the document.
    ///
    /// - [base_url]: The base url to append.
    pub fn generate_url(&self, base_url: String) -> String {
        format!("{}/{}/{}", base_url, self.token, self.id)
    }

    /// Fetch.
    ///
    /// Fetch the documents, via their ID.
    ///
    /// - [id]: The ID to look for.
    pub async fn fetch(db: &Database, id: Snowflake) -> Result<Option<Document>, AppError> {
        let paste_id: i64 = id.into();
        let query = sqlx::query!(
            "SELECT id, owner_token, paste_id, type FROM documents WHERE id = $1",
            paste_id
        )
        .fetch_optional(db.pool())
        .await?;

        if let Some(q) = query {
            return Ok(Some(Document::new(
                q.id.into(),
                q.owner_token,
                q.paste_id.into(),
                DocumentType::from(q.r#type),
            )));
        }

        Ok(None)
    }

    /// Fetch All Token.
    ///
    /// Fetch all documents owned by a token.
    ///
    /// - [token]: The Token to look for.
    pub async fn fetch_all_token(db: &Database, token: String) -> Result<Vec<Document>, AppError> {
        let query = sqlx::query!(
            "SELECT id, owner_token, paste_id, type FROM documents WHERE owner_token = $1",
            token
        )
        .fetch_all(db.pool())
        .await?;

        let mut documents: Vec<Document> = Vec::new();
        for record in query {
            documents.push(Document::new(
                record.id.into(),
                record.owner_token,
                record.paste_id.into(),
                DocumentType::from(record.r#type),
            ))
        }
        Ok(documents)
    }

    /// Fetch All Paste.
    ///
    /// Fetch all documents owned by a paste.
    ///
    /// - [id]: The paste ID to look for.
    pub async fn fetch_all_paste(db: &Database, id: Snowflake) -> Result<Vec<Document>, AppError> {
        let paste_id: i64 = id.into();
        let query = sqlx::query!(
            "SELECT id, owner_token, paste_id, type FROM documents WHERE paste_id = $1",
            paste_id
        )
        .fetch_all(db.pool())
        .await?;

        let mut documents: Vec<Document> = Vec::new();
        for record in query {
            documents.push(Document::new(
                record.id.into(),
                record.owner_token,
                record.paste_id.into(),
                DocumentType::from(record.r#type),
            ))
        }
        Ok(documents)
    }

    /// Update.
    ///
    /// Update a existing paste.
    pub async fn update(&self, db: &Database) -> Result<Paste, AppError> {
        todo!()
    }

    /// Delete.
    ///
    /// Delete an existing paste.
    ///
    /// - [id]: The ID to delete from.
    pub async fn delete(&self, db: &Database, id: Snowflake) -> Result<(), AppError> {
        let paste_id: i64 = id.into();
        sqlx::query!("DELETE FROM documents WHERE id = $1", paste_id,)
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
        sqlx::query!("DELETE FROM documents WHERE owner_token = $1", token,)
            .execute(db.pool())
            .await?;

        Ok(())
    }
}
