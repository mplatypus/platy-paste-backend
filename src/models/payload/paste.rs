//------//
// Path //
//------//

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
        error::AppError,
        paste::Paste,
        payload::document::PasteDocumentBody,
        snowflake::Snowflake,
        undefined::{Undefined, UndefinedOption},
    },
};

#[derive(Deserialize)]
pub struct PastePath {
    /// The paste ID.
    paste_id: Snowflake,
}

impl PastePath {
    #[inline]
    pub const fn paste_id(&self) -> &Snowflake {
        &self.paste_id
    }
}

pub type GetPastePath = PastePath;

pub type PatchPastePath = PastePath;

pub type DeletePastePath = PastePath;

//------//
// Body //
//------//

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
    documents: Vec<PasteDocumentBody>,
}

impl PostPasteBodyInner {
    pub fn documents(&self) -> &[PasteDocumentBody] {
        &self.documents
    }

    pub fn into_parts(self) -> (PostPasteBody, Vec<PasteDocumentBody>) {
        let body = PostPasteBody {
            name: self.name,
            expiry: self.expiry,
            max_views: self.max_views,
        };

        (body, self.documents)
    }
}

pub struct PostPasteBody {
    /// The name for the paste.
    name: UndefinedOption<String>,
    /// The expiry time for the paste.
    expiry: UndefinedOption<DtUtc>,
    /// The maximum allowed views for the paste.
    max_views: UndefinedOption<usize>,
}

impl PostPasteBody {
    #[inline]
    pub fn name(&self) -> UndefinedOption<&str> {
        self.name.as_deref()
    }

    #[inline]
    pub const fn expiry(&self) -> UndefinedOption<DtUtc> {
        self.expiry
    }

    #[inline]
    pub const fn max_views(&self) -> UndefinedOption<usize> {
        self.max_views
    }
}

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
    documents: Undefined<Vec<PasteDocumentBody>>,
}

impl PatchPasteBody {
    #[inline]
    pub fn name(&self) -> UndefinedOption<&str> {
        self.name.as_deref()
    }

    #[inline]
    pub const fn expiry(&self) -> UndefinedOption<DtUtc> {
        self.expiry
    }

    #[inline]
    pub const fn max_views(&self) -> UndefinedOption<usize> {
        self.max_views
    }

    #[inline]
    pub fn documents(&self) -> Undefined<&[PasteDocumentBody]> {
        self.documents.as_deref()
    }
}

//----------//
// Response //
//----------//

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

//------------//
// Extractors //
//------------//

pub struct PostPasteMultipartBody {
    pub payload: PostPasteBody,
    pub documents: Vec<(PasteDocumentBody, String, Mime)>,
}

impl FromRequest<App> for PostPasteMultipartBody {
    type Rejection = AppError;

    async fn from_request(
        req: axum::extract::Request,
        state: &App,
    ) -> Result<Self, Self::Rejection> {
        let name_regex = Regex::new(r"^files\[(?P<id>[0-9]+)\]$")?;

        let Some(content_type) = req.headers().get(CONTENT_TYPE) else {
            return Err(AppError::BadRequest(
                "The content type header is expected.".to_string(),
            ));
        };

        let mime: mime::Mime = content_type.to_str()?.parse()?;

        if mime.type_() != mime::MULTIPART || mime.subtype() != mime::FORM_DATA {
            return Err(AppError::BadRequest(format!(
                "Expected {} as content type.",
                mime::MULTIPART_FORM_DATA
            )));
        }

        let mut multipart = Multipart::from_request(req, state).await?;

        let mut payload: Option<PostPasteBodyInner> = None;
        let mut document_contents = HashMap::new();

        while let Some(field) = multipart.next_field().await? {
            let Some(name) = field.name() else {
                return Err(AppError::BadRequest(
                    "All multipart fields require a name.".to_string(),
                ));
            };

            let Some(content_type) = field.content_type() else {
                return Err(AppError::BadRequest(
                    "All multipart fields require a content type.".to_string(),
                ));
            };

            let content_type_mime: mime::Mime = content_type.parse()?;

            if name == "payload" {
                if content_type != mime::APPLICATION_JSON {
                    return Err(AppError::BadRequest(
                        "Payload must be of the content type application/json".to_string(),
                    ));
                }

                let data = field.bytes().await?;
                let json: PostPasteBodyInner = serde_json::from_slice(&data.to_vec())?;

                let document_ids: Vec<usize> = json.documents().iter().map(|v| v.id()).collect();

                let document_ids_set: HashSet<usize> =
                    HashSet::from_iter(document_ids.clone().into_iter());
                if document_ids.len() != document_ids_set.len() {
                    return Err(AppError::BadRequest(
                        "One or more documents provided has the same ID".to_string(),
                    ));
                }

                payload = Some(json);
                continue;
            }

            if let Some(captures) = name_regex.captures(name) {
                if contains_mime(UNSUPPORTED_MIMES, content_type) {
                    return Err(AppError::BadRequest(format!(
                        "Invalid mime type received for a document: {content_type}"
                    )));
                }

                let id: usize = (&captures["id"]).parse()?;

                if document_contents.contains_key(&id) {
                    return Err(AppError::BadRequest(
                        "A duplicate ID was found in the form data".to_string(),
                    ));
                }

                let data = field.bytes().await?;
                let content = String::from_utf8(data.to_vec())?;

                document_contents.insert(id, (content, content_type_mime));
                continue;
            }

            return Err(AppError::BadRequest(format!(
                "An unknown multipart item was received: {name}"
            )));
        }

        let Some(payload) = payload else {
            return Err(AppError::BadRequest(
                "Payload was not found in the form data".to_string(),
            ));
        };

        let (payload, body_documents) = payload.into_parts();

        let mut documents = Vec::new();
        for document in body_documents {
            let Some((content, mime)) = document_contents.remove(&document.id()) else {
                return Err(AppError::BadRequest(format!(
                    "A document with the ID of {} was not found",
                    document.id()
                )));
            };

            documents.push((document, content, mime));
        }

        if document_contents.len() > 0 {
            return Err(AppError::BadRequest(
                "More files were provided, than listed inside the payload".to_string(),
            ));
        }

        Ok(Self { payload, documents })
    }
}

