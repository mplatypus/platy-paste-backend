use derive_builder::Builder;
use dotenvy::from_filename;
use secrecy::SecretString;

#[derive(Debug, Clone, Builder)]
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
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    pub fn from_env() -> Self {
        from_filename(".env").ok();
        Self::builder()
            .host(std::env::var("HOST").expect("HOST environment variable must be set."))
            .port(
                std::env::var("PORT")
                    .expect("PORT environment variable must be set.")
                    .parse()
                    .expect("PORT requires an integer."),
            )
            .database_url(
                std::env::var("DATABASE_URL")
                    .expect("DATABASE_URL environment variable must be set."),
            )
            .domain(std::env::var("DOMAIN").expect("DOMAIN environment variable must be set."))
            .object_store(ObjectStoreConfig::from_env(false))
            .size_limits(SizeLimitConfig::from_env(false))
            .build()
            .expect("Failed to create application configuration.")
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub const fn object_store(&self) -> &ObjectStoreConfig {
        &self.object_store
    }

    pub const fn size_limits(&self) -> &SizeLimitConfig {
        &self.size_limits
    }
}

#[derive(Debug, Clone)]
pub enum ObjectStoreConfig {
    S3(S3ObjectStoreConfig),
}

impl ObjectStoreConfig {
    #[expect(clippy::too_many_lines)]
    pub fn from_env(fetch_env: bool) -> Self {
        if fetch_env {
            from_filename(".env").ok();
        }

        let obs_type =
            std::env::var("OBS_TYPE").expect("OBS_TYPE environment variable must be set.");

        match obs_type.as_str() {
            "MINIO" => Self::S3(S3ObjectStoreConfig::from_env(false)),
            unknown => panic!("The OBS_TYPE `{unknown}` is unknown."),
        }
    }
}

#[derive(Debug, Clone, Builder)]
pub struct S3ObjectStoreConfig {
    /// The S3 Service URL.
    url: String,
    /// The S3 Service Access Key.
    access_key: SecretString,
    /// The S3 Service Secret Key.
    secret_key: SecretString,
}

impl S3ObjectStoreConfig {
    pub fn builder() -> S3ObjectStoreConfigBuilder {
        S3ObjectStoreConfigBuilder::default()
    }

    #[expect(clippy::too_many_lines)]
    pub fn from_env(fetch_env: bool) -> Self {
        if fetch_env {
            from_filename(".env").ok();
        }

        Self::builder()
            .url(std::env::var("OBS_URL").expect("OBS_URL environment variable must be set."))
            .access_key(
                std::env::var("OBS_ACCESS_KEY")
                    .expect("OBS_ACCESS_KEY environment variable must be set.")
                    .into(),
            )
            .secret_key(
                std::env::var("OBS_SECRET_KEY")
                    .expect("OBS_SECRET_KEY environment variable must be set.")
                    .into(),
            )
            .build()
            .expect("Failed to build S3 object store config.")
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub const fn access_key(&self) -> &SecretString {
        &self.access_key
    }

    pub const fn secret_key(&self) -> &SecretString {
        &self.secret_key
    }
}

#[derive(Debug, Clone, Builder)]
#[builder(default)]
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
    pub fn builder() -> SizeLimitConfigBuilder {
        SizeLimitConfigBuilder::default()
    }

