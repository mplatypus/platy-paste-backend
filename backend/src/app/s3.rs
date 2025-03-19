use aws_sdk_s3::{error::SdkError, operation::head_bucket::HeadBucketError, primitives::ByteStream};
use bytes::{Bytes, BytesMut};

use crate::models::{error::AppError, snowflake::Snowflake};

use super::app::{ApplicationState, S3Client};

use std::sync::{Arc, Weak};

#[derive(Debug, Clone)]
pub struct S3Service {
    app: Weak<ApplicationState>,
    client: S3Client,
}

impl S3Service {
    pub const fn new(client: S3Client) -> Self {
        Self {
            client,
            app: Weak::new(),
        }
    }

    pub fn bind_to(&mut self, app: Weak<ApplicationState>) {
        self.app = app;
    }

    pub fn app(&self) -> Arc<ApplicationState> {
        self.app
            .upgrade()
            .expect("Application state has been dropped.")
    }

    pub const fn client(&self) -> &S3Client {
        &self.client
    }

    pub const fn document_bucket_name(&self) -> &'static str {
        "documents"
    }

    pub async fn create_buckets(&self) -> Result<(), AppError> {
        match self.client.head_bucket().bucket(self.document_bucket_name()).send().await {
            Ok(_) => {
                tracing::info!("S3 Bucket {} already exists, skipping creation.", self.document_bucket_name());
            }
            Err(SdkError::ServiceError(e)) if matches!(e.err(), HeadBucketError::NotFound(_)) => {
                self.client.create_bucket().bucket(self.document_bucket_name()).send().await?;
                tracing::info!("Created S3 bucket: {}", self.document_bucket_name());
            }
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    pub async fn fetch_document(&self, document_id: Snowflake) -> Result<Bytes, AppError> {
        let id: String = document_id.into();

        let mut data = self.client
            .get_object()
            .bucket(self.document_bucket_name())
            .key(id)
            .send()
            .await?;

        let mut bytes = BytesMut::new();
        while let Some(chunk) = data.body.next().await {
            bytes.extend_from_slice(&chunk.expect("Failed to read S3 object chunk"));
        }

        Ok(bytes.freeze())
    }

    pub async fn create_document(&self, document_id: Snowflake, data: Bytes) -> Result<(), AppError> {
        let id: String = document_id.into();

        self.client
            .put_object()
            .bucket(self.document_bucket_name())
            .content_type("text/plain")
            .key(format!("{}.txt", id))
            .body(data.into())
            .send()
            .await?;

        Ok(())
    }

    pub async fn delete_document(&self, document_id: Snowflake) -> Result<(), AppError> {
        let id: String = document_id.into();

        self.client
            .delete_object()
            .bucket(self.document_bucket_name())
            .key(id)
            .send()
            .await?;

        Ok(())
    }
}
