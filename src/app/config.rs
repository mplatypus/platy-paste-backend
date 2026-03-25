//! The configuration objects for the server.

#[cfg(test)]
use derive_builder::Builder;
use secrecy::SecretString;

/// ## Config
///
/// The base configuration that stores all other configuration items.
#[cfg_attr(test, derive(Builder, Default))]
#[cfg_attr(test, builder(default))]
#[derive(Debug, Clone)]
pub struct Config {
    /// The host to run on.
    host: String,
    /// The port to run on.
    port: u16,
    /// The database URL.
    database_url: String,
    /// The domain to use for cors.
    domain: String,
    /// Object store information.
    object_store: ObjectStoreConfig,
    /// Size limits.
    size_limits: SizeLimitConfig,
}

impl Config {
    // Testing item, docs not needed.
    #[expect(missing_docs)]
    #[cfg(test)]
    pub fn test_builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// ## From Env
    ///
    /// Create the configuration from environment values
    ///
    /// ## Panics
    /// Panics if an environment value is not set, or cannot be parsed to the expected type.
    ///
    /// ## Returns
    /// Returns the [`Config`] object.
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").expect("HOST environment variable must be set."),
            port: std::env::var("PORT")
                .expect("PORT environment variable must be set.")
                .parse()
                .expect("PORT requires an integer."),
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL environment variable must be set."),
            domain: std::env::var("DOMAIN").expect("DOMAIN environment variable must be set."),
            object_store: ObjectStoreConfig::from_env(),
            size_limits: SizeLimitConfig::from_env(),
        }
    }

    /// The host to run on.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// The port to run on.
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// The database URL.
    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    /// The domain to use for cors.
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// Object store information.
    pub const fn object_store(&self) -> &ObjectStoreConfig {
        &self.object_store
    }

    /// Size limits.
    pub const fn size_limits(&self) -> &SizeLimitConfig {
        &self.size_limits
    }
}

/// ## Object Store Config
///
/// The object storage configuration.
#[cfg_attr(test, derive(Default))]
#[derive(Debug, Clone)]
pub enum ObjectStoreConfig {
    /// ## S3
    ///
    /// The S3 Object Storage information.
    S3(S3ObjectStoreConfig),
    // Testing item, docs not needed.
    #[expect(missing_docs)]
    #[cfg(test)]
    #[cfg_attr(test, default)]
    Test,
}

impl ObjectStoreConfig {
    /// ## From Env
    ///
    /// Create the configuration from environment values
    ///
    /// ## Panics
    /// Panics if an environment value is not set, or cannot be parsed to the expected type.
    ///
    /// ## Returns
    /// Returns the [`ObjectStoreConfig`] object.
    #[expect(clippy::too_many_lines)]
    pub fn from_env() -> Self {
        let obs_type =
            std::env::var("OBS_TYPE").expect("OBS_TYPE environment variable must be set.");

        match obs_type.as_str() {
            "MINIO" => Self::S3(S3ObjectStoreConfig::from_env()),
            unknown => panic!("The OBS_TYPE `{unknown}` is unknown."),
        }
    }
}

/// ## S3 Object Store Config
///
/// The S3 Object Storage information.
#[derive(Debug, Clone)]
pub struct S3ObjectStoreConfig {
    /// The S3 Service URL.
    url: String,
    /// The S3 Service Access Key.
    access_key: SecretString,
    /// The S3 Service Secret Key.
    secret_key: SecretString,
}

impl S3ObjectStoreConfig {
    /// ## From Env
    ///
    /// Create the configuration from environment values
    ///
    /// ## Panics
    /// Panics if an environment value is not set, or cannot be parsed to the expected type.
    ///
    /// ## Returns
    /// Returns the [`S3ObjectStoreConfig`] object.
    #[expect(clippy::too_many_lines)]
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("OBS_URL").expect("OBS_URL environment variable must be set."),
            access_key: std::env::var("OBS_ACCESS_KEY")
                .expect("OBS_ACCESS_KEY environment variable must be set.")
                .into(),
            secret_key: std::env::var("OBS_SECRET_KEY")
                .expect("OBS_SECRET_KEY environment variable must be set.")
                .into(),
        }
    }

    /// The S3 Service URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The S3 Service Access Key.
    pub const fn access_key(&self) -> &SecretString {
        &self.access_key
    }

    /// The S3 Service Secret Key.
    pub const fn secret_key(&self) -> &SecretString {
        &self.secret_key
    }
}

