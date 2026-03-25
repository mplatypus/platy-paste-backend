//! Paths, Queries, Bodies and Responses related to the document endpoints.

use serde::Deserialize;

use crate::models::{
    document::DocumentUpdateParameters,
    errors::RESTError,
    snowflake::{PartialSnowflake, Snowflake},
    undefined::Undefined,
};

//------//
// Path //
//------//

/// ## Document Path
///
/// The values within the path of a document endpoint.
#[derive(Deserialize)]
pub struct DocumentPath {
    /// The paste ID.
    paste_id: Snowflake,
    /// The document ID.
    document_id: Snowflake,
}

impl DocumentPath {
    /// The paste ID found within the path.
    #[inline]
    pub const fn paste_id(&self) -> &Snowflake {
        &self.paste_id
    }

    /// The document ID found within the path.
    #[inline]
    pub const fn document_id(&self) -> &Snowflake {
        &self.document_id
    }
}

/// Used for getting documents.
pub type GetDocumentPath = DocumentPath;

//------//
// Body //
//------//

/// ## Post Paste Document Body
///
/// The document body extracted from the actual body after parsing.
#[derive(Deserialize, Clone)]
pub struct PostPasteDocumentBody {
    /// The ID of the document.
    ///
    /// This is **not** a snowflake.
    /// This is an integer, specifying which document it is referencing in the multipart form data.
    id: PartialSnowflake,
    /// The name of the document.
    name: String,
}

impl PostPasteDocumentBody {
    /// The ID of the document.
    ///
    /// This is **not** a snowflake.
    /// This is an integer, specifying which document it is referencing in the multipart form data.
    #[inline]
    pub const fn id(&self) -> &PartialSnowflake {
        &self.id
    }

    /// The name of the document.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// ## Patch Paste Document Body
///
/// The document body extracted from the actual body after parsing.
#[derive(Deserialize, Clone)]
pub struct PatchPasteDocumentBody {
    /// The ID of the document.
    ///
    /// This is **not** a snowflake.
    /// This is an integer, specifying which document it is referencing in the multipart form data.
    id: PartialSnowflake,
    /// The name of the document.
    #[serde(default)]
    name: Undefined<String>,
}

impl PatchPasteDocumentBody {
    /// The ID of the document.
    ///
    /// This is **not** a snowflake.
    /// This is an integer, specifying which document it is referencing in the multipart form data.
    #[inline]
    pub const fn id(&self) -> &PartialSnowflake {
        &self.id
    }

    /// The name of the document.
    #[inline]
    pub fn name(&self) -> Undefined<&str> {
        self.name.as_deref()
    }
}

impl TryFrom<PatchPasteDocumentBody> for PostPasteDocumentBody {
    type Error = RESTError;

    fn try_from(value: PatchPasteDocumentBody) -> Result<Self, Self::Error> {
        let Undefined::Some(name) = value.name else {
            return Err(RESTError::BadRequest(format!(
                "The new document {} requires the `name` parameter.",
                value.id()
            )));
        };

        Ok(PostPasteDocumentBody { id: value.id, name })
    }
}

impl From<PatchPasteDocumentBody> for DocumentUpdateParameters {
    fn from(value: PatchPasteDocumentBody) -> Self {
        Self::new(Undefined::Undefined, value.name, Undefined::Undefined)
    }
}

impl From<&PatchPasteDocumentBody> for DocumentUpdateParameters {
    fn from(value: &PatchPasteDocumentBody) -> Self {
        Self::new(
            Undefined::Undefined,
            value.name.clone(),
            Undefined::Undefined,
        )
    }
}
