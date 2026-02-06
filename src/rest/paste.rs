use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use chrono::{TimeDelta, Timelike, Utc};

use crate::{
    app::{application::App, config::Config, object_store::ObjectStoreExt as _},
    models::{
        DtUtc,
        authentication::{Token, generate_token},
        document::{
            Document, UNSUPPORTED_MIMES, contains_mime, document_limits, total_document_limits,
        },
        errors::{AuthenticationError, RESTError},
        paste::{Paste, validate_paste},
        payload::{
            DeletePastePath, GetPastePath, PatchPasteBody, PatchPastePath, PostPasteBody,
            ResponsePaste,
        },
        snowflake::Snowflake,
        undefined::UndefinedOption,
    },
};

pub fn generate_router(config: &Config) -> Router<App> {
    Router::new()
        .route("/pastes/{paste_id}", get(get_paste))
        .route("/pastes", post(post_paste))
        .route("/pastes/{paste_id}", patch(patch_paste))
        .route("/pastes/{paste_id}", delete(delete_paste))
        .layer(DefaultBodyLimit::max(
            config.size_limits().maximum_total_document_size(),
        ))
}

/// Get Paste.
///
/// Get an existing paste.
///
/// ## Path
///
/// - `paste_id` - The pastes ID.
///
/// ## Returns
///
/// - `404` - The paste was not found.
/// - `200` - The [`ResponsePaste`] object.
pub async fn get_paste(
    State(app): State<App>,
    Path(path): Path<GetPastePath>,
) -> Result<Response, RESTError> {
    let mut paste = validate_paste(app.database(), path.paste_id(), None).await?;

    let documents = Document::fetch_all(app.database().pool(), paste.id()).await?;

    let view_count = Paste::add_view(app.database().pool(), path.paste_id()).await?;

    paste.set_views(view_count);

    let paste_response = ResponsePaste::from_paste(&paste, None, documents);

    Ok((StatusCode::OK, Json(paste_response)).into_response())
}