    #[expect(clippy::too_many_lines)]
    pub fn from_env(fetch_env: bool) -> Self {
        if fetch_env {
            from_filename(".env").ok();
        }

        let defaults = Self::default();

        let builder = Self::builder()
            .default_expiry_hours(std::env::var("DEFAULT_EXPIRY_HOURS").ok().map_or(
                defaults.default_expiry_hours,
                |v| {
                    Some(
                        v.parse()
                            .expect("DEFAULT_EXPIRY_HOURS requires an integer."),
                    )
                },
            ))
            .default_maximum_views(std::env::var("DEFAULT_MAXIMUM_VIEWS").ok().map_or(
                defaults.default_maximum_views,
                |v| {
                    Some(
                        v.parse()
                            .expect("DEFAULT_MAXIMUM_VIEWS requires an integer."),
                    )
                },
            ))
            .default_paste_name(
                std::env::var("DEFAULT_PASTE_NAME")
                    .ok()
                    .map_or(defaults.default_paste_name, |v| Some(v)),
            )
            .minimum_expiry_hours(std::env::var("MINIMUM_EXPIRY_HOURS").ok().map_or(
                defaults.minimum_expiry_hours,
                |v| {
                    Some(
                        v.parse()
                            .expect("MINIMUM_EXPIRY_HOURS requires an integer."),
                    )
                },
            ))
            .minimum_total_document_count(
                std::env::var("MINIMUM_TOTAL_DOCUMENT_COUNT").ok().map_or(
                    defaults.minimum_total_document_count,
                    |v| {
                        v.parse()
                            .expect("MINIMUM_TOTAL_DOCUMENT_COUNT requires an integer.")
                    },
                ),
            )
            .minimum_document_size(std::env::var("MINIMUM_DOCUMENT_SIZE").ok().map_or(
                defaults.minimum_document_size,
                |v| {
                    v.parse()
                        .expect("MINIMUM_DOCUMENT_SIZE requires an integer.")
                },
            ))
            .minimum_total_document_size(std::env::var("MINIMUM_TOTAL_DOCUMENT_SIZE").ok().map_or(
                defaults.minimum_total_document_size,
                |v| {
                    v.parse()
                        .expect("MINIMUM_TOTAL_DOCUMENT_SIZE requires an integer.")
                },
            ))
            .minimum_document_name_size(std::env::var("MINIMUM_DOCUMENT_NAME_SIZE").ok().map_or(
                defaults.minimum_document_name_size,
                |v| {
                    v.parse()
                        .expect("MINIMUM_DOCUMENT_NAME_SIZE requires an integer.")
                },
            ))
            .minimum_paste_name_size(std::env::var("MINIMUM_PASTE_NAME_SIZE").ok().map_or(
                defaults.minimum_paste_name_size,
                |v| {
                    v.parse()
                        .expect("MINIMUM_PASTE_NAME_SIZE requires an integer.")
                },
            ))
            .maximum_expiry_hours(std::env::var("MAXIMUM_EXPIRY_HOURS").ok().map_or(
                defaults.maximum_expiry_hours,
                |v| {
                    Some(
                        v.parse()
                            .expect("MAXIMUM_EXPIRY_HOURS requires an integer."),
                    )
                },
            ))
            .maximum_total_document_count(
                std::env::var("MAXIMUM_TOTAL_DOCUMENT_COUNT").ok().map_or(
                    defaults.maximum_total_document_count,
                    |v| {
                        v.parse()
                            .expect("MAXIMUM_TOTAL_DOCUMENT_COUNT requires an integer.")
                    },
                ),
            )
            .maximum_document_size(std::env::var("MAXIMUM_DOCUMENT_SIZE").ok().map_or(
                defaults.maximum_document_size,
                |v| {
                    v.parse()
                        .expect("MAXIMUM_DOCUMENT_SIZE requires an integer.")
                },
            ))
            .maximum_total_document_size(std::env::var("MAXIMUM_TOTAL_DOCUMENT_SIZE").ok().map_or(
                defaults.maximum_total_document_size,
                |v| {
                    v.parse()
                        .expect("MAXIMUM_TOTAL_DOCUMENT_SIZE requires an integer.")
                },
            ))
            .maximum_document_name_size(std::env::var("MAXIMUM_DOCUMENT_NAME_SIZE").ok().map_or(
                defaults.maximum_document_name_size,
                |v| {
                    v.parse()
                        .expect("MAXIMUM_DOCUMENT_NAME_SIZE requires an integer.")
                },
            ))
            .maximum_paste_name_size(std::env::var("MAXIMUM_PASTE_NAME_SIZE").ok().map_or(
                defaults.maximum_paste_name_size,
                |v| {
                    v.parse()
                        .expect("MAXIMUM_PASTE_NAME_SIZE requires an integer.")
                },
            ))
            .build()
            .expect("Failed to create application size limit configuration.");

        if let Some(default_expiry_hours) = builder.default_expiry_hours {
            if let Some(minimum_expiry_hours) = builder.minimum_expiry_hours {
                assert!(
                    default_expiry_hours >= minimum_expiry_hours,
                    "The DEFAULT_EXPIRY_HOURS must be equal to or less than MAXIMUM_EXPIRY_HOURS"
                );
            }

            if let Some(maximum_expiry_hours) = builder.maximum_expiry_hours {
                assert!(
                    default_expiry_hours >= maximum_expiry_hours,
                    "The DEFAULT_EXPIRY_HOURS must be equal to or less than MAXIMUM_EXPIRY_HOURS"
                );
            }
        }

        if let (Some(minimum_expiry_hours), Some(maximum_expiry_hours)) =
            (builder.minimum_expiry_hours, builder.maximum_expiry_hours)
        {
            assert!(
                minimum_expiry_hours >= maximum_expiry_hours,
                "The MINIMUM_EXPIRY_HOURS must be equal to or less than MAXIMUM_EXPIRY_HOURS"
            );
        }

        assert!(
            builder.minimum_paste_name_size < builder.maximum_paste_name_size,
            "The MINIMUM_PASTE_NAME_SIZE must be equal to or less than MAXIMUM_PASTE_NAME_SIZE"
        );

        assert!(
            builder.minimum_paste_name_size > 0,
            "The MINIMUM_PASTE_NAME_SIZE must be greater than 0."
        );

        if let Some(default_paste_name) = &builder.default_paste_name {
            assert!(
                default_paste_name.len() > builder.minimum_paste_name_size,
                "The DEFAULT_PASTE_NAME must be equal to or greater than the MINIMUM_PASTE_NAME_SIZE"
            );

            assert!(
                default_paste_name.len() < builder.maximum_paste_name_size,
                "The DEFAULT_PASTE_NAME must be equal to or less than the MAXIMUM_PASTE_NAME_SIZE"
            );
        }

        assert!(
            builder.minimum_total_document_count > 0,
            "The MINIMUM_TOTAL_DOCUMENT_COUNT must be greater than 0."
        );

        assert!(
            builder.minimum_total_document_count < builder.maximum_total_document_count,
            "The MINIMUM_TOTAL_DOCUMENT_COUNT must be equal to or less than MAXIMUM_TOTAL_DOCUMENT_COUNT"
        );

        assert!(
            builder.minimum_document_size > 0,
            "The MINIMUM_DOCUMENT_SIZE must be greater than 0."
        );

        assert!(
            builder.minimum_document_size < builder.maximum_document_size,
            "The MINIMUM_DOCUMENT_SIZE must be equal to or less than MAXIMUM_DOCUMENT_SIZE"
        );

        assert!(
            builder.minimum_total_document_size > 0,
            "The MINIMUM_TOTAL_DOCUMENT_SIZE must be greater than 0."
        );

        assert!(
            builder.minimum_total_document_size < builder.maximum_total_document_size,
            "The MINIMUM_TOTAL_DOCUMENT_SIZE must be equal to or less than MAXIMUM_TOTAL_DOCUMENT_SIZE"
        );

        assert!(
            builder.minimum_document_name_size > 0,
            "The MINIMUM_DOCUMENT_NAME_SIZE must be greater than 0."
        );

        assert!(
            builder.minimum_document_name_size < builder.maximum_document_name_size,
            "The MINIMUM_DOCUMENT_NAME_SIZE must be equal to or less than MAXIMUM_DOCUMENT_NAME_SIZE"
        );

        assert!(
            builder.minimum_document_name_size > 0,
            "The MINIMUM_DOCUMENT_NAME_SIZE must be greater than 0."
        );

        builder
    }

