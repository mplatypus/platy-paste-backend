use chrono::{DateTime, TimeZone, Timelike as _, Utc};
use platy_paste::{
    app::database::Database,
    models::{
        DtUtc,
        paste::*,
        snowflake::Snowflake,
        undefined::{Undefined, UndefinedOption},
    },
};

use rstest::rstest;
use sqlx::PgPool;

#[test]
fn test_getters() {
    let paste_id = Snowflake::new(123);
    let creation = DateTime::from_timestamp(10, 0).expect("failed to generate timestamp.");
    let edited = DateTime::from_timestamp(15, 0).expect("failed to generate timestamp.");
    let expiry = DateTime::from_timestamp(20, 0).expect("failed to generate timestamp.");

    let paste = Paste::new(
        paste_id,
        Some("beans".to_string()),
        creation,
        Some(edited),
        Some(expiry),
        567,
        Some(1000),
    );

    assert_eq!(paste.id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(paste.name(), Some("beans"));

    assert_eq!(paste.edited(), Some(&edited), "Mismatched edited.");

    assert_eq!(paste.expiry(), Some(&expiry), "Mismatched expiry.");

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

    assert_eq!(paste.name(), Some("Test 1"), "Mismatched paste name.");

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

    assert_eq!(results[0].id(), &Snowflake::new(517815304354284601),);

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
        Some("Test".to_string()),
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

    assert_eq!(paste.name(), Some("Test"), "Mismatched paste name.");

    assert_eq!(result.creation(), &creation, "Mismatched creation time.");

    assert_eq!(result.edited(), Some(&edited), "Mismatched edited time.");

    assert_eq!(result.expiry(), Some(&expiry), "Mismatched expiry time.");

    assert_eq!(paste.views(), 53489, "Mismatched views.");

    assert_eq!(paste.max_views(), Some(100_000), "Mismatched max views.");
}

#[rstest]
#[case(
    PasteUpdateParameters::new(
        UndefinedOption::Undefined,
        UndefinedOption::Undefined,
        Undefined::Undefined,
        UndefinedOption::Undefined
    ),
    Some("Test 1"),
    Some(DateTime::from_timestamp(172800, 0).unwrap()),
    567,
    Some(1000),
    false,
)]
#[case(
    PasteUpdateParameters::new(
        UndefinedOption::Some("New Name".to_string()),
        UndefinedOption::Undefined,
        Undefined::Undefined,
        UndefinedOption::Undefined
    ),
    Some("New Name"),
    Some(DateTime::from_timestamp(172800, 0).unwrap()),
    567,
    Some(1000),
    true,
)]
#[case(
    PasteUpdateParameters::new(
        UndefinedOption::Undefined,
        UndefinedOption::Some(DateTime::from_timestamp(1000000, 0).unwrap()),
        Undefined::Undefined,
        UndefinedOption::Undefined
    ),
    Some("Test 1"),
    Some(DateTime::from_timestamp(1000000, 0).unwrap()),
    567,
    Some(1000),
    true,
)]
#[case(
    PasteUpdateParameters::new(
        UndefinedOption::Undefined,
        UndefinedOption::Undefined,
        Undefined::Some(20000),
        UndefinedOption::Undefined
    ),
    Some("Test 1"),
    Some(DateTime::from_timestamp(172800, 0).unwrap()),
    20000,
    Some(1000),
    true,
)]
#[case(
    PasteUpdateParameters::new(
        UndefinedOption::Undefined,
        UndefinedOption::Undefined,
        Undefined::Undefined,
        UndefinedOption::Some(80000)
    ),
    Some("Test 1"),
    Some(DateTime::from_timestamp(172800, 0).unwrap()),
    567,
    Some(80000),
    true,
)]
#[case(
    PasteUpdateParameters::new(
        UndefinedOption::None,
        UndefinedOption::Undefined,
        Undefined::Undefined,
        UndefinedOption::Undefined
    ),
    None,
    Some(DateTime::from_timestamp(172800, 0).unwrap()),
    567,
    Some(1000),
    true,
)]
#[case(
    PasteUpdateParameters::new(
        UndefinedOption::Undefined,
        UndefinedOption::None,
        Undefined::Undefined,
        UndefinedOption::Undefined
    ),
    Some("Test 1"),
    None,
    567,
    Some(1000),
    true
)]
#[case(
    PasteUpdateParameters::new(
        UndefinedOption::Undefined,
        UndefinedOption::Undefined,
        Undefined::Undefined,
        UndefinedOption::None
    ),
    Some("Test 1"),
    Some(DateTime::from_timestamp(172800, 0).unwrap()),
    567,
    None,
    true,
)]
#[case(
    PasteUpdateParameters::new(
        UndefinedOption::Some("New Name".to_string()),
        UndefinedOption::Some(DateTime::from_timestamp(1000000, 0).unwrap()),
        Undefined::Some(20000),
        UndefinedOption::Some(80000)
    ),
    Some("New Name"),
    Some(DateTime::from_timestamp(1000000, 0).unwrap()),
    20000,
    Some(80000),
    true,
)]
#[case(
    PasteUpdateParameters::new(
        UndefinedOption::None,
        UndefinedOption::None,
        Undefined::Undefined,
        UndefinedOption::None
    ),
    None,
    None,
    567,
    None,
    true
)]
#[sqlx::test(fixtures("pastes"))]
async fn test_update(
    #[ignore] pool: PgPool,
    #[case] parameters: PasteUpdateParameters,
    #[case] name: Option<&str>,
    #[case] expiry: Option<DtUtc>,
    #[case] views: usize,
    #[case] max_views: Option<usize>,
    #[case] was_updated: bool,
) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_284_601);
    let mut paste = Paste::fetch(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert_eq!(paste.id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(paste.name(), Some("Test 1"), "Mismatched paste name.");

    let old_edited_timestamp =
        DateTime::from_timestamp(86400, 0).expect("failed to generate timestamp.");

    assert_eq!(
        paste.edited(),
        Some(&old_edited_timestamp),
        "Mismatched edited time."
    );

    assert_eq!(
        paste.expiry(),
        Some(&DateTime::from_timestamp(172_800, 0).expect("Failed to build expected timestamp.")),
        "Mismatched expiry time."
    );

    assert_eq!(paste.max_views(), Some(1000), "Mismatched max views.");

    let edited_timestamp = match was_updated {
        true => Utc::now()
            .with_nanosecond(0)
            .expect("Failed to build current time with reset nanosecond."),
        false => old_edited_timestamp,
    };

    let updated = paste
        .update(db.pool(), parameters)
        .await
        .expect("Failed to fetch value from database.");

    assert_eq!(updated, was_updated, "The updated parameter did not match.");

    let result_paste = Paste::fetch(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert_eq!(
        result_paste
            .edited()
            .expect("Edited was not found.")
            .with_nanosecond(0)
            .expect("Failed to build current time with reset nanosecond."),
        edited_timestamp,
        "Mismatched edited time."
    );

    assert_eq!(
        paste
            .edited()
            .expect("Edited was not found.")
            .with_nanosecond(0)
            .expect("Failed to build current time with reset nanosecond."),
        edited_timestamp,
        "Mismatched edited time."
    );

    assert_eq!(result_paste.name(), name, "Mismatched name.");

    assert_eq!(paste.name(), name, "Mismatched name.");

    assert_eq!(result_paste.expiry(), expiry.as_ref(), "Mismatched expiry.");

    assert_eq!(paste.expiry(), expiry.as_ref(), "Mismatched expiry.");

    assert_eq!(result_paste.views(), views, "Mismatched views.");

    assert_eq!(paste.views(), views, "Mismatched views.");

    assert_eq!(result_paste.max_views(), max_views, "Mismatched views.");

    assert_eq!(paste.max_views(), max_views, "Mismatched max views.");
}

#[sqlx::test(fixtures("pastes"))]
fn test_add_view(pool: PgPool) {
    let db = Database::from_pool(pool);

    let paste_id = Snowflake::new(517_815_304_354_284_601);
    let mut paste = Paste::fetch(db.pool(), &paste_id)
        .await
        .expect("Failed to fetch value from database.")
        .expect("No paste was found.");

    assert_eq!(paste.id(), &paste_id, "Mismatched paste ID.");

    assert_eq!(paste.views(), 567, "Mismatched views count.");

    paste
        .add_view(db.pool())
        .await
        .expect("Failed to add view to paste.");

    assert_eq!(paste.views(), 568, "Mismatched view count.");

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
