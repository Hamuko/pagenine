use serde::{Deserialize, Serialize};

/// JSON response from https://a.4cdn.org/BOARD/catalog.json.
pub type APICatalog = Vec<APIPage>;

/// Top-level object in the catalog response.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct APIPage {
    pub page: i32,
    pub threads: Vec<APIThread>,
}

/// Partial schema for each thread.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct APIThread {
    pub no: i32,
    pub sub: String,
}
