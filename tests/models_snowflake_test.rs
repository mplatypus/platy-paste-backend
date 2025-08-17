use std::collections::HashSet;

use platy_paste::models::snowflake::Snowflake;

#[test]
fn test_uniqueness() {
    let snowflakes = vec![
        Snowflake::generate().expect("Failed to generate unique snowflake."),
        Snowflake::generate().expect("Failed to generate unique snowflake."),
        Snowflake::generate().expect("Failed to generate unique snowflake."),
        Snowflake::generate().expect("Failed to generate unique snowflake."),
        Snowflake::generate().expect("Failed to generate unique snowflake."),
        Snowflake::generate().expect("Failed to generate unique snowflake."),
        Snowflake::generate().expect("Failed to generate unique snowflake."),
        Snowflake::generate().expect("Failed to generate unique snowflake."),
        Snowflake::generate().expect("Failed to generate unique snowflake."),
        Snowflake::generate().expect("Failed to generate unique snowflake."),
    ];

    let set: HashSet<_> = snowflakes.iter().collect();

    assert_eq!(
        set.len(),
        snowflakes.len(),
        "Non-unique snowflake(s) found: {snowflakes:?}"
    );
}
