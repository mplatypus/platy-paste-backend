use serde::Deserialize;

use crate::models::{
    errors::RESTError,
    payload::paste::PastePath,
    snowflake::{PartialSnowflake, Snowflake},
    undefined::Undefined,
};

//------//
// Path //
//------//

#[derive(Deserialize)]
pub struct DocumentPath {
    /// The paste ID.
    paste_id: Snowflake,
    /// The document ID.
    document_id: Snowflake,
}

impl DocumentPath {
    #[inline]
    pub const fn paste_id(&self) -> &Snowflake {
        &self.paste_id
    }

    #[inline]
    pub const fn document_id(&self) -> &Snowflake {
        &self.document_id
    }
}

pub type PostDocumentPath = PastePath;

pub type GetDocumentPath = DocumentPath;

pub type PatchDocumentPath = DocumentPath;

pub type DeleteDocumentPath = DocumentPath;

//------//
// Body //
//------//

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
    #[inline]
    pub const fn id(&self) -> &PartialSnowflake {
        &self.id
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }
}

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
    #[inline]
    pub const fn id(&self) -> &PartialSnowflake {
        &self.id
    }

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

//----------//
// Response //
//----------//