/// ## Size Limit Config
///
/// The configuration information about size limits.
#[cfg_attr(test, derive(Builder))]
#[cfg_attr(test, builder(default))]
#[derive(Debug, Clone)]
pub struct SizeLimitConfig {
    /// The default expiry for pastes.
    default_expiry_hours: Option<usize>,
    /// The default value for maximum views.
    default_maximum_views: Option<usize>,
    /// The default value for the pastes name.
    default_paste_name: Option<String>,
    /// The minimum expiry hours for pastes.
    minimum_expiry_hours: Option<usize>,
    /// The minimum allowed documents in a paste.
    minimum_total_document_count: usize,
    /// The minimum document size (bytes).
    minimum_document_size: usize,
    /// The minimum total document size (bytes).
    minimum_total_document_size: usize,
    /// The minimum size of a document name (bytes).
    minimum_document_name_size: usize,
    /// The minimum size of a paste name (bytes).
    minimum_paste_name_size: usize,
    /// The maximum expiry for pastes.
    maximum_expiry_hours: Option<usize>,
    /// The maximum allowed documents in a paste.
    maximum_total_document_count: usize,
    /// The maximum document size.
    maximum_document_size: usize,
    /// The maximum total document size (bytes).
    maximum_total_document_size: usize,
    /// The maximum size of a document name (bytes).
    maximum_document_name_size: usize,
    /// The maximum size of the paste name (bytes).
    maximum_paste_name_size: usize,
}

impl SizeLimitConfig {
    // Testing item, docs not needed.
    #[expect(missing_docs)]
    #[cfg(test)]
    pub fn test_builder() -> SizeLimitConfigBuilder {
        SizeLimitConfigBuilder::default()
    }

