use derive_builder::Builder;
use dotenvy::from_filename;
use secrecy::{SecretBox, SecretString};

#[derive(Debug, Clone, Builder)]
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
    /// The maximum expiry for pastes.
    maximum_expiry_hours: Option<usize>,
    /// The default expiry for pastes.
    default_expiry_hours: Option<usize>,
    /// The maximum allowed documents in a paste.
    global_paste_total_document_count: usize,
    /// Maximum paste body size.
    global_paste_total_document_size_limit: usize,
    /// Individual paste document size.
    global_paste_document_size_limit: usize,
    // Rate limits.
    rate_limits: RateLimitConfig,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    #[allow(clippy::too_many_lines)]
    pub fn from_env() -> Self {
        from_filename(".env").ok();
        let builder = Self::builder()
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
            .maximum_expiry_hours(std::env::var("MAXIMUM_EXPIRY_HOURS").ok().map(|v| {
                v.parse()
                    .expect("MAXIMUM_EXPIRY_HOURS requires an integer.")
            }))
            .default_expiry_hours(std::env::var("DEFAULT_EXPIRY_HOURS").ok().map(|v| {
                v.parse()
                    .expect("DEFAULT_EXPIRY_HOURS requires an integer.")
            }))
            .global_paste_total_document_count(
                std::env::var("GLOBAL_PASTE_TOTAL_DOCUMENT_COUNT").map_or(
                    Self::default().global_paste_total_document_count,
                    |v| {
                        v.parse()
                            .expect("GLOBAL_PASTE_TOTAL_DOCUMENT_COUNT requires an integer.")
                    },
                ),
            )
            .global_paste_total_document_size_limit(
                std::env::var("SIZE_LIMIT_GLOBAL_PASTE_TOTAL_DOCUMENT").map_or(
                    Self::default().global_paste_document_size_limit,
                    |v| {
                        v.parse()
                            .expect("SIZE_LIMIT_GLOBAL_PASTE_TOTAL_DOCUMENT requires an integer.")
                    },
                ),
            )
            .global_paste_document_size_limit(
                std::env::var("SIZE_LIMIT_GLOBAL_PASTE_DOCUMENT").map_or(
                    Self::default().global_paste_document_size_limit,
                    |v| {
                        v.parse()
                            .expect("SIZE_LIMIT_GLOBAL_PASTE_DOCUMENT requires an integer.")
                    },
                ),
            )
            .rate_limits(RateLimitConfig::from_env(false))
            .build()
            .expect("Failed to create application configuration.");

        if let (Some(maximum_expiry_hours), Some(default_expiry_hours)) =
            (builder.maximum_expiry_hours, builder.default_expiry_hours)
        {
            assert!(
                (maximum_expiry_hours >= default_expiry_hours),
                "The DEFAULT_EXPIRY_HOURS must be equal to or less than MAXIMUM_EXPIRY_HOURS"
            );
        }

        builder
    }

    pub fn host(&self) -> String {
        self.host.clone()
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    pub fn database_url(&self) -> String {
        self.database_url.clone()
    }

    pub fn s3_url(&self) -> String {
        self.s3_url.clone()
    }

    pub fn s3_access_key(&self) -> SecretString {
        self.s3_access_key.clone()
    }

    pub fn s3_secret_key(&self) -> SecretString {
        self.s3_secret_key.clone()
    }

    pub fn minio_root_user(&self) -> String {
        self.minio_root_user.clone()
    }

    pub fn minio_root_password(&self) -> SecretString {
        self.minio_root_password.clone()
    }

    pub fn domain(&self) -> String {
        self.domain.clone()
    }

    pub const fn maximum_expiry_hours(&self) -> Option<usize> {
        self.maximum_expiry_hours
    }

    pub const fn default_expiry_hours(&self) -> Option<usize> {
        self.default_expiry_hours
    }

    pub const fn global_paste_total_document_count(&self) -> usize {
        self.global_paste_total_document_count
    }

    pub const fn global_paste_total_document_size_limit(&self) -> usize {
        self.global_paste_total_document_size_limit
    }

    pub const fn global_paste_document_size_limit(&self) -> usize {
        self.global_paste_document_size_limit
    }

    pub fn rate_limits(&self) -> RateLimitConfig {
        self.rate_limits.clone()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::default(),
            port: Default::default(),
            database_url: String::default(),
            s3_url: String::default(),
            s3_access_key: SecretBox::default(),
            s3_secret_key: SecretBox::default(),
            minio_root_user: String::default(),
            minio_root_password: SecretBox::default(),
            domain: String::default(),
            maximum_expiry_hours: None,
            default_expiry_hours: None,
            global_paste_total_document_count: 10,
            global_paste_total_document_size_limit: 100,
            global_paste_document_size_limit: 15,
            rate_limits: RateLimitConfig::default(),
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
    /// Get pastes rate limiter.
    get_pastes: u32,
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
            .get_pastes(
                std::env::var("RATE_LIMIT_GET_PASTES").map_or(defaults.get_pastes, |v| {
                    v.parse()
                        .expect("RATE_LIMIT_GET_PASTES requires an integer.")
                }),
            )
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
            .build()
            .expect("Failed to create application configuration.")
    }

    pub const fn global(&self) -> u32 {
        self.global
    }

    pub const fn global_paste(&self) -> u32 {
        self.global_paste
    }

    pub const fn get_pastes(&self) -> u32 {
        self.get_pastes
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
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global: 800,
            global_paste: 500,
            get_pastes: 40,
            get_paste: 200,
            post_paste: 100,
            patch_paste: 120,
            delete_paste: 200,
            global_document: 500,
            get_document: 200,
            post_document: 100,
            patch_document: 120,
            delete_document: 200,
        }
    }
}
