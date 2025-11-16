use chrono::prelude::{DateTime, Utc};
use clap::Parser;
use log::{info, warn, LevelFilter};
use simple_logger::SimpleLogger;
use std::time::Duration;
use tokio::{task, time};

mod api;
mod data;
mod pushover;

#[derive(Parser, Debug)]
pub struct PagenineArgs {
    /// Name of the board to scan.
    #[clap(value_parser = validate_board)]
    pub board: String,

    /// Title of the thread to scan.
    #[clap(value_parser)]
    pub title: String,

    /// Ignore threads that have reached bump limit.
    #[clap(long, value_parser)]
    pub no_bump_limit: bool,

    /// Pushover application API key.
    #[clap(long, value_parser)]
    pub pushover_application_api_token: Option<String>,

    /// Pushover user key.
    #[clap(long, value_parser)]
    pub pushover_user_key: Option<String>,
}

fn validate_board(value: &str) -> Result<String, String> {
    Ok(value.trim_matches('/').to_string())
}

async fn get_current_thread(
    board: &String,
    title: &String,
    if_modified_since: Option<DateTime<Utc>>,
) -> Option<data::Thread> {
    let catalog = match api::Catalog::fetch(board, if_modified_since).await {
        Ok(catalog) => catalog,
        Err(error) => {
            warn!("{}", error);
            return None;
        }
    };
    catalog.find(title)
}

async fn check(
    args: &PagenineArgs,
    pushover_client: &Option<impl pushover::PushoverClientTrait>,
    state: data::State,
) -> data::State {
    let refresh = state
        .thread
        .as_ref()
        .map_or(true, |thread| thread.check_if_needs_refresh());

    let thread = if refresh {
        let last_update_time = state.thread.as_ref().map(|thread| thread.time);
        get_current_thread(&args.board, &args.title, last_update_time).await
    } else {
        state.thread.clone()
    };
    let thread = match thread {
        Some(thread) => thread,
        None => return data::State::new(),
    };

    if refresh {
        info!(
            "\"{}\", page {} ({}/{})",
            thread.sub, thread.page, thread.position, thread.page_length
        );
    }

    return notify(state, thread, args.no_bump_limit, pushover_client).await;
}

async fn notify(
    state: data::State,
    thread: data::Thread,
    no_bump_limit: bool,
    pushover_client: &Option<impl pushover::PushoverClientTrait>,
) -> data::State {
    let mut notified = state.notified;
    if thread.page >= 9 && !(no_bump_limit && thread.bumplimit) && thread.page != state.notified {
        let notification_shown = match pushover_client {
            Some(pushover_client) => thread.send_pushover_notification(pushover_client).await,
            None => thread.show_notification(),
        };
        notified = match notification_shown {
            Ok(_) => thread.page,
            Err(_) => state.notified,
        }
    } else if thread.page < 9 {
        notified = 0;
    }
    return data::State {
        thread: Some(thread),
        notified,
    };
}

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .env()
        .init()
        .unwrap();
    let args = PagenineArgs::parse();

    let forever = task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        let mut state = data::State::new();
        let pushover_client: Option<pushover::PushoverClient> = match (
            &args.pushover_application_api_token,
            &args.pushover_user_key,
        ) {
            (Some(token), Some(user)) => Some(pushover::PushoverClient {
                token: token.to_string(),
                user: user.to_string(),
            }),
            _ => None,
        };

        loop {
            interval.tick().await;
            state = check(&args, &pushover_client, state).await;
        }
    });

    let _ = forever.await;
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::pushover::PushoverClientTrait;
    use async_trait::async_trait;
    use test_case::test_case;

    fn make_thread(page: i32) -> data::Thread {
        data::Thread {
            page,
            no: 123456,
            sub: String::from("x"),
            time: chrono::offset::Utc::now(),
            position: 1,
            page_length: 10,
            bumplimit: false,
        }
    }

    #[derive(Clone, Copy)]
    pub struct TestPushoverClient {
        disabled: bool,
        successful: bool,
    }

    impl TestPushoverClient {
        fn new() -> Self {
            return Self {
                disabled: false,
                successful: true,
            };
        }
    }

    #[async_trait]
    impl PushoverClientTrait for TestPushoverClient {
        async fn send_notification(
            self: &Self,
            _message: String,
            _title: Option<&String>,
        ) -> Result<(), ()> {
            assert!(!self.disabled);
            return match self.successful {
                true => Ok(()),
                false => Err(()),
            };
        }
    }

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        PagenineArgs::command().debug_assert()
    }

    #[test_case("vg", "vg"; "plain")]
    #[test_case("/vg/", "vg"; "with slashes")]
    fn args_validate_board(input: &str, output: &str) {
        assert_eq!(validate_board(&input), Ok(String::from(output)));
    }

    #[tokio::test]
    async fn notify_exceed_threshold() {
        let thread = make_thread(9);
        let state = data::State {
            thread: None,
            notified: 0,
        };
        let pushover_client = TestPushoverClient::new();
        let new_state = notify(state, thread.clone(), false, &Some(pushover_client)).await;
        assert_eq!(new_state.thread, Some(thread));
        assert_eq!(new_state.notified, 9);
    }

    #[tokio::test]
    async fn notify_exceed_threshold_no_bumplimit() {
        let thread = data::Thread {
            page: 9,
            no: 123456,
            sub: String::from("x"),
            time: chrono::offset::Utc::now(),
            position: 1,
            page_length: 10,
            bumplimit: true,
        };
        let state = data::State {
            thread: None,
            notified: 0,
        };
        let pushover_client = TestPushoverClient::new();
        let new_state = notify(state, thread.clone(), true, &Some(pushover_client)).await;
        assert_eq!(new_state.thread, Some(thread));
        assert_eq!(new_state.notified, 0);
    }

    #[tokio::test]
    async fn notify_exceed_threshold_notification_failure() {
        let thread = make_thread(9);
        let state = data::State {
            thread: None,
            notified: 0,
        };
        let mut pushover_client = TestPushoverClient::new();
        pushover_client.successful = false;
        let new_state = notify(state, thread.clone(), false, &Some(pushover_client)).await;
        assert_eq!(new_state.thread, Some(thread));
        assert_eq!(new_state.notified, 0);
    }

    #[tokio::test]
    async fn notify_over_threshold_already_notified() {
        let thread = make_thread(9);
        let state = data::State {
            thread: None,
            notified: 9,
        };
        let mut pushover_client = TestPushoverClient::new();
        pushover_client.disabled = true;
        let new_state = notify(state, thread.clone(), false, &Some(pushover_client)).await;
        assert_eq!(new_state.thread, Some(thread));
        assert_eq!(new_state.notified, 9);
    }

    #[tokio::test]
    async fn notify_over_threshold_page_after() {
        let thread = make_thread(10);
        let state = data::State {
            thread: None,
            notified: 9,
        };
        let pushover_client = TestPushoverClient::new();
        let new_state = notify(state, thread.clone(), false, &Some(pushover_client)).await;
        assert_eq!(new_state.thread, Some(thread));
        assert_eq!(new_state.notified, 10);
    }

    #[tokio::test]
    async fn notify_reset_notified() {
        let thread = make_thread(1);
        let state = data::State {
            thread: None,
            notified: 9,
        };
        let pushover_client = TestPushoverClient::new();
        let new_state = notify(state, thread.clone(), false, &Some(pushover_client)).await;
        assert_eq!(new_state.thread, Some(thread));
        assert_eq!(new_state.notified, 0);
    }
}
