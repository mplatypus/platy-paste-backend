use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use chrono::{TimeDelta, Timelike, Utc};

use crate::{
    app::{application::App, config::Config},
    models::{
        DtUtc,
        authentication::{Token, generate_token},
        document::{
            Document, UNSUPPORTED_MIMES, contains_mime, document_limits, total_document_limits,
        },
        error::{AppError, AuthError},
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
async fn get_paste(
    State(app): State<App>,
    Path(path): Path<GetPastePath>,
) -> Result<Response, AppError> {
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
async fn post_paste(
    State(app): State<App>,
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

    let expiry = validate_expiry(app.config(), body.expiry())?;

    let max_views = {
        match body.max_views() {
            UndefinedOption::Undefined => app.config().size_limits().default_maximum_views(),
            UndefinedOption::Some(max_views) => Some(max_views),
            UndefinedOption::None => None,
        }
    };

    let mut transaction = app.database().pool().begin().await?;

    let paste = Paste::new(
        Snowflake::generate()?,
        Utc::now()
            .with_nanosecond(0)
            .ok_or(AppError::InternalServer(
                "Failed to strip nanosecond from date time object.".to_string(),
            ))?,
        None,
        expiry.to_option(),
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
                        return Err(AppError::BadRequest(format!(
                            "Invalid mime type received: {content_type}"
                        )));
                    }

                    content_type.to_string()
                }
                None => {
                    return Err(AppError::BadRequest(
                        "The document must have a type.".to_string(),
                    ));
                }
            }
        };

        let name = field
            .file_name()
            .ok_or(AppError::BadRequest(
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
        app.s3().create_document(&document, content).await?;

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
async fn patch_paste(
    State(app): State<App>,
    Path(path): Path<PatchPastePath>,
    token: Token,
    Json(body): Json<PatchPasteBody>,
) -> Result<Response, AppError> {
    let mut paste = validate_paste(app.database(), path.paste_id(), Some(token)).await?;

    let new_expiry = validate_expiry(app.config(), body.expiry())?;

    if !new_expiry.is_undefined() {
        paste.set_expiry(new_expiry.to_option());
    }

    match body.max_views() {
        UndefinedOption::Some(max_views) => {
            if paste.views() >= max_views {
                return Err(AppError::BadRequest("You cannot set the maximum views to a value equal to or lower than the current view count.".to_string()));
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
async fn delete_paste(
    State(app): State<App>,
    Path(path): Path<DeletePastePath>,
    token: Token,
) -> Result<Response, AppError> {
    if token.paste_id() != path.paste_id() {
        return Err(AppError::Authentication(AuthError::InvalidCredentials));
    }

    if !Paste::delete(app.database().pool(), path.paste_id()).await? {
        return Err(AppError::NotFound("The paste was not found.".to_string()));
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
/// - [`AppError`] - The app error returned, if the provided expiry is invalid, or a timestamp was required.
///
/// ## Returns
///
/// - [`UndefinedOption::Some`] - The [`OffsetDateTime`] that was extracted, or defaulted to.
/// - [`UndefinedOption::Undefined`] - No default set, and it was undefined.
/// - [`UndefinedOption::None`] - None was given, and no maximum expiry has been set.
fn validate_expiry(
    config: &Config,
    expiry: UndefinedOption<DtUtc>,
) -> Result<UndefinedOption<DtUtc>, AppError> {
    let size_limits = config.size_limits();
    match expiry {
        UndefinedOption::Some(expiry) => {
            let expiry = expiry.with_nanosecond(0).ok_or(AppError::InternalServer(
                "Failed to strip nanosecond from date time object.".to_string(),
            ))?;
            let now = Utc::now()
                .with_nanosecond(0)
                .ok_or(AppError::InternalServer(
                    "Failed to strip nanosecond from date time object.".to_string(),
                ))?;

            let difference = expiry - now;

            if difference.num_seconds() <= 0 {
                return Err(AppError::BadRequest(
                    "The timestamp provided is invalid.".to_string(),
                ));
            }

            if let Some(minimum_expiry_hours) = size_limits.minimum_expiry_hours()
                && difference < TimeDelta::hours(minimum_expiry_hours as i64)
            {
                return Err(AppError::BadRequest(
                    "The timestamp provided is below the minimum.".to_string(),
                ));
            }

            if let Some(maximum_expiry_hours) = size_limits.maximum_expiry_hours()
                && difference > TimeDelta::hours(maximum_expiry_hours as i64)
            {
                return Err(AppError::BadRequest(
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
                        .ok_or(AppError::InternalServer(
                            "Failed to strip nanosecond from date time object.".to_string(),
                        ))?
                        + TimeDelta::hours(default_expiry_hours as i64),
                ));
            }

            if size_limits.minimum_expiry_hours().is_some()
                || size_limits.maximum_expiry_hours().is_some()
            {
                return Err(AppError::BadRequest(
                    "Timestamp must be provided.".to_string(),
                ));
            }

            Ok(UndefinedOption::Undefined)
        }
        UndefinedOption::None => {
            if size_limits.minimum_expiry_hours().is_some()
                || size_limits.maximum_expiry_hours().is_some()
            {
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
        app::config::{Config, SizeLimitConfigBuilder},
        models::error::AppError,
    };
    use chrono::Timelike;
    use rstest::*;

    fn make_config(
        default_expiry_hours: Option<usize>,
        minimum_expiry_hours: Option<usize>,
        maximum_expiry_hours: Option<usize>,
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
            .size_limits(
                SizeLimitConfigBuilder::default()
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

        if let AppError::BadRequest(response) = &returned_expiry {
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
