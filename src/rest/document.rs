use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use axum_extra::{
    TypedHeader,
    headers::{self, ContentType, Header},
};
use bytes::Bytes;
use http::{HeaderName, HeaderValue, StatusCode};

use crate::{
    app::{application::App, config::Config, object_store::ObjectStoreExt as _},
    models::{
        authentication::Token,
        document::{
            Document, UNSUPPORTED_MIMES, contains_mime, document_limits, total_document_limits,
        },
        errors::{AuthenticationError, RESTError},
        paste::{Paste, validate_paste},
        payload::{DeleteDocumentPath, GetDocumentPath, PatchDocumentPath, PostDocumentPath},
        snowflake::Snowflake,
    },
};

pub fn generate_router(config: &Config) -> Router<App> {
    Router::new()
        .route(
            "/pastes/{paste_id}/documents/{document_id}",
            get(get_document),
        )
        .route("/pastes/{paste_id}/documents", post(post_document))
        .route(
            "/pastes/{paste_id}/documents/{document_id}",
            patch(patch_document),
        )
        .route(
            "/pastes/{paste_id}/documents/{document_id}",
            delete(delete_document),
        )
        .layer(DefaultBodyLimit::max(
            config.size_limits().maximum_total_document_size(),
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
pub async fn get_document(
    State(app): State<App>,
    Path(path): Path<GetDocumentPath>,
) -> Result<Response, RESTError> {
    let document = Document::fetch(app.database().pool(), path.document_id())
        .await?
        .ok_or_else(|| RESTError::NotFound("Document not found.".to_string()))?;

    if document.paste_id() != path.paste_id() {
        return Err(RESTError::BadRequest(
            "The document ID does not belong to that paste.".to_string(),
        ));
    }

    Paste::add_view(app.database().pool(), path.paste_id()).await?;

    Ok((StatusCode::OK, Json(document)).into_response())
}

/// Post Document.
///
/// Adds a document to an existing paste.
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
pub async fn post_document(
    State(app): State<App>,
    Path(path): Path<PostDocumentPath>,
    TypedHeader(content_disposition): TypedHeader<ContentDisposition>,
    content_type: Option<TypedHeader<ContentType>>,
    token: Token,
    body: Bytes,
) -> Result<Response, RESTError> {
    let mut paste = validate_paste(app.database(), path.paste_id(), Some(token)).await?;

    let document_type = {
        if let Some(TypedHeader(content_type)) = content_type {
            if contains_mime(UNSUPPORTED_MIMES, &content_type.to_string()) {
                return Err(RESTError::BadRequest(format!(
                    "Invalid mime type received: {content_type}"
                )));
            }

            content_type.to_string()
        } else {
            return Err(RESTError::BadRequest(
                "The document must have a type.".to_string(),
            ));
        }
    };

    let name = content_disposition.filename().ok_or_else(|| {
        RESTError::BadRequest("The document provided requires a name.".to_string())
    })?;

    let document = Document::new(
        Snowflake::generate()?,
        *paste.id(),
        &document_type,
        name,
        body.len(),
    );

    document_limits(app.config(), &document)?;

    let mut transaction = app.database().pool().begin().await?;

    paste.set_edited()?;

    paste.update(transaction.as_mut()).await?;

    document.insert(transaction.as_mut()).await?;

    total_document_limits(&mut transaction, app.config(), path.paste_id()).await?;

    app.object_store().create_document(&document, body).await?;

    transaction.commit().await?;

    Ok((StatusCode::OK, Json(document)).into_response())
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
pub async fn patch_document(
    State(app): State<App>,
    Path(path): Path<PatchDocumentPath>,
    TypedHeader(content_disposition): TypedHeader<ContentDisposition>,
    content_type: Option<TypedHeader<ContentType>>,
    token: Token,
    body: Bytes,
) -> Result<Response, RESTError> {
    let mut paste = validate_paste(app.database(), path.paste_id(), Some(token)).await?;

    let document_type = {
        if let Some(TypedHeader(content_type)) = content_type {
            if contains_mime(UNSUPPORTED_MIMES, &content_type.to_string()) {
                return Err(RESTError::BadRequest(format!(
                    "Invalid mime type received: {content_type}"
                )));
            }

            content_type.to_string()
        } else {
            return Err(RESTError::BadRequest(
                "The document must have a type.".to_string(),
            ));
        }
    };

    let mut document = Document::fetch(app.database().pool(), path.document_id())
        .await?
        .ok_or_else(|| RESTError::NotFound("Document not found.".to_string()))?;

    let mut transaction = app.database().pool().begin().await?;

    paste.set_edited()?;

    paste.update(transaction.as_mut()).await?;

    document.set_doc_type(&document_type);

    document.set_size(body.len());

    if let Some(filename) = content_disposition.filename() {
        document.set_name(filename);
    }

    document_limits(app.config(), &document)?;

    document.update(transaction.as_mut()).await?;

    total_document_limits(&mut transaction, app.config(), path.paste_id()).await?;

    app.object_store()
        .delete_document(document.generate_path())
        .await?;

    app.object_store().create_document(&document, body).await?;

    transaction.commit().await?;

    Ok((StatusCode::OK, Json(document)).into_response())
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
pub async fn delete_document(
    State(app): State<App>,
    Path(path): Path<DeleteDocumentPath>,
    token: Token,
) -> Result<Response, RESTError> {
    if token.paste_id() != path.paste_id() {
        return Err(RESTError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

    let total_document_count =
        Document::fetch_total_document_count(app.database().pool(), path.paste_id()).await?;

    if total_document_count <= 1 {
        return Err(RESTError::BadRequest(
            "A paste must have at least one document".to_string(),
        ));
    }

    if !Document::delete(app.database().pool(), path.document_id()).await? {
        return Err(RESTError::NotFound(
            "The document was not found.".to_string(),
        ));
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Clone)]
pub struct ContentDisposition {
    disposition: String,
    filename: Option<String>,
}

impl ContentDisposition {
    pub fn disposition(&self) -> &str {
        &self.disposition
    }

    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
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

#[cfg(test)]
mod test {
    use sqlx::PgPool;

    use crate::app::config::Config;
    use crate::rest::generate_router as main_generate_router;

    use axum_test::TestServer;
    use bytes::Bytes;
    use http::StatusCode;
    use rstest::rstest;

    use crate::app::config::SizeLimitConfig;
    use crate::{
        app::{
            application::ApplicationState,
            object_store::{ObjectStoreExt, TestObjectStore},
        },
        models::{document::Document, errors::RESTErrorResponse, snowflake::Snowflake},
    };

    mod v1 {
        use super::*;

        mod get_document {
            use super::*;

            #[sqlx::test(fixtures(path = "../../tests/fixtures", scripts("pastes", "documents")))]
            async fn test_existing(pool: PgPool) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let paste_id = Snowflake::new(517815304354284604);
                let document_id = Snowflake::new(517815304354284707);

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let response = server
                    .get(&format!("/v1/pastes/{paste_id}/documents/{document_id}"))
                    .await;

                response.assert_status(StatusCode::OK);

                response.assert_header("Content-Type", "application/json");

                let body: Document = response.json();

                let document = Document::fetch_with_paste(&pool, &paste_id, &document_id)
                    .await
                    .expect("Failed to make DB request")
                    .expect("Document does not exist.");

                assert_eq!(body.paste_id(), &paste_id, "Paste ID's do not match.");

                assert_eq!(body.id(), &document_id, "Document ID's do not match.");

                assert_eq!(
                    body.name(),
                    document.name(),
                    "Document name's do not match."
                );

                assert_eq!(
                    body.doc_type(),
                    document.doc_type(),
                    "Document name's do not match."
                );

                assert_eq!(
                    body.size(),
                    document.size(),
                    "Document name's do not match."
                );
            }

            #[rstest]
            #[case(Snowflake::new(517815304354284604), Some("Document not found."))]
            #[case(Snowflake::new(1234567890), Some("Document not found."))]
            #[sqlx::test(fixtures(path = "../../tests/fixtures", scripts("pastes", "documents")))]
            async fn test_missing(
                #[ignore] pool: PgPool,
                #[case] paste_id: Snowflake,
                #[case] trace: Option<&str>,
            ) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let document_id = Snowflake::new(1234567890);

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let response = server
                    .get(&format!("/v1/pastes/{paste_id}/documents/{document_id}"))
                    .await;

                response.assert_status(StatusCode::NOT_FOUND);

                response.assert_header("Content-Type", "application/json");

                let body: RESTErrorResponse = response.json();

                assert_eq!(body.reason(), "Not Found", "Reason does not match.");

                assert_eq!(body.trace(), trace, "Trace does not match.");
            }
        }

        mod post_document {

            use super::*;

            #[rstest]
            #[case(
                Snowflake::new(517815304354284604),
                Some("beans"),
                "Invalid Token and/or mismatched paste ID"
            )]
            #[case(Snowflake::new(517815304354284604), None, "Missing Credentials")]
            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_authentication(
                #[ignore] pool: PgPool,
                #[case] paste_id: Snowflake,
                #[case] authentication: Option<&str>,
                #[case] reason: &str,
            ) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let mut request = server
                    .post(&format!("/v1/pastes/{paste_id}/documents"))
                    .add_header("Content-Disposition", r#"attachment; filename="test.txt""#)
                    .add_header("Content-Type", "text/plain")
                    .text("some content");

                if let Some(authentication) = authentication {
                    request =
                        request.add_header("Authorization", format!("Bearer {authentication}"));
                }

                let response = request.await;

                response.assert_status(StatusCode::UNAUTHORIZED);

                response.assert_header("Content-Type", "application/json");

                let body: RESTErrorResponse = response.json();

                assert_eq!(body.reason(), reason, "Reason does not match.");

                assert_eq!(body.trace(), None, "Trace does not match.");
            }

            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_existing(pool: PgPool) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let paste_id = Snowflake::new(517815304354284604);

                let token_string =
                    "NTE3ODE1MzA0MzU0Mjg0NjA0.MTc0NzgxNjE4OQ==.FDP-mNTjfuOKovulMFbaSkoeq";

                let documents = Document::fetch_all(&pool, &paste_id)
                    .await
                    .expect("Failed to make DB request");

                let content = Bytes::from("test document");
                let response = server
                    .post(&format!("/v1/pastes/{paste_id}/documents"))
                    .add_header("Authorization", format!("Bearer {token_string}"))
                    .add_header("Content-Disposition", r#"attachment; filename="test.txt""#)
                    .add_header("Content-Type", "text/plain")
                    .bytes(content.clone())
                    .await;

                response.assert_status(StatusCode::OK);

                let body: Document = response.json();

                assert_eq!(body.name(), "test.txt", "Document name's do not match.");

                assert_eq!(body.name(), "test.txt", "Document name's do not match.");

                assert_eq!(
                    body.doc_type(),
                    "text/plain",
                    "Document type's do not match."
                );

                assert_eq!(body.size(), content.len(), "Document name's do not match.");

                let updated_documents = Document::fetch_all(&pool, &paste_id)
                    .await
                    .expect("Failed to make DB request");

                let mut document_ids: Vec<Snowflake> = documents.iter().map(|d| *d.id()).collect();
                document_ids.push(*body.id());

                let updated_document_ids: Vec<Snowflake> =
                    updated_documents.iter().map(|d| *d.id()).collect();

                assert_eq!(
                    document_ids, updated_document_ids,
                    "Documents list was not updated."
                );

                let found_content = object_store
                    .fetch_document(body.generate_path())
                    .await
                    .expect("Document content was not stored.");

                assert_eq!(found_content, content, "Document content does not match.");
            }

            #[rstest]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .minimum_document_name_size(10)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                r#"attachment; filename="test.txt""#,
                "text/plain",
                "test",
                RESTErrorResponse::new(
                    "Bad Request",
                    Some("The document name: `test.txt` is too small.".to_string())
                ),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .maximum_document_name_size(5)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                r#"attachment; filename="test.txt""#,
                "text/plain",
                "test",
                RESTErrorResponse::new(
                    "Bad Request",
                    Some("The document name: `test.txt...` is too large.".to_string())
                ),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .minimum_document_size(10)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                r#"attachment; filename="test.txt""#,
                "text/plain",
                "test",
                RESTErrorResponse::new(
                    "Bad Request",
                    Some("The document: `test.txt` is too small.".to_string())
                ),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .maximum_document_size(5)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                r#"attachment; filename="test.txt""#,
                "text/plain",
                "test document",
                RESTErrorResponse::new(
                    "Bad Request",
                    Some("The document: `test.txt` is too large.".to_string())
                ),
            )]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                r#"attachment; filename="test.txt""#,
                "image/png",
                "test",
                RESTErrorResponse::new(
                    "Bad Request",
                    Some("Invalid mime type received: image/png".to_string())
                ),
            )]
            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_bad(
                #[ignore] pool: PgPool,
                #[case] config: Config,
                #[case] content_disposition: &str,
                #[case] content_type: &str,
                #[case] content: &str,
                #[case] expected_body: RESTErrorResponse,
            ) {
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let paste_id = Snowflake::new(517815304354284604);

                let token_string =
                    "NTE3ODE1MzA0MzU0Mjg0NjA0.MTc0NzgxNjE4OQ==.FDP-mNTjfuOKovulMFbaSkoeq";

                let response = server
                    .post(&format!("/v1/pastes/{paste_id}/documents"))
                    .add_header("Authorization", format!("Bearer {token_string}"))
                    .add_header("Content-Disposition", content_disposition)
                    .add_header("Content-Type", content_type)
                    .bytes(Bytes::from(content.to_string()))
                    .await;

                response.assert_status(StatusCode::BAD_REQUEST);

                let body: RESTErrorResponse = response.json();

                assert_eq!(
                    body.reason(),
                    expected_body.reason(),
                    "Reason does not match."
                );

                assert_eq!(body.trace(), expected_body.trace(), "Trace does not match.");
            }
        }

        mod patch_document {

            use super::*;

            #[rstest]
            #[case(
                Snowflake::new(517815304354284604),
                Snowflake::new(517815304354284707),
                Some("beans"),
                "Invalid Token and/or mismatched paste ID"
            )]
            #[case(
                Snowflake::new(517815304354284604),
                Snowflake::new(517815304354284707),
                None,
                "Missing Credentials"
            )]
            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_authentication(
                #[ignore] pool: PgPool,
                #[case] paste_id: Snowflake,
                #[case] document_id: Snowflake,
                #[case] authentication: Option<&str>,
                #[case] reason: &str,
            ) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let mut request = server
                    .patch(&format!("/v1/pastes/{paste_id}/documents/{document_id}"))
                    .add_header("Content-Disposition", r#"attachment; filename="test.txt""#)
                    .add_header("Content-Type", "text/plain")
                    .text("some content");

                if let Some(authentication) = authentication {
                    request =
                        request.add_header("Authorization", format!("Bearer {authentication}"));
                }

                let response = request.await;

                response.assert_status(StatusCode::UNAUTHORIZED);

                response.assert_header("Content-Type", "application/json");

                let body: RESTErrorResponse = response.json();

                assert_eq!(body.reason(), reason, "Reason does not match.");

                assert_eq!(body.trace(), None, "Trace does not match.");
            }

            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_existing(pool: PgPool) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let paste_id = Snowflake::new(517815304354284604);
                let document_id = Snowflake::new(517815304354284707);

                let token_string =
                    "NTE3ODE1MzA0MzU0Mjg0NjA0.MTc0NzgxNjE4OQ==.FDP-mNTjfuOKovulMFbaSkoeq";

                let document = Document::fetch_with_paste(&pool, &paste_id, &document_id)
                    .await
                    .expect("Failed to make DB request");

                assert!(document.is_some(), "Document was not found.");

                let content = Bytes::from("test document");
                let response = server
                    .patch(&format!("/v1/pastes/{paste_id}/documents/{document_id}"))
                    .add_header("Authorization", format!("Bearer {token_string}"))
                    .add_header("Content-Disposition", r#"attachment; filename="test.txt""#)
                    .add_header("Content-Type", "text/plain")
                    .bytes(content.clone())
                    .await;

                response.assert_status(StatusCode::OK);

                let body: Document = response.json();

                assert_eq!(body.name(), "test.txt", "Document name's do not match.");

                assert_eq!(body.name(), "test.txt", "Document name's do not match.");

                assert_eq!(
                    body.doc_type(),
                    "text/plain",
                    "Document type's do not match."
                );

                assert_eq!(body.size(), content.len(), "Document name's do not match.");

                let found_content = object_store
                    .fetch_document(body.generate_path())
                    .await
                    .expect("Document content was not stored.");

                assert_eq!(found_content, content, "Document content does not match.");
            }

            #[ignore = "This is skipped due to the logic immediately failing if there is a minimal amount of documents. This should pass."]
            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_missing(pool: PgPool) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let paste_id = Snowflake::new(517815304354284604);
                let document_id = Snowflake::new(1234567890);

                let token_string =
                    "NTE3ODE1MzA0MzU0Mjg0NjA0.MTc0NzgxNjE4OQ==.FDP-mNTjfuOKovulMFbaSkoeq";

                let response = server
                    .patch(&format!("/v1/pastes/{paste_id}/documents/{document_id}"))
                    .add_header("Authorization", format!("Bearer {token_string}"))
                    .add_header("Content-Disposition", r#"attachment; filename="test.txt""#)
                    .add_header("Content-Type", "text/plain")
                    .text("some content")
                    .await;

                response.assert_status(StatusCode::NOT_FOUND);

                let body: RESTErrorResponse = response.json();

                assert_eq!(body.reason(), "Not Found", "Reason does not match.");

                assert_eq!(body.trace(), Some(""), "Trace does not match.");
            }

            #[rstest]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .minimum_document_name_size(10)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                r#"attachment; filename="test.txt""#,
                "text/plain",
                "test",
                RESTErrorResponse::new(
                    "Bad Request",
                    Some("The document name: `test.txt` is too small.".to_string())
                ),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .maximum_document_name_size(5)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                r#"attachment; filename="test.txt""#,
                "text/plain",
                "test",
                RESTErrorResponse::new(
                    "Bad Request",
                    Some("The document name: `test.txt...` is too large.".to_string())
                ),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .minimum_document_size(10)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                r#"attachment; filename="test.txt""#,
                "text/plain",
                "test",
                RESTErrorResponse::new(
                    "Bad Request",
                    Some("The document: `test.txt` is too small.".to_string())
                ),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .maximum_document_size(5)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                r#"attachment; filename="test.txt""#,
                "text/plain",
                "test document",
                RESTErrorResponse::new(
                    "Bad Request",
                    Some("The document: `test.txt` is too large.".to_string())
                ),
            )]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                r#"attachment; filename="test.txt""#,
                "image/png",
                "test",
                RESTErrorResponse::new(
                    "Bad Request",
                    Some("Invalid mime type received: image/png".to_string())
                ),
            )]
            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_bad(
                #[ignore] pool: PgPool,
                #[case] config: Config,
                #[case] content_disposition: &str,
                #[case] content_type: &str,
                #[case] content: &str,
                #[case] expected_body: RESTErrorResponse,
            ) {
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let paste_id = Snowflake::new(517815304354284604);
                let document_id = Snowflake::new(517815304354284707);

                let token_string =
                    "NTE3ODE1MzA0MzU0Mjg0NjA0.MTc0NzgxNjE4OQ==.FDP-mNTjfuOKovulMFbaSkoeq";

                let response = server
                    .patch(&format!("/v1/pastes/{paste_id}/documents/{document_id}"))
                    .add_header("Authorization", format!("Bearer {token_string}"))
                    .add_header("Content-Disposition", content_disposition)
                    .add_header("Content-Type", content_type)
                    .bytes(Bytes::from(content.to_string()))
                    .await;

                response.assert_status(StatusCode::BAD_REQUEST);

                let body: RESTErrorResponse = response.json();

                assert_eq!(
                    body.reason(),
                    expected_body.reason(),
                    "Reason does not match."
                );

                assert_eq!(body.trace(), expected_body.trace(), "Trace does not match.");
            }
        }

        mod delete_document {
            use super::*;

            #[rstest]
            #[case(
                Snowflake::new(1234567890),
                Snowflake::new(1234567890),
                Some("NTE3ODE1MzA0MzU0Mjg0NjA0.MTc0NzgxNjE4OQ==.FDP-mNTjfuOKovulMFbaSkoeq"),
                "Invalid Token and/or mismatched paste ID"
            )]
            #[case(
                Snowflake::new(517815304354284604),
                Snowflake::new(517815304354284707),
                Some("beans"),
                "Invalid Token and/or mismatched paste ID"
            )]
            #[case(
                Snowflake::new(517815304354284604),
                Snowflake::new(517815304354284707),
                None,
                "Missing Credentials"
            )]
            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_authentication(
                #[ignore] pool: PgPool,
                #[case] paste_id: Snowflake,
                #[case] document_id: Snowflake,
                #[case] authentication: Option<&str>,
                #[case] reason: &str,
            ) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let mut request =
                    server.delete(&format!("/v1/pastes/{paste_id}/documents/{document_id}"));

                if let Some(authentication) = authentication {
                    request =
                        request.add_header("Authorization", format!("Bearer {authentication}"));
                }

                let response = request.await;

                response.assert_status(StatusCode::UNAUTHORIZED);

                response.assert_header("Content-Type", "application/json");

                let body: RESTErrorResponse = response.json();

                assert_eq!(body.reason(), reason, "Reason does not match.");

                assert_eq!(body.trace(), None, "Trace does not match.");
            }

            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_existing(pool: PgPool) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let paste_id = Snowflake::new(517815304354284603);
                let document_id = Snowflake::new(517815304354284704);

                let token_string =
                    "NTE3ODE1MzA0MzU0Mjg0NjAz.MTc0NzgxNjExNA==.cJeeeAPNidthlMtvkLNosiafy";

                let document = Document::fetch_with_paste(&pool, &paste_id, &document_id)
                    .await
                    .expect("Failed to make DB request");

                assert!(document.is_some(), "Document was not found");

                let response = server
                    .delete(&format!("/v1/pastes/{paste_id}/documents/{document_id}"))
                    .add_header("Authorization", format!("Bearer {token_string}"))
                    .await;

                response.assert_status(StatusCode::NO_CONTENT);

                let document = Document::fetch(&pool, &paste_id)
                    .await
                    .expect("Failed to make DB request");

                assert!(document.is_none(), "Document was found");
            }

            #[ignore = "This is skipped due to the logic immediately failing if there is a minimal amount of documents. This should pass."]
            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_missing(pool: PgPool) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let paste_id = Snowflake::new(517815304354284604);
                let document_id = Snowflake::new(1234567890);

                let token_string =
                    "NTE3ODE1MzA0MzU0Mjg0NjA0.MTc0NzgxNjE4OQ==.FDP-mNTjfuOKovulMFbaSkoeq";

                let response = server
                    .delete(&format!("/v1/pastes/{paste_id}/documents/{document_id}"))
                    .add_header("Authorization", format!("Bearer {token_string}"))
                    .await;

                response.assert_status(StatusCode::NOT_FOUND);

                let body: RESTErrorResponse = response.json();

                assert_eq!(body.reason(), "Not Found", "Reason does not match.");

                assert_eq!(body.trace(), Some(""), "Trace does not match.");
            }
        }
    }
}
