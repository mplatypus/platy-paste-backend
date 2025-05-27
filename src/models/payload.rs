use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::app::config::Config;

use super::{
    authentication::Token, document::Document, paste::Paste, snowflake::Snowflake,
    undefined::UndefinedOption,
};

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

pub type PatchPasteQuery = IncludeContentQuery;

pub type PostDocumentQuery = IncludeContentQuery;

pub type PatchDocumentQuery = IncludeContentQuery;

#[derive(Deserialize)]
pub struct PasteBody {
    /// The expiry time for the paste.
    #[serde(default)]
    pub expiry: UndefinedOption<usize>,
    /// The maximum allowed views for a paste.
    #[serde(default)]
    pub max_views: Option<usize>,
}

pub type PostPasteBody = PasteBody;

pub type PatchPasteBody = PasteBody;

#[derive(Serialize)]
pub struct ResponseConfig {
    /// The default expiry in hours.
    pub default_expiry: Option<usize>,
    /// The maximum expiry in hours.
    pub maximum_expiry: Option<usize>,
    /// The maximum document count.
    pub maximum_document_count: usize,
    /// The maximum individual document size in mb.
    pub maximum_document_size: f64,
    /// The maximum total size of all documents in mb. (includes payload)
    pub maximum_total_document_size: f64,
}

impl ResponseConfig {
    /// New.
    ///
    /// Create a new [`ResponseConfig`] object.
    pub const fn new(
        default_expiry: Option<usize>,
        maximum_expiry: Option<usize>,
        maximum_document_count: usize,
        maximum_document_size: f64,
        maximum_total_document_size: f64,
    ) -> Self {
        Self {
            default_expiry,
            maximum_expiry,
            maximum_document_count,
            maximum_document_size,
            maximum_total_document_size,
        }
    }

    pub const fn from_config(config: &Config) -> Self {
        Self::new(
            config.default_expiry_hours(),
            config.maximum_expiry_hours(),
            config.global_paste_total_document_count(),
            config.global_paste_document_size_limit(),
            config.global_paste_total_document_size_limit(),
        )
    }
}

#[derive(Serialize)]
pub struct ResponsePaste {
    /// The ID for the paste.
    pub id: Snowflake,
    /// The token attached to the paste.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// The time at which the paste was created.
    #[serde(rename = "timestamp")]
    pub creation: usize,
    /// Whether the paste has been edited.
    #[serde(rename = "edited_timestamp")]
    pub edited: Option<usize>,
    /// The expiry time of the paste.
    #[serde(rename = "expiry_timestamp")]
    pub expiry: Option<usize>,
    /// The view count for the paste.
    pub views: usize,
    /// The maximum amount of views the paste can have.
    pub max_views: Option<usize>,
    /// The documents attached to the paste.
    pub documents: Vec<ResponseDocument>,
}

impl ResponsePaste {
    /// New.
    ///
    /// Create a new [`ResponsePaste`] object.
    pub fn new(
        id: Snowflake,
        token: Option<String>,
        creation: OffsetDateTime,
        edited: Option<OffsetDateTime>,
        expiry: Option<OffsetDateTime>,
        views: usize,
        max_views: Option<usize>,
        documents: Vec<ResponseDocument>,
    ) -> Self {
        Self {
            id,
            token,
            creation: creation.unix_timestamp() as usize,
            edited: edited.map(|t| t.unix_timestamp() as usize),
            expiry: expiry.map(|t| t.unix_timestamp() as usize),
            views,
            max_views,
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

        Self::new(
            paste.id,
            token_value,
            paste.creation,
            paste.edited,
            paste.expiry,
            paste.views,
            paste.max_views,
            documents,
        )
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
