use async_trait::async_trait;
use log::error;

const PUSHOVER_API_URL: &str = "https://api.pushover.net/1/messages.json";

#[derive(Default, Debug)]
pub struct PushoverClient {
    pub token: String,
    pub user: String,
}

#[async_trait]
pub trait PushoverClientTrait {
    async fn send_notification(&self, message: String, title: Option<&String>) -> Result<(), ()>;
}

#[async_trait]
impl PushoverClientTrait for PushoverClient {
    async fn send_notification(&self, message: String, title: Option<&String>) -> Result<(), ()> {
        let mut params = Vec::from([
            ("token", &self.token),
            ("user", &self.user),
            ("message", &message),
        ]);
        if let Some(title) = &title {
            params.push(("title", title));
        }
        let client = reqwest::Client::new();
        let response = match client.post(PUSHOVER_API_URL).form(&params).send().await {
            Ok(response) => response,
            Err(e) => {
                error!("{:?}", e);
                return Err(());
            }
        };
        return Ok(());
    }
}
