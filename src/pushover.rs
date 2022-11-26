const PUSHOVER_API_URL: &str = "https://api.pushover.net/1/messages.json";

#[derive(Default, Debug)]
pub struct PushoverClient {
    pub token: String,
    pub user: String,
}

impl PushoverClient {
    pub async fn send_notification(self: &Self, message: String, title: Option<&String>) -> Result<(), ()> {
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
                println!("{:?}", e);
                return Err(());
            }
        };
        return Ok(());
    }
}