pub struct PatchPasteMultipartBody {
    pub payload: PatchPasteBody,
    pub documents: Undefined<Vec<(PasteDocumentBody, String, Mime)>>,
}

impl PatchPasteMultipartBody {
    pub async fn from_json(req: axum::extract::Request, state: &App) -> Result<Self, AppError> {
        let bytes = Bytes::from_request(req, state).await?;
        let json: PatchPasteBody = serde_json::from_slice(&bytes.to_vec())?;
        if let Undefined::Some(documents) = json.documents() {
            let document_ids: Vec<usize> = documents.iter().map(|v| v.id()).collect();

            let document_ids_set: HashSet<usize> =
                HashSet::from_iter(document_ids.clone().into_iter());
            if document_ids.len() != document_ids_set.len() {
                return Err(AppError::BadRequest(
                    "One or more documents provided has the same ID".to_string(),
                ));
            }
        }

        Ok(Self {
            payload: json,
            documents: Undefined::Undefined,
        })
    }

    pub async fn from_multipart(
        req: axum::extract::Request,
        state: &App,
    ) -> Result<Self, AppError> {
        let name_regex = Regex::new(r"^files\[(?P<id>[0-9]+)\]$")?;

        let mut multipart = Multipart::from_request(req, state).await?;

        let mut payload = None;
        let mut document_contents: Option<HashMap<usize, (String, Mime)>> = None;

        while let Some(field) = multipart.next_field().await? {
            let Some(name) = field.name() else {
                return Err(AppError::BadRequest(
                    "All multipart fields require a name.".to_string(),
                ));
            };

            let Some(content_type) = field.content_type() else {
                return Err(AppError::BadRequest(
                    "All multipart fields require a content type.".to_string(),
                ));
            };

            let content_type_mime: mime::Mime = content_type.parse()?;

            if name == "payload" {
                if content_type != mime::APPLICATION_JSON {
                    return Err(AppError::BadRequest(
                        "Payload must be of the content type application/json".to_string(),
                    ));
                }

                let data = field.bytes().await?;
                let json: PatchPasteBody = serde_json::from_slice(&data.to_vec())?;

                if let Undefined::Some(documents) = json.documents() {
                    let document_ids: Vec<usize> = documents.iter().map(|v| v.id()).collect();

                    let document_ids_set: HashSet<usize> =
                        HashSet::from_iter(document_ids.clone().into_iter());
                    if document_ids.len() != document_ids_set.len() {
                        return Err(AppError::BadRequest(
                            "One or more documents provided has the same ID".to_string(),
                        ));
                    }
                }

                payload = Some(json);
                continue;
            }

            if let Some(captures) = name_regex.captures(name) {
                if contains_mime(UNSUPPORTED_MIMES, content_type) {
                    return Err(AppError::BadRequest(format!(
                        "Invalid mime type received for a document: {content_type}"
                    )));
                }

                let id: usize = (&captures["id"]).parse()?;

                if let Some(document_contents) = &document_contents {
                    if document_contents.contains_key(&id) {
                        return Err(AppError::BadRequest(
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

            return Err(AppError::BadRequest(format!(
                "An unknown multipart item was received: {name}"
            )));
        }

        let Some(payload) = payload else {
            return Err(AppError::BadRequest(
                "Payload was not found in the form data".to_string(),
            ));
        };

        let (new_payload, documents) = match payload {
            PatchPasteBody {
                documents: Undefined::Some(body_documents),
                ..
            } => {
                let mut docs_map: HashMap<usize, PasteDocumentBody> =
                    body_documents.into_iter().map(|d| (d.id(), d)).collect();

                let mut documents_inner = Vec::new();

                if let Some(document_contents) = document_contents {
                    for (id, (content, mime)) in document_contents {
                        let Some(body) = docs_map.remove(&id) else {
                            return Err(AppError::BadRequest(format!(
                                "A document with the ID of {} was not found",
                                id
                            )));
                        };

                        document_limits(state.config(), body.name(), &content)?;
                        documents_inner.push((body, content, mime));
                    }
                }

                let remaining_documents: Vec<PasteDocumentBody> = docs_map.into_values().collect();

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
    type Rejection = AppError;

    async fn from_request(
        req: axum::extract::Request,
        state: &App,
    ) -> Result<Self, Self::Rejection> {
        let Some(content_type) = req.headers().get(CONTENT_TYPE) else {
            return Err(AppError::BadRequest(
                "The content type header is expected.".to_string(),
            ));
        };

        let mime: mime::Mime = content_type.to_str()?.parse()?;

        if mime.type_() == mime::APPLICATION && mime.subtype() == mime::JSON {
            Self::from_json(req, state).await
        } else if mime.type_() == mime::MULTIPART && mime.subtype() == mime::FORM_DATA {
            Self::from_multipart(req, state).await
        } else {
            return Err(AppError::BadRequest(format!(
                "Expected {} or {} as content type.",
                mime::APPLICATION_JSON,
                mime::MULTIPART_FORM_DATA
            )));
        }
    }
}
