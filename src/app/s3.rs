use aws_sdk_s3::{
    error::SdkError, operation::head_bucket::HeadBucketError, primitives::ByteStream,
};
use bytes::{Bytes, BytesMut};

use crate::models::{document::Document, error::AppError};

use super::application::{ApplicationState, S3Client};

use std::sync::{Arc, Weak};

#[derive(Debug, Clone)]
pub struct S3Service {
    app: Weak<ApplicationState>,
    client: S3Client,
}

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

impl S3Service {
    /// New.
    ///
    /// Create a new S3 Service.
    ///
    /// ## Arguments
    ///
    /// - `client`: The client to use.
    pub const fn new(client: S3Client) -> Self {
        Self {
            client,
            app: Weak::new(),
        }
    }

    /// Bind to.
    ///
    /// Bind the application to the S3 Service.
    ///
    /// ## Arguments
    ///
    /// - `app`: The application to bind.
    pub fn bind_to(&mut self, app: Weak<ApplicationState>) {
        self.app = app;
    }

    /// The application attached to the client.
    pub fn app(&self) -> Arc<ApplicationState> {
        self.app
            .upgrade()
            .expect("Application state has been dropped.")
    }

    /// The S3 client attached to this service.
    pub const fn client(&self) -> &S3Client {
        &self.client
    }

    /// The document bucket name.
    pub const fn document_bucket_name(&self) -> &'static str {
        "documents"
    }

    /// Create buckets.
    ///
    /// Create the initial set of bucket(s).
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - If the buckets fail to be created.
    pub async fn create_buckets(&self) -> Result<(), AppError> {
        match self
            .client
            .head_bucket()
            .bucket(self.document_bucket_name())
            .send()
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "S3 Bucket {} already exists, skipping creation.",
                    self.document_bucket_name()
                );
            }
            Err(SdkError::ServiceError(e)) if matches!(e.err(), HeadBucketError::NotFound(_)) => {
                self.client
                    .create_bucket()
                    .bucket(self.document_bucket_name())
                    .send()
                    .await?;

                self.client
                    .put_bucket_policy()
                    .bucket(self.document_bucket_name())
                    .policy(POLICY.replace("{name}", self.document_bucket_name()))
                    .send()
                    .await?;

                tracing::info!("Created S3 bucket: {}", self.document_bucket_name());
            }
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    /// Fetch a document
    ///
    /// Fetch an existing document.
    ///
    /// ## Arguments
    ///
    /// - `document_path` - The built path of the document.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - When the document cannot be found, or a read failure happens.
    ///
    /// ## Returns
    ///
    /// The [`Bytes`] of the document.
    pub async fn fetch_document(&self, document_path: String) -> Result<Bytes, AppError> {
        let mut data = self
            .client
            .get_object()
            .bucket(self.document_bucket_name())
            .key(document_path)
            .send()
            .await?;

        let mut bytes = BytesMut::new();
        while let Some(chunk) = data.body.next().await {
            bytes.extend_from_slice(&chunk.expect("Failed to read S3 object chunk"));
        }

        Ok(bytes.freeze())
    }

    /// Create a document
    ///
    /// Create a new document.
    ///
    /// ## Arguments
    ///
    /// - `document`: The [`Document`].
    /// - `content`: The content of the document.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - When the document could not be created.
    pub async fn create_document(
        &self,
        document: &Document,
        content: impl Into<Bytes>,
    ) -> Result<(), AppError> {
        self.client
            .put_object()
            .bucket(self.document_bucket_name())
            .content_type(document.doc_type())
            .key(document.generate_path())
            .body(ByteStream::from(content.into()))
            .send()
            .await?;

        Ok(())
    }

    /// Delete a document
    ///
    /// Delete an existing document.
    ///
    /// ## Arguments
    ///
    /// - `document_path`: The built path of the document.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - When the document could not be deleted.
    pub async fn delete_document(&self, document_path: String) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(self.document_bucket_name())
            .key(document_path)
            .send()
            .await?;

        Ok(())
    }
}
