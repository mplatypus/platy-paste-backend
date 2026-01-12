use core::fmt;
use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Visitor};

/// Undefined Option
///
/// Indicates the difference between a value being null, undefined and some.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UndefinedOption<T> {
    /// The value extracted.
    Some(T),
    /// The value was not defined.
    #[default]
    Undefined,
    /// The value was null.
    None,
}

impl<T> UndefinedOption<T> {
    /// ## Is Some
    ///
    /// True if the value is defined, and not null.
    pub const fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    /// ## Is None
    ///
    /// True if the value is defined, and is null.
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// ## Is Undefined
    ///
    /// True if the value was undefined.
    pub const fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    pub const fn as_ref(&self) -> UndefinedOption<&T> {
        match *self {
            Self::Some(ref x) => UndefinedOption::Some(x),
            Self::Undefined => UndefinedOption::Undefined,
            Self::None => UndefinedOption::None,
        }
    }

    pub fn as_deref(&self) -> UndefinedOption<&T::Target>
    where
        T: Deref,
    {
        self.as_ref().map(Deref::deref)
    }

    pub fn map<U, F>(self, f: F) -> UndefinedOption<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Some(x) => UndefinedOption::Some(f(x)),
            Self::Undefined => UndefinedOption::Undefined,
            Self::None => UndefinedOption::None,
        }
    }

    #[expect(clippy::wrong_self_convention)]
    pub fn is_none_or(self, f: impl FnOnce(T) -> bool) -> bool {
        match self {
            Self::Undefined | Self::None => true,
            Self::Some(x) => f(x),
        }
    }
}

impl<T> Serialize for UndefinedOption<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Some(t) => serializer.serialize_some(t),
            Self::Undefined => serializer.serialize_unit(),
            Self::None => serializer.serialize_none(),
        }
    }
}

impl<'de, T> Deserialize<'de> for UndefinedOption<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_option(UndefinedOptionVisitor {
            marker: std::marker::PhantomData,
        })
    }
}

impl<T> From<Option<T>> for UndefinedOption<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or_else(|| Self::None, |v| Self::Some(v))
    }
}

impl<T> From<UndefinedOption<T>> for Option<T> {
    fn from(value: UndefinedOption<T>) -> Self {
        match value {
            UndefinedOption::Some(v) => Some(v),
            UndefinedOption::Undefined | UndefinedOption::None => None,
        }
    }
}

impl<T> From<UndefinedOption<T>> for Undefined<T> {
    fn from(value: UndefinedOption<T>) -> Self {
        match value {
            UndefinedOption::Some(v) => Undefined::Some(v),
            UndefinedOption::Undefined | UndefinedOption::None => Undefined::Undefined,
        }
    }
}

/// Undefined Option Visitor
struct UndefinedOptionVisitor<T> {
    /// Phantom data marker.
    marker: std::marker::PhantomData<T>,
}

impl<'de, T> Visitor<'de> for UndefinedOptionVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = UndefinedOption<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an optional value")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(UndefinedOption::None)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(UndefinedOption::None)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(UndefinedOption::Some(T::deserialize(deserializer)?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Undefined<T> {
    Some(T),
    #[default]
    Undefined,
}

impl<T> Undefined<T> {
    /// ## Is Some
    ///
    /// True if the value is defined, and not null.
    pub const fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    /// ## Is Undefined
    ///
    /// True if the value was undefined.
    pub const fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    pub const fn as_ref(&self) -> Undefined<&T> {
        match *self {
            Self::Some(ref x) => Undefined::Some(x),
            Self::Undefined => Undefined::Undefined,
        }
    }

    pub fn as_deref(&self) -> Undefined<&T::Target>
    where
        T: Deref,
    {
        self.as_ref().map(Deref::deref)
    }

    pub fn map<U, F>(self, f: F) -> Undefined<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Some(x) => Undefined::Some(f(x)),
            Self::Undefined => Undefined::Undefined,
        }
    }
}

impl<T> From<Option<T>> for Undefined<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or_else(|| Self::Undefined, |v| Self::Some(v))
    }
}

impl<T> From<Undefined<T>> for Option<T> {
    fn from(value: Undefined<T>) -> Self {
        match value {
            Undefined::Some(v) => Some(v),
            Undefined::Undefined => None,
        }
    }
}

impl<T> From<Undefined<T>> for UndefinedOption<T> {
    fn from(value: Undefined<T>) -> Self {
        match value {
            Undefined::Some(t) => Self::Some(t),
            Undefined::Undefined => Self::Undefined,
        }
    }
}

impl<T> Serialize for Undefined<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Some(t) => serializer.serialize_some(t),
            Self::Undefined => serializer.serialize_unit(),
        }
    }
}

impl<'de, T> Deserialize<'de> for Undefined<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_option(UndefinedVisitor {
            marker: std::marker::PhantomData,
        })
    }
}

