use crate::data;
use chrono::prelude::{DateTime, Utc};
use reqwest::header::{HeaderValue, IF_MODIFIED_SINCE};
use serde::{Deserialize, Serialize};
use std::iter::IntoIterator;

/// 4chan API catalog response.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Catalog(Vec<Page>);

impl IntoIterator for Catalog {
    type Item = Page;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Catalog {
    /// Fetch the current catalog from the API.
    pub async fn fetch(
        board: &String,
        if_modified_since: Option<DateTime<Utc>>,
    ) -> Result<Catalog, Box<dyn std::error::Error>> {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(dt) = if_modified_since {
            let dt_str = dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
            if let Ok(header_value) = HeaderValue::from_str(dt_str.as_str()) {
                headers.insert(IF_MODIFIED_SINCE, header_value);
            }
        }

        let client = reqwest::Client::new();
        let url = format!("https://a.4cdn.org/{}/catalog.json", board);
        let response = client.get(url).headers(headers).send().await?;
        let catalog = response.json::<Catalog>().await?;
        Ok(catalog)
    }

    /// Find the first thread with the matching title.
    pub fn find(self: &Self, title: &String) -> Option<data::Thread> {
        for page in self.clone() {
            let page_length = page.threads.len() as i32;
            for (index, thread) in page.threads.into_iter().enumerate() {
                if thread.sub.contains(title) {
                    return Some(data::Thread {
                        page: page.page,
                        no: thread.no,
                        sub: thread.sub,
                        time: chrono::offset::Utc::now(),
                        position: index as i32 + 1,
                        page_length: page_length,
                    });
                }
            }
        }
        return None;
    }
}

/// Top-level object in the catalog response.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub page: i32,
    pub threads: Vec<Thread>,
}

/// Partial schema for each thread.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    pub no: i32,
    pub sub: String,
}