    pub const fn default_expiry_hours(&self) -> Option<usize> {
        self.default_expiry_hours
    }

    pub const fn default_maximum_views(&self) -> Option<usize> {
        self.default_maximum_views
    }

    pub fn default_paste_name(&self) -> Option<&str> {
        self.default_paste_name.as_deref()
    }

    pub const fn minimum_expiry_hours(&self) -> Option<usize> {
        self.minimum_expiry_hours
    }

    pub const fn minimum_total_document_count(&self) -> usize {
        self.minimum_total_document_count
    }

    pub const fn minimum_document_size(&self) -> usize {
        self.minimum_document_size
    }

    pub const fn minimum_total_document_size(&self) -> usize {
        self.minimum_total_document_size
    }

    pub const fn minimum_document_name_size(&self) -> usize {
        self.minimum_document_name_size
    }

    pub const fn minimum_paste_name_size(&self) -> usize {
        self.minimum_paste_name_size
    }

    pub const fn maximum_expiry_hours(&self) -> Option<usize> {
        self.maximum_expiry_hours
    }

    pub const fn maximum_total_document_count(&self) -> usize {
        self.maximum_total_document_count
    }

    pub const fn maximum_document_size(&self) -> usize {
        self.maximum_document_size
    }

    pub const fn maximum_total_document_size(&self) -> usize {
        self.maximum_total_document_size
    }

    pub const fn maximum_document_name_size(&self) -> usize {
        self.maximum_document_name_size
    }

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