struct UndefinedVisitor<T> {
    marker: std::marker::PhantomData<T>,
}

impl<'de, T> Visitor<'de> for UndefinedVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = Undefined<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an optional value")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Undefined::Undefined)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Undefined::Some(T::deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::{Undefined, UndefinedOption};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct UndefinedOptionString {
        #[serde(default, skip_serializing_if = "UndefinedOption::is_undefined")]
        pub value: UndefinedOption<String>,
    }

    impl UndefinedOptionString {
        pub fn new(value: UndefinedOption<String>) -> Self {
            Self { value }
        }
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct UndefinedString {
        #[serde(default, skip_serializing_if = "Undefined::is_undefined")]
        pub value: Undefined<String>,
    }

    impl UndefinedString {
        pub fn new(value: Undefined<String>) -> Self {
            Self { value }
        }
    }

    #[rstest::rstest]
    #[case("{\"value\":\"a cool value\"}", UndefinedOption::Some("a cool value".to_string()))]
    #[case("{\"value\":null}", UndefinedOption::None)]
    #[case("{}", UndefinedOption::Undefined)]
    fn test_undefined_option_deserialize(
        #[case] payload: &str,
        #[case] expected: UndefinedOption<String>,
    ) {
        let payload: UndefinedOptionString =
            serde_json::from_str(payload).expect("Failed to extract value from payload");

        assert_eq!(
            payload.value, expected,
            "Payload's value was not of expected."
        );
    }

    #[rstest::rstest]
    #[case("{\"value\":\"a cool value\"}", Undefined::Some("a cool value".to_string()))]
    #[case("{}", Undefined::Undefined)]
    fn test_undefined_deserialize(#[case] payload: &str, #[case] expected: Undefined<String>) {
        let payload: UndefinedString =
            serde_json::from_str(payload).expect("Failed to extract value from payload");

        assert_eq!(
            payload.value, expected,
            "Payload's value was not of expected."
        );
    }

    #[rstest::rstest]
    #[case(UndefinedOption::Some("a cool value".to_string()), "{\"value\":\"a cool value\"}")]
    #[case(UndefinedOption::None, "{\"value\":null}")]
    #[case(UndefinedOption::Undefined, "{}")]
    fn test_undefined_option_serialize(
        #[case] value: UndefinedOption<String>,
        #[case] expected: &str,
    ) {
        let payload = serde_json::to_string(&UndefinedOptionString::new(value))
            .expect("Failed to extract value from payload");

        assert_eq!(payload, expected, "Payload's value was not of expected.");
    }

    #[rstest::rstest]
    #[case(Undefined::Some("a cool value".to_string()), "{\"value\":\"a cool value\"}")]
    #[case(Undefined::Undefined, "{}")]
    fn test_undefined_serialize(#[case] value: Undefined<String>, #[case] expected: &str) {
        let payload = serde_json::to_string(&UndefinedString::new(value))
            .expect("Failed to extract value from payload");

        assert_eq!(payload, expected, "Payload's value was not of expected.");
    }

    #[rstest::rstest]
    #[case(UndefinedOption::Some("a cool value"), true, false, false)]
    #[case(UndefinedOption::None, false, true, false)]
    #[case(UndefinedOption::Undefined, false, false, true)]
    fn test_undefined_option_is_functions(
        #[case] value: UndefinedOption<&str>,
        #[case] is_some: bool,
        #[case] is_none: bool,
        #[case] is_undefined: bool,
    ) {
        assert_eq!(value.is_some(), is_some);
        assert_eq!(value.is_none(), is_none);
        assert_eq!(value.is_undefined(), is_undefined);
    }

    #[rstest::rstest]
    #[case(Undefined::Some("a cool value"), true, false)]
    #[case(Undefined::Undefined, false, true)]
    fn test_undefined_is_functions(
        #[case] value: Undefined<&str>,
        #[case] is_some: bool,
        #[case] is_undefined: bool,
    ) {
        assert_eq!(value.is_some(), is_some);
        assert_eq!(value.is_undefined(), is_undefined);
    }

    #[rstest::rstest]
    #[case(UndefinedOption::Some("test".to_string()), UndefinedOption::Some(&"test".to_string()))]
    #[case(UndefinedOption::Undefined, UndefinedOption::Undefined)]
    #[case(UndefinedOption::None, UndefinedOption::None)]
    fn test_undefined_option_as_ref(
        #[case] value: UndefinedOption<String>,
        #[case] expected: UndefinedOption<&String>,
    ) {
        assert_eq!(value.as_ref(), expected,);
    }

    #[rstest::rstest]
    #[case(Undefined::Some("test".to_string()), Undefined::Some(&"test".to_string()))]
    #[case(Undefined::Undefined, Undefined::Undefined)]
    fn test_undefined_as_ref(
        #[case] value: Undefined<String>,
        #[case] expected: Undefined<&String>,
    ) {
        assert_eq!(value.as_ref(), expected,);
    }

    #[rstest::rstest]
    #[case(UndefinedOption::Some("test".to_string()), UndefinedOption::Some("test"))]
    #[case(UndefinedOption::Undefined, UndefinedOption::Undefined)]
    #[case(UndefinedOption::None, UndefinedOption::None)]
    fn test_undefined_option_as_deref(
        #[case] value: UndefinedOption<String>,
        #[case] expected: UndefinedOption<&str>,
    ) {
        assert_eq!(value.as_deref(), expected,);
    }

    #[rstest::rstest]
    #[case(Undefined::Some("test".to_string()), Undefined::Some("test"))]
    #[case(Undefined::Undefined, Undefined::Undefined)]
    fn test_undefined_as_deref(
        #[case] value: Undefined<String>,
        #[case] expected: Undefined<&str>,
    ) {
        assert_eq!(value.as_deref(), expected,);
    }

    #[rstest::rstest]
    #[case(UndefinedOption::Some("test"), UndefinedOption::Some("test".to_string()))]
    #[case(UndefinedOption::Undefined, UndefinedOption::Undefined)]
    fn test_undefined_option_map(
        #[case] value: UndefinedOption<&str>,
        #[case] expected: UndefinedOption<String>,
    ) {
        assert_eq!(value.map(ToString::to_string), expected);
    }

    #[rstest::rstest]
    #[case(Undefined::Some("test"), Undefined::Some("test".to_string()))]
    #[case(Undefined::Undefined, Undefined::Undefined)]
    fn test_undefined_map(#[case] value: Undefined<&str>, #[case] expected: Undefined<String>) {
        assert_eq!(value.map(ToString::to_string), expected);
    }

    #[rstest::rstest]
    #[case(UndefinedOption::Some("test".to_string()), Undefined::Some("test".to_string()))]
    #[case(UndefinedOption::Undefined, Undefined::Undefined)]
    #[case(UndefinedOption::None, Undefined::Undefined)]
    fn test_undefined_option_into_undefined(
        #[case] value: UndefinedOption<String>,
        #[case] expected: Undefined<String>,
    ) {
        let val = Undefined::<String>::from(value.clone());
        assert_eq!(val, expected);

        let val: Undefined<String> = value.into();
        assert_eq!(val, expected);
    }

    #[rstest::rstest]
    #[case(Undefined::Some("test".to_string()), UndefinedOption::Some("test".to_string()))]
    #[case(Undefined::Undefined, UndefinedOption::Undefined)]
    fn test_undefined_into_undefined_option(
        #[case] value: Undefined<String>,
        #[case] expected: UndefinedOption<String>,
    ) {
        let val = UndefinedOption::<String>::from(value.clone());
        assert_eq!(val, expected);

        let val: UndefinedOption<String> = value.into();
        assert_eq!(val, expected);
    }

    #[rstest::rstest]
    #[case(UndefinedOption::Some("test".to_string()), Some("test".to_string()))]
    #[case(UndefinedOption::Undefined, None)]
    #[case(UndefinedOption::None, None)]
    fn test_undefined_option_into_option(
        #[case] value: UndefinedOption<String>,
        #[case] expected: Option<String>,
    ) {
        let val = Option::<String>::from(value.clone());
        assert_eq!(val, expected);

        let val: Option<String> = value.into();
        assert_eq!(val, expected);
    }

    #[rstest::rstest]
    #[case(Some("test".to_string()), UndefinedOption::Some("test".to_string()))]
    #[case(None, UndefinedOption::None)]
    fn test_option_into_undefined_option(
        #[case] value: Option<String>,
        #[case] expected: UndefinedOption<String>,
    ) {
        let val = UndefinedOption::<String>::from(value.clone());
        assert_eq!(val, expected);

        let val: UndefinedOption<String> = value.into();
        assert_eq!(val, expected);
    }

    #[rstest::rstest]
    #[case(Undefined::Some("test".to_string()), Some("test".to_string()))]
    #[case(Undefined::Undefined, None)]
    fn test_undefined_into_option(
        #[case] value: Undefined<String>,
        #[case] expected: Option<String>,
    ) {
        let val = Option::<String>::from(value.clone());
        assert_eq!(val, expected);

        let val: Option<String> = value.into();
        assert_eq!(val, expected);
    }

    #[rstest::rstest]
    #[case(Some("test".to_string()), Undefined::Some("test".to_string()))]
    #[case(None, Undefined::Undefined)]
    fn test_option_into_undefined(
        #[case] value: Option<String>,
        #[case] expected: Undefined<String>,
    ) {
        let val = Undefined::<String>::from(value.clone());
        assert_eq!(val, expected);

        let val: Undefined<String> = value.into();
        assert_eq!(val, expected);
    }
}
