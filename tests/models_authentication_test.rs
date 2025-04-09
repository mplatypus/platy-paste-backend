use platy_paste::{
    app::database::Database,
    models::{authentication::*, snowflake::Snowflake},
};

use secrecy::{ExposeSecret, SecretString};

use sqlx::PgPool;

#[test]
fn test_getters() {
    let paste_id = Snowflake::new(123);
    let token = SecretString::from("test.token");

    let paste_token = Token::new(paste_id, token.clone());

    assert!(paste_token.paste_id() == paste_id, "Mismatched paste ID.");

    assert!(
        paste_token.token().expose_secret() == token.expose_secret(),
        "Mismatched token."
    );
}

#[sqlx::test]
fn test_fetch(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let token = SecretString::from("test.token");

    let paste_db_id: i64 = paste_id.into();

    sqlx::query!(
        "INSERT INTO pastes(id, edited) VALUES ($1, false)",
        paste_db_id,
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO paste_tokens(paste_id, token) VALUES ($1, $2)",
        paste_db_id,
        token.expose_secret()
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    let paste_token = Token::fetch(&db, token.expose_secret().to_string())
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste token was found.");

    assert!(paste_token.paste_id() == paste_id, "Mismatched paste ID.");

    assert!(
        paste_token.token().expose_secret() == token.expose_secret(),
        "Mismatched token."
    );
}

#[sqlx::test]
fn test_fetch_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let token = SecretString::from("test.token");

    assert!(
        Token::fetch(&db, token.expose_secret().to_string())
            .await
            .expect("Failed to fetch value from database.")
            .is_none()
    );
}

#[sqlx::test]
fn test_insert(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let token = SecretString::from("test.token");

    let paste_token = Token::new(paste_id, token.clone());

    let paste_db_id: i64 = paste_id.into();

    sqlx::query!(
        "INSERT INTO pastes(id, edited) VALUES ($1, false)",
        paste_db_id,
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    let mut transaction = db
        .pool()
        .begin()
        .await
        .expect("Failed to make transaction.");

    paste_token
        .insert(&mut transaction)
        .await
        .expect("Failed to insert paste token");

    transaction
        .commit()
        .await
        .expect("Failed to commit transaction");

    let result = sqlx::query!(
        "SELECT paste_id, token FROM paste_tokens WHERE token = $1",
        token.expose_secret()
    )
    .fetch_optional(db.pool())
    .await
    .expect("Failed to fetch value from database.")
    .expect("No paste token was found.");

    assert!(
        result.paste_id as u64 == paste_id.id(),
        "Mismatched paste ID."
    );

    assert!(result.token == token.expose_secret(), "Mismatched token.");
}

#[sqlx::test]
fn test_delete(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let token = SecretString::from("test.token");

    let paste_db_id: i64 = paste_id.into();

    sqlx::query!(
        "INSERT INTO pastes(id, edited) VALUES ($1, false)",
        paste_db_id,
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO paste_tokens(paste_id, token) VALUES ($1, $2)",
        paste_db_id,
        token.expose_secret()
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    let result = sqlx::query!(
        "SELECT paste_id, token FROM paste_tokens WHERE token = $1",
        token.expose_secret()
    )
    .fetch_optional(db.pool())
    .await
    .expect("Failed to fetch value from database.")
    .expect("No paste token was found.");

    assert!(
        result.paste_id as u64 == paste_id.id(),
        "Mismatched paste ID."
    );

    assert!(result.token == token.expose_secret(), "Mismatched token.");

    Token::delete(&db, token.expose_secret().to_string())
        .await
        .expect("Failed to delete value from database.");

    assert!(
        sqlx::query!(
            "SELECT paste_id, token FROM paste_tokens WHERE token = $1",
            token.expose_secret()
        )
        .fetch_optional(db.pool())
        .await
        .expect("Failed to fetch value from database.")
        .is_none(),
        "Found paste_token in db."
    );
}
