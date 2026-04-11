//! Paths, Queries, Bodies and Responses related to the configuration endpoints.

#[cfg(test)]
use serde::Deserialize;
use serde::Serialize;

use crate::app::config::Config;

//----------//
// Response //
//----------//

/// ## Response Status
///
/// The status object returned when requested.
#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize, Debug)]
pub struct ResponseStatus {
    /// The current status of the server.
    status: String,
}

impl ResponseStatus {
    /// New.
    ///
    /// Create a new [`ResponseStatus`] object.
    pub const fn new(status: String) -> Self {
        Self { status }
    }
}

/// ## Response Information
///
/// The information object returned when requested.
#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize, Debug)]
pub struct ResponseInformation {
    version: ResponseVersionInformation,
}

impl ResponseInformation {
    /// From Env.
    ///
    /// Creates a new [`ResponseInformation`] object from environment variables.
    ///
    /// ## Errors
    /// This will return an error if a environment variable was expected, but was unset.
    /// The error value is the name of the missing environment variable.
    ///
    /// ## Returns
    /// The [`ResponseInformation`] object built from environment variables.
    pub fn from_env() -> Result<Self, Vec<String>> {
        let mut errors = Vec::new();

        let version = ResponseVersionInformation::from_env()
            .map_err(|e| errors.extend_from_slice(&e))
            .ok();

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(Self {
            version: version.expect("Expected version information."),
        })
    }
}

/// ## Response Rust Information
///
/// The rust information object returned when requested.
#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize, Debug)]
pub struct ResponseVersionInformation {
    /// The semantic versioning of the current server.
    semver: String,
    /// The major version.
    major: usize,
    /// The minor version.
    minor: usize,
    /// The patch version.
    patch: usize,
}

impl ResponseVersionInformation {
    /// From Env.
    ///
    /// Creates a new [`ResponseVersionInformation`] object from environment variables.
    ///
    /// ## Errors
    /// This will return an error if a environment variable was expected, but was unset.
    /// The error value is the name of the missing environment variable.
    ///
    /// ## Returns
    /// The [`ResponseVersionInformation`] object built from environment variables.
    pub fn from_env() -> Result<Self, Vec<String>> {
        let mut errors: Vec<String> = Vec::new();

        let semver = std::env::var("CARGO_PKG_VERSION")
            .map_err(|_| errors.push("CARGO_PKG_VERSION missing.".to_string()))
            .ok();

        let major = std::env::var("CARGO_PKG_VERSION_MAJOR")
            .map_err(|_| errors.push("CARGO_PKG_VERSION_MAJOR missing.".to_string()))
            .ok()
            .and_then(|v| {
                v.parse::<usize>()
                    .map_err(|_| {
                        errors.push(
                            "CARGO_PKG_VERSION_MAJOR Failed to parse to integer.".to_string(),
                        );
                    })
                    .ok()
            });

        let minor = std::env::var("CARGO_PKG_VERSION_MINOR")
            .map_err(|_| errors.push("CARGO_PKG_VERSION_MINOR missing.".to_string()))
            .ok()
            .and_then(|v| {
                v.parse::<usize>()
                    .map_err(|_| {
                        errors.push(
                            "CARGO_PKG_VERSION_MINOR Failed to parse to integer.".to_string(),
                        );
                    })
                    .ok()
            });

        let patch = std::env::var("CARGO_PKG_VERSION_PATCH")
            .map_err(|_| errors.push("CARGO_PKG_VERSION_PATCH missing.".to_string()))
            .ok()
            .and_then(|v| {
                v.parse::<usize>()
                    .map_err(|_| {
                        errors.push(
                            "CARGO_PKG_VERSION_PATCH Failed to parse to integer.".to_string(),
                        );
                    })
                    .ok()
            });

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(Self {
            semver: semver.expect("Expected CARGO_PKG_VERSION environment variable."),
            major: major.expect("Expected CARGO_PKG_VERSION_MAJOR environment variable."),
            minor: minor.expect("Expected CARGO_PKG_VERSION_MINOR environment variable."),
            patch: patch.expect("Expected CARGO_PKG_VERSION_PATCH environment variable."),
        })
    }
}

/// ## Response Config
///
/// The configuration object returned when requested.
#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize, Debug)]
pub struct ResponseConfig {
    /// Defaults.
    defaults: ResponseDefaultsConfig,
    /// Size limits.
    size_limits: ResponseSizeLimitsConfig,
}

