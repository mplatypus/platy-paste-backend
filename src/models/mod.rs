pub mod authentication;
pub mod document;
pub mod error;
pub mod paste;
pub mod payload;
pub mod snowflake;
pub mod undefined;

/// A type implementation of the a chrono datetime that uses UTC as its timezone.
pub type DtUtc = chrono::DateTime<chrono::Utc>;
