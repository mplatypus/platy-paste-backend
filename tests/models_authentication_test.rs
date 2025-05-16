use std::collections::HashSet;

use base64::{Engine, prelude::BASE64_URL_SAFE};
use platy_paste::{
    app::database::Database,
    models::{authentication::*, snowflake::Snowflake},
};

use secrecy::{ExposeSecret, SecretString};

use sqlx::PgPool;
use time::UtcDateTime;

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

#[sqlx::test(fixtures("pastes", "tokens"))]
fn test_fetch(pool: PgPool) {
    let db = Database::from_pool(pool);

    let token_string = "NTE3ODE1MzA0MzU0NzYzNjUw.cMtyBLeeyOCsHyXjOyDZFRDUe".to_string();

    let token = Token::fetch(&db, token_string.clone())
        .await
        .expect("Could not fetch a token.")
        .expect("No token found.");

    assert_eq!(
        token.token().expose_secret(),
        token_string,
        "Mismatched token."
    );
    assert_eq!(
        token.paste_id(),
        Snowflake::new(517_815_304_354_763_650),
        "Mismatched paste ID."
    );
}

#[sqlx::test(fixtures("pastes", "tokens"))]
fn test_fetch_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let token = Token::fetch(&db, "missing.token".to_string())
        .await
        .expect("Could not fetch a token.");

    assert!(token.is_none(), "Token was found.");
}

#[sqlx::test(fixtures("pastes"))]
fn test_insert(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_763_650);
    let token = SecretString::from("test.token");

    let paste_token = Token::new(paste_id, token.clone());

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

    let result_token = Token::fetch(&db, token.expose_secret().to_string())
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste token was found.");

    assert_eq!(
        result_token.token().expose_secret(),
        token.expose_secret().to_string(),
        "Mismatched token."
    );
    assert_eq!(result_token.paste_id(), paste_id, "Mismatched paste ID.");
}

#[sqlx::test(fixtures("pastes", "tokens"))]
fn test_delete(pool: PgPool) {
    let db = Database::from_pool(pool);

    let token_string = "NTE3ODE1MzA0MzU0NzYzNjUw.cMtyBLeeyOCsHyXjOyDZFRDUe".to_string();

    Token::fetch(&db, token_string.clone())
        .await
        .expect("Could not fetch a token.")
        .expect("No token found.");

    Token::delete(&db, token_string.clone())
        .await
        .expect("Failed to delete value from database.");

    let paste_token = Token::fetch(&db, token_string.clone())
        .await
        .expect("Could not fetch a token.");

    assert!(paste_token.is_none(), "Found paste_token in db.");
}

#[test]
fn test_generate_token() {
    let current = UtcDateTime::now();

    let token =
        generate_token(Snowflake::new(517_815_304_354_763_650)).expect("Failed to generate token");

    let values: Vec<&str> = token.expose_secret().split('.').collect();

    assert_eq!(values.len(), 3);

    assert_eq!(
        values[0], "NTE3ODE1MzA0MzU0NzYzNjUw",
        "Base64 ID does not match."
    );

    let timestamp: i64 = String::from_utf8_lossy(
        &BASE64_URL_SAFE
            .decode(values[1])
            .expect("Failed to decode timestamp."),
    )
    .to_string()
    .parse()
    .expect("Failed to convert timestamp string to integer.");

    assert_eq!(timestamp, current.unix_timestamp());
}

#[test]
fn test_generate_token_uniqueness() {
    let snowflake = Snowflake::new(123);
    let tokens = vec![
        generate_token(snowflake)
            .expect("Failed to generate token")
            .expose_secret()
            .to_owned(),
        generate_token(snowflake)
            .expect("Failed to generate token")
            .expose_secret()
            .to_owned(),
        generate_token(snowflake)
            .expect("Failed to generate token")
            .expose_secret()
            .to_owned(),
        generate_token(snowflake)
            .expect("Failed to generate token")
            .expose_secret()
            .to_owned(),
        generate_token(snowflake)
            .expect("Failed to generate token")
            .expose_secret()
            .to_owned(),
        generate_token(snowflake)
            .expect("Failed to generate token")
            .expose_secret()
            .to_owned(),
        generate_token(snowflake)
            .expect("Failed to generate token")
            .expose_secret()
            .to_owned(),
        generate_token(snowflake)
            .expect("Failed to generate token")
            .expose_secret()
            .to_owned(),
        generate_token(snowflake)
            .expect("Failed to generate token")
            .expose_secret()
            .to_owned(),
        generate_token(snowflake)
            .expect("Failed to generate token")
            .expose_secret()
            .to_owned(),
    ];

    let set: HashSet<_> = tokens.iter().collect();

    assert!(
        set.len() == tokens.len(),
        "Non-unique snowflake(s) found: {tokens:?}"
    );
}
