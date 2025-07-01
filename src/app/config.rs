use derive_builder::Builder;
use dotenvy::from_filename;
use secrecy::SecretString;

#[derive(Debug, Clone, Builder)]
#[builder(default)]
#[derive(Default)]
pub struct Config {
    /// The host to run on.
    host: String,
    /// The port to run on.
    port: u16,
    /// The database URL.
    database_url: String,
    /// The S3 Service URL.
    s3_url: String,
    /// The S3 Service Access Key.
    s3_access_key: SecretString,
    /// The S3 Service Secret Key.
    s3_secret_key: SecretString,
    /// The Minio User.
    minio_root_user: String,
    /// The Minio Password.
    minio_root_password: SecretString,
    /// The domain to use for cors.
    domain: String,
    /// Size limits.
    size_limits: SizeLimitConfig,
    /// Rate limits.
    rate_limits: RateLimitConfig,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    #[allow(clippy::too_many_lines)]
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
            .s3_url(std::env::var("S3_URL").expect("S3_URL environment variable must be set."))
            .s3_access_key(
                std::env::var("S3_ACCESS_KEY")
                    .expect("S3_ACCESS_KEY environment variable must be set.")
                    .into(),
            )
            .s3_secret_key(
                std::env::var("S3_SECRET_KEY")
                    .expect("S3_SECRET_KEY environment variable must be set.")
                    .into(),
            )
            .minio_root_user(
                std::env::var("MINIO_ROOT_USER")
                    .expect("MINIO_ROOT_USER environment variable must be set."),
            )
            .minio_root_password(
                std::env::var("MINIO_ROOT_PASSWORD")
                    .expect("MINIO_ROOT_PASSWORD environment variable must be set.")
                    .into(),
            )
            .domain(std::env::var("DOMAIN").expect("DOMAIN environment variable must be set."))
            .size_limits(SizeLimitConfig::from_env(false))
            .rate_limits(RateLimitConfig::from_env(false))
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

    pub fn s3_url(&self) -> &str {
        &self.s3_url
    }

    pub const fn s3_access_key(&self) -> &SecretString {
        &self.s3_access_key
    }

    pub const fn s3_secret_key(&self) -> &SecretString {
        &self.s3_secret_key
    }

    pub fn minio_root_user(&self) -> &str {
        &self.minio_root_user
    }

    pub const fn minio_root_password(&self) -> &SecretString {
        &self.minio_root_password
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub const fn size_limits(&self) -> &SizeLimitConfig {
        &self.size_limits
    }

    pub const fn rate_limits(&self) -> &RateLimitConfig {
        &self.rate_limits
    }
}

#[derive(Debug, Clone, Builder)]
#[builder(default)]
pub struct SizeLimitConfig {
    /// The default expiry for pastes.
    default_expiry_hours: Option<usize>,
    /// The default value for maximum views.
    default_maximum_views: Option<usize>,
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
}

impl SizeLimitConfig {
    pub fn builder() -> SizeLimitConfigBuilder {
        SizeLimitConfigBuilder::default()
    }

    #[allow(clippy::too_many_lines)]
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
            builder.minimum_total_document_count > 0,
            "The MINIMUM_TOTAL_DOCUMENT_COUNT must be greater than 0."
        );

        assert!(
            builder.minimum_total_document_count < builder.maximum_total_document_count,
            "The MINIMUM_TOTAL_DOCUMENT_COUNT must be equal to or less than MAXIMUM_TOTAL_DOCUMENT_COUNT"
        );

        println!("{}", builder.minimum_document_size);

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

        builder
    }

    pub const fn default_expiry_hours(&self) -> Option<usize> {
        self.default_expiry_hours
    }

    pub const fn default_maximum_views(&self) -> Option<usize> {
        self.default_maximum_views
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
}

impl Default for SizeLimitConfig {
    fn default() -> Self {
        Self {
            default_expiry_hours: None,
            default_maximum_views: None,
            minimum_expiry_hours: None,
            minimum_total_document_count: 1,
            minimum_document_size: 1,
            minimum_total_document_size: 1,
            minimum_document_name_size: 3,
            maximum_expiry_hours: None,
            maximum_total_document_count: 10,
            maximum_document_size: 5_000_000,
            maximum_total_document_size: 10_000_000,
            maximum_document_name_size: 50,
        }
    }
}

#[derive(Debug, Clone, Builder)]
#[builder(default)]
pub struct RateLimitConfig {
    /// Global rate limiter.
    global: u32,
    /// Global paste rate limiter.
    global_paste: u32,
    /// Get paste rate limiter.
    get_paste: u32,
    /// Post paste rate limiter.
    post_paste: u32,
    /// Patch paste rate limiter.
    patch_paste: u32,
    /// Delete paste rate limiter.
    delete_paste: u32,
    /// Global paste rate limiter.
    global_document: u32,
    /// Get paste rate limiter.
    get_document: u32,
    /// Post paste rate limiter.
    post_document: u32,
    /// Patch paste rate limiter.
    patch_document: u32,
    /// Delete paste rate limiter.
    delete_document: u32,
    /// Global config rate limiter.
    global_config: u32,
    /// Get config rate limiter.
    get_config: u32,
}

