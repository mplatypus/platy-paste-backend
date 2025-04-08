use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use super::{authentication::Token, document::Document, paste::Paste, snowflake::Snowflake};

#[derive(Deserialize)]
pub struct IncludeContentQuery {
    /// Whether to return the content(s) of the document(s).
    ///
    /// Defaults to False.
    #[serde(default, rename = "content")]
    pub include_content: bool,
}

pub type GetPasteQuery = IncludeContentQuery;

pub type PostPasteQuery = IncludeContentQuery;

#[derive(Deserialize)]
pub struct MultiplePastesBody {
    /// The ID's to use.
    pub ids: Vec<Snowflake>,
}

pub type GetPastesBody = MultiplePastesBody;

pub type DeletePastesBody = MultiplePastesBody;

#[derive(Deserialize)]
pub struct PostPasteBody {
    /// The expiry time for the paste.
    #[serde(default)]
    pub expiry: Option<usize>,
}

#[derive(Serialize)]
pub struct ResponsePaste {
    /// The ID for the paste.
    pub id: Snowflake,
    /// The token attached to the paste.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Whether the paste has been edited.
    pub edited: bool,
    /// The expiry time of the paste.
    pub expiry: Option<usize>,
    /// The documents attached to the paste.
    pub documents: Vec<ResponseDocument>,
}

impl ResponsePaste {
    /// New.
    ///
    /// Create a new [`ResponsePaste`] object.
    pub const fn new(
        id: Snowflake,
        token: Option<String>,
        edited: bool,
        expiry: Option<usize>,
        documents: Vec<ResponseDocument>,
    ) -> Self {
        Self {
            id,
            token,
            edited,
            expiry,
            documents,
        }
    }

    /// From Paste.
    ///
    /// Create a new [`ResponsePaste`] from a [`Paste`] and [`ResponseDocument`]'s
    ///
    /// ## Arguments
    ///
    /// - `paste` - The paste to extract from.
    /// - `token` - The token to use (if provided).
    /// - `documents` - The documents to attach.
    ///
    /// ## Returns
    ///
    /// The [`ResponsePaste`].
    pub fn from_paste(
        paste: &Paste,
        token: Option<Token>,
        documents: Vec<ResponseDocument>,
    ) -> Self {
        let token_value: Option<String> = { token.map(|t| t.token().expose_secret().to_string()) };

        let expiry = paste.expiry.map(|v| v.unix_timestamp() as usize);

        Self::new(paste.id, token_value, paste.edited, expiry, documents)
    }
}

#[derive(Serialize)]
pub struct ResponseDocument {
    /// The ID for the document.
    pub id: Snowflake,
    /// The paste ID the document is attached too.
    pub paste_id: Snowflake,
    /// The type of document.
    #[serde(rename = "type")]
    pub document_type: String,
    /// The name of the document.
    pub name: String,
    /// The content of the document.
    pub content: Option<String>,
}

impl ResponseDocument {
    /// New.
    ///
    /// Create a new [`ResponseDocument`] object.
    pub const fn new(
        id: Snowflake,
        paste_id: Snowflake,
        document_type: String,
        name: String,
        content: Option<String>,
    ) -> Self {
        Self {
            id,
            paste_id,
            document_type,
            name,
            content,
        }
    }

    /// From Document.
    ///
    /// Create a new [`ResponseDocument`] from a [`Document`] and its content.
    ///
    /// ## Arguments
    ///
    /// - `document` - The document to extract from.
    /// - `content` - The content to use (if provided).
    ///
    /// ## Returns
    ///
    /// The [`ResponseDocument`].
    pub fn from_document(document: Document, content: Option<String>) -> Self {
        Self::new(
            document.id,
            document.paste_id,
            document.document_type,
            document.name,
            content,
        )
    }
}
