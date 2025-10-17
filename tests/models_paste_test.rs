use chrono::{DateTime, TimeZone, Timelike, Utc};
use platy_paste::{
    app::database::Database,
    models::{paste::*, snowflake::Snowflake},
};

use sqlx::PgPool;

#[test]
fn test_getters() {
    let paste_id = Snowflake::new(123);
    let creation = DateTime::from_timestamp(10, 0).expect("failed to generate timestamp.");
    let edited = DateTime::from_timestamp(15, 0).expect("failed to generate timestamp.");
    let expiry = DateTime::from_timestamp(20, 0).expect("failed to generate timestamp.");

    let paste = Paste::new(
        paste_id,
        creation,
        Some(edited),
        Some(expiry),
        567,
        Some(1000),
    );

    assert_eq!(paste.id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(paste.edited(), Some(&edited), "Mismatched edited.");

    assert_eq!(paste.expiry(), Some(&expiry), "Mismatched expiry.");

    assert_eq!(paste.views(), 567, "Mismatched views.");

    assert_eq!(paste.max_views(), Some(1000), "Mismatched max views.");
}

#[test]
fn test_set_edited() {
    let paste_id = Snowflake::new(123);
    let creation = DateTime::from_timestamp(10, 0).expect("failed to generate timestamp.");
    let expiry = DateTime::from_timestamp(20, 0).expect("failed to generate timestamp.");

    let mut paste = Paste::new(paste_id, creation, None, Some(expiry), 567, Some(1000));

    assert_eq!(paste.edited(), None, "Mismatched edited.");

    let current = Utc::now();
    paste.set_edited().expect("Failed to set edited timestamp.");

    assert_eq!(paste.id(), &paste_id, "Mismatched paste ID.");

    let paste_edited = paste.edited().expect("Edited was not found.");

    assert_eq!(
        paste_edited.date_naive(),
        current.date_naive(),
        "Mismatched edited Date."
    );
    assert_eq!(
        paste_edited
            .time()
            .with_nanosecond(0)
            .expect("Failed to build current time with reset nanosecond."),
        current
            .time()
            .with_nanosecond(0)
            .expect("Failed to build current time with reset nanosecond."),
        "Mismatched edited HMS."
    );

    assert_eq!(paste.expiry(), Some(&expiry), "Mismatched expiry.");

    assert_eq!(paste.views(), 567, "Mismatched views.");

    assert_eq!(paste.max_views(), Some(1000), "Mismatched max views.");
}

#[test]
fn test_set_expiry() {
    let paste_id = Snowflake::new(123);
    let creation = DateTime::from_timestamp(10, 0).expect("failed to generate timestamp.");
    let expiry = DateTime::from_timestamp(0, 0).expect("failed to generate timestamp.");

    let mut paste = Paste::new(paste_id, creation, None, Some(expiry), 567, Some(1000));

    assert_eq!(paste.expiry(), Some(&expiry), "Mismatched expiry.");

    paste.set_expiry(None);

    assert_eq!(paste.id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(paste.edited(), None, "Mismatched edited.");

    assert!(paste.expiry().is_none(), "Mismatched expiry.");

    assert_eq!(paste.views(), 567, "Mismatched views.");

    assert_eq!(paste.max_views(), Some(1000), "Mismatched max views.");
}

#[sqlx::test(fixtures("pastes"))]
fn test_fetch(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_284_601);
    let creation = DateTime::from_timestamp(0, 0).expect("failed to generate creation timestamp.");
    let edited = DateTime::from_timestamp(86400, 0).expect("failed to generate edited timestamp.");
    let expiry =
        DateTime::from_timestamp(172_800, 0).expect("failed to generate expiry timestamp.");

    let paste = Paste::fetch(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert_eq!(paste.id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(paste.creation(), &creation, "Mismatched creation time.");

    assert_eq!(paste.edited(), Some(&edited), "Mismatched edited time.");

    assert_eq!(paste.expiry(), Some(&expiry), "Mismatched expiry time.");

    assert_eq!(paste.views(), 567, "Mismatched views.");

    assert_eq!(paste.max_views(), Some(1000), "Mismatched max views.");
}

#[sqlx::test]
fn test_fetch_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let id = Snowflake::new(123);

    assert!(
        Paste::fetch(db.pool(), &id)
            .await
            .expect("Failed to fetch value from database.")
            .is_none()
    );
}

#[sqlx::test(fixtures("pastes"))]
fn test_fetch_between(pool: PgPool) {
    let db = Database::from_pool(pool);

    let results = Paste::fetch_between(
        db.pool(),
        &Utc.with_ymd_and_hms(1970, 1, 2, 0, 0, 0).unwrap(),
        &Utc.with_ymd_and_hms(1970, 1, 4, 0, 0, 0).unwrap(),
    )
    .await
    .expect("Failed to fetch value from database.");

    assert_eq!(results.len(), 1, "Not enough or too many results received.");

    assert_eq!(
        results[0].expiry(),
        Some(&DateTime::from_timestamp(172_800, 0).expect("Failed to build result time.")),
        "Invalid expiry. Expected: {:?}, Received: {:?}",
        Some(&DateTime::from_timestamp(172_800, 0).expect("Failed to build result time.")),
        results[0].expiry(),
    );
}

#[sqlx::test(fixtures("pastes"))]
fn test_fetch_between_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let results = Paste::fetch_between(
        db.pool(),
        &Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(),
        &Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).unwrap(),
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
    let creation = DateTime::from_timestamp(10, 0).expect("failed to generate timestamp.");
    let edited = DateTime::from_timestamp(15, 0).expect("failed to generate timestamp.");
    let expiry = DateTime::from_timestamp(20, 0).expect("failed to generate timestamp.");

    let paste = Paste::new(
        paste_id,
        creation,
        Some(edited),
        Some(expiry),
        53489,
        Some(100_000),
    );

    paste
        .insert(db.pool())
        .await
        .expect("Failed to insert paste");

    let result = Paste::fetch(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert_eq!(result.id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(result.creation(), &creation, "Mismatched creation time.");

    assert_eq!(result.edited(), Some(&edited), "Mismatched edited time.");

    assert_eq!(result.expiry(), Some(&expiry), "Mismatched expiry time.");

    assert_eq!(paste.views(), 53489, "Mismatched views.");

    assert_eq!(paste.max_views(), Some(100_000), "Mismatched max views.");
}

#[sqlx::test(fixtures("pastes"))]
fn test_update(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_284_601);
    let mut paste = Paste::fetch(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert_eq!(paste.id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(
        paste.edited(),
        Some(&DateTime::from_timestamp(86400, 0).expect("failed to generate timestamp.")),
        "Mismatched edited time."
    );

    assert_eq!(
        paste.expiry(),
        Some(&DateTime::from_timestamp(172_800, 0).expect("Failed to build expected timestamp.")),
        "Mismatched expiry time."
    );

    let current = Utc::now();

    paste.set_edited().expect("Failed to set edited timestamp.");

    paste.set_expiry(None);

    paste
        .update(db.pool())
        .await
        .expect("Failed to update paste.");

    let result = Paste::fetch(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    let paste_edited = paste.edited().expect("Edited was not found.");

    assert_eq!(
        paste_edited.date_naive(),
        current.date_naive(),
        "Mismatched edited Date."
    );
    assert_eq!(
        paste_edited
            .time()
            .with_nanosecond(0)
            .expect("Failed to build current time with reset nanosecond."),
        current
            .time()
            .with_nanosecond(0)
            .expect("Failed to build current time with reset nanosecond."),
        "Mismatched edited HMS."
    );

    assert!(result.expiry().is_none(), "Mismatched expiry time.");
}

#[sqlx::test(fixtures("pastes"))]
fn test_add_view(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_284_601);
    let paste = Paste::fetch(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert_eq!(paste.id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(paste.views(), 567, "Mismatched views count.");

    let value = Paste::add_view(db.pool(), &paste_id)
        .await
        .expect("Failed to add view to paste.");

    assert_eq!(value, 568, "Mismatched view count.");

    let result = Paste::fetch(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert_eq!(result.views(), 568, "Mismatched views count.");
}

#[sqlx::test(fixtures("pastes"))]
fn test_delete(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_284_601);

    Paste::fetch(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    Paste::delete(db.pool(), &paste_id)
        .await
        .expect("Failed to delete value from database.");

    let result = Paste::fetch(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.");

    assert!(result.is_none(), "Found paste in db.");
}
