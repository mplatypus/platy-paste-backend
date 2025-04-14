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

pub type PatchPasteQuery = IncludeContentQuery;

#[derive(Deserialize)]
pub struct PasteBody {
    /// The expiry time for the paste.
    #[serde(default)]
    pub expiry: Option<usize>,
}

pub type PostPasteBody = PasteBody;

pub type PatchPasteBody = PasteBody;

pub type PostDocumentQuery = IncludeContentQuery;

pub type PatchDocumentQuery = IncludeContentQuery;

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
    pub documents: Vec<Document>,
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
        documents: Vec<Document>,
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
    /// Create a new [`ResponsePaste`] from a [`Paste`] and [`Document`]'s
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
    pub fn from_paste(paste: &Paste, token: Option<Token>, documents: &Vec<Document>) -> Self {
        let token_value: Option<String> = { token.map(|t| t.token().expose_secret().to_string()) };

        let expiry = paste.expiry.map(|v| v.unix_timestamp() as usize);

        Self::new(
            paste.id,
            token_value,
            paste.edited,
            expiry,
            documents.to_owned(),
        )
    }
}
