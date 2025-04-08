use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use axum_extra::{
    TypedHeader,
    headers::{self, ContentType, Header},
};
use bytes::Bytes;
use http::{HeaderName, HeaderValue, StatusCode};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::{
    app::{application::App, config::Config},
    models::{
        authentication::Token,
        document::{DEFAULT_MIME, Document, UNSUPPORTED_MIMES, contains_mime},
        error::{AppError, AuthError},
        paste::Paste,
        payload::{PatchDocumentQuery, PostDocumentQuery, ResponseDocument},
        snowflake::Snowflake,
    },
};

pub fn generate_router(config: &Config) -> Router<App> {
    let global_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.global_document_rate_limiter())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build global document limiter."),
        ),
    };

    let get_document_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.get_document_rate_limiter())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build get document limiter."),
        ),
    };

    let post_document_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.post_document_rate_limiter())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build post document limiter."),
        ),
    };

    let patch_document_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.patch_document_rate_limiter())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build patch document limiter."),
        ),
    };

    let delete_document_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.delete_document_rate_limiter())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build delete document limiter."),
        ),
    };

    Router::new()
        .route(
            "/pastes/{paste_id}/documents/{document_id}",
            get(get_document).layer(get_document_limiter),
        )
        .route(
            "/pastes/{paste_id}/documents",
            post(post_document).layer(post_document_limiter),
        )
        .route(
            "/pastes/{paste_id}/documents/{document_id}",
            patch(patch_document).layer(patch_document_limiter),
        )
        .route(
            "/pastes/{paste_id}/documents/{document_id}",
            delete(delete_document).layer(delete_document_limiter),
        )
        .layer(global_limiter)
        .layer(DefaultBodyLimit::max(
            config.global_paste_total_document_size_limit() * 1024 * 1024,
        ))
}

