use chrono;
use chrono::prelude::{DateTime, Utc};
use clap::Parser;
use std::time::Duration;
use tokio::{task, time};

mod api;
mod data;

#[derive(Parser, Debug)]
pub struct PagenineArgs {
    /// Name of the board to scan.
    #[clap(value_parser = validate_board)]
    pub board: String,

    /// Title of the thread to scan.
    #[clap(value_parser)]
    pub title: String,
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
        Err(_) => { return None; }
    };
    catalog.find(title)
}

async fn check(args: &PagenineArgs, state: data::State) -> data::State {
    let refresh = state
        .thread
        .as_ref()
        .map_or(true, |thread| thread.check_if_needs_refresh());

    let thread = if refresh {
        let last_update_time = state.thread.map(|thread| thread.time);
        get_current_thread(&args.board, &args.title, last_update_time).await
    } else {
        state.thread
    };
    let thread = match thread {
        Some(thread) => thread,
        None => return data::State::new(),
    };

    let mut notified = state.notified;
    if thread.page >= 9 && thread.page != state.notified {
        println!("Page >{}", thread.page);
        let notification_shown = thread.show_notification();
        notified = match notification_shown {
            Ok(_) => thread.page,
            Err(_) => state.notified,
        }
    } else if thread.page < 9 {
        notified = 0;
    }
    return data::State {
        thread: Some(thread),
        notified: notified,
    };
}

#[tokio::main]
async fn main() {
    let args = PagenineArgs::parse();

    let forever = task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));
        let mut state = data::State::new();

        loop {
            interval.tick().await;
            state = check(&args, state).await;
        }
    });

    let _ = forever.await;
}