/// Post Paste.
///
/// Create a new paste.
///
/// The first object in the multipart must be the body object.
///
/// The following items will be the documents.
///
/// ## Body
///
/// References: [`PostPasteBody`]
///
/// - `expiry` - The expiry of the paste.
///
/// ## Returns
///
/// - `400` - The body and/or documents are invalid.
/// - `200` - The [`ResponsePaste`] object.
pub async fn post_paste(
    State(app): State<App>,
    mut multipart: Multipart,
) -> Result<Response, RESTError> {
    let body: PostPasteBody = {
        if let Some(field) = multipart.next_field().await? {
            if field
                .content_type()
                .is_none_or(|content_type| content_type != "application/json")
            {
                return Err(RESTError::BadRequest(
                    "Payload must be of the type application/json.".to_string(),
                ));
            }

            let bytes = field.bytes().await?;

            serde_json::from_slice(&bytes)?
        } else {
            return Err(RESTError::BadRequest("Payload missing.".to_string()));
        }
    };

    let expiry = validate_expiry(app.config(), body.expiry())?;

    let max_views = {
        match body.max_views() {
            UndefinedOption::Undefined => app.config().size_limits().default_maximum_views(),
            UndefinedOption::Some(max_views) => Some(max_views),
            UndefinedOption::None => None,
        }
    };

    let name = {
        match body.name() {
            UndefinedOption::Undefined => app
                .config()
                .size_limits()
                .default_paste_name()
                .map(ToString::to_string),
            UndefinedOption::Some(name) => {
                let name = name.to_string();

                if name.len() > app.config().size_limits().maximum_paste_name_size() {
                    return Err(RESTError::BadRequest(
                        "The pastes name is too long.".to_string(),
                    ));
                }

                if name.len() < app.config().size_limits().minimum_paste_name_size() {
                    return Err(RESTError::BadRequest(
                        "The pastes name is too short.".to_string(),
                    ));
                }

                Some(name)
            }
            UndefinedOption::None => None,
        }
    };

    let mut transaction = app.database().pool().begin().await?;

    let paste = Paste::new(
        Snowflake::generate()?,
        name,
        Utc::now()
            .with_nanosecond(0)
            .ok_or(RESTError::InternalServer(
                "Failed to strip nanosecond from date time object.".to_string(),
            ))?,
        None,
        expiry.into(),
        0,
        max_views,
    );

    paste.insert(transaction.as_mut()).await?;

    let mut documents: Vec<(Document, String)> = Vec::new();
    while let Some(field) = multipart.next_field().await? {
        let document_type = {
            match field.content_type() {
                Some(content_type) => {
                    if contains_mime(UNSUPPORTED_MIMES, content_type) {
                        return Err(RESTError::BadRequest(format!(
                            "Invalid mime type received: {content_type}"
                        )));
                    }

                    content_type.to_string()
                }
                None => {
                    return Err(RESTError::BadRequest(
                        "The document must have a type.".to_string(),
                    ));
                }
            }
        };

        let name = field
            .file_name()
            .ok_or(RESTError::BadRequest(
                "One or more of the documents provided require a name.".to_string(),
            ))?
            .to_string();

        let data = field.bytes().await?;

        let document = Document::new(
            Snowflake::generate()?,
            *paste.id(),
            &document_type,
            &name,
            data.len(),
        );

        document_limits(app.config(), &document)?;

        documents.push((document, String::from_utf8_lossy(&data).to_string()));
    }

    let mut response_documents = Vec::new();
    for (document, content) in documents {
        app.object_store()
            .create_document(&document, content)
            .await?;

        document.insert(transaction.as_mut()).await?;

        response_documents.push(document);
    }

    total_document_limits(&mut transaction, app.config(), paste.id()).await?;

    let paste_token = Token::new(*paste.id(), generate_token(*paste.id())?);

    paste_token.insert(transaction.as_mut()).await?;

    transaction.commit().await?;

    let response = ResponsePaste::from_paste(&paste, Some(paste_token), response_documents);

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Patch Paste.
///
/// Edit an existing paste.
///
/// **Requires authentication.**
///
/// ## Path
///
/// - `paste_id` - The paste ID to edit.
///
/// ## Body
///
/// References: [`PatchPasteBody`]
///
/// - `expiry` - The expiry of the paste.
///
/// ## Returns
///
/// - `401` - Invalid token and/or paste ID.
/// - `400` - The body is invalid.
/// - `200` - The [`ResponsePaste`] object.
pub async fn patch_paste(
    State(app): State<App>,
    Path(path): Path<PatchPastePath>,
    token: Token,
    Json(body): Json<PatchPasteBody>,
) -> Result<Response, RESTError> {
    let mut paste = validate_paste(app.database(), path.paste_id(), Some(token)).await?;

    let new_expiry = validate_expiry(app.config(), body.expiry())?;

    match body.name() {
        UndefinedOption::Some(name) => {
            let name = name.to_string();

            if name.len() > app.config().size_limits().maximum_paste_name_size() {
                return Err(RESTError::BadRequest(
                    "The pastes name is too long.".to_string(),
                ));
            }

            if name.len() < app.config().size_limits().minimum_paste_name_size() {
                return Err(RESTError::BadRequest(
                    "The pastes name is too short.".to_string(),
                ));
            }

            paste.set_name(Some(name));
        }
        UndefinedOption::None => {
            paste.set_name(None);
        }
        UndefinedOption::Undefined => (),
    }

    if !new_expiry.is_undefined() {
        paste.set_expiry(new_expiry.into());
    }

    match body.max_views() {
        UndefinedOption::Some(max_views) => {
            if paste.views() >= max_views {
                return Err(RESTError::BadRequest("You cannot set the maximum views to a value equal to or lower than the current view count.".to_string()));
            }

            paste.set_max_views(Some(max_views));
        }
        UndefinedOption::None => paste.set_max_views(None),
        UndefinedOption::Undefined => (),
    }

    let mut transaction = app.database().pool().begin().await?;

    paste.set_edited()?;

    paste.update(transaction.as_mut()).await?;

    let documents = Document::fetch_all(transaction.as_mut(), paste.id()).await?;

    transaction.commit().await?;

    let paste_response = ResponsePaste::from_paste(&paste, None, documents);

    Ok((StatusCode::OK, Json(paste_response)).into_response())
}

/// Delete Paste.
///
/// Delete an existing paste.
///
/// **Requires authentication.**
///
/// ## Path
///
/// - `content` - Whether to include the content or not.
///
/// ## Body
///
/// References: [`PostPasteBody`]
///
/// - `expiry` - The expiry of the paste.
///
/// ## Returns
///
/// - `401` - Invalid token and/or paste ID.
/// - `204` - Successful deletion of the paste.
pub async fn delete_paste(
    State(app): State<App>,
    Path(path): Path<DeletePastePath>,
    token: Token,
) -> Result<Response, RESTError> {
    if token.paste_id() != path.paste_id() {
        return Err(RESTError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

    if !Paste::delete(app.database().pool(), path.paste_id()).await? {
        return Err(RESTError::NotFound("The paste was not found.".to_string()));
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Validate Expiry.
///
/// Checks if the expiry time is valid (if provided)
/// Otherwise, if not provided, returns the default, or None.
///
/// This will also strip the nanoseconds off the timestamp.
///
/// ## Arguments
///
/// - `config` - The config values to use.
/// - `expiry` - The expiry to validate (if provided).
///
/// ## Errors
///
/// - [`RESTError`] - The app error returned, if the provided expiry is invalid, or a timestamp was required.
///
/// ## Returns
///
/// - [`UndefinedOption::Some`] - The [`OffsetDateTime`] that was extracted, or defaulted to.
/// - [`UndefinedOption::Undefined`] - No default set, and it was undefined.
/// - [`UndefinedOption::None`] - None was given, and no maximum expiry has been set.
fn validate_expiry(
    config: &Config,
    expiry: UndefinedOption<DtUtc>,
) -> Result<UndefinedOption<DtUtc>, RESTError> {
    let size_limits = config.size_limits();
    match expiry {
        UndefinedOption::Some(expiry) => {
            let expiry = expiry.with_nanosecond(0).ok_or(RESTError::InternalServer(
                "Failed to strip nanosecond from date time object.".to_string(),
            ))?;
            let now = Utc::now()
                .with_nanosecond(0)
                .ok_or(RESTError::InternalServer(
                    "Failed to strip nanosecond from date time object.".to_string(),
                ))?;

            let difference = expiry - now;

            if difference.num_seconds() <= 0 {
                return Err(RESTError::BadRequest(
                    "The timestamp provided is invalid.".to_string(),
                ));
            }

            if let Some(minimum_expiry_hours) = size_limits.minimum_expiry_hours()
                && difference < TimeDelta::hours(minimum_expiry_hours as i64)
            {
                return Err(RESTError::BadRequest(
                    "The timestamp provided is below the minimum.".to_string(),
                ));
            }

            if let Some(maximum_expiry_hours) = size_limits.maximum_expiry_hours()
                && difference > TimeDelta::hours(maximum_expiry_hours as i64)
            {
                return Err(RESTError::BadRequest(
                    "The timestamp provided is above the maximum.".to_string(),
                ));
            }

            Ok(UndefinedOption::Some(expiry))
        }
        UndefinedOption::Undefined => {
            if let Some(default_expiry_hours) = size_limits.default_expiry_hours() {
                return Ok(UndefinedOption::Some(
                    Utc::now()
                        .with_nanosecond(0)
                        .ok_or(RESTError::InternalServer(
                            "Failed to strip nanosecond from date time object.".to_string(),
                        ))?
                        + TimeDelta::hours(default_expiry_hours as i64),
                ));
            }

            if size_limits.minimum_expiry_hours().is_some()
                || size_limits.maximum_expiry_hours().is_some()
            {
                return Err(RESTError::BadRequest(
                    "Timestamp must be provided.".to_string(),
                ));
            }

            Ok(UndefinedOption::Undefined)
        }
        UndefinedOption::None => {
            if size_limits.minimum_expiry_hours().is_some()
                || size_limits.maximum_expiry_hours().is_some()
            {
                return Err(RESTError::BadRequest(
                    "Timestamp must be provided.".to_string(),
                ));
            }

            Ok(UndefinedOption::None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::generate_router as main_generate_router;
    use crate::{
        app::{
            application::ApplicationState,
            config::{Config, SizeLimitConfig},
            object_store::TestObjectStore,
        },
        models::errors::{RESTError, RESTErrorResponse},
    };
    use axum_test::{
        TestServer,
        multipart::{MultipartForm, Part},
    };
    use bytes::Bytes;
    use chrono::Timelike;
    use rstest::*;
    use serde_json::json;
    use sqlx::PgPool;

    mod v1 {
        use super::*;

        mod get_paste {
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

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let response = server.get(&format!("/v1/pastes/{paste_id}")).await;

                response.assert_status(StatusCode::OK);

                response.assert_header("Content-Type", "application/json");

                let body = response.as_bytes();

                let paste = Paste::fetch(&pool, &paste_id)
                    .await
                    .expect("Failed to make DB request")
                    .expect("Failed to find paste.");
                let documents = Document::fetch_all(&pool, &paste_id)
                    .await
                    .expect("Failed to make DB request");

                let expected_body =
                    serde_json::to_vec(&ResponsePaste::from_paste(&paste, None, documents))
                        .expect("Failed to build expected body.");
                assert_eq!(body.to_vec(), expected_body, "Body does not match.");
            }

            #[sqlx::test(fixtures(path = "../../tests/fixtures", scripts("pastes", "documents")))]
            async fn test_missing(pool: PgPool) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let paste_id = Snowflake::new(1234567890);

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let response = server.get(&format!("/v1/pastes/{paste_id}")).await;

                response.assert_status(StatusCode::NOT_FOUND);

                response.assert_header("Content-Type", "application/json");

                let body: RESTErrorResponse = response.json();

                assert_eq!(body.reason(), "Not Found", "Reason does not match.");

                assert_eq!(
                    body.trace(),
                    Some("The paste requested could not be found"),
                    "Trace does not match."
                );
            }
        }

        mod post_paste {
            use super::*;

            #[sqlx::test]
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

                let payload_expiry = Utc::now() + TimeDelta::days(1);

                // TODO: I think I want to build these properly, as this seems like it could easily break without easy trace backs.
                let body = json!({
                    "name": "test paste",
                    "expiry_timestamp": payload_expiry.to_rfc3339(),
                    "max_views": 100
                });

                let payload = serde_json::to_string(&body).expect("Failed to build request body.");

                let payload_part = Part::bytes(Bytes::from(payload))
                    .add_header("Content-Type", "application/json");

                let document_1_content = Bytes::from(r#"{"test": "test_value"}"#);
                let document_1_part = Part::bytes(document_1_content.clone())
                    .add_header("Content-Type", "application/json")
                    .add_header(
                        "Content-Disposition",
                        r#"attachment; filename="custom.json""#,
                    );

                let document_2_content = Bytes::from(r#"Just some random text."#);
                let document_2_part = Part::bytes(document_2_content.clone())
                    .add_header("Content-Type", "text/plain")
                    .add_header(
                        "Content-Disposition",
                        r#"attachment; filename="random.txt""#,
                    );

                let form = MultipartForm::new()
                    .add_part("payload", payload_part)
                    .add_part("document_1", document_1_part)
                    .add_part("document_2", document_2_part);

                let response = server.post("/v1/pastes").multipart(form).await;

                response.assert_status(StatusCode::OK);

                response.assert_header("Content-Type", "application/json");

                let body: ResponsePaste = response.json();

                assert_eq!(
                    body.name(),
                    Some("test paste"),
                    "Paste name does not match."
                );

                assert!(body.token().is_some(), "Token was not returned.");

                // TODO: Check that timestamp is recent? within 5~ seconds of  the call?

                assert!(body.edited().is_none(), "Edited was set.");

                assert_eq!(
                    body.expiry(),
                    Some(
                        &payload_expiry
                            .with_nanosecond(0)
                            .expect("Failed to strip nanoseconds")
                    ),
                    "Expiry does not match."
                );

                assert_eq!(body.views(), 0, "Views does not match.");

                assert_eq!(body.max_views(), Some(100), "Maximum views does not match.");

                let documents = body.documents();
                assert_eq!(documents.len(), 2, "Document count does not match.");

                let paste_id = body.id();

                let Some(document_1) = documents.get(0) else {
                    panic!("Document 1 could not be found.");
                };

                assert_eq!(
                    document_1.paste_id(),
                    &paste_id,
                    "Document 1 paste ID does not match parent paste ID.",
                );

                assert_eq!(
                    document_1.name(),
                    "custom.json",
                    "Document 1 name does not match.",
                );

                assert_eq!(
                    document_1.doc_type(),
                    "application/json",
                    "Document 1 doc type does not match.",
                );

                let document_1_contents = object_store
                    .fetch_document(document_1.generate_path())
                    .await
                    .expect("Failed to find document_1's contents.");

                assert_eq!(
                    document_1_contents, document_1_content,
                    "Document 1 contents do not match.",
                );

                let Some(document_2) = documents.get(1) else {
                    panic!("Document 2 could not be found.");
                };

                assert_eq!(
                    document_2.paste_id(),
                    &paste_id,
                    "Document 2 paste ID does not match parent paste ID.",
                );

                assert_eq!(
                    document_2.name(),
                    "random.txt",
                    "Document 2 name does not match.",
                );

                assert_eq!(
                    document_2.doc_type(),
                    "text/plain",
                    "Document 2 doc type does not match.",
                );

                let document_2_contents = object_store
                    .fetch_document(document_2.generate_path())
                    .await
                    .expect("Failed to find document_1's contents.");

                assert_eq!(
                    document_2_contents, document_2_content,
                    "Document 2 contents do not match.",
                );
            }

            #[rstest]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "name": "test paste",
                    "expiry_timestamp": Utc::now() + TimeDelta::hours(5),
                    "max_views": 100,
                }),
                Some("test paste"),
                Some((Utc::now() + TimeDelta::hours(5)).with_nanosecond(0).expect("Failed to strip nanoseconds")),
                Some(100)
            )]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "name": null,
                    "expiry_timestamp": null,
                    "max_views": null,
                }),
                None,
                None,
                None,
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                        SizeLimitConfig::test_builder()
                            .default_paste_name(None)
                            .build()
                            .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "expiry_timestamp": null,
                    "max_views": null,
                }),
                None,
                None,
                None,
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                        SizeLimitConfig::test_builder()
                            .default_paste_name(Some("default_name".to_string()))
                            .build()
                            .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "expiry_timestamp": null,
                    "max_views": null,
                }),
                Some("default_name"),
                None,
                None,
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                        SizeLimitConfig::test_builder()
                            .default_expiry_hours(None)
                            .build()
                            .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "name": null,
                    "max_views": null,
                }),
                None,
                None,
                None,
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                        SizeLimitConfig::test_builder()
                            .default_expiry_hours(Some(5))
                            .build()
                            .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "name": null,
                    "max_views": null,
                }),
                None,
                Some((Utc::now() + TimeDelta::hours(5)).with_nanosecond(0).expect("Failed to strip nanoseconds")),
                None,
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                        SizeLimitConfig::test_builder()
                            .default_maximum_views(None)
                            .build()
                            .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "name": null,
                    "expiry_timestamp": null,
                }),
                None,
                None,
                None,
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                        SizeLimitConfig::test_builder()
                            .default_maximum_views(Some(100))
                            .build()
                            .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "name": null,
                    "expiry_timestamp": null,
                }),
                None,
                None,
                Some(100),
            )]
            #[sqlx::test]
            async fn test_defaults(
                #[ignore] pool: PgPool,
                #[case] config: Config,
                #[case] payload: serde_json::Value,
                #[case] expected_name: Option<&str>,
                #[case] expected_expiry: Option<DtUtc>,
                #[case] expected_maximum_views: Option<usize>,
            ) {
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let payload =
                    serde_json::to_string(&payload).expect("Failed to build request body.");

                let payload_part = Part::bytes(Bytes::from(payload))
                    .add_header("Content-Type", "application/json");

                let document_1_part = Part::bytes(Bytes::from("test"))
                    .add_header("Content-Type", "application/json")
                    .add_header(
                        "Content-Disposition",
                        r#"attachment; filename="custom.json""#,
                    );

                let form = MultipartForm::new()
                    .add_part("payload", payload_part)
                    .add_part("document_1", document_1_part);

                let response = server.post("/v1/pastes").multipart(form).await;

                response.assert_status(StatusCode::OK);

                response.assert_header("Content-Type", "application/json");

                let body: ResponsePaste = response.json();

                assert_eq!(body.name(), expected_name, "Names do not match.");

                assert_eq!(body.name(), expected_name, "Names do not match.");

                assert_eq!(
                    body.expiry(),
                    expected_expiry.as_ref(),
                    "Expiries do not match."
                );

                assert_eq!(
                    body.max_views(),
                    expected_maximum_views,
                    "Maximum views do not match."
                );
            }

            #[rstest]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from("{}")).add_header("Content-Type", "application/json")),
                StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("One or more documents is below the minimum total document count.".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from("{}"))),
                StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("Payload must be of the type application/json.".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from("{}")).add_header("Content-Type", "application/json"))
                    .add_part("document_1", Part::bytes(Bytes::new()).add_header("Content-Type", "image/png")),
                    StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("Invalid mime type received: image/png".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from("{}")).add_header("Content-Type", "application/json"))
                    .add_part("document_1", Part::bytes(Bytes::new()).add_header("Content-Type", "text/plain")),
                    StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("One or more of the documents provided require a name.".to_string())),
            )]
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
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from("{}")).add_header("Content-Type", "application/json"))
                    .add_part("document_1", Part::bytes(Bytes::from("test")).add_header("Content-Type", "text/plain").add_header("Content-Disposition", r#"attachment; filename="test.txt""#)),
                    StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("The document name: `test.txt` is too small.".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .maximum_document_name_size(10)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from("{}")).add_header("Content-Type", "application/json"))
                    .add_part("document_1", Part::bytes(Bytes::from("test")).add_header("Content-Type", "text/plain").add_header("Content-Disposition", r#"attachment; filename="test_file.txt""#)),
                    StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("The document name: `test_file.txt...` is too large.".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .minimum_document_size(100)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from("{}")).add_header("Content-Type", "application/json"))
                    .add_part("document_1", Part::bytes(Bytes::new()).add_header("Content-Type", "text/plain").add_header("Content-Disposition", r#"attachment; filename="test.txt""#)),
                    StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("The document: `test.txt` is too small.".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .maximum_document_size(100)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from("{}")).add_header("Content-Type", "application/json"))
                    .add_part("document_1", Part::bytes(Bytes::from(vec![0; 110])).add_header("Content-Type", "text/plain").add_header("Content-Disposition", r#"attachment; filename="test.txt""#)),
                    StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("The document: `test.txt` is too large.".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .minimum_total_document_count(2)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from("{}")).add_header("Content-Type", "application/json"))
                    .add_part("document_1", Part::bytes(Bytes::from("test")).add_header("Content-Type", "text/plain").add_header("Content-Disposition", r#"attachment; filename="test.txt""#)),
                    StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("One or more documents is below the minimum total document count.".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .maximum_total_document_count(1)
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from("{}")).add_header("Content-Type", "application/json"))
                    .add_part("document_1", Part::bytes(Bytes::from("test")).add_header("Content-Type", "text/plain").add_header("Content-Disposition", r#"attachment; filename="test.txt""#))
                    .add_part("document_2", Part::bytes(Bytes::from("test2")).add_header("Content-Type", "text/plain").add_header("Content-Disposition", r#"attachment; filename="test2.txt""#)),
                    StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("One or more documents exceed the maximum total document count.".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .minimum_expiry_hours(Some(1))
                                .maximum_expiry_hours(Some(5))
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from(serde_json::to_vec(&json!({
                        "expiry_timestamp": Utc::now().to_rfc3339(),
                    })).expect("Failed to build payload"))).add_header("Content-Type", "application/json"))
                    .add_part("document_1", Part::bytes(Bytes::from("test")).add_header("Content-Type", "text/plain").add_header("Content-Disposition", r#"attachment; filename="test.txt""#)),
                    StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("The timestamp provided is invalid.".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .minimum_expiry_hours(Some(1))
                                .maximum_expiry_hours(Some(5))
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                MultipartForm::new()
                    .add_part("payload", Part::bytes(Bytes::from(serde_json::to_vec(&json!({
                        "expiry_timestamp": (Utc::now() + TimeDelta::hours(6)).to_rfc3339(),
                    })).expect("Failed to build payload"))).add_header("Content-Type", "application/json"))
                    .add_part("document_1", Part::bytes(Bytes::from("test")).add_header("Content-Type", "text/plain").add_header("Content-Disposition", r#"attachment; filename="test.txt""#)),
                    StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("The timestamp provided is above the maximum.".to_string())),
            )]
            #[sqlx::test]
            async fn test_bad(
                #[ignore] pool: PgPool,
                #[case] config: Config,
                #[case] form: MultipartForm,
                #[case] expected_status: StatusCode,
                #[case] expected_body: RESTErrorResponse,
            ) {
                let object_store = TestObjectStore::new();
                let state =
                    ApplicationState::new_tests(config.clone(), pool.clone(), object_store.clone())
                        .await
                        .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let response = server.post("/v1/pastes").multipart(form).await;

                response.assert_status(expected_status);

                response.assert_header("Content-Type", "application/json");

                let body: RESTErrorResponse = response.json();

                assert_eq!(
                    body.reason(),
                    expected_body.reason(),
                    "Reason does not match."
                );

                assert_eq!(body.trace(), expected_body.trace(), "Trace does not match.");
            }
        }

        mod patch_paste {
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

                let mut request = server.patch(&format!("/v1/pastes/{paste_id}"));

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

            #[rstest]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                json!({}),
                None,
                None,
                None
            )]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "name": "beans"
                }),
                Some("beans"),
                None,
                None
            )]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "expiry_timestamp": Utc::now() + TimeDelta::hours(5),
                }),
                None,
                Some((Utc::now() + TimeDelta::hours(5)).with_nanosecond(0).expect("Failed to strip nanoseconds")),
                None
            )]
            #[case(
                Config::test_builder()
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "max_views": 5000
                }),
                None,
                None,
                Some(5000)
            )]
            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_existing(
                #[ignore] pool: PgPool,
                #[case] config: Config,
                #[case] body: serde_json::Value,
                #[case] expected_name: Option<&str>,
                #[case] expected_expiry: Option<DtUtc>,
                #[case] expected_max_views: Option<usize>,
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
                    .patch(&format!("/v1/pastes/{paste_id}"))
                    .add_header("Authorization", format!("Bearer {token_string}"))
                    .json(&body)
                    .await;

                response.assert_status(StatusCode::OK);

                response.assert_header("Content-Type", "application/json");

                let body: ResponsePaste = response.json();

                assert_eq!(body.name(), expected_name, "Names do not match.");

                assert_eq!(
                    body.expiry(),
                    expected_expiry.as_ref(),
                    "Expiry's do not match."
                );

                assert_eq!(
                    body.max_views(),
                    expected_max_views,
                    "Max views do not match."
                );
            }

            #[rstest]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .minimum_expiry_hours(Some(1))
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "expiry_timestamp": null,
                }),
                StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("Timestamp must be provided.".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .minimum_expiry_hours(Some(1))
                                .maximum_expiry_hours(Some(5))
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "expiry_timestamp": Utc::now().to_rfc3339(),
                }),
                StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("The timestamp provided is invalid.".to_string())),
            )]
            #[case(
                Config::test_builder()
                    .size_limits(
                            SizeLimitConfig::test_builder()
                                .minimum_expiry_hours(Some(1))
                                .maximum_expiry_hours(Some(5))
                                .build()
                                .expect("Failed to build size limit config.")
                    )
                    .build()
                    .expect("Failed to build config."),
                json!({
                    "expiry_timestamp": (Utc::now() + TimeDelta::hours(6)).to_rfc3339(),
                }),
                StatusCode::BAD_REQUEST,
                RESTErrorResponse::new("Bad Request", Some("The timestamp provided is above the maximum.".to_string())),
            )]
            #[sqlx::test(fixtures(
                path = "../../tests/fixtures",
                scripts("pastes", "documents", "tokens")
            ))]
            async fn test_bad(
                #[ignore] pool: PgPool,
                #[case] config: Config,
                #[case] body: serde_json::Value,
                #[case] expected_status: StatusCode,
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
                    .patch(&format!("/v1/pastes/{paste_id}"))
                    .add_header("Authorization", format!("Bearer {token_string}"))
                    .json(&body)
                    .await;

                response.assert_status(expected_status);

                response.assert_header("Content-Type", "application/json");

                let body: RESTErrorResponse = response.json();

                assert_eq!(
                    body.reason(),
                    expected_body.reason(),
                    "Reason does not match."
                );

                assert_eq!(body.trace(), expected_body.trace(), "Trace does not match.");
            }
        }

        mod delete_paste {
            use super::*;

            #[rstest]
            #[case(
                Snowflake::new(1234567890),
                Some("NTE3ODE1MzA0MzU0Mjg0NjA0.MTc0NzgxNjE4OQ==.FDP-mNTjfuOKovulMFbaSkoeq"),
                "Invalid Token and/or mismatched paste ID"
            )]
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

                let mut request = server.delete(&format!("/v1/pastes/{paste_id}"));

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

                let paste = Paste::fetch(&pool, &paste_id)
                    .await
                    .expect("Failed to make DB request");
                let documents = Document::fetch_all(&pool, &paste_id)
                    .await
                    .expect("Failed to make DB request");
                let token = Token::fetch(&pool, token_string)
                    .await
                    .expect("Failed to make DB request");

                assert!(paste.is_some(), "Paste was not found");

                assert_eq!(documents.len(), 1, "Incorrect amount of documents found");

                assert!(token.is_some(), "Token was not found");

                let response = server
                    .delete(&format!("/v1/pastes/{}", paste_id))
                    .add_header("Authorization", format!("Bearer {token_string}"))
                    .await;

                response.assert_status(StatusCode::NO_CONTENT);

                let paste = Paste::fetch(&pool, &paste_id)
                    .await
                    .expect("Failed to make DB request");
                let documents = Document::fetch_all(&pool, &paste_id)
                    .await
                    .expect("Failed to make DB request");
                let token = Token::fetch(&pool, token_string)
                    .await
                    .expect("Failed to make DB request");

                assert!(paste.is_none(), "Paste was found");

                assert!(documents.is_empty(), "one or more documents were found");

                assert!(token.is_none(), "Token was found");
            }
        }
    }

    fn make_config(
        default_expiry_hours: Option<usize>,
        minimum_expiry_hours: Option<usize>,
        maximum_expiry_hours: Option<usize>,
    ) -> Config {
        Config::test_builder()
            .size_limits(
                SizeLimitConfig::test_builder()
                    .default_expiry_hours(default_expiry_hours)
                    .minimum_expiry_hours(minimum_expiry_hours)
                    .maximum_expiry_hours(maximum_expiry_hours)
                    .build()
                    .expect("Failed to build rate limits"),
            )
            .build()
            .expect("Failed to build config.")
    }

    pub fn valid_time() -> DtUtc {
        Utc::now()
            .with_nanosecond(0)
            .expect("Failed to build current time with reset nanosecond.")
            + TimeDelta::hours(50)
    }

    pub fn invalid_time() -> DtUtc {
        Utc::now()
            .with_nanosecond(0)
            .expect("Failed to build current time with reset nanosecond.")
            + TimeDelta::minutes(10)
    }

    #[rstest]
    // Expiry cases.
    #[case(
        make_config(None, None, None),
        UndefinedOption::Some(valid_time()),
        UndefinedOption::Some(valid_time())
    )]
    #[case(
        make_config(Some(10), None, None),
        UndefinedOption::Some(valid_time()),
        UndefinedOption::Some(valid_time())
    )]
    #[case(
        make_config(None, Some(1), None),
        UndefinedOption::Some(valid_time()),
        UndefinedOption::Some(valid_time())
    )]
    #[case(
        make_config(None, None, Some(100)),
        UndefinedOption::Some(valid_time()),
        UndefinedOption::Some(valid_time())
    )]
    #[case(
        make_config(Some(10), Some(1), None),
        UndefinedOption::Some(valid_time()),
        UndefinedOption::Some(valid_time())
    )]
    #[case(
        make_config(None, Some(1), Some(100)),
        UndefinedOption::Some(valid_time()),
        UndefinedOption::Some(valid_time())
    )]
    #[case(
        make_config(Some(10), None, Some(100)),
        UndefinedOption::Some(valid_time()),
        UndefinedOption::Some(valid_time())
    )]
    #[case(
        make_config(Some(10), Some(1), Some(100)),
        UndefinedOption::Some(valid_time()),
        UndefinedOption::Some(valid_time())
    )]
    // Missing expiry cases.
    #[case(
        make_config(None, None, None),
        UndefinedOption::None,
        UndefinedOption::None
    )]
    #[case(
        make_config(Some(10), None, None),
        UndefinedOption::None,
        UndefinedOption::None
    )]
    // Undefined expiry cases.
    #[case(
        make_config(None, None, None),
        UndefinedOption::Undefined,
        UndefinedOption::Undefined
    )]
    fn test_validate_expiry_valid(
        #[case] config: Config,
        #[case] expiry: UndefinedOption<DtUtc>,
        #[case] expected: UndefinedOption<DtUtc>,
    ) {
        let returned_expiry =
            validate_expiry(&config, expiry).expect("Expected a undefined option.");

        assert_eq!(returned_expiry, expected, "Mismatched expiry.");
    }

    #[rstest]
    // Missing expiry cases.
    #[case(
        make_config(None, Some(1), None),
        UndefinedOption::None,
        "Timestamp must be provided."
    )]
    #[case(
        make_config(None, None, Some(100)),
        UndefinedOption::None,
        "Timestamp must be provided."
    )]
    #[case(
        make_config(Some(10), Some(1), None),
        UndefinedOption::None,
        "Timestamp must be provided."
    )]
    #[case(
        make_config(None, Some(1), Some(100)),
        UndefinedOption::None,
        "Timestamp must be provided."
    )]
    #[case(
        make_config(Some(10), None, Some(100)),
        UndefinedOption::None,
        "Timestamp must be provided."
    )]
    #[case(
        make_config(Some(10), Some(1), Some(100)),
        UndefinedOption::None,
        "Timestamp must be provided."
    )]
    // Undefined expiry cases.
    #[case(
        make_config(None, Some(1), None),
        UndefinedOption::Undefined,
        "Timestamp must be provided."
    )]
    #[case(
        make_config(None, None, Some(100)),
        UndefinedOption::Undefined,
        "Timestamp must be provided."
    )]
    #[case(
        make_config(None, Some(1), Some(100)),
        UndefinedOption::Undefined,
        "Timestamp must be provided."
    )]
    // Invalid expiry cases.
    #[case(
        make_config(None, Some(1), None),
        UndefinedOption::Some(invalid_time()),
        "The timestamp provided is below the minimum."
    )]
    #[case(
        make_config(None, None, Some(10)),
        UndefinedOption::Some(valid_time()),
        "The timestamp provided is above the maximum."
    )]
    #[case(
        make_config(None, Some(1), Some(10)),
        UndefinedOption::Some(invalid_time()),
        "The timestamp provided is below the minimum."
    )]
    #[case(
        make_config(None, Some(1), Some(10)),
        UndefinedOption::Some(valid_time()),
        "The timestamp provided is above the maximum."
    )]
    fn test_validate_expiry_invalid(
        #[case] config: Config,
        #[case] expiry: UndefinedOption<DtUtc>,
        #[case] expected: &str,
    ) {
        let returned_expiry = validate_expiry(&config, expiry).expect_err("Expected an error.");

        if let RESTError::BadRequest(response) = &returned_expiry {
            assert_eq!(response, expected, "Invalid response received.");
        } else {
            panic!(
                "Unexpected error received.\nExpected - {returned_expiry:?}\nActual - {expected:?}"
            );
        }
    }

    #[rstest]
    #[case(make_config(Some(10), None, None))]
    #[case(make_config(Some(10), Some(1), None))]
    #[case(make_config(Some(10), None, Some(100)))]
    #[case(make_config(Some(10), Some(1), Some(100)))]
    fn test_validate_expiry_default(#[case] config: Config) {
        let returned_expiry = validate_expiry(&config, UndefinedOption::Undefined)
            .expect("Expected a undefined option.");

        if let UndefinedOption::Some(returned_time) = returned_expiry {
            let expected = Utc::now()
                .with_nanosecond(0)
                .expect("Failed to build current time with reset nanosecond.")
                + TimeDelta::hours(10);

            assert_eq!(
                returned_time.date_naive(),
                expected.date_naive(),
                "Mismatching date."
            );
            assert_eq!(returned_time.time(), expected.time(), "Mismatching hms.");
        } else {
            panic!("Expected a timestamp to be returned.");
        }
    }
}
