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

async fn check(args: &PagenineArgs, previous_thread: Option<data::Thread>) -> Option<data::Thread> {
    let refresh = match &previous_thread {
        Some(thread) => thread.check_if_needs_refresh(),
        None => true,
    };
    let thread = if refresh {
        let last_update_time = previous_thread.map(|thread| thread.time);
        get_current_thread(&args.board, &args.title, last_update_time).await?
    } else {
        previous_thread.unwrap()
    };

    if refresh && thread.page >= 9 {
        println!("Page >{}", thread.page);
        let _ = thread.show_notification();
    }

    return Some(thread);
}

#[tokio::main]
async fn main() {
    let args = PagenineArgs::parse();

    let forever = task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));
        let mut thread: Option<data::Thread> = None;

        loop {
            interval.tick().await;
            thread = check(&args, thread).await;
        }
    });

    let _ = forever.await;
}
