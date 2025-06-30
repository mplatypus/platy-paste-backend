use platy_paste::{
    app::{
        config::{Config, RateLimitConfigBuilder, SizeLimitConfigBuilder},
        database::Database,
    },
    models::{document::*, error::AppError, snowflake::Snowflake},
};

use rstest::*;
use sqlx::PgPool;

#[test]
fn test_getters() {
    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);
    let document_type = String::from("example/document");
    let name = String::from("test.document");
    let size = 329;

    let document = Document::new(
        document_id,
        paste_id,
        document_type.clone(),
        name.clone(),
        size,
    );

    assert!(document.id == document_id, "Mismatched document ID.");

    assert!(document.paste_id == paste_id, "Mismatched paste ID.");

    assert!(
        document.document_type == document_type,
        "Mismatched document type."
    );

    assert!(document.name == name, "Mismatched name.");

    assert!(document.size == size, "Mismatched size.");

    assert!(
        document.generate_url("http://example.com")
            == format!("http://example.com/documents/{paste_id}/{document_id}/{name}"),
        "Mismatched URL."
    );

    assert!(
        document.generate_path() == format!("{paste_id}/{document_id}/{name}"),
        "Mismatched path."
    );
}

#[test]
fn test_set_document_type() {
    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);
    let document_type = String::from("example/document");
    let name = String::from("test.document");
    let size = 6908;

    let mut document = Document::new(
        document_id,
        paste_id,
        document_type.clone(),
        name.clone(),
        size,
    );

    assert_eq!(
        document.document_type, document_type,
        "Mismatched document type."
    );

    let new_document_type = String::from("new/document");

    document.set_document_type(new_document_type.clone());

    assert_eq!(document.id, document_id, "Mismatched document ID.");

    assert_eq!(document.paste_id, paste_id, "Mismatched paste ID.");

    assert_eq!(
        document.document_type, new_document_type,
        "Mismatched document type."
    );

    assert_eq!(document.name, name, "Mismatched name.");

    assert_eq!(document.size, size, "Mismatched size.");
}

#[test]
fn test_set_name() {
    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);
    let document_type = String::from("example/document");
    let name = String::from("test.document");
    let size = 6908;

    let mut document = Document::new(
        document_id,
        paste_id,
        document_type.clone(),
        name.clone(),
        size,
    );

    assert_eq!(document.name, name, "Mismatched name.");

    let new_name = String::from("cool.new");

    document.set_name(new_name.clone());

    assert_eq!(document.id, document_id, "Mismatched document ID.");

    assert_eq!(document.paste_id, paste_id, "Mismatched paste ID.");

    assert_eq!(
        document.document_type, document_type,
        "Mismatched document type."
    );

    assert_eq!(document.name, new_name, "Mismatched name.");

    assert_eq!(document.size, size, "Mismatched size.");
}

