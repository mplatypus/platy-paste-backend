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
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

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
}
