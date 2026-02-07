use std::{fmt, num::ParseIntError, str::FromStr};

use chrono::Utc;
use serde::{Deserializer, Serialize, Serializer, de::Error as DEError};
use serde_json::Value;
use sqlx::{Decode, Encode};

use crate::models::errors::{GenerateError, ParseError};

/// ## Partial Snowflake
///
/// A snowflake implementation, with the possibility of not being a complete snowflake.
#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartialSnowflake(u64);

impl PartialSnowflake {
    /// New.
    ///
    /// Create a new [`PartialSnowflake`] object.
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Id.
    ///
    /// Get the raw ID for the snowflake.
    pub const fn id(&self) -> u64 {
        self.0
    }
}

impl TryFrom<PartialSnowflake> for Snowflake {
    type Error = ParseError;

    fn try_from(value: PartialSnowflake) -> Result<Self, Self::Error> {
        let timestamp = value.id() >> 22;
        let id = value.id() & 0x003F_FFFF;

        if timestamp as i64 >= Utc::now().timestamp() {
            return Err(ParseError::ParseSnowflake(
                "Snowflakes cannot exist from the future.".to_string(),
            ));
        }

        if id != 0 {
            return Err(ParseError::ParseSnowflake("ID cannot be zero.".to_string()));
        }

        Ok(Snowflake::new(value.id()))
    }
}

impl From<Snowflake> for PartialSnowflake {
    fn from(value: Snowflake) -> Self {
        Self::new(value.id())
    }
}

impl Serialize for PartialSnowflake {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.id().to_string().as_str())
    }
}

impl<'de> serde::Deserialize<'de> for PartialSnowflake {
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

impl fmt::Display for PartialSnowflake {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

impl FromStr for PartialSnowflake {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl TryFrom<String> for PartialSnowflake {
    type Error = ParseIntError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(value.parse()?))
    }
}

impl TryFrom<&str> for PartialSnowflake {
    type Error = ParseIntError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self(value.parse()?))
    }
}

impl PartialEq<Snowflake> for PartialSnowflake {
    fn eq(&self, other: &Snowflake) -> bool {
        self.0 == other.0
    }
}

/// ## Snowflake
///
/// A Simple snowflake implementation.
#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// - [`GenerateError`] - Failed to get a random value.
    ///
    /// ## Returns
    ///
    /// A [`Snowflake`].
    pub fn generate() -> Result<Self, GenerateError> {
        let timestamp = Utc::now().timestamp() as u64;

        let id = getrandom::u64()?;

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

impl PartialEq<PartialSnowflake> for Snowflake {
    fn eq(&self, other: &PartialSnowflake) -> bool {
        self.0 == other.0
    }
}