#[test]
fn test_set_size() {
    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);
    let document_type = String::from("example/document");
    let name = String::from("test.document");
    let size = 6908;

    let mut document = Document::new(
        document_id,
        paste_id,
        document_type.clone(),
        name.clone(),
        size,
    );

    assert_eq!(document.size, size, "Mismatched size.");

    let new_size = 39485;

    document.set_size(new_size);

    assert_eq!(document.id, document_id, "Mismatched document ID.");

    assert_eq!(document.paste_id, paste_id, "Mismatched paste ID.");

    assert_eq!(
        document.document_type, document_type,
        "Mismatched document type."
    );

    assert_eq!(document.name, name, "Mismatched name.");

    assert_eq!(document.size, new_size, "Mismatched size.");
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch(pool: PgPool) {
    let db = Database::from_pool(pool);

    let document_id = Snowflake::new(517_815_304_354_284_701);

    let document = Document::fetch(db.pool(), document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No document was found.");

    assert_eq!(document.id, document_id, "Mismatched document ID.");

    assert_eq!(
        document.paste_id,
        Snowflake::new(517_815_304_354_284_601),
        "Mismatched paste ID."
    );

    assert_eq!(
        document.document_type, "plain/text",
        "Mismatched document type."
    );

    assert_eq!(document.name, "cool.txt", "Mismatched document type.");

    assert_eq!(document.size, 811, "Mismatched document size.");
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let document_id = Snowflake::new(456);

    assert!(
        Document::fetch(db.pool(), document_id)
            .await
            .expect("Failed to fetch value from database.")
            .is_none()
    );
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_with_paste(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_284_601);
    let document_id = Snowflake::new(517_815_304_354_284_701);

    let document = Document::fetch_with_paste(db.pool(), paste_id, document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No document was found.");

    assert_eq!(document.id, document_id, "Mismatched document ID.");

    assert_eq!(document.paste_id, paste_id, "Mismatched paste ID.");

    assert_eq!(
        document.document_type, "plain/text",
        "Mismatched document type."
    );

    assert_eq!(document.name, "cool.txt", "Mismatched document type.");

    assert_eq!(document.size, 811, "Mismatched document size.");
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_with_paste_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);

    assert!(
        Document::fetch_with_paste(db.pool(), paste_id, document_id)
            .await
            .expect("Failed to fetch value from database.")
            .is_none()
    );
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_all(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_284_603);

    let documents = Document::fetch_all(db.pool(), paste_id)
        .await
        .expect("Failed to fetch value from database.");

    assert!(
        documents.len() == 3,
        "Not enough or too many results received."
    );

    assert!(
        documents[0].id == Snowflake::new(517_815_304_354_284_704),
        "Mismatched document ID 1."
    );

    assert!(documents[0].paste_id == paste_id, "Mismatched paste ID 1.");

    assert!(
        documents[1].id == Snowflake::new(517_815_304_354_284_705),
        "Mismatched document ID 2."
    );

    assert!(documents[1].paste_id == paste_id, "Mismatched paste ID 2.");

    assert!(
        documents[2].id == Snowflake::new(517_815_304_354_284_706),
        "Mismatched document ID 3."
    );

    assert!(documents[2].paste_id == paste_id, "Mismatched paste ID 3.");
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_all_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(456);

    assert!(
        Document::fetch_all(db.pool(), paste_id)
            .await
            .expect("Failed to fetch value from database.")
            .is_empty()
    );
}

#[sqlx::test(fixtures("pastes"))]
fn test_insert(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_284_601);
    let document_id = Snowflake::new(456);
    let document_type = String::from("example/document");
    let name = String::from("test.document");
    let size = 475;

    let document = Document::new(
        document_id,
        paste_id,
        document_type.clone(),
        name.clone(),
        size,
    );

    document
        .insert(db.pool())
        .await
        .expect("Failed to insert paste");

    let result = Document::fetch(db.pool(), document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No document was found.");

    assert_eq!(result.id, document_id, "Mismatched document ID.");

    assert_eq!(result.paste_id, paste_id, "Mismatched paste ID.");

    assert_eq!(
        result.document_type, document_type,
        "Mismatched document type."
    );

    assert_eq!(result.name, name, "Mismatched document type.");

    assert_eq!(result.size, size);
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_update(pool: PgPool) {
    let db = Database::from_pool(pool);

    let document_id = Snowflake::new(517_815_304_354_284_701);

    let mut document = Document::fetch(db.pool(), document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("Document was not found.");

    assert_eq!(document.id, document_id, "Mismatched document ID.");

    assert_eq!(
        document.paste_id,
        Snowflake::new(517_815_304_354_284_601),
        "Mismatched paste ID."
    );

    assert_eq!(
        document.document_type, "plain/text",
        "Mismatched document type."
    );

    assert_eq!(document.name, "cool.txt", "Mismatched document type.");

    assert_eq!(document.size, 811, "Mismatched document size.");

    let new_document_type = String::from("example/new");

    let new_name = String::from("test_new_name");

    let new_size = 83247;

    document.set_document_type(new_document_type.clone());

    document.set_name(new_name.clone());

    document.set_size(new_size);

    document
        .update(db.pool())
        .await
        .expect("Failed to update paste.");

    let result_document = Document::fetch(db.pool(), document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("Document was not found.");

    assert_eq!(result_document.id, document_id, "Mismatched document ID.");

    assert_eq!(
        result_document.paste_id,
        Snowflake::new(517_815_304_354_284_601),
        "Mismatched paste ID."
    );

    assert_eq!(
        document.document_type, new_document_type,
        "Mismatched document type."
    );

    assert_eq!(result_document.name, new_name, "Mismatched document type.");

    assert_eq!(result_document.size, new_size, "Mismatched document size.");
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_delete(pool: PgPool) {
    let db = Database::from_pool(pool);

    let document_id = Snowflake::new(517_815_304_354_284_701);

    Document::fetch(db.pool(), document_id)
        .await
        .expect("Could not fetch a document.")
        .expect("No token found.");

    Document::delete(db.pool(), document_id)
        .await
        .expect("Failed to delete value from database.");

    let paste_token = Document::fetch(db.pool(), document_id)
        .await
        .expect("Could not fetch a token.");

    assert!(paste_token.is_none(), "Found paste_token in db.");
}

#[rstest]
#[case(&["application/json", "text/*"], "application/json", true)]
#[case(&["application/json", "text/*"], "text/plain", true)]
#[case(&["application/json", "text/*"], "image/png", false)]
fn test_contains_mime(#[case] mimes: &[&str], #[case] mime: &str, #[case] expected: bool) {
    assert!(
        contains_mime(mimes, mime) == expected,
        "Did not find {mime} in {mimes:?}."
    );
}

fn make_document_limits_config(
    minimum_document_size: usize,
    minimum_document_name_size: usize,
    maximum_document_size: usize,
    maximum_document_name_size: usize,
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
                .minimum_document_size(minimum_document_size)
                .minimum_document_name_size(minimum_document_name_size)
                .maximum_document_size(maximum_document_size)
                .maximum_document_name_size(maximum_document_name_size)
                .build()
                .expect("Failed to build rate limits"),
        )
        .rate_limits(
            RateLimitConfigBuilder::default()
                .build()
                .expect("Failed to build rate limits"),
        )
        .build()
        .expect("Failed to build config.")
}

#[test]
fn test_document_limits() {
    let document = Document::new(
        Snowflake::new(456),
        Snowflake::new(123),
        "text/plain".to_string(),
        "test_doc.txt".to_string(),
        489,
    );

    document_limits(&make_document_limits_config(1, 3, 1_000_000, 50), &document)
        .expect("An error occurred.");
}

#[rstest]
#[case(
    make_document_limits_config(1, 50, 1_000_000, 50),
    "The document name: `test_doc.txt` is too small."
)]
#[case(
    make_document_limits_config(1, 3, 1_000_000, 10),
    "The document name: `test_doc.txt...` is too large."
)]
#[case(
    make_document_limits_config(500, 3, 1_000_000, 50),
    "The document: `test_doc.txt` is too small."
)]
#[case(
    make_document_limits_config(1, 3, 250, 50),
    "The document: `test_doc.txt` is too large."
)]
fn test_document_limits_errors(#[case] config: Config, #[case] expected: &str) {
    let document = Document::new(
        Snowflake::new(456),
        Snowflake::new(123),
        "text/plain".to_string(),
        "test_doc.txt".to_string(),
        489,
    );

    let error = document_limits(&config, &document).expect_err("No error received.");

    if let AppError::BadRequest(bad_request) = error {
        assert_eq!(
            bad_request, expected,
            "The bad request message received was unexpected."
        );
    } else {
        panic!("The error received, was not expected.");
    }
}

