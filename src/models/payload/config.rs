use secrecy::ExposeSecret as _;
#[cfg(test)]
use serde::Deserialize;
use serde::Serialize;

use crate::{
    app::config::Config,
    models::{
        DtUtc, authentication::Token, document::Document, paste::Paste, snowflake::Snowflake,
    },
};

//----------//
// Response //
//----------//

#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize)]
pub struct ResponseConfig {
    /// Defaults.
    defaults: ResponseDefaultsConfig,
    /// Size limits.
    size_limits: ResponseSizeLimitsConfig,
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

#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize)]
pub struct ResponseDefaultsConfig {
    /// The default expiry for pastes.
    expiry_hours: Option<usize>,
    /// The default value for maximum views.
    maximum_views: Option<usize>,
    /// The default value for paste names.
    paste_name: Option<String>,
}

impl ResponseDefaultsConfig {
    /// New.
    ///
    /// Create a new [`ResponseConfig`] object.
    pub const fn new(
        expiry_hours: Option<usize>,
        maximum_views: Option<usize>,
        paste_name: Option<String>,
    ) -> Self {
        Self {
            expiry_hours,
            maximum_views,
            paste_name,
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
            size_limits.default_paste_name().map(ToString::to_string),
        )
    }
}

#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize)]
pub struct ResponseSizeLimitsConfig {
    /// The minimum size of a paste name.
    minimum_paste_name_size: usize,
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
    /// The maximum size of a paste name.
    maximum_paste_name_size: usize,
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
    #[expect(clippy::too_many_arguments)]
    pub const fn new(
        minimum_paste_name_size: usize,
        minimum_expiry_hours: Option<usize>,
        minimum_total_document_count: usize,
        minimum_document_size: usize,
        minimum_total_document_size: usize,
        minimum_document_name_size: usize,
        maximum_paste_name_size: usize,
        maximum_expiry_hours: Option<usize>,
        maximum_total_document_count: usize,
        maximum_document_size: usize,
        maximum_total_document_size: usize,
        maximum_document_name_size: usize,
    ) -> Self {
        Self {
            minimum_paste_name_size,
            minimum_expiry_hours,
            minimum_total_document_count,
            minimum_document_size,
            minimum_total_document_size,
            minimum_document_name_size,
            maximum_paste_name_size,
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
    pub const fn from_config(config: &Config) -> Self {
        let size_limits = config.size_limits();
        Self::new(
            size_limits.minimum_paste_name_size(),
            size_limits.minimum_expiry_hours(),
            size_limits.minimum_total_document_count(),
            size_limits.minimum_document_size(),
            size_limits.minimum_total_document_size(),
            size_limits.minimum_document_name_size(),
            size_limits.maximum_paste_name_size(),
            size_limits.maximum_expiry_hours(),
            size_limits.maximum_total_document_count(),
            size_limits.maximum_document_size(),
            size_limits.maximum_total_document_size(),
            size_limits.maximum_document_name_size(),
        )
    }
}

#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize)]
pub struct ResponsePaste {
    /// The ID for the paste.
    id: Snowflake,
    /// The name for the paste.
    name: Option<String>,
    /// The token attached to the paste.
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    /// The time at which the paste was created.
    #[serde(rename = "timestamp")]
    creation: DtUtc,
    /// Whether the paste has been edited.
    #[serde(rename = "edited_timestamp")]
    edited: Option<DtUtc>,
    /// The expiry time of the paste.
    #[serde(rename = "expiry_timestamp")]
    expiry: Option<DtUtc>,
    /// The view count for the paste.
    views: usize,
    /// The maximum amount of views the paste can have.
    max_views: Option<usize>,
    /// The documents attached to the paste.
    documents: Vec<Document>,
}

impl ResponsePaste {
    /// New.
    ///
    /// Create a new [`ResponsePaste`] object.
    #[expect(clippy::too_many_arguments)]
    pub const fn new(
        id: Snowflake,
        name: Option<String>,
        token: Option<String>,
        creation: DtUtc,
        edited: Option<DtUtc>,
        expiry: Option<DtUtc>,
        views: usize,
        max_views: Option<usize>,
        documents: Vec<Document>,
    ) -> Self {
        Self {
            id,
            name,
            token,
            creation,
            edited,
            expiry,
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
            *paste.id(),
            paste.name().map(ToString::to_string),
            token_value,
            *paste.creation(),
            paste.edited().copied(),
            paste.expiry().copied(),
            paste.views(),
            paste.max_views(),
            documents,
        )
    }
}

#[cfg(test)]
impl ResponsePaste {
    pub fn id(&self) -> Snowflake {
        self.id
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    pub fn creation(&self) -> &DtUtc {
        &self.creation
    }

    pub fn edited(&self) -> Option<&DtUtc> {
        self.edited.as_ref()
    }

    pub fn expiry(&self) -> Option<&DtUtc> {
        self.expiry.as_ref()
    }

    pub fn views(&self) -> usize {
        self.views
    }

    pub fn max_views(&self) -> Option<usize> {
        self.max_views
    }

    pub fn documents(&self) -> &Vec<Document> {
        &self.documents
    }
}
