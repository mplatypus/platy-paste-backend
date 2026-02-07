use aws_config::{BehaviorVersion, Region};

use aws_sdk_s3::{
    Client as S3Client, Config as S3Config, config::Credentials, error::SdkError,
    operation::head_bucket::HeadBucketError, primitives::ByteStream,
};
use bytes::{Bytes, BytesMut};
use secrecy::ExposeSecret as _;
#[cfg(test)]
use tokio::sync::Mutex;

use crate::{
    app::config::{ObjectStoreConfig, S3ObjectStoreConfig},
    models::{document::Document, errors::ObjectStoreError},
};

use super::application::ApplicationState;

#[cfg(test)]
use std::collections::HashMap;
use std::sync::{Arc, Weak};

/// The document buckets name.
const DOCUMENT_BUCKET: &str = "documents";

/// All the buckets that this application uses.
const BUCKETS: [&str; 1] = [DOCUMENT_BUCKET];

pub trait ObjectStoreExt: Sized {
    /// Binds the application to the object store.
    fn bind_app(&mut self, app: Weak<ApplicationState>);

    /// The application related to the object store.
    fn app(&self) -> Arc<ApplicationState>;

    /// Create buckets.
    ///
    /// Create the initial set of bucket(s).
    ///
    /// ## Errors
    ///
    /// - [`ObjectStoreError`] - If the buckets fail to be created.
    async fn create_buckets(&self) -> Result<(), ObjectStoreError>;

    /// Fetch a document
    ///
    /// Fetch an existing document.
    ///
    /// ## Arguments
    ///
    /// - `document` - The document object to fetch.
    ///
    /// ## Errors
    ///
    /// - [`ObjectStoreError`] - When the document cannot be found, or a read failure happens.
    ///
    /// ## Returns
    ///
    async fn fetch_document(&self, document: &Document) -> Result<Bytes, ObjectStoreError>;

    /// Create a document
    ///
    /// Create a new document.
    ///
    /// ## Arguments
    ///
    /// - `document` - The [`Document`].
    /// - `content` - The content of the document.
    ///
    /// ## Errors
    ///
    /// - [`ObjectStoreError`] - When the document could not be created.
    async fn create_document(
        &self,
        document: &Document,
        content: impl Into<Bytes>,
    ) -> Result<(), ObjectStoreError>;

    /// Delete a document
    ///
    /// Delete an existing document.
    ///
    /// ## Arguments
    ///
    /// - `document` - The document object to fetch.
    ///
    /// ## Errors
    ///
    /// - [`ObjectStoreError`] - When the document could not be deleted.
    async fn delete_document(&self, document: &Document) -> Result<(), ObjectStoreError>;
}

#[derive(Debug, Clone)]
pub enum ObjectStore {
    S3(S3ObjectStore),
    #[cfg(test)]
    Test(TestObjectStore),
}

impl ObjectStore {
    pub fn from_config(config: &ObjectStoreConfig) -> Result<Self, ObjectStoreError> {
        match config {
            ObjectStoreConfig::S3(config) => Ok(Self::S3(S3ObjectStore::from_config(config))),
            #[cfg(test)]
            ObjectStoreConfig::Test => Ok(ObjectStore::Test(TestObjectStore::new())),
        }
    }
}

impl ObjectStoreExt for ObjectStore {
    fn bind_app(&mut self, app: Weak<ApplicationState>) {
        match self {
            Self::S3(os) => os.bind_app(app),
            #[cfg(test)]
            Self::Test(os) => os.bind_app(app),
        }
    }

    fn app(&self) -> Arc<ApplicationState> {
        match self {
            Self::S3(os) => os.app(),
            #[cfg(test)]
            Self::Test(os) => os.app(),
        }
    }

    async fn create_buckets(&self) -> Result<(), ObjectStoreError> {
        match self {
            Self::S3(os) => os.create_buckets().await,
            #[cfg(test)]
            Self::Test(os) => os.create_buckets().await,
        }
    }

    async fn fetch_document(&self, document: &Document) -> Result<Bytes, ObjectStoreError> {
        match self {
            Self::S3(os) => os.fetch_document(document).await,
            #[cfg(test)]
            Self::Test(os) => os.fetch_document(document).await,
        }
    }

    async fn create_document(
        &self,
        document: &Document,
        content: impl Into<Bytes>,
    ) -> Result<(), ObjectStoreError> {
        match self {
            Self::S3(os) => os.create_document(document, content).await,
            #[cfg(test)]
            Self::Test(os) => os.create_document(document, content).await,
        }
    }

    async fn delete_document(&self, document: &Document) -> Result<(), ObjectStoreError> {
        match self {
            Self::S3(os) => os.delete_document(document).await,
            #[cfg(test)]
            Self::Test(os) => os.delete_document(document).await,
        }
    }
}

#[derive(Debug, Clone)]
pub struct S3ObjectStore {
    app: Weak<ApplicationState>,
    client: S3Client,
}

impl S3ObjectStore {
    const POLICY: &str = r#"
    {
    "Version": "2012-10-17",
    "Statement": [
        {
        "Effect": "Allow",
        "Principal": "*",
        "Action": "s3:GetObject",
        "Resource": "arn:aws:s3:::{name}/*"
        }
    ]
    }
    "#;

