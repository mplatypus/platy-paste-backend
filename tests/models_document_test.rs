use platy_paste::{
    app::database::Database,
    models::{document::*, snowflake::Snowflake},
};

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
            == format!("http://example.com/documents/{paste_id}/{document_id}-{name}"),
        "Mismatched URL."
    );

    assert!(
        document.generate_path() == format!("{paste_id}/{document_id}-{name}"),
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

    let document_id = Snowflake::new(517_815_304_355_368_628);

    let document = Document::fetch(&db, document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No document was found.");

    assert_eq!(document.id, document_id, "Mismatched document ID.");

    assert_eq!(
        document.paste_id,
        Snowflake::new(517_815_304_354_763_650),
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
        Document::fetch(&db, document_id)
            .await
            .expect("Failed to fetch value from database.")
            .is_none()
    );
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_with_paste(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_763_650);
    let document_id = Snowflake::new(517_815_304_355_368_628);

    let document = Document::fetch_with_paste(&db, paste_id, document_id)
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
        Document::fetch_with_paste(&db, paste_id, document_id)
            .await
            .expect("Failed to fetch value from database.")
            .is_none()
    );
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_all(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_355_658_605);

    let documents = Document::fetch_all(&db, paste_id)
        .await
        .expect("Failed to fetch value from database.");

    assert!(
        documents.len() == 3,
        "Not enough or too many results received."
    );

    assert!(
        documents[0].id == Snowflake::new(517_815_304_353_663_922),
        "Mismatched document ID 1."
    );

    assert!(documents[0].paste_id == paste_id, "Mismatched paste ID 1.");

    assert!(
        documents[1].id == Snowflake::new(517_815_304_357_264_399),
        "Mismatched document ID 2."
    );

    assert!(documents[1].paste_id == paste_id, "Mismatched paste ID 2.");

    assert!(
        documents[2].id == Snowflake::new(517_815_304_353_794_606),
        "Mismatched document ID 3."
    );

    assert!(documents[2].paste_id == paste_id, "Mismatched paste ID 3.");
}

#[sqlx::test(fixtures("pastes", "documents"))]
fn test_fetch_all_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(456);

    assert!(
        Document::fetch_all(&db, paste_id)
            .await
            .expect("Failed to fetch value from database.")
            .is_empty()
    );
}

#[sqlx::test(fixtures("pastes"))]
fn test_insert(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_763_650);
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

    let mut transaction = db
        .pool()
        .begin()
        .await
        .expect("Failed to make transaction.");

    document
        .insert(&mut transaction)
        .await
        .expect("Failed to insert paste");

    transaction
        .commit()
        .await
        .expect("Failed to commit transaction");

    let result = Document::fetch(&db, document_id)
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

    let document_id = Snowflake::new(517_815_304_355_368_628);

    let mut document = Document::fetch(&db, document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("Document was not found.");

    assert_eq!(document.id, document_id, "Mismatched document ID.");

    assert_eq!(
        document.paste_id,
        Snowflake::new(517_815_304_354_763_650),
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

    let mut transaction = db
        .pool()
        .begin()
        .await
        .expect("Failed to make transaction.");

    document
        .update(&mut transaction)
        .await
        .expect("Failed to update paste.");

    transaction
        .commit()
        .await
        .expect("Failed to commit transaction");

    let result_document = Document::fetch(&db, document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("Document was not found.");

    assert_eq!(result_document.id, document_id, "Mismatched document ID.");

    assert_eq!(
        result_document.paste_id,
        Snowflake::new(517_815_304_354_763_650),
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

    let document_id = Snowflake::new(517_815_304_355_368_628);

    Document::fetch(&db, document_id)
        .await
        .expect("Could not fetch a document.")
        .expect("No token found.");

    Document::delete(&db, document_id)
        .await
        .expect("Failed to delete value from database.");

    let paste_token = Document::fetch(&db, document_id)
        .await
        .expect("Could not fetch a token.");

    assert!(paste_token.is_none(), "Found paste_token in db.");
}

#[test]
fn test_contains_mime() {
    let mimes: &[&str] = &["application/json", "text/*"];

    assert!(
        contains_mime(mimes, "application/json"),
        "Did not find application/json in mimes."
    );

    assert!(
        contains_mime(mimes, "text/plain"),
        "Did not find text/plain in mimes."
    );

    assert!(
        !contains_mime(mimes, "image/png"),
        "Found image/png in mimes."
    );
}
