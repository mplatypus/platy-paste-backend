//! Paths, Queries, Bodies and Responses related to the paste endpoints.

use std::collections::{HashMap, HashSet};

use axum::extract::{FromRequest, Multipart};
use bytes::Bytes;
use http::header::CONTENT_TYPE;
use mime::Mime;
use regex::Regex;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use crate::{
    app::application::App,
    models::{
        DtUtc,
        authentication::Token,
        document::{Document, UNSUPPORTED_MIMES, contains_mime, document_limits},
        errors::RESTError,
        paste::Paste,
        payload::document::{PatchPasteDocumentBody, PostPasteDocumentBody},
        snowflake::{PartialSnowflake, Snowflake},
        undefined::{Undefined, UndefinedOption},
    },
};

//------//
// Path //
//------//

/// ## Paste Path
///
/// The values within the path of a paste endpoint.
#[derive(Deserialize)]
pub struct PastePath {
    /// The paste ID.
    paste_id: Snowflake,
}

impl PastePath {
    /// The paste ID.
    #[inline]
    pub const fn paste_id(&self) -> &Snowflake {
        &self.paste_id
    }
}

/// Used for getting pastes.
pub type GetPastePath = PastePath;

/// Used for editing pastes.
pub type PatchPastePath = PastePath;

/// Used for deleting pastes.
pub type DeletePastePath = PastePath;

//------//
// Body //
//------//

/// ## Post Paste Body Inner
///
/// The inner, or raw body of the paste, parsed directly from the client.
#[derive(Deserialize)]
pub struct PostPasteBodyInner {
    /// The name for the paste.
    #[serde(default)]
    name: UndefinedOption<String>,
    /// The expiry time for the paste.
    #[serde(default, rename = "expiry_timestamp")]
    expiry: UndefinedOption<DtUtc>,
    /// The maximum allowed views for the paste.
    #[serde(default)]
    max_views: UndefinedOption<usize>,
    /// The documents attached to the paste.
    documents: Vec<PostPasteDocumentBody>,
}

impl PostPasteBodyInner {
    /// The documents within this paste.
    pub fn documents(&self) -> &[PostPasteDocumentBody] {
        &self.documents
    }

    /// ## Into Parts
    ///
    /// Convert the paste body into its individual parts
    ///
    /// ## Returns
    /// A tuple of the [`PostPasteBody`] and the documents within this paste.
    pub fn into_parts(self) -> (PostPasteBody, Vec<PostPasteDocumentBody>) {
        let body = PostPasteBody {
            name: self.name,
            expiry: self.expiry,
            max_views: self.max_views,
        };

        (body, self.documents)
    }
}

/// ## Post Paste Body
///
/// The paste body extracted from the actual body after parsing.
pub struct PostPasteBody {
    /// The name for the paste.
    name: UndefinedOption<String>,
    /// The expiry time for the paste.
    expiry: UndefinedOption<DtUtc>,
    /// The maximum allowed views for the paste.
    max_views: UndefinedOption<usize>,
}

impl PostPasteBody {
    /// The name for the paste.
    #[inline]
    pub fn name(&self) -> UndefinedOption<&str> {
        self.name.as_deref()
    }

    /// The expiry time for the paste.
    #[inline]
    pub const fn expiry(&self) -> UndefinedOption<DtUtc> {
        self.expiry
    }

    /// The maximum allowed views for the paste.
    #[inline]
    pub const fn max_views(&self) -> UndefinedOption<usize> {
        self.max_views
    }
}

/// ## Post Paste Body
///
/// The paste body extracted from the actual body after parsing.
#[derive(Deserialize)]
pub struct PatchPasteBody {
    /// The name for the paste.
    #[serde(default)]
    name: UndefinedOption<String>,
    /// The expiry time for the paste.
    #[serde(default, rename = "expiry_timestamp")]
    expiry: UndefinedOption<DtUtc>,
    /// The maximum allowed views for the paste.
    #[serde(default)]
    max_views: UndefinedOption<usize>,
    /// The documents attached to the paste.
    #[serde(default)]
    documents: Undefined<Vec<PatchPasteDocumentBody>>,
}

