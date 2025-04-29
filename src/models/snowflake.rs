use std::{
    fmt,
    num::ParseIntError,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserializer, Serialize, Serializer, de::Error as DEError};
use serde_json::Value;
use sqlx::{Decode, Encode};

use super::error::AppError;

#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// ## Snowflake
///
/// A Simple snowflake implementation.
pub struct Snowflake(u64);

impl Snowflake {
    /// New.
    ///
    /// Create a new [`Snowflake`] object.
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Generate.
    ///
    /// Generate a new snowflake.
    ///
    /// ## Panics
    ///
    /// If time went backwards
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - Failed to get a random value.
    ///
    /// ## Returns
    ///
    /// A [`Snowflake`].
    pub fn generate() -> Result<Self, AppError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;

        let id = getrandom::u64().map_err(|e| {
            AppError::InternalServer(format!("Failed to obtain a random integer: {e}"))
        })?;

        let new_snowflake = Self::new((timestamp << 22) | (id as u64 & 0x003F_FFFF));

        Ok(new_snowflake)
    }

    /// Id.
    ///
    /// Get the raw ID for the snowflake.
    pub const fn id(&self) -> u64 {
        self.0
    }

    /// Created At.
    ///
    /// The time (since epoch) that this ID was created at.
    pub const fn created_at(&self) -> u64 {
        self.id() >> 22
    }
}

impl Serialize for Snowflake {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.id().to_string().as_str())
    }
}

impl<'de> serde::Deserialize<'de> for Snowflake {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(d)?;
        let snowflake: u64 = match value {
            Value::Number(v) => v
                .as_u64()
                .ok_or_else(|| DEError::custom(format!("Unexpected number: {v}")))?,
            Value::String(v) => v.parse().map_err(DEError::custom)?,
            v => return Err(DEError::custom(format!("Unexpected type: {v}"))),
        };
        Ok(Self::new(snowflake))
    }
}

impl fmt::Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

impl FromStr for Snowflake {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
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
        Ok(Self(value.parse()?))
    }
}

impl TryFrom<&str> for Snowflake {
    type Error = ParseIntError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self(value.parse()?))
    }
}

impl From<Snowflake> for u64 {
    fn from(value: Snowflake) -> Self {
        value.id()
    }
}

impl From<u64> for Snowflake {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Snowflake> for i64 {
    fn from(value: Snowflake) -> Self {
        value.id() as Self
    }
}

impl From<i64> for Snowflake {
    fn from(value: i64) -> Self {
        Self(value as u64)
    }
}
