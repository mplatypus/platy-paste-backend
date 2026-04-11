//! Document related endpoints and router generator.

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    routing::get,
};
use axum_extra::headers::{self, Header};
use http::{HeaderName, HeaderValue, StatusCode};

use crate::{
    app::{application::App, config::Config},
    models::{
        document::Document, errors::RESTError, paste::validate_paste,
        payload::document::GetDocumentPath,
    },
};

/// ## Generate Router
///
/// Generates the router for document related endpoints.
///
/// ## Returns
/// The router with all the document related endpoints attached.
pub fn generate_router(config: &Config) -> Router<App> {
    Router::new()
        .route(
            "/pastes/{paste_id}/documents/{document_id}",
            get(get_document),
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
/// ## Errors
/// Returns an error if the request failed.
///
/// ## Returns
///
/// - `404` - The paste or document was not found.
/// - `200` - The [`ResponseDocument`] object.
pub async fn get_document(
    State(app): State<App>,
    Path(path): Path<GetDocumentPath>,
) -> Result<(StatusCode, Json<Document>), RESTError> {
    let mut paste = validate_paste(app.database(), path.paste_id(), None).await?;

    let document = Document::fetch(app.database().pool(), path.document_id())
        .await?
        .ok_or_else(|| RESTError::not_found("Document not found."))?;

    if document.paste_id() != path.paste_id() {
        return Err(RESTError::bad_request(
            "The document ID does not belong to that paste.".to_string(),
        ));
    }

    paste.add_view(app.database().pool()).await?;

    Ok((StatusCode::OK, Json(document)))
}

/// ## Content Disposition
///
/// Custom content disposition header, with filename parser.
#[derive(Debug, Clone)]
pub struct ContentDisposition {
    disposition: String,
    filename: Option<String>,
}

impl ContentDisposition {
    /// The contents disposition type.
    pub fn disposition(&self) -> &str {
        &self.disposition
    }

    /// The content dispositions filename (if found).
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
    use http::StatusCode;
    use rstest::rstest;

    use crate::{
        app::{application::ApplicationState, object_store::TestObjectStore},
        models::{
            document::Document, errors::RESTErrorResponse, paste::Paste, snowflake::Snowflake,
        },
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

                let app = main_generate_router(state);
                let server = TestServer::new(app);

                let paste_id = Snowflake::new(517_815_304_354_284_605);
                let document_id = Snowflake::new(517_815_304_354_284_708);

                let views = Paste::fetch(&pool, &paste_id)
                    .await
                    .expect("Failed to make DB request")
                    .expect("Failed to find paste.")
                    .views();

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

                let updated_views = Paste::fetch(&pool, &paste_id)
                    .await
                    .expect("Failed to make DB request")
                    .expect("Failed to find paste.")
                    .views();

                assert_eq!(views + 1, updated_views, "Views was not updated.");
            }

            #[rstest]
            #[case(Snowflake::new(517_815_304_354_284_605), "Document not found.")]
            #[case(
                Snowflake::new(1_234_567_890),
                "The paste requested could not be found"
            )]
            #[sqlx::test(fixtures(path = "../../tests/fixtures", scripts("pastes", "documents")))]
            async fn test_missing(
                #[ignore] pool: PgPool,
                #[case] paste_id: Snowflake,
                #[case] message: &str,
            ) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let document_id = Snowflake::new(1_234_567_890);

                let app = main_generate_router(state);
                let server = TestServer::new(app);

                let response = server
                    .get(&format!("/v1/pastes/{paste_id}/documents/{document_id}"))
                    .await;

                response.assert_status(StatusCode::NOT_FOUND);

                response.assert_header("Content-Type", "application/json");

                let body: RESTErrorResponse = response.json();

                assert_eq!(body.reason(), "Not Found", "Reason does not match.");

                assert_eq!(body.message(), message, "Trace does not match.");
            }
        }
    }
}
