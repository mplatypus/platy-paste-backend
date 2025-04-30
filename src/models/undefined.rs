use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Visitor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndefinedOption<T> {
    Some(T),
    Undefined,
    None,
}

impl<T> UndefinedOption<T> {
    pub const fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub const fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    pub fn to_option(self) -> Option<T> {
        match self {
            Self::Some(t) => Some(t),
            _ => None,
        }
    }
}

impl<T> Default for UndefinedOption<T> {
    fn default() -> Self {
        Self::Undefined
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

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::{from_str, to_string};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct UndefinedString {
        #[serde(default, skip_serializing_if = "UndefinedOption::is_undefined")]
        pub value: UndefinedOption<String>,
    }

    impl UndefinedString {
        pub const fn new(value: UndefinedOption<String>) -> Self {
            Self { value }
        }
    }

    #[test]
    fn test_serialize_omitted() {
        let value = UndefinedString::new(UndefinedOption::Undefined);

        assert_eq!(
            to_string(&value).expect("Failed to serialize value."),
            r"{}"
        );
    }

    #[test]
    fn test_serialize_null() {
        let value = UndefinedString::new(UndefinedOption::None);

        assert_eq!(
            to_string(&value).expect("Failed to serialize value."),
            r#"{"value":null}"#
        );
    }

    #[test]
    fn test_serialize_existing() {
        let value = UndefinedString::new(UndefinedOption::Some("hello world!".to_string()));

        assert_eq!(
            to_string(&value).expect("Failed to serialize value."),
            r#"{"value":"hello world!"}"#
        );
    }

    #[test]
    fn test_deserialize_omitted() {
        let value = r"{}";

        let deserialized_value: UndefinedString =
            from_str(value).expect("Failed to deserialize value.");

        assert_eq!(
            deserialized_value,
            UndefinedString::new(UndefinedOption::Undefined)
        );
    }

    #[test]
    fn test_deserialize_null() {
        let value = r#"{"value":null}"#;

        let deserialized_value: UndefinedString =
            from_str(value).expect("Failed to deserialize value.");

        assert_eq!(
            deserialized_value,
            UndefinedString::new(UndefinedOption::None)
        );
    }

    #[test]
    fn test_deserialize_existing() {
        let value = r#"{"value":"hello world!"}"#;

        let deserialized_value: UndefinedString =
            from_str(value).expect("Failed to deserialize value.");

        assert_eq!(
            deserialized_value,
            UndefinedString::new(UndefinedOption::Some("hello world!".to_string()))
        );
    }

    #[test]
    fn test_functions() {
        let omitted_string = UndefinedString::new(UndefinedOption::Undefined);

        assert!(omitted_string.value.is_undefined());
        assert!(!omitted_string.value.is_none());
        assert!(!omitted_string.value.is_some());
        assert_eq!(omitted_string.value.to_option(), None);

        let null_string = UndefinedString::new(UndefinedOption::None);

        assert!(!null_string.value.is_undefined());
        assert!(null_string.value.is_none());
        assert!(!null_string.value.is_some());
        assert_eq!(null_string.value.to_option(), None);

        let some_string = UndefinedString::new(UndefinedOption::Some("hello world!".to_string()));

        assert!(!some_string.value.is_undefined());
        assert!(!some_string.value.is_none());
        assert!(some_string.value.is_some());
        assert_eq!(
            some_string.value.to_option(),
            Some("hello world!".to_string())
        );
    }
}
