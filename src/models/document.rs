use std::fmt;

use serde::{Deserialize, Serialize, Serializer};

use crate::app::database::Database;

use super::{error::AppError, snowflake::Snowflake};

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
    pub fn from_file_type(file_type: &str) -> Self {
        match file_type.to_lowercase().as_str() {
            // TODO: Is there more file types that should be matched?
            "txt" => Self::Text,
            "py" => Self::Python,
            "rs" => Self::Rust,
            "sql" => Self::Sql,
            "md" => Self::Markdown,
            value => Self::Unknown(value.to_string()),
        }
    }

    pub const fn to_file_type(&self) -> &'static str {
        match self {
            Self::Text => "txt",
            Self::Python => "py",
            Self::Rust => "rs",
            Self::Sql => "sql",
            Self::Markdown => "md",
            #[allow(clippy::match_same_arms)]
            // This is only here due to the fact that "unknown" might change in the future.
            Self::Unknown(_) => "txt",
        }
    }
}

impl Serialize for DocumentType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for DocumentType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = String::deserialize(d)?;

        Ok(Self::from(value))
    }
}

impl From<String> for DocumentType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "text" => Self::Text,
            "python" => Self::Python,
            "rust" => Self::Rust,
            "sql" => Self::Sql,
            "markdown" => Self::Markdown,
            "unknown" => Self::Unknown("unknown".to_string()), // the file type unknown is locked to unknown.
            unknown => Self::Unknown(unknown.to_string()),
        }
    }
}

impl fmt::Display for DocumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let doc_type = match self {
            Self::Text => "text",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::Sql => "sql",
            Self::Markdown => "markdown",
            Self::Unknown(unknown_type) => unknown_type,
        };
        write!(f, "{doc_type}")
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Document {
    /// The ID of the document.
    pub id: Snowflake,
    /// The paste that owns the document.
    pub paste_id: Snowflake,
    /// The type of document.
    pub doc_type: DocumentType,
}

impl Document {
    pub const fn new(id: Snowflake, paste_id: Snowflake, doc_type: DocumentType) -> Self {
        Self {
            id,
            paste_id,
            doc_type,
        }
    }

    /// Generate URL.
    ///
    /// Generate a URL to fetch the location of the document.
    ///
    /// - [`base_url`]: The base url to append.
    pub fn generate_url(&self, base_url: &str) -> String {
        format!("{}/documents/{}", base_url, self.generate_path())
    }

    /// Generate Path.
    ///
    /// Generate the path to the resource.
    pub fn generate_path(&self) -> String {
        format!(
            "{}/{}.{}",
            self.paste_id,
            self.id,
            self.doc_type.to_file_type()
        )
    }

    /// Fetch.
    ///
    /// Fetch the documents, via their ID.
    ///
    /// - [id]: The ID to look for.
    pub async fn fetch(db: &Database, id: Snowflake) -> Result<Option<Self>, AppError> {
        let paste_id: i64 = id.into();
        let query = sqlx::query!(
            "SELECT id, paste_id, type FROM documents WHERE id = $1",
            paste_id
        )
        .fetch_optional(db.pool())
        .await?;

        if let Some(q) = query {
            return Ok(Some(Self::new(
                q.id.into(),
                q.paste_id.into(),
                DocumentType::from(q.r#type),
            )));
        }

        Ok(None)
    }

    /// Fetch All Paste.
    ///
    /// Fetch all documents owned by a paste.
    ///
    /// - [id]: The paste ID to look for.
    pub async fn fetch_all_paste(db: &Database, id: Snowflake) -> Result<Vec<Self>, AppError> {
        let paste_id: i64 = id.into();
        let query = sqlx::query!(
            "SELECT id, paste_id, type FROM documents WHERE paste_id = $1",
            paste_id
        )
        .fetch_all(db.pool())
        .await?;

        let mut documents: Vec<Self> = Vec::new();
        for record in query {
            documents.push(Self::new(
                record.id.into(),
                record.paste_id.into(),
                DocumentType::from(record.r#type),
            ));
        }
        Ok(documents)
    }

    /// Update.
    ///
    /// Update a existing paste.
    pub async fn update(&self, db: &Database) -> Result<(), AppError> {
        let document_id: i64 = self.id.into();
        let paste_id: i64 = self.paste_id.into();

        sqlx::query!(
            "INSERT INTO documents(id, paste_id, type) VALUES ($1, $2, $3) ON CONFLICT (id) DO UPDATE SET type = $3",
            document_id,
            paste_id,
            self.doc_type.to_string()
        ).execute(db.pool()).await?;

        Ok(())
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
}