impl ResponseConfig {
    /// New.
    ///
    /// Create a new [`ResponseConfig`] object.
    pub const fn new(
        defaults: ResponseDefaultsConfig,
        size_limits: ResponseSizeLimitsConfig,
    ) -> Self {
        Self {
            defaults,
            size_limits,
        }
    }

    /// From config.
    ///
    /// Create a new [`ResponseDefaultsConfig`] object, with a [`Config`] object.
    pub fn from_config(config: &Config) -> Self {
        Self::new(
            ResponseDefaultsConfig::from_config(config),
            ResponseSizeLimitsConfig::from_config(config),
        )
    }
}

/// ## Response Defaults Config
///
/// The default values for configuration information.
#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize, Debug)]
pub struct ResponseDefaultsConfig {
    /// The default expiry for pastes.
    expiry_hours: Option<usize>,
    /// The default value for maximum views.
    maximum_views: Option<usize>,
    /// The default value for paste names.
    paste_name: Option<String>,
}

impl ResponseDefaultsConfig {
    /// New.
    ///
    /// Create a new [`ResponseConfig`] object.
    pub const fn new(
        expiry_hours: Option<usize>,
        maximum_views: Option<usize>,
        paste_name: Option<String>,
    ) -> Self {
        Self {
            expiry_hours,
            maximum_views,
            paste_name,
        }
    }

    /// From config.
    ///
    /// Create a new [`ResponseDefaultsConfig`] object, with a [`Config`] object.
    pub fn from_config(config: &Config) -> Self {
        let size_limits = config.size_limits();

        Self::new(
            size_limits.default_expiry_hours(),
            size_limits.default_maximum_views(),
            size_limits.default_paste_name().map(ToString::to_string),
        )
    }
}

/// ## Response Size Limits Config
///
/// The size limits for configuration information.
#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize, Debug)]
pub struct ResponseSizeLimitsConfig {
    /// The minimum size of a paste name.
    minimum_paste_name_size: usize,
    /// The minimum expiry hours for pastes.
    minimum_expiry_hours: Option<usize>,
    /// The minimum allowed documents in a paste.
    minimum_total_document_count: usize,
    /// The minimum document size.
    minimum_document_size: usize,
    /// The minimum total document size.
    minimum_total_document_size: usize,
    /// The minimum size of a document name.
    minimum_document_name_size: usize,
    /// The maximum size of a paste name.
    maximum_paste_name_size: usize,
    /// The maximum expiry for pastes.
    maximum_expiry_hours: Option<usize>,
    /// The maximum allowed documents in a paste.
    maximum_total_document_count: usize,
    /// The individual paste document size.
    maximum_document_size: usize,
    /// The maximum paste body size, including all documents.
    maximum_total_document_size: usize,
    /// The maximum size of a document name.
    maximum_document_name_size: usize,
}

impl ResponseSizeLimitsConfig {
    /// New.
    ///
    /// Create a new [`ResponseConfig`] object.
    #[expect(clippy::too_many_arguments)]
    pub const fn new(
        minimum_paste_name_size: usize,
        minimum_expiry_hours: Option<usize>,
        minimum_total_document_count: usize,
        minimum_document_size: usize,
        minimum_total_document_size: usize,
        minimum_document_name_size: usize,
        maximum_paste_name_size: usize,
        maximum_expiry_hours: Option<usize>,
        maximum_total_document_count: usize,
        maximum_document_size: usize,
        maximum_total_document_size: usize,
        maximum_document_name_size: usize,
    ) -> Self {
        Self {
            minimum_paste_name_size,
            minimum_expiry_hours,
            minimum_total_document_count,
            minimum_document_size,
            minimum_total_document_size,
            minimum_document_name_size,
            maximum_paste_name_size,
            maximum_expiry_hours,
            maximum_total_document_count,
            maximum_document_size,
            maximum_total_document_size,
            maximum_document_name_size,
        }
    }

    /// From config.
    ///
    /// Create a new [`ResponseDefaultsConfig`] object, with a [`Config`] object.
    pub const fn from_config(config: &Config) -> Self {
        let size_limits = config.size_limits();
        Self::new(
            size_limits.minimum_paste_name_size(),
            size_limits.minimum_expiry_hours(),
            size_limits.minimum_total_document_count(),
            size_limits.minimum_document_size(),
            size_limits.minimum_total_document_size(),
            size_limits.minimum_document_name_size(),
            size_limits.maximum_paste_name_size(),
            size_limits.maximum_expiry_hours(),
            size_limits.maximum_total_document_count(),
            size_limits.maximum_document_size(),
            size_limits.maximum_total_document_size(),
            size_limits.maximum_document_name_size(),
        )
    }
}
