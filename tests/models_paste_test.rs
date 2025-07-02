use platy_paste::{
    app::database::Database,
    models::{paste::*, snowflake::Snowflake},
};

use sqlx::PgPool;
use time::{Date, OffsetDateTime, Time};

#[test]
fn test_getters() {
    let paste_id = Snowflake::new(123);
    let creation = OffsetDateTime::from_unix_timestamp(10).expect("failed to generate timestamp.");
    let edited = OffsetDateTime::from_unix_timestamp(15).expect("failed to generate timestamp.");
    let expiry = OffsetDateTime::from_unix_timestamp(20).expect("failed to generate timestamp.");

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

    assert!(paste.expiry() == Some(&expiry), "Mismatched expiry.");

    assert_eq!(paste.views(), 567, "Mismatched views.");

    assert_eq!(paste.max_views(), Some(1000), "Mismatched max views.");
}

#[test]
fn test_set_edited() {
    let paste_id = Snowflake::new(123);
    let creation = OffsetDateTime::from_unix_timestamp(10).expect("failed to generate timestamp.");
    let expiry = OffsetDateTime::from_unix_timestamp(20).expect("failed to generate timestamp.");

    let mut paste = Paste::new(paste_id, creation, None, Some(expiry), 567, Some(1000));

    assert_eq!(paste.edited(), None, "Mismatched edited.");

    let current = OffsetDateTime::now_utc();
    paste.set_edited();

    assert_eq!(paste.id(), &paste_id, "Mismatched paste ID.");

    let paste_edited = paste.edited().expect("Edited was not found.");

    assert_eq!(
        paste_edited.date(),
        current.date(),
        "Mismatched edited Date."
    );
    assert_eq!(
        paste_edited.to_hms(),
        current.to_hms(),
        "Mismatched edited HMS."
    );

    assert_eq!(paste.expiry(), Some(&expiry), "Mismatched expiry.");

    assert_eq!(paste.views(), 567, "Mismatched views.");

    assert_eq!(paste.max_views(), Some(1000), "Mismatched max views.");
}

#[test]
fn test_set_expiry() {
    let paste_id = Snowflake::new(123);
    let creation = OffsetDateTime::from_unix_timestamp(10).expect("failed to generate timestamp.");
    let expiry = OffsetDateTime::from_unix_timestamp(0).expect("failed to generate timestamp.");

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
    let creation =
        OffsetDateTime::from_unix_timestamp(0).expect("failed to generate creation timestamp.");
    let edited =
        OffsetDateTime::from_unix_timestamp(86400).expect("failed to generate edited timestamp.");
    let expiry =
        OffsetDateTime::from_unix_timestamp(172_800).expect("failed to generate expiry timestamp.");

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
        &OffsetDateTime::new_utc(
            Date::from_calendar_date(1970, time::Month::January, 2)
                .expect("Failed to build date start."),
            Time::from_hms(0, 0, 0).expect("Failed to build time start."),
        ),
        &OffsetDateTime::new_utc(
            Date::from_calendar_date(1970, time::Month::January, 4)
                .expect("Failed to build date end."),
            Time::from_hms(0, 0, 0).expect("Failed to build time end."),
        ),
    )
    .await
    .expect("Failed to fetch value from database.");

    assert_eq!(results.len(), 1, "Not enough or too many results received.");

    assert_eq!(
        results[0].expiry(),
        Some(&OffsetDateTime::from_unix_timestamp(172_800).expect("Failed to build result time.")),
        "Invalid expiry. Expected: {:?}, Received: {:?}",
        Some(OffsetDateTime::from_unix_timestamp(172_800).expect("Failed to build result time.")),
        results[0].expiry(),
    );
}

#[sqlx::test(fixtures("pastes"))]
fn test_fetch_between_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let results = Paste::fetch_between(
        db.pool(),
        &OffsetDateTime::new_utc(
            Date::from_calendar_date(2000, time::Month::January, 1)
                .expect("Failed to build date start."),
            Time::from_hms(0, 0, 0).expect("Failed to build time start."),
        ),
        &OffsetDateTime::new_utc(
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
    let creation = OffsetDateTime::from_unix_timestamp(10).expect("failed to generate timestamp.");
    let edited = OffsetDateTime::from_unix_timestamp(15).expect("failed to generate timestamp.");
    let expiry = OffsetDateTime::from_unix_timestamp(20).expect("failed to generate timestamp.");

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
        Some(&OffsetDateTime::from_unix_timestamp(86400).expect("failed to generate timestamp.")),
        "Mismatched edited time."
    );

    assert_eq!(
        paste.expiry(),
        Some(
            &OffsetDateTime::from_unix_timestamp(172_800)
                .expect("Failed to build expected timestamp.")
        ),
        "Mismatched expiry time."
    );

    let current = OffsetDateTime::now_utc();

    paste.set_edited();

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
        paste_edited.date(),
        current.date(),
        "Mismatched edited Date."
    );
    assert_eq!(
        paste_edited.to_hms(),
        current.to_hms(),
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
