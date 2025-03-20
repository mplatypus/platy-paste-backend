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
    minio_user: String,
    /// The Minio Password.
    minio_password: SecretString,
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
            .minio_user(
                std::env::var("MINIO_USER").expect("MINIO_USER environment variable must be set."),
            )
            .minio_password(
                std::env::var("MINIO_PASSWORD")
                    .expect("MINIO_PASSWORD environment variable must be set.")
                    .into(),
            )
            .build()
            .expect("Failed to create application configuration.")
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

    pub fn minio_user(&self) -> String {
        self.minio_user.clone()
    }

    pub fn minio_password(&self) -> SecretString {
        self.minio_password.clone()
    }
}