impl PatchPasteBody {
    /// The name for the paste.
    #[inline]
    pub fn name(&self) -> UndefinedOption<&str> {
        self.name.as_deref()
    }

    /// The expiry time for the paste.
    #[inline]
    pub const fn expiry(&self) -> UndefinedOption<DtUtc> {
        self.expiry
    }

    /// The maximum allowed views for the paste.
    #[inline]
    pub const fn max_views(&self) -> UndefinedOption<usize> {
        self.max_views
    }

    /// The documents attached to the paste.
    #[inline]
    pub fn documents(&self) -> Undefined<&[PatchPasteDocumentBody]> {
        self.documents.as_deref()
    }
}

//----------//
// Response //
//----------//

/// ## Response Paste
///
/// The paste returned when requested.
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
    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn id(&self) -> Snowflake {
        self.id
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn creation(&self) -> &DtUtc {
        &self.creation
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn edited(&self) -> Option<&DtUtc> {
        self.edited.as_ref()
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn expiry(&self) -> Option<&DtUtc> {
        self.expiry.as_ref()
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn views(&self) -> usize {
        self.views
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn max_views(&self) -> Option<usize> {
        self.max_views
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn documents(&self) -> &Vec<Document> {
        &self.documents
    }
}

//------------//
// Extractors //
//------------//

/// ## Post Paste Multipart Body
///
/// The multipart extractor for a paste creation.
pub struct PostPasteMultipartBody {
    /// The payload of the multipart body.
    pub payload: PostPasteBody,
    /// The documents attached to the multipart body.
    pub documents: Vec<(PostPasteDocumentBody, String, Mime)>,
}

impl FromRequest<App> for PostPasteMultipartBody {
    type Rejection = RESTError;

    async fn from_request(
        req: axum::extract::Request,
        state: &App,
    ) -> Result<Self, Self::Rejection> {
        let name_regex = Regex::new(r"^files\[(?P<id>[0-9]+)\]$")?;

        let Some(content_type) = req.headers().get(CONTENT_TYPE) else {
            return Err(RESTError::BadRequest(
                "The content type header is expected.".to_string(),
            ));
        };

        let mime: mime::Mime = content_type.to_str()?.parse()?;

        if mime.type_() != mime::MULTIPART || mime.subtype() != mime::FORM_DATA {
            return Err(RESTError::BadRequest(format!(
                "Expected {} as content type.",
                mime::MULTIPART_FORM_DATA
            )));
        }

        let mut multipart = Multipart::from_request(req, state).await?;

        let mut payload: Option<PostPasteBodyInner> = None;
        let mut document_contents = HashMap::new();

        while let Some(field) = multipart.next_field().await? {
            let Some(name) = field.name() else {
                return Err(RESTError::BadRequest(
                    "All multipart fields require a name.".to_string(),
                ));
            };

            let Some(content_type) = field.content_type() else {
                return Err(RESTError::BadRequest(
                    "All multipart fields require a content type.".to_string(),
                ));
            };

            let content_type_mime: mime::Mime = content_type.parse()?;

            if name == "payload" {
                if content_type != mime::APPLICATION_JSON {
                    return Err(RESTError::BadRequest(
                        "Payload must have a content type of application/json".to_string(),
                    ));
                }

                let data = field.bytes().await?;
                let json: PostPasteBodyInner = serde_json::from_slice(&data.to_vec())?;

                let document_ids: Vec<PartialSnowflake> =
                    json.documents().iter().map(|v| *v.id()).collect();

                let document_ids_set: HashSet<PartialSnowflake> =
                    HashSet::from_iter(document_ids.clone().into_iter());
                if document_ids.len() != document_ids_set.len() {
                    return Err(RESTError::BadRequest(
                        "One or more documents provided has the same ID".to_string(),
                    ));
                }

                payload = Some(json);
                continue;
            }

            if let Some(captures) = name_regex.captures(name) {
                if contains_mime(UNSUPPORTED_MIMES, content_type) {
                    return Err(RESTError::BadRequest(format!(
                        "Invalid mime type: {content_type} received for the document: {}",
                        &captures["id"]
                    )));
                }

                let id: PartialSnowflake = (&captures["id"]).try_into()?;

                if document_contents.contains_key(&id) {
                    return Err(RESTError::BadRequest(
                        "A duplicate ID was found in the form data".to_string(),
                    ));
                }

                let data = field.bytes().await?;
                let content = String::from_utf8(data.to_vec())?;

                document_contents.insert(id, (content, content_type_mime));
                continue;
            }

            return Err(RESTError::BadRequest(format!(
                "An unknown multipart item was received: {name}"
            )));
        }

        let Some(payload) = payload else {
            return Err(RESTError::BadRequest(
                "Payload was not found in the form data".to_string(),
            ));
        };

        let (payload, body_documents) = payload.into_parts();

        let mut documents = Vec::new();
        for document in body_documents {
            let Some((content, mime)) = document_contents.remove(document.id()) else {
                return Err(RESTError::BadRequest(format!(
                    "A document with the ID of {} was not found",
                    document.id()
                )));
            };

            document_limits(
                state.config(),
                document.id(),
                Undefined::Some(document.name()),
                Undefined::Some(&content),
            )?;

            documents.push((document, content, mime));
        }

        if document_contents.len() > 0 {
            return Err(RESTError::BadRequest(
                "More files were provided, than listed inside the payload".to_string(),
            ));
        }

        Ok(Self { payload, documents })
    }
}

/// ## Patch Paste Multipart Body
///
/// The multipart extractor for paste modification.
pub struct PatchPasteMultipartBody {
    /// The payload of the multipart body.
    pub payload: PatchPasteBody,
    /// The documents attached to the multipart body.
    pub documents: Undefined<Vec<(PatchPasteDocumentBody, String, Mime)>>,
}

impl PatchPasteMultipartBody {
    /// ## From Json
    ///
    /// Extracts the pastes body via JSON.
    ///
    /// ## Errors
    /// Throws a [`RESTError`] if the body is not valid JSON, or does not parse into the expected data.
    ///
    /// ## Returns
    /// The expected [`PatchPasteMultipartBody`] object.
    pub async fn from_json(req: axum::extract::Request, state: &App) -> Result<Self, RESTError> {
        let bytes = Bytes::from_request(req, state).await?;

        let json: PatchPasteBody = serde_json::from_slice(&bytes.to_vec())?;

        if let Undefined::Some(documents) = json.documents() {
            let document_ids: Vec<PartialSnowflake> = documents.iter().map(|v| *v.id()).collect();

            let document_ids_set: HashSet<PartialSnowflake> =
                HashSet::from_iter(document_ids.clone().into_iter());
            if document_ids.len() != document_ids_set.len() {
                return Err(RESTError::BadRequest(
                    "One or more documents provided has the same ID".to_string(),
                ));
            }
        }

        Ok(Self {
            payload: json,
            documents: Undefined::Undefined,
        })
    }

    /// ## From Multipart
    ///
    /// Extracts the pastes body via multipart.
    ///
    /// ## Errors
    /// Throws a [`RESTError`] if the body is not valid multipart, or does not parse into the expected data.
    ///
    /// ## Returns
    /// The expected [`PatchPasteMultipartBody`] object.
    pub async fn from_multipart(
        req: axum::extract::Request,
        state: &App,
    ) -> Result<Self, RESTError> {
        let name_regex = Regex::new(r"^files\[(?P<id>[0-9]+)\]$")?;

        let mut multipart = Multipart::from_request(req, state).await?;

        let mut payload = None;
        let mut document_contents: Option<HashMap<PartialSnowflake, (String, Mime)>> = None;

        while let Some(field) = multipart.next_field().await? {
            let Some(name) = field.name() else {
                return Err(RESTError::BadRequest(
                    "All multipart fields require a name.".to_string(),
                ));
            };

            let Some(content_type) = field.content_type() else {
                return Err(RESTError::BadRequest(
                    "All multipart fields require a content type.".to_string(),
                ));
            };

            let content_type_mime: mime::Mime = content_type.parse()?;

            if name == "payload" {
                if content_type != mime::APPLICATION_JSON {
                    return Err(RESTError::BadRequest(
                        "Payload must have a content type of application/json".to_string(),
                    ));
                }

                let data = field.bytes().await?;
                let json: PatchPasteBody = serde_json::from_slice(&data.to_vec())?;

                if let Undefined::Some(documents) = json.documents() {
                    let document_ids: Vec<PartialSnowflake> =
                        documents.iter().map(|v| *v.id()).collect();

                    let document_ids_set: HashSet<PartialSnowflake> =
                        HashSet::from_iter(document_ids.clone().into_iter());
                    if document_ids.len() != document_ids_set.len() {
                        return Err(RESTError::BadRequest(
                            "One or more documents provided has the same ID".to_string(),
                        ));
                    }
                }

                payload = Some(json);
                continue;
            }

            if let Some(captures) = name_regex.captures(name) {
                if contains_mime(UNSUPPORTED_MIMES, content_type) {
                    return Err(RESTError::BadRequest(format!(
                        "Invalid mime type received for a document: {content_type}"
                    )));
                }

                let id: PartialSnowflake = (&captures["id"]).try_into()?;

                if let Some(document_contents) = &document_contents {
                    if document_contents.contains_key(&id) {
                        return Err(RESTError::BadRequest(
                            "A duplicate ID was found in the form data".to_string(),
                        ));
                    }
                }

                let data = field.bytes().await?;
                let content = String::from_utf8(data.to_vec())?;

                let document_contents = document_contents.get_or_insert_default();

                document_contents.insert(id, (content, content_type_mime));
                continue;
            }

            return Err(RESTError::BadRequest(format!(
                "An unknown multipart item was received: {name}"
            )));
        }

        let Some(payload) = payload else {
            return Err(RESTError::BadRequest(
                "Payload was not found in the form data".to_string(),
            ));
        };

        let (new_payload, documents) = match payload {
            PatchPasteBody {
                documents: Undefined::Some(body_documents),
                ..
            } => {
                let mut docs_map: HashMap<PartialSnowflake, PatchPasteDocumentBody> =
                    body_documents.into_iter().map(|d| (*d.id(), d)).collect();

                let mut documents_inner = Vec::new();

                if let Some(document_contents) = document_contents {
                    for (id, (content, mime)) in document_contents {
                        let Some(body) = docs_map.remove(&id) else {
                            return Err(RESTError::BadRequest(format!(
                                "A document with the ID of {} was not found",
                                id
                            )));
                        };

                        document_limits(
                            state.config(),
                            &id,
                            body.name(),
                            Undefined::Some(&content),
                        )?;

                        documents_inner.push((body, content, mime));
                    }
                }

                let remaining_documents: Vec<PatchPasteDocumentBody> =
                    docs_map.into_values().collect();

                let new_payload = PatchPasteBody {
                    documents: if remaining_documents.is_empty() {
                        Undefined::Undefined
                    } else {
                        Undefined::Some(remaining_documents)
                    },
                    ..payload
                };

                (new_payload, Undefined::Some(documents_inner))
            }

            payload => (payload, Undefined::Undefined),
        };

        Ok(Self {
            payload: new_payload,
            documents,
        })
    }
}

impl FromRequest<App> for PatchPasteMultipartBody {
    type Rejection = RESTError;

    async fn from_request(
        req: axum::extract::Request,
        state: &App,
    ) -> Result<Self, Self::Rejection> {
        let Some(content_type) = req.headers().get(CONTENT_TYPE) else {
            return Err(RESTError::BadRequest(
                "The content type header is expected.".to_string(),
            ));
        };

        let mime: mime::Mime = content_type.to_str()?.parse()?;

        if mime.type_() == mime::APPLICATION && mime.subtype() == mime::JSON {
            Self::from_json(req, state).await
        } else if mime.type_() == mime::MULTIPART && mime.subtype() == mime::FORM_DATA {
            Self::from_multipart(req, state).await
        } else {
            return Err(RESTError::BadRequest(format!(
                "Expected {} or {} as content type.",
                mime::APPLICATION_JSON,
                mime::MULTIPART_FORM_DATA
            )));
        }
    }
}
