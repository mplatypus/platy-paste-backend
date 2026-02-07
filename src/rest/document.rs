use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    routing::get,
};
use axum_extra::headers::{self, Header};
use http::{HeaderName, HeaderValue, StatusCode};

use crate::{
    app::{application::App, config::Config},
    models::{document::Document, errors::RESTError, paste::Paste, payload::document::*},
};

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
/// ## Returns
///
/// - `404` - The paste or document was not found.
/// - `200` - The [`ResponseDocument`] object.
pub async fn get_document(
    State(app): State<App>,
    Path(path): Path<GetDocumentPath>,
) -> Result<(StatusCode, Json<Document>), RESTError> {
    let document = Document::fetch(app.database().pool(), path.document_id())
        .await?
        .ok_or_else(|| RESTError::NotFound("Document not found.".to_string()))?;

    if document.paste_id() != path.paste_id() {
        return Err(RESTError::BadRequest(
            "The document ID does not belong to that paste.".to_string(),
        ));
    }

    Paste::add_view(app.database().pool(), path.paste_id()).await?;

    Ok((StatusCode::OK, Json(document)))
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
                let server = TestServer::new(app).expect("Failed to build server.");

                let paste_id = Snowflake::new(517815304354284605);
                let document_id = Snowflake::new(517815304354284708);

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
            #[case(Snowflake::new(517815304354284605), Some("Document not found."))]
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
    }
}
