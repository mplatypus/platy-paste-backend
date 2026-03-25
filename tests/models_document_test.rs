//! Tests for document model objects.

use platy_paste::{
    app::database::Database,
    models::{document::*, snowflake::Snowflake, undefined::Undefined},
};

use rstest::*;
use sqlx::PgPool;

#[test]
fn test_getters() {
    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);
    let doc_type = "example/document";
    let name = "test.document";
    let size = 329;

    let document = Document::new(document_id, paste_id, doc_type, name, size);

    assert_eq!(document.id(), &document_id, "Mismatched document ID.");

    assert_eq!(document.paste_id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(document.doc_type(), doc_type, "Mismatched document type.");

    assert_eq!(document.name(), name, "Mismatched name.");

    assert_eq!(document.size(), size, "Mismatched size.");

    assert_eq!(
        document.generate_url("http://example.com"),
        format!("http://example.com/documents/{paste_id}/{document_id}/{name}"),
        "Mismatched URL."
    );

    assert_eq!(
        document.generate_path(),
        format!("{paste_id}/{document_id}/{name}"),
        "Mismatched path."
    );
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch(pool: PgPool) {
    let db = Database::from_pool(pool);

    let document_id = Snowflake::new(517_815_304_354_284_701);

    let document = Document::fetch(db.pool(), &document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No document was found.");

    assert_eq!(document.id(), &document_id, "Mismatched document ID.");

    assert_eq!(
        document.paste_id(),
        &Snowflake::new(517_815_304_354_284_601),
        "Mismatched paste ID."
    );

    assert_eq!(
        document.doc_type(),
        "plain/text",
        "Mismatched document type."
    );

    assert_eq!(document.name(), "cool.txt", "Mismatched document type.");

    assert_eq!(document.size(), 811, "Mismatched document size.");
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let document_id = Snowflake::new(456);

    assert!(
        Document::fetch(db.pool(), &document_id)
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

    let document = Document::fetch_with_paste(db.pool(), &paste_id, &document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No document was found.");

    assert_eq!(document.id(), &document_id, "Mismatched document ID.");

    assert_eq!(document.paste_id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(
        document.doc_type(),
        "plain/text",
        "Mismatched document type."
    );

    assert_eq!(document.name(), "cool.txt", "Mismatched document type.");

    assert_eq!(document.size(), 811, "Mismatched document size.");
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_with_paste_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);

    assert!(
        Document::fetch_with_paste(db.pool(), &paste_id, &document_id)
            .await
            .expect("Failed to fetch value from database.")
            .is_none()
    );
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_all(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_284_603);

    let documents = Document::fetch_all(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.");

    assert_eq!(
        documents.len(),
        3,
        "Not enough or too many results received."
    );

    assert_eq!(
        documents[0].id(),
        &Snowflake::new(517_815_304_354_284_704),
        "Mismatched document ID 1."
    );

    assert_eq!(documents[0].paste_id(), &paste_id, "Mismatched paste ID 1.");

    assert_eq!(
        documents[1].id(),
        &Snowflake::new(517_815_304_354_284_705),
        "Mismatched document ID 2."
    );

    assert_eq!(documents[1].paste_id(), &paste_id, "Mismatched paste ID 2.");

    assert_eq!(
        documents[2].id(),
        &Snowflake::new(517_815_304_354_284_706),
        "Mismatched document ID 3."
    );

    assert_eq!(documents[2].paste_id(), &paste_id, "Mismatched paste ID 3.");
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_all_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(456);

    assert!(
        Document::fetch_all(db.pool(), &paste_id)
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
    let doc_type = "example/document";
    let name = "test.document";
    let size = 475;

    let document = Document::new(document_id, paste_id, doc_type, name, size);

    document
        .insert(db.pool())
        .await
        .expect("Failed to insert paste");

    let result = Document::fetch(db.pool(), &document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No document was found.");

    assert_eq!(result.id(), &document_id, "Mismatched document ID.");

    assert_eq!(result.paste_id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(result.doc_type(), doc_type, "Mismatched document type.");

    assert_eq!(result.name(), name, "Mismatched document type.");

    assert_eq!(result.size(), size);
}

#[rstest]
#[case(
    DocumentUpdateParameters::new(
        Undefined::Undefined,
        Undefined::Undefined,
        Undefined::Undefined,
    ),
    "plain/text",
    "cool.txt",
    811,
    false
)]
#[case(
    DocumentUpdateParameters::new(
        Undefined::Some("text/plain".to_string()),
        Undefined::Undefined,
        Undefined::Undefined,
    ),
    "text/plain",
    "cool.txt",
    811,
    true,
)]
#[case(
    DocumentUpdateParameters::new(
        Undefined::Undefined,
        Undefined::Some("updated.txt".to_string()),
        Undefined::Undefined,
    ),
    "plain/text",
    "updated.txt",
    811,
    true,
)]
#[case(
    DocumentUpdateParameters::new(
        Undefined::Undefined,
        Undefined::Undefined,
        Undefined::Some(400),
    ),
    "plain/text",
    "cool.txt",
    400,
    true
)]
#[case(
    DocumentUpdateParameters::new(
        Undefined::Some("text/plain".to_string()),
        Undefined::Some("updated.txt".to_string()),
        Undefined::Some(400),
    ),
    "text/plain",
    "updated.txt",
    400,
    true,
)]
#[sqlx::test(fixtures("pastes", "documents"))]
async fn test_update(
    #[ignore] pool: PgPool,
    #[case] parameters: DocumentUpdateParameters,
    #[case] doc_type: &str,
    #[case] name: &str,
    #[case] size: usize,
    #[case] was_updated: bool,
) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_284_601);
    let document_id = Snowflake::new(517_815_304_354_284_701);

    let mut document = Document::fetch(db.pool(), &document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("Document was not found.");

    assert_eq!(document.id(), &document_id, "Mismatched document ID.");

    assert_eq!(
        document.paste_id(),
        &Snowflake::new(517_815_304_354_284_601),
        "Mismatched paste ID."
    );

    assert_eq!(
        document.doc_type(),
        "plain/text",
        "Mismatched document type."
    );

    assert_eq!(document.name(), "cool.txt", "Mismatched document type.");

    assert_eq!(document.size(), 811, "Mismatched document size.");

    let updated = document
        .update(db.pool(), parameters)
        .await
        .expect("Failed to fetch value from database.");

    assert_eq!(updated, was_updated, "The updated parameter did not match.");

    let result_document = Document::fetch(db.pool(), &document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("Document was not found.");

    assert_eq!(
        result_document.id(),
        &document_id,
        "Mismatched document ID."
    );

    assert_eq!(document.id(), &document_id, "Mismatched document ID.");

    assert_eq!(
        result_document.paste_id(),
        &paste_id,
        "Mismatched paste ID."
    );

    assert_eq!(document.paste_id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(
        result_document.doc_type(),
        doc_type,
        "Mismatched document type."
    );

    assert_eq!(document.doc_type(), doc_type, "Mismatched document type.");

    assert_eq!(result_document.name(), name, "Mismatched document type.");

    assert_eq!(document.name(), name, "Mismatched document type.");

    assert_eq!(result_document.size(), size, "Mismatched document size.");

    assert_eq!(document.size(), size, "Mismatched document size.");
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_delete(pool: PgPool) {
    let db = Database::from_pool(pool);

    let document_id = Snowflake::new(517_815_304_354_284_701);

    Document::fetch(db.pool(), &document_id)
        .await
        .expect("Could not fetch a document.")
        .expect("No token found.");

    Document::delete(db.pool(), &document_id)
        .await
        .expect("Failed to delete value from database.");

    let paste_token = Document::fetch(db.pool(), &document_id)
        .await
        .expect("Could not fetch a token.");

    assert!(paste_token.is_none(), "Found paste_token in db.");
}

#[rstest]
#[case(&["application/json", "text/*"], "application/json", true)]
#[case(&["application/json", "text/*"], "text/plain", true)]
#[case(&["application/json", "text/*"], "image/png", false)]
fn test_contains_mime(#[case] mimes: &[&str], #[case] mime: &str, #[case] expected: bool) {
    assert_eq!(
        contains_mime(mimes, mime),
        expected,
        "Did not find {mime} in {mimes:?}."
    );
}
