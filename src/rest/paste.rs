use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use time::OffsetDateTime;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::{
    app::{application::App, config::Config},
    models::{
        authentication::{Token, generate_token},
        document::{DEFAULT_MIME, Document, UNSUPPORTED_MIMES, contains_mime},
        error::{AppError, AuthError},
        paste::Paste,
        payload::{
            GetPasteQuery, PatchPasteBody, PatchPasteQuery, PostPasteBody, PostPasteQuery,
            ResponseDocument, ResponsePaste,
        },
        snowflake::Snowflake,
        undefined::UndefinedOption,
    },
};

pub fn generate_router(config: &Config) -> Router<App> {
    let global_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.rate_limits().global_paste())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build global paste limiter."),
        ),
    };

    let get_pastes_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.rate_limits().get_pastes())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build get pastes limiter."),
        ),
    };

    let get_paste_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.rate_limits().get_paste())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build get paste limiter."),
        ),
    };

    let post_paste_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.rate_limits().post_paste())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build post paste limiter."),
        ),
    };

    let patch_paste_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.rate_limits().patch_paste())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build patch paste limiter."),
        ),
    };

    let delete_paste_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.rate_limits().delete_paste())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build delete paste limiter."),
        ),
    };

    Router::new()
        .route("/pastes", get(get_pastes).layer(get_pastes_limiter))
        .route(
            "/pastes/{paste_id}",
            get(get_paste).layer(get_paste_limiter),
        )
        .route("/pastes", post(post_paste).layer(post_paste_limiter))
        .route(
            "/pastes/{paste_id}",
            patch(patch_paste).layer(patch_paste_limiter),
        )
        .route(
            "/pastes/{paste_id}",
            delete(delete_paste).layer(delete_paste_limiter),
        )
        .layer(global_limiter)
        .layer(DefaultBodyLimit::max(
            config.global_paste_total_document_size_limit() * 1024 * 1024,
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
/// ## Query
///
/// References: [`GetPasteQuery`]
///
/// - `content` - Whether to include the content or not.
///
/// ## Returns
///
/// - `404` - The paste was not found.
/// - `200` - The [`ResponsePaste`] object.
async fn get_paste(
    State(app): State<App>,
    Path(paste_id): Path<Snowflake>,
    Query(query): Query<GetPasteQuery>,
) -> Result<Response, AppError> {
    let paste = Paste::fetch(&app.database, paste_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Paste not found.".to_string()))?;

    if let Some(expiry) = paste.expiry {
        if expiry < OffsetDateTime::now_utc() {
            Paste::delete(&app.database, paste.id).await?;
            return Err(AppError::NotFound("Paste not found.".to_string()));
        }
    }

    let documents = Document::fetch_all(&app.database, paste.id).await?;

    let mut response_documents = Vec::new();
    for document in documents {
        let content = {
            if query.include_content {
                let data = app.s3.fetch_document(document.generate_path()).await?;
                let d: &str = &String::from_utf8_lossy(&data);
                Some(d.to_string())
            } else {
                None
            }
        };

        let response_document = ResponseDocument::from_document(document, content);

        response_documents.push(response_document);
    }

    let paste_response = ResponsePaste::from_paste(&paste, None, response_documents);

    Ok((StatusCode::OK, Json(paste_response)).into_response())
}

/// Get Pastes.
///
/// Get a list of existing pastes.
///
/// ## Body
///
/// An array of [`Snowflake`]'s.
///
/// ## Returns
///
/// - `200` - A list of [`ResponsePaste`] objects.
async fn get_pastes(
    State(app): State<App>,
    Json(body): Json<Vec<Snowflake>>,
) -> Result<Response, AppError> {
    let mut response_pastes: Vec<ResponsePaste> = Vec::new();

    for paste_id in body {
        let paste = Paste::fetch(&app.database, paste_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Paste not found.".to_string()))?;

        if let Some(expiry) = paste.expiry {
            if expiry < OffsetDateTime::now_utc() {
                Paste::delete(&app.database, paste.id).await?;
                return Err(AppError::NotFound("Paste not found.".to_string()));
            }
        }

        let documents = Document::fetch_all(&app.database, paste.id).await?;

        let mut response_documents = Vec::new();
        for document in documents {
            let response_document = ResponseDocument::from_document(document, None);

            response_documents.push(response_document);
        }

        let response_paste = ResponsePaste::from_paste(&paste, None, response_documents);

        response_pastes.push(response_paste);
    }

    Ok((StatusCode::OK, Json(response_pastes)).into_response())
}

/// Post Paste.
///
/// Create a new paste.
///
/// The first object in the multipart must be the body object.
///
/// The following items will be the documents.
///
/// ## Query
///
/// References: [`PostPasteQuery`]
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
/// - `400` - The body and/or documents are invalid.
/// - `200` - The [`ResponsePaste`] object.
async fn post_paste(
    State(app): State<App>,
    Query(query): Query<PostPasteQuery>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let body: PostPasteBody = {
        if let Some(field) = multipart.next_field().await? {
            if field
                .content_type()
                .is_none_or(|content_type| content_type != "application/json")
            {
                return Err(AppError::BadRequest(
                    "Payload must be of the type application/json.".to_string(),
                ));
            }

            let bytes = field.bytes().await?;

            serde_json::from_slice(&bytes)?
        } else {
            return Err(AppError::BadRequest("Payload missing.".to_string()));
        }
    };

    let expiry = validate_expiry(&app.config, body.expiry)?;

    let mut transaction = app.database.pool().begin().await?;

    let paste = Paste::new(Snowflake::generate()?, false, expiry.to_option());

    paste.insert(&mut transaction).await?;

    let mut documents: Vec<(Document, String)> = Vec::new();
    while let Some(field) = multipart.next_field().await? {
        let document_type = {
            match field.content_type() {
                Some(content_type) => {
                    if contains_mime(UNSUPPORTED_MIMES, content_type) {
                        return Err(AppError::BadRequest(format!(
                            "Invalid mime type received: {content_type}"
                        )));
                    }

                    content_type.to_string()
                }
                None => DEFAULT_MIME.to_string(),
            }
        };
        let name = field
            .file_name()
            .ok_or(AppError::BadRequest(
                "The filename of the document is required".to_string(),
            ))?
            .to_string();
        let data = field.bytes().await?;

        if data.len() > (app.config.global_paste_document_size_limit() * 1024 * 1024) {
            return Err(AppError::NotFound("Document too large.".to_string()));
        }

        let document = Document::new(
            Snowflake::generate()?,
            paste.id,
            document_type,
            name,
            data.len(),
        );

        documents.push((document, String::from_utf8_lossy(&data).to_string()));
    }

    let final_documents: Vec<ResponseDocument> = documents
        .iter()
        .map(|(d, c)| {
            let content = {
                if query.include_content {
                    Some(c.clone())
                } else {
                    None
                }
            };

            ResponseDocument::from_document(d.clone(), content)
        })
        .collect();

    if documents.len() > app.config.global_paste_total_document_count() {
        return Err(AppError::BadRequest(
            "Too many documents provided.".to_string(),
        ));
    }

    if documents.is_empty() {
        return Err(AppError::BadRequest("No documents provided.".to_string()));
    }

    for (document, content) in documents {
        app.s3.create_document(&document, content.into()).await?;

        document.insert(&mut transaction).await?;
    }

    let paste_token = Token::new(paste.id, generate_token(paste.id)?);

    paste_token.insert(&mut transaction).await?;

    transaction.commit().await?;

    let response = ResponsePaste::from_paste(&paste, Some(paste_token), final_documents);

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
/// ## Query
///
/// References: [`PatchPasteQuery`]
///
/// - `content` - Whether to include the content or not.
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
async fn patch_paste(
    State(app): State<App>,
    Path(paste_id): Path<Snowflake>,
    Query(query): Query<PatchPasteQuery>,
    token: Token,
    Json(body): Json<PatchPasteBody>,
) -> Result<Response, AppError> {
    if token.paste_id() != paste_id {
        return Err(AppError::Authentication(AuthError::ForbiddenPasteId));
    }

    let mut paste = Paste::fetch(&app.database, paste_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Paste not found.".to_string()))?;

    if let Some(expiry) = paste.expiry {
        if expiry < OffsetDateTime::now_utc() {
            Paste::delete(&app.database, paste.id).await?;
            return Err(AppError::NotFound("Paste not found.".to_string()));
        }
    }

    let new_expiry = validate_expiry(&app.config, body.expiry)?;

    if !new_expiry.is_undefined() {
        paste.set_expiry(new_expiry.to_option());
    }

    let mut transaction = app.database.pool().begin().await?;

    paste.update(&mut transaction).await?;

    transaction.commit().await?;

    let documents = Document::fetch_all(&app.database, paste.id).await?;

    let mut response_documents = Vec::new();
    for document in documents {
        let content = {
            if query.include_content {
                let data = app.s3.fetch_document(document.generate_path()).await?;
                let d: &str = &String::from_utf8_lossy(&data);
                Some(d.to_string())
            } else {
                None
            }
        };

        let response_document = ResponseDocument::from_document(document, content);

        response_documents.push(response_document);
    }

    let paste_response = ResponsePaste::from_paste(&paste, None, response_documents);

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
async fn delete_paste(
    State(app): State<App>,
    Path(paste_id): Path<Snowflake>,
    token: Token,
) -> Result<Response, AppError> {
    if token.paste_id() != paste_id {
        return Err(AppError::Authentication(AuthError::ForbiddenPasteId));
    }

    if !Paste::delete(&app.database, paste_id).await? {
        return Err(AppError::NotFound("The paste was not found.".to_string()));
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Validate Expiry.
///
/// Checks if the expiry time is valid (if provided)
/// Otherwise, if not provided, returns the default, or None.
///
/// ## Arguments
///
/// - `config` - The config values to use.
/// - `expiry` - The expiry to validate (if provided).
///
/// ## Errors
///
/// - [`AppError`] - The app error returned, if the provided expiry is invalid.
///
/// ## Returns
///
/// - [`Option::Some`] - The [`OffsetDateTime`] that was extracted, or defaulted to.
/// - [`Option::None`] - No datetime was provided, and no default was set.
fn validate_expiry(
    config: &Config,
    expiry: UndefinedOption<usize>,
) -> Result<UndefinedOption<OffsetDateTime>, AppError> {
    match expiry {
        UndefinedOption::Some(expiry) => {
            let time = OffsetDateTime::from_unix_timestamp(expiry as i64)
                .map_err(|e| AppError::BadRequest(format!("Failed to build timestamp: {e}")))?;
            let now = OffsetDateTime::now_utc();
            let difference = (time - now).whole_seconds();

            if difference.is_negative() {
                return Err(AppError::BadRequest(
                    "The timestamp provided is invalid.".to_string(),
                ));
            }

            if let Some(maximum_expiry_hours) = config.maximum_expiry_hours() {
                if difference as usize > maximum_expiry_hours * 3600 {
                    return Err(AppError::BadRequest(
                        "The timestamp provided is above the maximum.".to_string(),
                    ));
                }
            }

            Ok(UndefinedOption::Some(time))
        }
        UndefinedOption::Undefined => {
            if let Some(default_expiry_hours) = config.default_expiry_hours() {
                return Ok(UndefinedOption::Some(
                    OffsetDateTime::now_utc()
                        .saturating_add(time::Duration::hours(default_expiry_hours as i64)),
                ));
            }

            if config.maximum_expiry_hours().is_some() {
                return Err(AppError::BadRequest(
                    "Timestamp must be provided.".to_string(),
                ));
            }

            Ok(UndefinedOption::Undefined)
        }
        UndefinedOption::None => {
            if config.maximum_expiry_hours().is_some() {
                return Err(AppError::BadRequest(
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
    use crate::{
        app::config::{Config, RateLimitConfigBuilder},
        models::error::AppError,
    };
    use time::Duration;

    fn make_config(
        maximum_expiry_hours: Option<usize>,
        default_expiry_hours: Option<usize>,
    ) -> Config {
        Config::builder()
            .host(String::new())
            .port(5454)
            .database_url(String::new())
            .s3_url(String::new())
            .s3_access_key(String::new().into())
            .s3_secret_key(String::new().into())
            .minio_root_user(String::new())
            .minio_root_password(String::new().into())
            .domain(String::new())
            .maximum_expiry_hours(maximum_expiry_hours)
            .default_expiry_hours(default_expiry_hours)
            .global_paste_total_document_count(0)
            .global_paste_total_document_size_limit(0)
            .global_paste_document_size_limit(0)
            .rate_limits(
                RateLimitConfigBuilder::default()
                    .build()
                    .expect("Failed to build rate limits"),
            )
            .build()
            .expect("Failed to build config.")
    }

    #[test]
    fn test_validate_expiry_valid() {
        let config = make_config(Some(100), Some(10));

        let expiry = OffsetDateTime::now_utc()
            .saturating_add(Duration::hours(5))
            .replace_nanosecond(0)
            .expect("Failed to remove nanoseconds");

        let validated_expiry = validate_expiry(
            &config,
            UndefinedOption::Some(expiry.unix_timestamp() as usize),
        )
        .expect("validate timestamp returned an unexpected error.");

        assert_eq!(
            validated_expiry,
            UndefinedOption::Some(expiry),
            "Timestamp was modified or changed."
        );
    }

    #[test]
    fn test_validate_expiry_invalid() {
        let config = make_config(Some(100), Some(10));

        let expiry = OffsetDateTime::now_utc().saturating_sub(Duration::hours(5));

        let validated_expiry = validate_expiry(
            &config,
            UndefinedOption::Some(expiry.unix_timestamp() as usize),
        );

        if let Err(validated_expiry) = validated_expiry {
            match validated_expiry {
                AppError::BadRequest(bad_request) => {
                    assert_eq!(
                        bad_request,
                        "The timestamp provided is invalid.".to_string(),
                        "Invalid bad request received."
                    );
                }
                _ => panic!("Wrong error received."),
            }
        } else {
            panic!("Expected no timestamp to be returned.");
        }
    }

    #[test]
    fn test_validate_expiry_undefined() {
        let config = make_config(Some(100), Some(10));

        let validated_expiry = validate_expiry(&config, UndefinedOption::Undefined)
            .expect("validate timestamp returned an unexpected error.");

        if let UndefinedOption::Some(validated_expiry) = validated_expiry {
            let current_time = OffsetDateTime::now_utc().saturating_add(Duration::hours(10));

            assert_eq!(
                validated_expiry.date(),
                current_time.date(),
                "Mismatching date."
            );
            assert_eq!(
                validated_expiry.to_hms(),
                current_time.to_hms(),
                "Mismatching hms."
            );
        } else {
            panic!("Expected no timestamp to be returned.");
        }
    }

    #[test]
    fn test_validate_expiry_null() {
        let config = make_config(Some(100), Some(10));

        let validated_expiry = validate_expiry(&config, UndefinedOption::None);

        if let Err(validated_expiry) = validated_expiry {
            match validated_expiry {
                AppError::BadRequest(bad_request) => {
                    assert_eq!(
                        bad_request,
                        "Timestamp must be provided.".to_string(),
                        "Invalid bad request received."
                    );
                }
                _ => panic!("Wrong error received."),
            }
        } else {
            panic!("Expected no timestamp to be returned.");
        }
    }

    #[test]
    fn test_validate_expiry_valid_no_maximum() {
        let config = make_config(None, Some(10));

        let expiry = OffsetDateTime::now_utc()
            .saturating_add(Duration::hours(5))
            .replace_nanosecond(0)
            .expect("Failed to remove nanoseconds");

        let validated_expiry = validate_expiry(
            &config,
            UndefinedOption::Some(expiry.unix_timestamp() as usize),
        )
        .expect("validate timestamp returned an unexpected error.");

        assert_eq!(
            validated_expiry,
            UndefinedOption::Some(expiry),
            "Timestamp was modified or changed."
        );
    }

    #[test]
    fn test_validate_expiry_invalid_no_maximum() {
        let config = make_config(None, Some(10));

        let expiry = OffsetDateTime::now_utc().saturating_sub(Duration::hours(5));

        let validated_expiry = validate_expiry(
            &config,
            UndefinedOption::Some(expiry.unix_timestamp() as usize),
        );

        if let Err(validated_expiry) = validated_expiry {
            match validated_expiry {
                AppError::BadRequest(bad_request) => {
                    assert_eq!(
                        bad_request,
                        "The timestamp provided is invalid.".to_string(),
                        "Invalid bad request received."
                    );
                }
                _ => panic!("Wrong error received."),
            }
        } else {
            panic!("Expected no timestamp to be returned.");
        }
    }

    #[test]
    fn test_validate_expiry_undefined_no_maximum() {
        let config = make_config(None, Some(10));

        let validated_expiry = validate_expiry(&config, UndefinedOption::Undefined)
            .expect("validate timestamp returned an unexpected error.");

        if let UndefinedOption::Some(validated_expiry) = validated_expiry {
            let current_time = OffsetDateTime::now_utc().saturating_add(Duration::hours(10));

            assert_eq!(
                validated_expiry.date(),
                current_time.date(),
                "Mismatching date."
            );
            assert_eq!(
                validated_expiry.to_hms(),
                current_time.to_hms(),
                "Mismatching hms."
            );
        } else {
            panic!("Expected no timestamp to be returned.");
        }
    }

    #[test]
    fn test_validate_expiry_null_no_maximum() {
        let config = make_config(None, Some(10));

        let validated_expiry = validate_expiry(&config, UndefinedOption::None)
            .expect("validate timestamp returned an unexpected error.");

        assert_eq!(
            validated_expiry,
            UndefinedOption::None,
            "Timestamp was found."
        );
    }

    #[test]
    fn test_validate_expiry_valid_no_default() {
        let config = make_config(Some(100), None);

        let expiry = OffsetDateTime::now_utc()
            .saturating_add(Duration::hours(5))
            .replace_nanosecond(0)
            .expect("Failed to remove nanoseconds");

        let validated_expiry = validate_expiry(
            &config,
            UndefinedOption::Some(expiry.unix_timestamp() as usize),
        )
        .expect("validate timestamp returned an unexpected error.");

        assert_eq!(
            validated_expiry,
            UndefinedOption::Some(expiry),
            "Timestamp was modified or changed."
        );
    }

    #[test]
    fn test_validate_expiry_invalid_no_default() {
        let config = make_config(Some(100), None);

        let expiry = OffsetDateTime::now_utc().saturating_sub(Duration::hours(5));

        let validated_expiry = validate_expiry(
            &config,
            UndefinedOption::Some(expiry.unix_timestamp() as usize),
        );

        if let Err(validated_expiry) = validated_expiry {
            match validated_expiry {
                AppError::BadRequest(bad_request) => {
                    assert_eq!(
                        bad_request,
                        "The timestamp provided is invalid.".to_string(),
                        "Invalid bad request received."
                    );
                }
                _ => panic!("Wrong error received."),
            }
        } else {
            panic!("Expected no timestamp to be returned.");
        }
    }

    #[test]
    fn test_validate_expiry_undefined_no_default() {
        let config = make_config(Some(100), None);

        let validated_expiry = validate_expiry(&config, UndefinedOption::Undefined);

        if let Err(validated_expiry) = validated_expiry {
            match validated_expiry {
                AppError::BadRequest(bad_request) => {
                    assert_eq!(
                        bad_request,
                        "Timestamp must be provided.".to_string(),
                        "Invalid bad request received."
                    );
                }
                _ => panic!("Wrong error received."),
            }
        } else {
            panic!("Expected no timestamp to be returned. {validated_expiry:?}");
        }
    }

    #[test]
    fn test_validate_expiry_null_no_default() {
        let config = make_config(Some(100), None);

        let validated_expiry = validate_expiry(&config, UndefinedOption::None);

        if let Err(validated_expiry) = validated_expiry {
            match validated_expiry {
                AppError::BadRequest(bad_request) => {
                    assert_eq!(
                        bad_request,
                        "Timestamp must be provided.".to_string(),
                        "Invalid bad request received."
                    );
                }
                _ => panic!("Wrong error received."),
            }
        } else {
            panic!("Expected no timestamp to be returned.");
        }
    }

    #[test]
    fn test_validate_expiry_valid_no_both() {
        let config = make_config(None, None);

        let expiry = OffsetDateTime::now_utc()
            .saturating_add(Duration::hours(5))
            .replace_nanosecond(0)
            .expect("Failed to remove nanoseconds");

        let validated_expiry = validate_expiry(
            &config,
            UndefinedOption::Some(expiry.unix_timestamp() as usize),
        )
        .expect("validate timestamp returned an unexpected error.");

        assert_eq!(
            validated_expiry,
            UndefinedOption::Some(expiry),
            "Timestamp was modified or changed."
        );
    }

    #[test]
    fn test_validate_expiry_invalid_no_both() {
        let config = make_config(None, None);

        let expiry = OffsetDateTime::now_utc().saturating_sub(Duration::hours(5));

        let validated_expiry = validate_expiry(
            &config,
            UndefinedOption::Some(expiry.unix_timestamp() as usize),
        );

        if let Err(validated_expiry) = validated_expiry {
            match validated_expiry {
                AppError::BadRequest(bad_request) => {
                    assert_eq!(
                        bad_request,
                        "The timestamp provided is invalid.".to_string(),
                        "Invalid bad request received."
                    );
                }
                _ => panic!("Wrong error received."),
            }
        } else {
            panic!("Expected no timestamp to be returned.");
        }
    }

    #[test]
    fn test_validate_expiry_undefined_no_both() {
        let config = make_config(None, None);

        let validated_expiry = validate_expiry(&config, UndefinedOption::Undefined)
            .expect("validate timestamp returned an unexpected error.");

        assert_eq!(
            validated_expiry,
            UndefinedOption::Undefined,
            "Timestamp was provided."
        );
    }

    #[test]
    fn test_validate_expiry_null_no_both() {
        let config = make_config(None, None);

        let validated_expiry = validate_expiry(&config, UndefinedOption::None)
            .expect("validate timestamp returned an unexpected error.");

        assert_eq!(
            validated_expiry,
            UndefinedOption::None,
            "Timestamp was provided."
        );
    }
}
