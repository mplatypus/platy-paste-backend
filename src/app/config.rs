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
    /// Global rate limiter.
    global_rate_limiter: u32,
    /// Global paste rate limiter.
    global_paste_rate_limiter: u32,
    /// Get pastes rate limiter.
    get_pastes_rate_limiter: u32,
    /// Get paste rate limiter.
    get_paste_rate_limiter: u32,
    /// Post paste rate limiter.
    post_paste_rate_limiter: u32,
    /// Patch paste rate limiter.
    patch_paste_rate_limiter: u32,
    /// Delete paste rate limiter.
    delete_paste_rate_limiter: u32,
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
                std::env::var("GLOBAL_PASTE_TOTAL_DOCUMENT_COUNT").map_or(10, |v| {
                    v.parse()
                        .expect("GLOBAL_PASTE_TOTAL_DOCUMENT_COUNT requires an integer.")
                }),
            )
            .global_paste_total_document_size_limit(
                std::env::var("SIZE_LIMIT_GLOBAL_PASTE_TOTAL_DOCUMENT").map_or(100, |v| {
                    v.parse()
                        .expect("SIZE_LIMIT_GLOBAL_PASTE_TOTAL_DOCUMENT requires an integer.")
                }),
            )
            .global_paste_document_size_limit(
                std::env::var("SIZE_LIMIT_GLOBAL_PASTE_DOCUMENT").map_or(15, |v| {
                    v.parse()
                        .expect("SIZE_LIMIT_GLOBAL_PASTE_DOCUMENT requires an integer.")
                }),
            )
            .global_rate_limiter(std::env::var("RATE_LIMIT_GLOBAL").map_or(500, |v| {
                v.parse().expect("RATE_LIMIT_GLOBAL requires an integer.")
            }))
            .global_paste_rate_limiter(std::env::var("RATE_LIMIT_GLOBAL_PASTE").map_or(500, |v| {
                v.parse()
                    .expect("RATE_LIMIT_GLOBAL_PASTE requires an integer.")
            }))
            .get_pastes_rate_limiter(std::env::var("RATE_LIMIT_GET_PASTES").map_or(40, |v| {
                v.parse()
                    .expect("RATE_LIMIT_GET_PASTES requires an integer.")
            }))
            .get_paste_rate_limiter(std::env::var("RATE_LIMIT_GET_PASTE").map_or(200, |v| {
                v.parse()
                    .expect("RATE_LIMIT_GET_PASTE requires an integer.")
            }))
            .post_paste_rate_limiter(std::env::var("RATE_LIMIT_POST_PASTE").map_or(100, |v| {
                v.parse()
                    .expect("RATE_LIMIT_POST_PASTE requires an integer.")
            }))
            .patch_paste_rate_limiter(std::env::var("RATE_LIMIT_PATCH_PASTE").map_or(120, |v| {
                v.parse()
                    .expect("RATE_LIMIT_PATCH_PASTE requires an integer.")
            }))
            .delete_paste_rate_limiter(std::env::var("RATE_LIMIT_DELETE_PASTE").map_or(200, |v| {
                v.parse()
                    .expect("RATE_LIMIT_DELETE_PASTE requires an integer.")
            }))
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

    pub const fn global_rate_limiter(&self) -> u32 {
        self.global_rate_limiter
    }

    pub const fn global_paste_rate_limiter(&self) -> u32 {
        self.global_paste_rate_limiter
    }

    pub const fn get_pastes_rate_limiter(&self) -> u32 {
        self.get_pastes_rate_limiter
    }

    pub const fn get_paste_rate_limiter(&self) -> u32 {
        self.get_paste_rate_limiter
    }

    pub const fn post_paste_rate_limiter(&self) -> u32 {
        self.post_paste_rate_limiter
    }

    pub const fn patch_paste_rate_limiter(&self) -> u32 {
        self.patch_paste_rate_limiter
    }

    pub const fn delete_paste_rate_limiter(&self) -> u32 {
        self.delete_paste_rate_limiter
    }
}
