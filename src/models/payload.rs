use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::app::config::Config;

use super::{
    authentication::Token, document::Document, paste::Paste, snowflake::Snowflake,
    undefined::UndefinedOption,
};

#[derive(Deserialize)]
pub struct PastePath {
    /// The paste ID.
    pub paste_id: Snowflake,
}

pub type GetPastePath = PastePath;

pub type PatchPastePath = PastePath;

pub type DeletePastePath = PastePath;

pub type PostDocumentPath = PastePath;

#[derive(Deserialize)]
pub struct DocumentPath {
    /// The paste ID.
    pub paste_id: Snowflake,
    /// The document ID.
    pub document_id: Snowflake,
}

pub type GetDocumentPath = DocumentPath;

pub type PatchDocumentPath = DocumentPath;

pub type DeleteDocumentPath = DocumentPath;

#[derive(Deserialize)]
pub struct PasteBody {
    /// The expiry time for the paste.
    #[serde(default)]
    pub expiry: UndefinedOption<usize>,
    /// The maximum allowed views for the paste.
    #[serde(default)]
    pub max_views: UndefinedOption<usize>,
}

pub type PostPasteBody = PasteBody;

pub type PatchPasteBody = PasteBody;

#[derive(Serialize)]
pub struct ResponseConfig {
    /// Defaults.
    pub defaults: ResponseDefaultsConfig,
    /// Size limits.
    pub size_limits: ResponseSizeLimitsConfig,
}

impl ResponseConfig {
    /// New.
    ///
    /// Create a new [`ResponseConfig`] object.
    pub const fn new(
        defaults: ResponseDefaultsConfig,
        size_limits: ResponseSizeLimitsConfig,
    ) -> Self {
        Self {
            defaults,
            size_limits,
        }
    }

    /// From config.
    ///
    /// Create a new [`ResponseDefaultsConfig`] object, with a [`Config`] object.
    pub fn from_config(config: &Config) -> Self {
        Self::new(
            ResponseDefaultsConfig::from_config(config),
            ResponseSizeLimitsConfig::from_config(config),
        )
    }
}

#[derive(Serialize)]
pub struct ResponseDefaultsConfig {
    /// The default expiry for pastes.
    expiry_hours: Option<usize>,
    /// The default value for maximum views.
    maximum_views: Option<usize>,
}

impl ResponseDefaultsConfig {
    /// New.
    ///
    /// Create a new [`ResponseConfig`] object.
    pub const fn new(expiry_hours: Option<usize>, maximum_views: Option<usize>) -> Self {
        Self {
            expiry_hours,
            maximum_views,
        }
    }

    /// From config.
    ///
    /// Create a new [`ResponseDefaultsConfig`] object, with a [`Config`] object.
    pub fn from_config(config: &Config) -> Self {
        let size_limits = config.size_limits();

        Self::new(
            size_limits.default_expiry_hours(),
            size_limits.default_maximum_views(),
        )
    }
}

#[derive(Serialize)]
pub struct ResponseSizeLimitsConfig {
    /// The minimum expiry hours for pastes.
    minimum_expiry_hours: Option<usize>,
    /// The minimum allowed documents in a paste.
    minimum_total_document_count: usize,
    /// The minimum document size.
    minimum_document_size: usize,
    /// The minimum total document size.
    minimum_total_document_size: usize,
    /// The minimum size of a document name.
    minimum_document_name_size: usize,
    /// The maximum expiry for pastes.
    maximum_expiry_hours: Option<usize>,
    /// The maximum allowed documents in a paste.
    maximum_total_document_count: usize,
    /// The individual paste document size.
    maximum_document_size: usize,
    /// The maximum paste body size, including all documents.
    maximum_total_document_size: usize,
    /// The maximum size of a document name.
    maximum_document_name_size: usize,
}

impl ResponseSizeLimitsConfig {
    /// New.
    ///
    /// Create a new [`ResponseConfig`] object.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        minimum_expiry_hours: Option<usize>,
        minimum_total_document_count: usize,
        minimum_document_size: usize,
        minimum_total_document_size: usize,
        minimum_document_name_size: usize,
        maximum_expiry_hours: Option<usize>,
        maximum_total_document_count: usize,
        maximum_document_size: usize,
        maximum_total_document_size: usize,
        maximum_document_name_size: usize,
    ) -> Self {
        Self {
            minimum_expiry_hours,
            minimum_total_document_count,
            minimum_document_size,
            minimum_total_document_size,
            minimum_document_name_size,
            maximum_expiry_hours,
            maximum_total_document_count,
            maximum_document_size,
            maximum_total_document_size,
            maximum_document_name_size,
        }
    }

    /// From config.
    ///
    /// Create a new [`ResponseDefaultsConfig`] object, with a [`Config`] object.
    pub fn from_config(config: &Config) -> Self {
        let size_limits = config.size_limits();
        Self::new(
            size_limits.minimum_expiry_hours(),
            size_limits.minimum_total_document_count(),
            size_limits.minimum_document_size(),
            size_limits.minimum_total_document_size(),
            size_limits.minimum_document_name_size(),
            size_limits.maximum_expiry_hours(),
            size_limits.maximum_total_document_count(),
            size_limits.maximum_document_size(),
            size_limits.maximum_total_document_size(),
            size_limits.maximum_document_name_size(),
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
    pub documents: Vec<Document>,
}

impl ResponsePaste {
    /// New.
    ///
    /// Create a new [`ResponsePaste`] object.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Snowflake,
        token: Option<String>,
        creation: OffsetDateTime,
        edited: Option<OffsetDateTime>,
        expiry: Option<OffsetDateTime>,
        views: usize,
        max_views: Option<usize>,
        documents: Vec<Document>,
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
    pub fn from_paste(paste: &Paste, token: Option<Token>, documents: Vec<Document>) -> Self {
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
