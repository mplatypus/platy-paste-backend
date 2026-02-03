use serde::Deserialize;

use crate::models::{payload::paste::PastePath, snowflake::Snowflake};

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
pub struct PasteDocumentBody {
    /// The ID of the document.
    ///
    /// This is **not** a snowflake.
    /// This is an integer, specifying which document it is referencing in the multipart form data.
    id: usize,
    /// The name of the document.
    name: String,
}

impl PasteDocumentBody {
    #[inline]
    pub const fn id(&self) -> usize {
        self.id
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }
}

//----------//
// Response //
//----------//