fn make_total_document_limits_config(
    minimum_total_document_count: usize,
    minimum_total_document_size: usize,
    maximum_total_document_count: usize,
    maximum_total_document_size: usize,
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
                .minimum_total_document_count(minimum_total_document_count)
                .minimum_total_document_size(minimum_total_document_size)
                .maximum_total_document_count(maximum_total_document_count)
                .maximum_total_document_size(maximum_total_document_size)
                .build()
                .expect("Failed to build rate limits"),
        )
        .rate_limits(
            RateLimitConfigBuilder::default()
                .build()
                .expect("Failed to build rate limits"),
        )
        .build()
        .expect("Failed to build config.")
}

#[sqlx::test(fixtures("pastes", "documents"))]
async fn test_total_document_limits(pool: PgPool) {
    let db = Database::from_pool(pool);

    let mut transaction = db
        .pool()
        .begin()
        .await
        .expect("Failed to generate a transaction.");

    total_document_limits(
        &mut transaction,
        &make_total_document_limits_config(1, 1, 10, 10_000_000),
        Snowflake::new(517_815_304_354_284_601),
    )
    .await
    .expect("An error occurred.");
}

#[rstest]
#[case(
    make_total_document_limits_config(5, 1, 5, 5000),
    "One or more documents is below the minimum total document count."
)]
#[case(
    make_total_document_limits_config(1, 1, 1, 5000),
    "One or more documents exceed the maximum total document count."
)]
#[case(
    make_total_document_limits_config(1, 2500, 5, 5000),
    "One or more documents is below the minimum individual document size."
)]
#[case(
    make_total_document_limits_config(1, 1, 5, 2000),
    "One or more documents exceed the maximum individual document size."
)]
#[sqlx::test(fixtures("pastes", "documents"))]
async fn test_total_document_limits_errors(
    #[ignore] pool: PgPool,
    #[case] config: Config,
    #[case] expected: &str,
) {
    let db = Database::from_pool(pool);

    let mut transaction = db
        .pool()
        .begin()
        .await
        .expect("Failed to generate a transaction.");

    let error = total_document_limits(
        &mut transaction,
        &config,
        Snowflake::new(517_815_304_354_284_602),
    )
    .await
    .expect_err("No error received.");

    if let AppError::BadRequest(bad_request) = error {
        assert_eq!(
            bad_request, expected,
            "The bad request message received was unexpected."
        );
    } else {
        panic!("The error received, was not expected.");
    }
}
