use chrono::prelude::{DateTime, Utc};
use clap::Parser;
use log::{info, warn, LevelFilter};
use simple_logger::SimpleLogger;
use std::time::Duration;
use tokio::select;
use tokio::signal;
use tokio::{task, time};
use tokio_util::sync::CancellationToken;

mod api;
mod data;
mod pushover;

#[derive(Parser, Debug)]
pub struct PagenineArgs {
    /// Name of the board to scan.
    #[clap(value_parser = validate_board, env = "PAGENINE_BOARD")]
    pub board: String,

    /// Title of the thread to scan.
    #[clap(value_parser, env = "PAGENINE_TITLE")]
    pub title: String,

    /// Ignore threads that have reached bump limit.
    #[clap(long, value_parser, env = "PAGENINE_NO_BUMP_LIMIT")]
    pub no_bump_limit: bool,

    /// Pushover application API key.
    #[clap(long, value_parser, env = "PAGENINE_PUSHOVER_APPLICATION_API_TOKEN")]
    pub pushover_application_api_token: Option<String>,

    /// Pushover user key.
    #[clap(long, value_parser, env = "PAGENINE_PUSHOVER_USER_KEY")]
    pub pushover_user_key: Option<String>,
}

fn validate_board(value: &str) -> Result<String, String> {
    Ok(value.trim_matches('/').to_string())
}

async fn get_current_thread(
    board: &String,
    title: &String,
    if_modified_since: Option<DateTime<Utc>>,
) -> Result<Option<data::Thread>, Box<dyn std::error::Error>> {
    let catalog = api::Catalog::fetch(board, if_modified_since).await?;
    Ok(catalog.find(title))
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
        match get_current_thread(&args.board, &args.title, last_update_time).await {
            Ok(thread) => thread,
            Err(error) => {
                warn!("Error fetching thread: {}", error);
                return state;
            }
        }
    } else {
        state.thread.clone()
    };
    let thread = match thread {
        Some(thread) => thread,
        None => return data::State::new(),
    };

    if refresh {
        if thread.bumplimit {
            info!(
                "\"{}\", page {} ({}/{}), over bump limit",
                thread.sub, thread.page, thread.position, thread.page_length
            );
        } else {
            info!(
                "\"{}\", page {} ({}/{})",
                thread.sub, thread.page, thread.position, thread.page_length
            );
        }
    }

    return notify(state, thread, args.no_bump_limit, pushover_client).await;
}

async fn notify(
    state: data::State,
    thread: data::Thread,
    no_bump_limit: bool,
    pushover_client: &Option<impl pushover::PushoverClientTrait>,
) -> data::State {
    // Thread needs to have hit page 9 for any notification to happen.
    if thread.page < 9 {
        return data::State {
            thread: Some(thread),
            notified: 0,
        };
    }

    // Do not notify if a notification for the same page has already been sent,
    // or the thread has hit the bump limit when bump limits are ignored.
    if thread.page == state.notified || (no_bump_limit && thread.bumplimit) {
        return data::State {
            thread: Some(thread),
            notified: state.notified,
        };
    }

    let notification_shown = match pushover_client {
        Some(pushover_client) => thread.send_pushover_notification(pushover_client).await,
        None => thread.show_notification(),
    };
    let notified = match notification_shown {
        Ok(_) => thread.page,
        Err(_) => state.notified,
    };
    data::State {
        thread: Some(thread),
        notified,
    }
}

#[cfg(unix)]
/// Wait for SIGINT (Ctrl-C) or SIGTERM to end the client (Unix).
async fn wait_termination() {
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("Could not register SIGTERM handler");
    select! {
        _ = signal::ctrl_c() => log::debug!("Received SIGINT"),
        _ = sigterm.recv() => log::debug!("Received SIGTERM"),
    }
}

#[cfg(not(unix))]
/// Wait for SIGINT (Ctrl-C) to end the client (Windows).
async fn wait_termination() {
    match signal::ctrl_c().await {
        Ok(()) => log::debug!("Received SIGINT"),
        Err(err) => {
            log::error!("Unable to listen to shutdown signal: {}", err);
        }
    }
}

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .env()
        .init()
        .unwrap();
    let args = PagenineArgs::parse();

    let cancellation_token = CancellationToken::new();
    let cloned_cancellation_token = cancellation_token.clone();

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
            tokio::select! {
                _ = interval.tick() => {},
                _ = cloned_cancellation_token.cancelled() => break,
            };
            state = check(&args, &pushover_client, state).await;
        }
    });

    wait_termination().await;
    cancellation_token.cancel();

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
