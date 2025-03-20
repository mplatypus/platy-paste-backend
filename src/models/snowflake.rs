use std::{
    fmt,
    num::ParseIntError,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sqlx::{Decode, Encode};

use crate::app::database::Database;

use super::error::AppError;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// ## Snowflake
///
/// A Simple snowflake implementation.
pub struct Snowflake(u64);

impl Snowflake {
    pub fn new(id: u64) -> Snowflake {
        Snowflake(id)
    }

    pub async fn generate(db: &Database) -> Result<Snowflake, AppError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;

        let id = getrandom::u64().map_err(|e| AppError::NotFound(e.to_string()))?;

        let new_snowflake = Snowflake::new((timestamp << 22) | (id as u64 & 0x3FFFFF));

        Ok(new_snowflake)
    }

    /// Id
    ///
    /// Get the ID for the snowflake.
    pub fn id(&self) -> u64 {
        self.0
    }

    /// Created At
    ///
    /// The time since epoch, that this ID was created at.
    pub fn created_at(&self) -> u64 {
        self.id() >> 22
    }
}

impl fmt::Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromStr for Snowflake {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Snowflake(s.parse()?))
    }
}

impl From<Snowflake> for String {
    fn from(value: Snowflake) -> Self {
        value.to_string()
    }
}

impl TryFrom<String> for Snowflake {
    type Error = ParseIntError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Snowflake(value.parse()?))
    }
}

impl TryFrom<&str> for Snowflake {
    type Error = ParseIntError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Snowflake(value.parse()?))
    }
}

impl Into<u64> for Snowflake {
    fn into(self) -> u64 {
        self.id()
    }
}

impl From<u64> for Snowflake {
    fn from(value: u64) -> Self {
        Snowflake(value)
    }
}

impl Into<i64> for Snowflake {
    fn into(self) -> i64 {
        self.0 as i64
    }
}

impl From<i64> for Snowflake {
    fn from(value: i64) -> Self {
        Snowflake(value as u64)
    }
}