/// Get Document.
///
/// Get an existing document.
///
/// ## Path
///
/// - `paste_id` - The pastes ID.
/// - `document_id` - The documents ID.
///
/// ## Returns
///
/// - `404` - The paste or document was not found.
/// - `200` - The [`ResponseDocument`] object.
async fn get_document(
    State(app): State<App>,
    Path(paste_id): Path<Snowflake>,
) -> Result<Response, AppError> {
    let document = Document::fetch(&app.database, paste_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Document not found.".to_string()))?;

    let data = app.s3.fetch_document(document.generate_path()).await?;
    let d: &str = &String::from_utf8_lossy(&data);

    let response_document = ResponseDocument::from_document(document, Some(d.to_string()));

    Ok((StatusCode::OK, Json(response_document)).into_response())
}

/// Post Document.
///
/// Adds a document to an existing paste.
///
/// ## Query
///
/// References: [`PostDocumentQuery`]
///
/// - `content` - Whether to return the content.
///
/// ## Body
///
/// The body of the document, will be the content of the document.
///
/// ## Returns
///
/// - `401` - Invalid token and/or paste ID.
/// - `400` - The body and/or documents are invalid.
/// - `200` - The [`ResponseDocument`].
/// - `204` - If content is set to false.
async fn post_document(
    State(app): State<App>,
    Path(paste_id): Path<Snowflake>,
    TypedHeader(content_disposition): TypedHeader<ContentDisposition>,
    content_type: Option<TypedHeader<ContentType>>,
    Query(query): Query<PostDocumentQuery>,
    token: Token,
    body: Bytes,
) -> Result<Response, AppError> {
    if token.paste_id() != paste_id {
        return Err(AppError::Authentication(AuthError::ForbiddenPasteId));
    }

    let document_type = {
        if let Some(TypedHeader(content_type)) = content_type {
            if contains_mime(UNSUPPORTED_MIMES, &content_type.to_string()) {
                return Err(AppError::BadRequest(format!(
                    "Invalid mime type received: {content_type}"
                )));
            }

            content_type.to_string()
        } else {
            DEFAULT_MIME.to_string()
        }
    };

    let mut paste = Paste::fetch(&app.database, paste_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Paste not found.".to_string()))?;

    let total_document_count =
        Document::fetch_total_document_count(&app.database, paste.id).await?;

    if app.config.global_paste_total_document_count() > (total_document_count + 1) {
        return Err(AppError::BadRequest(
            "The new document exceeds the pastes total document limit.".to_string(),
        ));
    }

    let total_document_size = Document::fetch_total_document_size(&app.database, paste_id).await?;

    if app.config.global_paste_total_document_size_limit() >= (total_document_size + body.len()) {
        return Err(AppError::BadRequest(
            "The new content exceeds the total document limit.".to_string(),
        ));
    }

    let document = Document::new(
        Snowflake::generate()?,
        paste.id,
        document_type,
        content_disposition
            .filename()
            .unwrap_or_else(|| "unknown".to_string()),
        body.len(),
    );

    let mut transaction = app.database.pool().begin().await?;

    paste.set_edited();

    paste.update(&mut transaction).await?;

    document.update(&mut transaction).await?;

    app.s3.delete_document(document.generate_path()).await?;

    app.s3.create_document(&document, body.clone()).await?;

    transaction.commit().await?;

    let content = {
        if query.include_content {
            let d: &str = &String::from_utf8_lossy(&body);
            Some(d.to_string())
        } else {
            None
        }
    };

    let document_response = ResponseDocument::from_document(document, content);

    Ok((StatusCode::OK, Json(document_response)).into_response())
}

/// Patch Document.
///
/// Adds a document to an existing paste.
///
/// ## Path
///
/// - `paste_id` - The paste ID of the document.
/// - `document_id` - The document ID to edit.
///
/// ## Query
///
/// References: [`PatchDocumentQuery`]
///
/// - `content` - Whether to return the content.
///
/// ## Body
///
/// The body of the document, will be the content of the document.
///
/// ## Returns
///
/// - `401` - Invalid token and/or paste ID.
/// - `400` - The body and/or documents are invalid.
/// - `200` - The [`ResponseDocument`].
/// - `204` - If content is set to false.
async fn patch_document(
    State(app): State<App>,
    Path((paste_id, document_id)): Path<(Snowflake, Snowflake)>,
    TypedHeader(content_disposition): TypedHeader<ContentDisposition>,
    content_type: Option<TypedHeader<ContentType>>,
    Query(query): Query<PatchDocumentQuery>,
    token: Token,
    body: Bytes,
) -> Result<Response, AppError> {
    if token.paste_id() != paste_id {
        return Err(AppError::Authentication(AuthError::ForbiddenPasteId));
    }

    let document_type = {
        if let Some(TypedHeader(content_type)) = content_type {
            if contains_mime(UNSUPPORTED_MIMES, &content_type.to_string()) {
                return Err(AppError::BadRequest(format!(
                    "Invalid mime type received: {content_type}"
                )));
            }

            content_type.to_string()
        } else {
            DEFAULT_MIME.to_string()
        }
    };

    let mut paste = Paste::fetch(&app.database, paste_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Paste not found.".to_string()))?;

    let total_document_size = Document::fetch_total_document_size(&app.database, paste_id).await?;

    if app.config.global_paste_total_document_size_limit() >= (total_document_size + body.len()) {
        return Err(AppError::BadRequest(
            "The new content exceeds the total document limit.".to_string(),
        ));
    }

    let mut document = Document::fetch(&app.database, document_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Document not found.".to_string()))?;

    let mut transaction = app.database.pool().begin().await?;

    paste.set_edited();

    paste.update(&mut transaction).await?;

    document.set_document_type(document_type);

    if let Some(filename) = content_disposition.filename() {
        document.set_name(filename);
    }

    document.update(&mut transaction).await?;

    app.s3.delete_document(document.generate_path()).await?;

    app.s3.create_document(&document, body.clone()).await?;

    transaction.commit().await?;

    let content = {
        if query.include_content {
            let d: &str = &String::from_utf8_lossy(&body);
            Some(d.to_string())
        } else {
            None
        }
    };

    let paste_response = ResponseDocument::from_document(document, content);

    Ok((StatusCode::OK, Json(paste_response)).into_response())
}

/// Patch Document.
///
/// Adds a document to an existing paste.
///
/// ## Path
///
/// - `paste_id` - The paste ID of the document.
/// - `document_id` - The document ID to delete.
///
/// ## Returns
///
/// - `401` - Invalid token and/or paste ID.
/// - `400` - A paste must have at least one document.
/// - `204` - Successful deletion of the document.
async fn delete_document(
    State(app): State<App>,
    Path((paste_id, document_id)): Path<(Snowflake, Snowflake)>,
    token: Token,
) -> Result<Response, AppError> {
    if token.paste_id() != paste_id {
        return Err(AppError::Authentication(AuthError::ForbiddenPasteId));
    }

    let total_document_count =
        Document::fetch_total_document_count(&app.database, paste_id).await?;

    if total_document_count == 1 {
        return Err(AppError::BadRequest(
            "A paste must have at least one document".to_string(),
        ));
    }

    Document::delete(&app.database, document_id).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Clone)]
struct ContentDisposition {
    disposition: String,
    filename: Option<String>,
}

impl ContentDisposition {
    #[allow(dead_code)]
    pub fn disposition(&self) -> String {
        self.disposition.clone()
    }

    pub fn filename(&self) -> Option<String> {
        self.filename.clone()
    }
}

impl Header for ContentDisposition {
    fn name() -> &'static HeaderName {
        &axum::http::header::CONTENT_DISPOSITION
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;

        let s = value.to_str().map_err(|_| headers::Error::invalid())?;

        let mut disposition = String::new();
        let mut filename = None;

        for (i, part) in s.split(';').enumerate() {
            let part = part.trim();
            if i == 0 {
                disposition = part.to_string();
            } else if let Some(rest) = part.strip_prefix("filename=") {
                filename = Some(rest.trim_matches('"').to_string());
            }
        }

        Ok(Self {
            disposition,
            filename,
        })
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let mut parts = vec![self.disposition.clone()];

        if let Some(filename) = &self.filename {
            parts.push(format!("filename=\"{filename}\""));
        }

        let full = parts.join("; ");

        if let Ok(header_value) = HeaderValue::from_str(&full) {
            values.extend(std::iter::once(header_value));
        }
    }
}
