use platy_paste::{
    app::database::Database,
    models::{paste::*, snowflake::Snowflake},
};

use sqlx::PgPool;
use time::{Date, OffsetDateTime, Time};

#[test]
fn test_getters() {
    let paste_id = Snowflake::new(123);
    let edited = false;
    let expiry = OffsetDateTime::from_unix_timestamp(0).expect("failed to generate timestamp.");

    let paste = Paste::new(paste_id, edited, Some(expiry));

    assert!(paste.id == paste_id, "Mismatched paste ID.");

    assert!(!paste.edited, "Mismatched edited.");

    assert!(paste.expiry == Some(expiry), "Mismatched expiry.");
}

#[test]
fn test_set_edited() {
    let paste_id = Snowflake::new(123);
    let edited = false;
    let expiry = OffsetDateTime::from_unix_timestamp(0).expect("failed to generate timestamp.");

    let mut paste = Paste::new(paste_id, edited, Some(expiry));

    assert!(!paste.edited, "Mismatched edited.");

    paste.set_edited();

    assert_eq!(paste.id, paste_id, "Mismatched paste ID.");

    assert!(paste.edited, "Mismatched edited.");

    assert_eq!(paste.expiry, Some(expiry), "Mismatched expiry.");
}

#[test]
fn test_set_expiry() {
    let paste_id = Snowflake::new(123);
    let edited = false;
    let expiry = OffsetDateTime::from_unix_timestamp(0).expect("failed to generate timestamp.");

    let mut paste = Paste::new(paste_id, edited, Some(expiry));

    assert_eq!(paste.expiry, Some(expiry), "Mismatched expiry.");

    paste.set_expiry(None);

    assert_eq!(paste.id, paste_id, "Mismatched paste ID.");

    assert!(!paste.edited, "Mismatched edited.");

    assert!(paste.expiry.is_none(), "Mismatched expiry.");
}

#[sqlx::test(fixtures("pastes"))]
fn test_fetch(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_763_650);
    let expiry = OffsetDateTime::from_unix_timestamp(0).expect("failed to generate timestamp.");

    let paste = Paste::fetch(&db, paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert_eq!(paste.id, paste_id, "Mismatched paste ID.");

    assert!(!paste.edited, "Mismatched edited.");

    assert_eq!(paste.expiry, Some(expiry), "Mismatched expiry.");
}

#[sqlx::test]
fn test_fetch_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let id = Snowflake::new(123);

    assert!(
        Paste::fetch(&db, id)
            .await
            .expect("Failed to fetch value from database.")
            .is_none()
    );
}

#[sqlx::test(fixtures("pastes"))]
fn test_fetch_between(pool: PgPool) {
    let db = Database::from_pool(pool);

    let results = Paste::fetch_between(
        &db,
        OffsetDateTime::new_utc(
            Date::from_calendar_date(1970, time::Month::January, 2)
                .expect("Failed to build date start."),
            Time::from_hms(0, 0, 0).expect("Failed to build time start."),
        ),
        OffsetDateTime::new_utc(
            Date::from_calendar_date(1970, time::Month::January, 4)
                .expect("Failed to build date end."),
            Time::from_hms(0, 0, 0).expect("Failed to build time end."),
        ),
    )
    .await
    .expect("Failed to fetch value from database.");

    assert_eq!(results.len(), 1, "Not enough or too many results received.");

    assert_eq!(
        results[0].expiry,
        Some(OffsetDateTime::from_unix_timestamp(172_800).expect("Failed to build result time.")),
        "Invalid expiry. Expected: {:?}, Received: {:?}",
        Some(OffsetDateTime::from_unix_timestamp(172_800).expect("Failed to build result time.")),
        results[0].expiry,
    );
}

#[sqlx::test(fixtures("pastes"))]
fn test_fetch_between_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let results = Paste::fetch_between(
        &db,
        OffsetDateTime::new_utc(
            Date::from_calendar_date(2000, time::Month::January, 1)
                .expect("Failed to build date start."),
            Time::from_hms(0, 0, 0).expect("Failed to build time start."),
        ),
        OffsetDateTime::new_utc(
            Date::from_calendar_date(2001, time::Month::January, 1)
                .expect("Failed to build date end."),
            Time::from_hms(0, 0, 0).expect("Failed to build time end."),
        ),
    )
    .await
    .expect("Failed to fetch value from database.");

    assert!(
        results.is_empty(),
        "Received items, when none were expected."
    );
}

#[sqlx::test(fixtures("pastes"))]
fn test_insert(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let edited = false;
    let expiry = OffsetDateTime::from_unix_timestamp(0).expect("failed to generate timestamp.");

    let paste = Paste::new(paste_id, edited, Some(expiry));

    let mut transaction = db
        .pool()
        .begin()
        .await
        .expect("Failed to make transaction.");

    paste
        .insert(&mut transaction)
        .await
        .expect("Failed to insert paste");

    transaction
        .commit()
        .await
        .expect("Failed to commit transaction");

    let result = Paste::fetch(&db, paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert!(result.id == paste_id, "Mismatched paste ID.");

    assert!(!result.edited, "Mismatched edited.");

    assert!(result.expiry == Some(expiry), "Mismatched expiry.");
}

#[sqlx::test(fixtures("pastes"))]
fn test_update(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_763_650);

    let mut paste = Paste::fetch(&db, paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert_eq!(paste.id, paste_id, "Mismatched paste ID.");

    assert!(!paste.edited, "Mismatched edited.");

    assert_eq!(
        paste.expiry,
        Some(OffsetDateTime::from_unix_timestamp(0).expect("Failed to build expected timestamp.")),
        "Mismatched expiry."
    );

    let mut transaction = db
        .pool()
        .begin()
        .await
        .expect("Failed to make transaction.");

    paste.set_edited();

    paste.set_expiry(None);

    paste
        .update(&mut transaction)
        .await
        .expect("Failed to update paste.");

    transaction
        .commit()
        .await
        .expect("Failed to commit transaction");

    let result = Paste::fetch(&db, paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert!(result.edited, "Mismatched edited.");

    assert!(result.expiry.is_none(), "Mismatched expiry.");
}

#[sqlx::test(fixtures("pastes"))]
fn test_delete(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_763_650);

    Paste::fetch(&db, paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    Paste::delete(&db, paste_id)
        .await
        .expect("Failed to delete value from database.");

    let result = Paste::fetch(&db, paste_id)
        .await
        .expect("Failed to fetch value from database.");

    assert!(result.is_none(), "Found paste in db.");
}