    /// ## From Env
    ///
    /// Create the configuration from environment values
    ///
    /// ## Panics
    /// Panics if an environment value is not set, or cannot be parsed to the expected type.
    ///
    /// ## Returns
    /// Returns the [`S3ObjectStoreConfig`] object.
    #[expect(clippy::too_many_lines)]
    pub fn from_env() -> Self {
        let defaults = Self::default();

        let value =
            Self {
                default_expiry_hours: std::env::var("DEFAULT_EXPIRY_HOURS").ok().map_or(
                    defaults.default_expiry_hours,
                    |v| {
                        Some(
                            v.parse()
                                .expect("DEFAULT_EXPIRY_HOURS requires an integer."),
                        )
                    },
                ),
                default_maximum_views: std::env::var("DEFAULT_MAXIMUM_VIEWS").ok().map_or(
                    defaults.default_maximum_views,
                    |v| {
                        Some(
                            v.parse()
                                .expect("DEFAULT_MAXIMUM_VIEWS requires an integer."),
                        )
                    },
                ),
                default_paste_name: std::env::var("DEFAULT_PASTE_NAME")
                    .ok()
                    .map_or(defaults.default_paste_name, |v| Some(v)),
                minimum_expiry_hours: std::env::var("MINIMUM_EXPIRY_HOURS").ok().map_or(
                    defaults.minimum_expiry_hours,
                    |v| {
                        Some(
                            v.parse()
                                .expect("MINIMUM_EXPIRY_HOURS requires an integer."),
                        )
                    },
                ),
                minimum_total_document_count: std::env::var("MINIMUM_TOTAL_DOCUMENT_COUNT")
                    .ok()
                    .map_or(defaults.minimum_total_document_count, |v| {
                        v.parse()
                            .expect("MINIMUM_TOTAL_DOCUMENT_COUNT requires an integer.")
                    }),
                minimum_document_size: std::env::var("MINIMUM_DOCUMENT_SIZE").ok().map_or(
                    defaults.minimum_document_size,
                    |v| {
                        v.parse()
                            .expect("MINIMUM_DOCUMENT_SIZE requires an integer.")
                    },
                ),
                minimum_total_document_size: std::env::var("MINIMUM_TOTAL_DOCUMENT_SIZE")
                    .ok()
                    .map_or(defaults.minimum_total_document_size, |v| {
                        v.parse()
                            .expect("MINIMUM_TOTAL_DOCUMENT_SIZE requires an integer.")
                    }),
                minimum_document_name_size: std::env::var("MINIMUM_DOCUMENT_NAME_SIZE")
                    .ok()
                    .map_or(defaults.minimum_document_name_size, |v| {
                        v.parse()
                            .expect("MINIMUM_DOCUMENT_NAME_SIZE requires an integer.")
                    }),
                minimum_paste_name_size: std::env::var("MINIMUM_PASTE_NAME_SIZE").ok().map_or(
                    defaults.minimum_paste_name_size,
                    |v| {
                        v.parse()
                            .expect("MINIMUM_PASTE_NAME_SIZE requires an integer.")
                    },
                ),
                maximum_expiry_hours: std::env::var("MAXIMUM_EXPIRY_HOURS").ok().map_or(
                    defaults.maximum_expiry_hours,
                    |v| {
                        Some(
                            v.parse()
                                .expect("MAXIMUM_EXPIRY_HOURS requires an integer."),
                        )
                    },
                ),
                maximum_total_document_count: std::env::var("MAXIMUM_TOTAL_DOCUMENT_COUNT")
                    .ok()
                    .map_or(defaults.maximum_total_document_count, |v| {
                        v.parse()
                            .expect("MAXIMUM_TOTAL_DOCUMENT_COUNT requires an integer.")
                    }),
                maximum_document_size: std::env::var("MAXIMUM_DOCUMENT_SIZE").ok().map_or(
                    defaults.maximum_document_size,
                    |v| {
                        v.parse()
                            .expect("MAXIMUM_DOCUMENT_SIZE requires an integer.")
                    },
                ),
                maximum_total_document_size: std::env::var("MAXIMUM_TOTAL_DOCUMENT_SIZE")
                    .ok()
                    .map_or(defaults.maximum_total_document_size, |v| {
                        v.parse()
                            .expect("MAXIMUM_TOTAL_DOCUMENT_SIZE requires an integer.")
                    }),
                maximum_document_name_size: std::env::var("MAXIMUM_DOCUMENT_NAME_SIZE")
                    .ok()
                    .map_or(defaults.maximum_document_name_size, |v| {
                        v.parse()
                            .expect("MAXIMUM_DOCUMENT_NAME_SIZE requires an integer.")
                    }),
                maximum_paste_name_size: std::env::var("MAXIMUM_PASTE_NAME_SIZE").ok().map_or(
                    defaults.maximum_paste_name_size,
                    |v| {
                        v.parse()
                            .expect("MAXIMUM_PASTE_NAME_SIZE requires an integer.")
                    },
                ),
            };

        if let Some(default_expiry_hours) = value.default_expiry_hours {
            if let Some(minimum_expiry_hours) = value.minimum_expiry_hours {
                assert!(
                    default_expiry_hours >= minimum_expiry_hours,
                    "The DEFAULT_EXPIRY_HOURS must be equal to or less than MAXIMUM_EXPIRY_HOURS"
                );
            }

            if let Some(maximum_expiry_hours) = value.maximum_expiry_hours {
                assert!(
                    default_expiry_hours >= maximum_expiry_hours,
                    "The DEFAULT_EXPIRY_HOURS must be equal to or less than MAXIMUM_EXPIRY_HOURS"
                );
            }
        }

        if let (Some(minimum_expiry_hours), Some(maximum_expiry_hours)) =
            (value.minimum_expiry_hours, value.maximum_expiry_hours)
        {
            assert!(
                minimum_expiry_hours >= maximum_expiry_hours,
                "The MINIMUM_EXPIRY_HOURS must be equal to or less than MAXIMUM_EXPIRY_HOURS"
            );
        }

        assert!(
            value.minimum_paste_name_size < value.maximum_paste_name_size,
            "The MINIMUM_PASTE_NAME_SIZE must be equal to or less than MAXIMUM_PASTE_NAME_SIZE"
        );

        assert!(
            value.minimum_paste_name_size > 0,
            "The MINIMUM_PASTE_NAME_SIZE must be greater than 0."
        );

        if let Some(default_paste_name) = &value.default_paste_name {
            assert!(
                default_paste_name.len() > value.minimum_paste_name_size,
                "The DEFAULT_PASTE_NAME must be equal to or greater than the MINIMUM_PASTE_NAME_SIZE"
            );

            assert!(
                default_paste_name.len() < value.maximum_paste_name_size,
                "The DEFAULT_PASTE_NAME must be equal to or less than the MAXIMUM_PASTE_NAME_SIZE"
            );
        }

        assert!(
            value.minimum_total_document_count > 0,
            "The MINIMUM_TOTAL_DOCUMENT_COUNT must be greater than 0."
        );

        assert!(
            value.minimum_total_document_count < value.maximum_total_document_count,
            "The MINIMUM_TOTAL_DOCUMENT_COUNT must be equal to or less than MAXIMUM_TOTAL_DOCUMENT_COUNT"
        );

        assert!(
            value.minimum_document_size > 0,
            "The MINIMUM_DOCUMENT_SIZE must be greater than 0."
        );

        assert!(
            value.minimum_document_size < value.maximum_document_size,
            "The MINIMUM_DOCUMENT_SIZE must be equal to or less than MAXIMUM_DOCUMENT_SIZE"
        );

        assert!(
            value.minimum_total_document_size > 0,
            "The MINIMUM_TOTAL_DOCUMENT_SIZE must be greater than 0."
        );

        assert!(
            value.minimum_total_document_size < value.maximum_total_document_size,
            "The MINIMUM_TOTAL_DOCUMENT_SIZE must be equal to or less than MAXIMUM_TOTAL_DOCUMENT_SIZE"
        );

        assert!(
            value.minimum_document_name_size > 0,
            "The MINIMUM_DOCUMENT_NAME_SIZE must be greater than 0."
        );

        assert!(
            value.minimum_document_name_size < value.maximum_document_name_size,
            "The MINIMUM_DOCUMENT_NAME_SIZE must be equal to or less than MAXIMUM_DOCUMENT_NAME_SIZE"
        );

        value
    }