impl RateLimitConfig {
    pub fn builder() -> RateLimitConfigBuilder {
        RateLimitConfigBuilder::default()
    }

    #[allow(clippy::too_many_lines)]
    pub fn from_env(fetch_env: bool) -> Self {
        if fetch_env {
            from_filename(".env").ok();
        }

        let defaults = Self::default();

        Self::builder()
            .global(
                std::env::var("RATE_LIMIT_GLOBAL").map_or(defaults.global, |v| {
                    v.parse().expect("RATE_LIMIT_GLOBAL requires an integer.")
                }),
            )
            .global_paste(std::env::var("RATE_LIMIT_GLOBAL_PASTE").map_or(
                defaults.global_paste,
                |v| {
                    v.parse()
                        .expect("RATE_LIMIT_GLOBAL_PASTE requires an integer.")
                },
            ))
            .get_paste(
                std::env::var("RATE_LIMIT_GET_PASTE").map_or(defaults.get_paste, |v| {
                    v.parse()
                        .expect("RATE_LIMIT_GET_PASTE requires an integer.")
                }),
            )
            .post_paste(
                std::env::var("RATE_LIMIT_POST_PASTE").map_or(defaults.post_paste, |v| {
                    v.parse()
                        .expect("RATE_LIMIT_POST_PASTE requires an integer.")
                }),
            )
            .patch_paste(std::env::var("RATE_LIMIT_PATCH_PASTE").map_or(
                defaults.patch_paste,
                |v| {
                    v.parse()
                        .expect("RATE_LIMIT_PATCH_PASTE requires an integer.")
                },
            ))
            .delete_paste(std::env::var("RATE_LIMIT_DELETE_PASTE").map_or(
                defaults.delete_paste,
                |v| {
                    v.parse()
                        .expect("RATE_LIMIT_DELETE_PASTE requires an integer.")
                },
            ))
            .global_document(std::env::var("RATE_LIMIT_GLOBAL_DOCUMENT").map_or(
                defaults.global_document,
                |v| {
                    v.parse()
                        .expect("RATE_LIMIT_GLOBAL_DOCUMENT requires an integer.")
                },
            ))
            .get_document(std::env::var("RATE_LIMIT_GET_DOCUMENT").map_or(
                defaults.get_document,
                |v| {
                    v.parse()
                        .expect("RATE_LIMIT_GET_DOCUMENT requires an integer.")
                },
            ))
            .post_document(std::env::var("RATE_LIMIT_POST_DOCUMENT").map_or(
                defaults.post_document,
                |v| {
                    v.parse()
                        .expect("RATE_LIMIT_POST_DOCUMENT requires an integer.")
                },
            ))
            .patch_document(std::env::var("RATE_LIMIT_PATCH_DOCUMENT").map_or(
                defaults.patch_document,
                |v| {
                    v.parse()
                        .expect("RATE_LIMIT_PATCH_DOCUMENT requires an integer.")
                },
            ))
            .delete_document(std::env::var("RATE_LIMIT_DELETE_DOCUMENT").map_or(
                defaults.delete_document,
                |v| {
                    v.parse()
                        .expect("RATE_LIMIT_DELETE_DOCUMENT requires an integer.")
                },
            ))
            .global_config(std::env::var("RATE_LIMIT_GLOBAL_CONFIG").map_or(
                defaults.global_config,
                |v| {
                    v.parse()
                        .expect("RATE_LIMIT_GLOBAL_CONFIG requires an integer.")
                },
            ))
            .get_config(
                std::env::var("RATE_LIMIT_GET_CONFIG").map_or(defaults.get_config, |v| {
                    v.parse()
                        .expect("RATE_LIMIT_GET_CONFIG requires an integer.")
                }),
            )
            .build()
            .expect("Failed to create application rate limit configuration.")
    }

    pub const fn global(&self) -> u32 {
        self.global
    }

    pub const fn global_paste(&self) -> u32 {
        self.global_paste
    }

    pub const fn get_paste(&self) -> u32 {
        self.get_paste
    }

    pub const fn post_paste(&self) -> u32 {
        self.post_paste
    }

    pub const fn patch_paste(&self) -> u32 {
        self.patch_paste
    }

    pub const fn delete_paste(&self) -> u32 {
        self.delete_paste
    }

    pub const fn global_document(&self) -> u32 {
        self.global_document
    }

    pub const fn get_document(&self) -> u32 {
        self.get_document
    }

    pub const fn post_document(&self) -> u32 {
        self.post_document
    }

    pub const fn patch_document(&self) -> u32 {
        self.patch_document
    }

    pub const fn delete_document(&self) -> u32 {
        self.delete_document
    }

    pub const fn global_config(&self) -> u32 {
        self.global_config
    }

    pub const fn get_config(&self) -> u32 {
        self.get_config
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global: 800,
            global_paste: 500,
            get_paste: 200,
            post_paste: 100,
            patch_paste: 120,
            delete_paste: 200,
            global_document: 500,
            get_document: 200,
            post_document: 100,
            patch_document: 120,
            delete_document: 200,
            global_config: 200,
            get_config: 200,
        }
    }
}
