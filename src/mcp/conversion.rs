use rmcp::model::{Content};

// Extension trait to add from_raw functionality to Content
pub trait ContentExt {
    fn from_raw<T: Into<String>>(text: T) -> Self;
}

impl ContentExt for Content {
    fn from_raw<T: Into<String>>(text: T) -> Self {
        Content {
            raw: rmcp::model::RawContent::text(text),
            annotations: None,
        }
    }
}
