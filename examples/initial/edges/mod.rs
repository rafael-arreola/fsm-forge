//! One type per transition.

mod approve_document;
mod move_orphan;
mod submit_document;

pub use approve_document::ApproveDocument;
pub use move_orphan::MoveOrphan;
pub use submit_document::SubmitDocument;