    pub fn from_config(config: &S3ObjectStoreConfig) -> Self {
        let s3creds = Credentials::new(
            config.access_key().expose_secret(),
            config.secret_key().expose_secret(),
            None,
            None,
            "paste",
        );

        let s3conf = S3Config::builder()
            //.region(Region::new("vault"))
            .endpoint_url(config.url())
            .credentials_provider(s3creds)
            .region(Region::new("direct"))
            .force_path_style(true) // MinIO does not support virtual hosts
            .behavior_version(BehaviorVersion::v2025_08_07())
            .build();

        Self {
            app: Weak::new(),
            client: S3Client::from_conf(s3conf),
        }
    }

    /// The S3 client attached to this service.
    pub const fn client(&self) -> &S3Client {
        &self.client
    }
}

impl ObjectStoreExt for S3ObjectStore {
    /// Bind app.
    ///
    /// Bind the application to the S3 Service.
    ///
    /// ## Arguments
    ///
    /// - `app`: The application to bind.
    fn bind_app(&mut self, app: Weak<ApplicationState>) {
        self.app = app;
    }

    /// The application attached to the client.
    fn app(&self) -> Arc<ApplicationState> {
        self.app
            .upgrade()
            .expect("Application state has been dropped.")
    }

    async fn create_buckets(&self) -> Result<(), ObjectStoreError> {
        for bucket in BUCKETS {
            match self.client.head_bucket().bucket(bucket).send().await {
                Ok(_) => {
                    tracing::info!("S3 Bucket {} already exists, skipping creation.", bucket);
                }
                Err(SdkError::ServiceError(e))
                    if matches!(e.err(), HeadBucketError::NotFound(_)) =>
                {
                    self.client.create_bucket().bucket(bucket).send().await?;

                    self.client
                        .put_bucket_policy()
                        .bucket(bucket)
                        .policy(Self::POLICY.replace("{name}", bucket))
                        .send()
                        .await?;

                    tracing::info!("Created S3 bucket: {}", bucket);
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(())
    }

    async fn fetch_document(&self, document: &Document) -> Result<Bytes, ObjectStoreError> {
        let mut data = self
            .client
            .get_object()
            .bucket(DOCUMENT_BUCKET)
            .key(document.generate_path())
            .send()
            .await?;

        let mut bytes = BytesMut::new();
        while let Some(chunk) = data.body.next().await {
            bytes.extend_from_slice(&chunk.expect("Failed to read S3 object chunk"));
        }

        Ok(bytes.freeze())
    }

    async fn create_document(
        &self,
        document: &Document,
        content: impl Into<Bytes>,
    ) -> Result<(), ObjectStoreError> {
        self.client
            .put_object()
            .bucket(DOCUMENT_BUCKET)
            .content_type(document.doc_type())
            .key(document.generate_path())
            .body(ByteStream::from(content.into()))
            .send()
            .await?;

        Ok(())
    }

    async fn delete_document(&self, document: &Document) -> Result<(), ObjectStoreError> {
        self.client
            .delete_object()
            .bucket(DOCUMENT_BUCKET)
            .key(document.generate_path())
            .send()
            .await?;

        Ok(())
    }
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct TestObjectStore {
    app: Weak<ApplicationState>,
    buckets: Arc<Mutex<Vec<String>>>,
    data: Arc<Mutex<HashMap<(String, String), Bytes>>>,
}

#[cfg(test)]
impl TestObjectStore {
    pub fn new() -> Self {
        use tokio::sync::Mutex;

        Self {
            app: Weak::new(),
            buckets: Arc::new(Mutex::new(Vec::new())),
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
impl ObjectStoreExt for TestObjectStore {
    fn bind_app(&mut self, app: Weak<ApplicationState>) {
        self.app = app;
    }

    fn app(&self) -> Arc<ApplicationState> {
        self.app
            .upgrade()
            .expect("Application state has been dropped.")
    }

    async fn create_buckets(&self) -> Result<(), ObjectStoreError> {
        for bucket in BUCKETS {
            let mut bucket_lock = self.buckets.lock().await;

            if bucket_lock.contains(&bucket.to_string()) {
                continue;
            }

            bucket_lock.push(bucket.to_string());
        }

        Ok(())
    }

    async fn fetch_document(&self, document: &Document) -> Result<Bytes, ObjectStoreError> {
        let data_lock = self.data.lock().await;

        let document_contents =
            data_lock.get(&(DOCUMENT_BUCKET.to_string(), document.generate_path()));

        match document_contents {
            Some(contents) => Ok(contents.clone()),
            None => Ok(Bytes::new()),
        }
    }

    async fn create_document(
        &self,
        document: &Document,
        content: impl Into<Bytes>,
    ) -> Result<(), ObjectStoreError> {
        // FIXME: Check bucket exists.
        let mut data_lock = self.data.lock().await;

        if data_lock.contains_key(&(DOCUMENT_BUCKET.to_string(), document.generate_path())) {
            panic!("Key already exists!")
        }

        data_lock.insert(
            (DOCUMENT_BUCKET.to_string(), document.generate_path()),
            content.into(),
        );

        Ok(())
    }

    async fn delete_document(&self, document: &Document) -> Result<(), ObjectStoreError> {
        let mut data_lock = self.data.lock().await;

        data_lock.remove(&(DOCUMENT_BUCKET.to_string(), document.generate_path()));

        Ok(())
    }
}
