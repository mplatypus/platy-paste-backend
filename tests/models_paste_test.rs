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
fn test_setters() {
    let paste_id = Snowflake::new(123);
    let edited = false;
    let expiry = OffsetDateTime::from_unix_timestamp(0).expect("failed to generate timestamp.");

    let mut paste = Paste::new(paste_id, edited, Some(expiry));

    assert!(!paste.edited, "Mismatched edited.");

    assert!(paste.expiry == Some(expiry), "Mismatched expiry.");

    paste.set_edited();

    assert!(paste.edited, "Edited not set.");

    paste.set_expiry(None);

    assert!(paste.expiry.is_none(), "Expiry not set.");
}

#[sqlx::test]
fn test_fetch(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let edited = false;
    let expiry = OffsetDateTime::from_unix_timestamp(0).expect("failed to generate timestamp.");

    let paste_db_id: i64 = paste_id.into();

    sqlx::query!(
        "INSERT INTO pastes VALUES ($1, $2, $3)",
        paste_db_id,
        edited,
        expiry
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    let paste = Paste::fetch(&db, paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert!(paste.id == paste_id, "Mismatched paste ID.");

    assert!(!paste.edited, "Mismatched edited.");

    assert!(paste.expiry.is_some(), "Expiry does not exist.");

    assert!(paste.expiry == Some(expiry), "Mismatched expiry.");
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

#[sqlx::test]
fn test_fetch_between(pool: PgPool) {
    let db = Database::from_pool(pool);

    let date =
        Date::from_calendar_date(2000, time::Month::January, 1).expect("Failed to build date.");

    let date_time_1 = OffsetDateTime::new_utc(
        date,
        Time::from_hms(1, 0, 0).expect("Failed to build time 1."),
    );
    let date_time_2 = OffsetDateTime::new_utc(
        date,
        Time::from_hms(2, 0, 0).expect("Failed to build time 2."),
    );
    let date_time_3 = OffsetDateTime::new_utc(
        date,
        Time::from_hms(4, 0, 0).expect("Failed to build time 3."),
    );
    let date_time_4 = OffsetDateTime::new_utc(
        date,
        Time::from_hms(8, 0, 0).expect("Failed to build time 4."),
    );
    let date_time_5 = OffsetDateTime::new_utc(
        date,
        Time::from_hms(16, 0, 0).expect("Failed to build time 5."),
    );

    sqlx::query!(
        "INSERT INTO pastes VALUES (123, false, $1)",
        Some(date_time_1)
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO pastes VALUES (456, false, $1)",
        Some(date_time_2)
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO pastes VALUES (789, false, $1)",
        Some(date_time_3)
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO pastes VALUES (101, false, $1)",
        Some(date_time_4)
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO pastes VALUES (112, false, $1)",
        Some(date_time_5)
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    let results = Paste::fetch_between(
        &db,
        OffsetDateTime::new_utc(
            date,
            Time::from_hms(2, 0, 0).expect("Failed to build time start."),
        ),
        OffsetDateTime::new_utc(
            date,
            Time::from_hms(10, 0, 0).expect("Failed to build time end."),
        ),
    )
    .await
    .expect("Failed to fetch value from database.");

    assert!(
        results.len() == 3,
        "Not enough or too many results received."
    );

    assert!(
        results[0].expiry == Some(date_time_2),
        "Invalid expiry. Expected: {:?}, Received: {}",
        results[0].expiry,
        date_time_2
    );

    assert!(
        results[1].expiry == Some(date_time_3),
        "Invalid expiry. Expected: {:?}, Received: {}",
        results[1].expiry,
        date_time_3
    );

    assert!(
        results[2].expiry == Some(date_time_4),
        "Invalid expiry. Expected: {:?}, Received: {}",
        results[2].expiry,
        date_time_4
    );
}

#[sqlx::test]
fn test_fetch_between_missing(pool: PgPool) {
    let db = Database::from_pool(pool);

    let date =
        Date::from_calendar_date(2000, time::Month::January, 1).expect("Failed to build date.");

    let date_time_1 = OffsetDateTime::new_utc(
        date,
        Time::from_hms(1, 0, 0).expect("Failed to build time 1."),
    );

    let date_time_2 = OffsetDateTime::new_utc(
        date,
        Time::from_hms(16, 0, 0).expect("Failed to build time 5."),
    );

    sqlx::query!(
        "INSERT INTO pastes VALUES (123, false, $1)",
        Some(date_time_1)
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    sqlx::query!(
        "INSERT INTO pastes VALUES (456, false, $1)",
        Some(date_time_2)
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    let results = Paste::fetch_between(
        &db,
        OffsetDateTime::new_utc(
            date,
            Time::from_hms(2, 0, 0).expect("Failed to build time start."),
        ),
        OffsetDateTime::new_utc(
            date,
            Time::from_hms(10, 0, 0).expect("Failed to build time end."),
        ),
    )
    .await
    .expect("Failed to fetch value from database.");

    assert!(
        results.is_empty(),
        "Received items, when none were expected."
    );
}

#[sqlx::test]
fn test_insert(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let edited = false;
    let expiry = OffsetDateTime::from_unix_timestamp(0).expect("failed to generate timestamp.");

    let paste = Paste::new(paste_id, edited, Some(expiry));

    let paste_db_id: i64 = paste_id.into();

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

    let result = sqlx::query!(
        "SELECT id, edited, expiry FROM pastes WHERE id = $1",
        paste_db_id
    )
    .fetch_optional(db.pool())
    .await
    .expect("Failed to fetch value from database.")
    .expect("No paste was found.");

    assert!(result.id as u64 == paste_id.id(), "Mismatched paste ID.");

    assert!(!result.edited, "Mismatched edited.");

    assert!(result.expiry == Some(expiry), "Mismatched expiry.");
}

#[sqlx::test]
fn test_update(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let edited = false;
    let expiry = OffsetDateTime::from_unix_timestamp(0).expect("failed to generate timestamp.");

    let mut paste = Paste::new(paste_id, edited, Some(expiry));

    let paste_db_id: i64 = paste_id.into();

    let mut transaction = db
        .pool()
        .begin()
        .await
        .expect("Failed to make transaction.");

    paste
        .insert(&mut transaction)
        .await
        .expect("Failed to insert paste.");

    transaction
        .commit()
        .await
        .expect("Failed to commit transaction");

    let result = sqlx::query!(
        "SELECT id, edited, expiry FROM pastes WHERE id = $1",
        paste_db_id
    )
    .fetch_optional(db.pool())
    .await
    .expect("Failed to fetch value from database.")
    .expect("No paste was found.");

    assert!(result.id as u64 == paste_id.id(), "Mismatched paste ID.");

    assert!(!result.edited, "Mismatched edited.");

    assert!(result.expiry == Some(expiry), "Mismatched expiry.");

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

    let result = sqlx::query!(
        "SELECT id, edited, expiry FROM pastes WHERE id = $1",
        paste_db_id
    )
    .fetch_optional(db.pool())
    .await
    .expect("Failed to fetch value from database.")
    .expect("No paste was found.");

    assert!(result.edited, "Mismatched edited.");

    assert!(result.expiry.is_none(), "Mismatched expiry.");
}

#[sqlx::test]
fn test_delete(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(123);
    let edited = false;
    let expiry = OffsetDateTime::from_unix_timestamp(0).expect("failed to generate timestamp.");

    let paste_db_id: i64 = paste_id.into();

    sqlx::query!(
        "INSERT INTO pastes(id, edited, expiry) VALUES ($1, $2, $3)",
        paste_db_id,
        edited,
        Some(expiry)
    )
    .execute(db.pool())
    .await
    .expect("Failed to execute command.");

    let result = sqlx::query!("SELECT * FROM pastes WHERE id = $1", paste_db_id)
        .fetch_optional(db.pool())
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert!(result.id as u64 == paste_id.id(), "Mismatched paste ID.");

    assert!(!result.edited, "Mismatched edited.");

    assert!(result.expiry == Some(expiry), "Mismatched expiry.");

    Paste::delete_with_id(&db, paste_id)
        .await
        .expect("Failed to delete value from database.");

    assert!(
        sqlx::query!(
            "SELECT id, edited, expiry FROM pastes WHERE id = $1",
            paste_db_id
        )
        .fetch_optional(db.pool())
        .await
        .expect("Failed to fetch value from database.")
        .is_none(),
        "Found paste in db."
    );
}

#[ignore]
#[sqlx::test]
fn test_expiry_tasks(_pool: PgPool) {
    // FIXME: Implement me.
}