    /// The default expiry for pastes.
    pub const fn default_expiry_hours(&self) -> Option<usize> {
        self.default_expiry_hours
    }

    /// The default value for maximum views.
    pub const fn default_maximum_views(&self) -> Option<usize> {
        self.default_maximum_views
    }

    /// The default value for the pastes name.
    pub fn default_paste_name(&self) -> Option<&str> {
        self.default_paste_name.as_deref()
    }

    /// The minimum expiry hours for pastes.
    pub const fn minimum_expiry_hours(&self) -> Option<usize> {
        self.minimum_expiry_hours
    }

    /// The minimum allowed documents in a paste.
    pub const fn minimum_total_document_count(&self) -> usize {
        self.minimum_total_document_count
    }

    /// The minimum document size (bytes).
    pub const fn minimum_document_size(&self) -> usize {
        self.minimum_document_size
    }

    /// The minimum total document size (bytes).
    pub const fn minimum_total_document_size(&self) -> usize {
        self.minimum_total_document_size
    }

    /// The minimum size of a document name (bytes).
    pub const fn minimum_document_name_size(&self) -> usize {
        self.minimum_document_name_size
    }

    /// The minimum size of a paste name (bytes).
    pub const fn minimum_paste_name_size(&self) -> usize {
        self.minimum_paste_name_size
    }

    /// The maximum expiry for pastes.
    pub const fn maximum_expiry_hours(&self) -> Option<usize> {
        self.maximum_expiry_hours
    }

    /// The maximum allowed documents in a paste.
    pub const fn maximum_total_document_count(&self) -> usize {
        self.maximum_total_document_count
    }

    /// The maximum document size.
    pub const fn maximum_document_size(&self) -> usize {
        self.maximum_document_size
    }

    /// The maximum total document size (bytes).
    pub const fn maximum_total_document_size(&self) -> usize {
        self.maximum_total_document_size
    }

    /// The maximum size of a document name (bytes).
    pub const fn maximum_document_name_size(&self) -> usize {
        self.maximum_document_name_size
    }

    /// The maximum size of the paste name (bytes).
    pub const fn maximum_paste_name_size(&self) -> usize {
        self.maximum_paste_name_size
    }
}

impl Default for SizeLimitConfig {
    fn default() -> Self {
        Self {
            default_expiry_hours: None,
            default_maximum_views: None,
            default_paste_name: None,
            minimum_expiry_hours: None,
            minimum_total_document_count: 1,
            minimum_document_size: 1,
            minimum_total_document_size: 1,
            minimum_document_name_size: 3,
            minimum_paste_name_size: 3,
            maximum_expiry_hours: None,
            maximum_total_document_count: 10,
            maximum_document_size: 5_000_000,
            maximum_total_document_size: 10_000_000,
            maximum_document_name_size: 50,
            maximum_paste_name_size: 50,
        }
    }
}
