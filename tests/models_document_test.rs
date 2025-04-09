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

    let document = Document::new(document_id, paste_id, document_type.clone(), name.clone());

    assert!(document.id == document_id, "Mismatched document ID.");

    assert!(document.paste_id == paste_id, "Mismatched paste ID.");

    assert!(
        document.document_type == document_type,
        "Mismatched document type."
    );

    assert!(document.name == name, "Mismatched name.");

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
fn test_setters() {
    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);
    let document_type = String::from("example/document");
    let name = String::from("test.document");

    let document = Document::new(document_id, paste_id, document_type.clone(), name.clone());

    assert!(document.id == document_id, "Mismatched document ID.");

    assert!(document.paste_id == paste_id, "Mismatched paste ID.");

    assert!(
        document.document_type == document_type,
        "Mismatched document type."
    );

    assert!(document.name == name, "Mismatched name.");
}

#[sqlx::test]
fn test_fetch(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);
    let document_type = String::from("example/document");
    let name = String::from("test.document");

    let document_db_id: i64 = document_id.into();
    let paste_db_id: i64 = paste_id.into();

    sqlx::query!(
        "INSERT INTO pastes(id, edited) VALUES ($1, false)",
        paste_db_id
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO documents(id, paste_id, type, name) VALUES ($1, $2, $3, $4)",
        document_db_id,
        paste_db_id,
        document_type,
        name
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    let document = Document::fetch(&db, document_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No document was found.");

    assert!(document.id == document_id, "Mismatched document ID.");

    assert!(document.paste_id == paste_id, "Mismatched paste ID.");

    assert!(
        document.document_type == document_type,
        "Mismatched document type."
    );

    assert!(document.name == name, "Mismatched document type.");
}

#[sqlx::test]
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

#[sqlx::test]
fn test_fetch_all(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let document_id_1 = Snowflake::new(456);
    let document_id_2 = Snowflake::new(789);
    let document_id_3 = Snowflake::new(101);

    let paste_db_id: i64 = paste_id.into();
    let document_db_id_1: i64 = document_id_1.into();
    let document_db_id_2: i64 = document_id_2.into();
    let document_db_id_3: i64 = document_id_3.into();

    sqlx::query!(
        "INSERT INTO pastes(id, edited) VALUES ($1, false)",
        paste_db_id
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO documents(id, paste_id, type, name) VALUES ($1, $2, 'example', 'test.example')",
        document_db_id_1,
        paste_db_id,
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO documents(id, paste_id, type, name) VALUES ($1, $2, 'example', 'test.example')",
        document_db_id_2,
        paste_db_id,
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO documents(id, paste_id, type, name) VALUES ($1, $2, 'example', 'test.example')",
        document_db_id_3,
        paste_db_id,
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    let documents = Document::fetch_all(&db, paste_id)
        .await
        .expect("Failed to fetch value from database.");

    assert!(
        documents.len() == 3,
        "Not enough or too many results received."
    );

    assert!(
        documents[0].id == document_id_1,
        "Mismatched document ID 1."
    );

    assert!(documents[0].paste_id == paste_id, "Mismatched paste ID 1.");

    assert!(
        documents[1].id == document_id_2,
        "Mismatched document ID 2."
    );

    assert!(documents[1].paste_id == paste_id, "Mismatched paste ID 2.");

    assert!(
        documents[2].id == document_id_3,
        "Mismatched document ID 3."
    );

    assert!(documents[2].paste_id == paste_id, "Mismatched paste ID 3.");
}

#[sqlx::test]
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

#[sqlx::test]
fn test_insert(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);
    let document_type = String::from("example/document");
    let name = String::from("test.document");

    let document = Document::new(document_id, paste_id, document_type.clone(), name.clone());

    let paste_db_id: i64 = paste_id.into();
    let document_db_id: i64 = document_id.into();

    sqlx::query!(
        "INSERT INTO pastes(id, edited) VALUES ($1, false)",
        paste_db_id
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

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

    let result = sqlx::query!(
        "SELECT id, paste_id, type, name FROM documents WHERE id = $1",
        document_db_id
    )
    .fetch_optional(db.pool())
    .await
    .expect("Failed to fetch value from database.")
    .expect("No document was found.");

    assert!(
        result.id as u64 == document_id.id(),
        "Mismatched document ID."
    );

    assert!(
        result.paste_id as u64 == paste_id.id(),
        "Mismatched paste ID."
    );

    assert!(result.r#type == document_type, "Mismatched document type.");

    assert!(result.name == name, "Mismatched document type.");
}

#[sqlx::test]
fn test_update(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);
    let document_type = String::from("example/document");
    let name = String::from("test.document");

    let mut document = Document::new(document_id, paste_id, document_type.clone(), name.clone());

    let paste_db_id: i64 = paste_id.into();
    let document_db_id: i64 = document_id.into();

    sqlx::query!(
        "INSERT INTO pastes(id, edited) VALUES ($1, false)",
        paste_db_id
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    let mut transaction = db
        .pool()
        .begin()
        .await
        .expect("Failed to make transaction.");

    document
        .insert(&mut transaction)
        .await
        .expect("Failed to insert paste.");

    transaction
        .commit()
        .await
        .expect("Failed to commit transaction");

    let result = sqlx::query!(
        "SELECT id, paste_id, type, name FROM documents WHERE id = $1",
        document_db_id
    )
    .fetch_optional(db.pool())
    .await
    .expect("Failed to fetch value from database.")
    .expect("No document was found.");

    assert!(
        result.id as u64 == document_id.id(),
        "Mismatched document ID."
    );

    assert!(
        result.paste_id as u64 == paste_id.id(),
        "Mismatched paste ID."
    );

    assert!(result.r#type == document_type, "Mismatched document type.");

    assert!(result.name == name, "Mismatched document type.");

    let mut transaction = db
        .pool()
        .begin()
        .await
        .expect("Failed to make transaction.");

    let new_document_type = String::from("example/new");

    let new_name = String::from("test_new_name");

    document.set_document_type(new_document_type.clone());

    document.set_name(new_name.clone());

    document
        .update(&mut transaction)
        .await
        .expect("Failed to update paste.");

    transaction
        .commit()
        .await
        .expect("Failed to commit transaction");

    let result = sqlx::query!(
        "SELECT id, paste_id, type, name FROM documents WHERE id = $1",
        document_db_id
    )
    .fetch_optional(db.pool())
    .await
    .expect("Failed to fetch value from database.")
    .expect("No paste was found.");

    assert!(
        result.id as u64 == document_id.id(),
        "Mismatched document ID."
    );

    assert!(
        result.paste_id as u64 == paste_id.id(),
        "Mismatched paste ID."
    );

    assert!(
        result.r#type == new_document_type,
        "Mismatched document type."
    );

    assert!(result.name == new_name, "Mismatched document type.");
}

#[sqlx::test]
fn test_delete(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let document_id = Snowflake::new(456);
    let document_type = String::from("example/document");
    let name = String::from("test.document");

    let paste_db_id: i64 = paste_id.into();
    let document_db_id: i64 = document_id.into();

    sqlx::query!(
        "INSERT INTO pastes(id, edited) VALUES ($1, false)",
        paste_db_id
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO documents(id, paste_id, type, name) VALUES ($1, $2, $3, $4)",
        document_db_id,
        paste_db_id,
        document_type,
        name
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    let result = sqlx::query!(
        "SELECT id, paste_id, type, name FROM documents WHERE id = $1",
        document_db_id
    )
    .fetch_optional(db.pool())
    .await
    .expect("Failed to fetch value from database.")
    .expect("No paste was found.");

    assert!(
        result.id as u64 == document_id.id(),
        "Mismatched document ID."
    );

    assert!(
        result.paste_id as u64 == paste_id.id(),
        "Mismatched paste ID."
    );

    assert!(result.r#type == document_type, "Mismatched document type.");

    assert!(result.name == name, "Mismatched document type.");

    Document::delete(&db, document_id)
        .await
        .expect("Failed to delete value from database.");

    assert!(
        sqlx::query!(
            "SELECT id, paste_id, type, name FROM documents WHERE id = $1",
            document_db_id
        )
        .fetch_optional(db.pool())
        .await
        .expect("Failed to fetch value from database.")
        .is_none(),
        "Found document in db."
    );
}
