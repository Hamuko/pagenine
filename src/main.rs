use chrono;
use clap::Parser;
use std::time::Duration;
use tokio::{task, time};

mod data;
mod response;

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

async fn get_catalog(board: &String) -> Result<response::APICatalog, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("https://a.4cdn.org/{}/catalog.json", board);
    let response = client.get(url).send().await?;
    let catalog = response.json::<response::APICatalog>().await?;
    Ok(catalog)
}

async fn get_current_thread(board: &String, title: &String) -> Option<data::Thread> {
    let catalog = match get_catalog(board).await {
        Ok(catalog) => catalog,
        Err(_) => { return None; }
    };
    for page in catalog {
        for thread in page.threads {
            if thread.sub.contains(title) {
                return Some(data::Thread {
                    page: page.page,
                    no: thread.no,
                    sub: thread.sub,
                    time: chrono::offset::Utc::now(),
                });
            }
        }
    }
    return None;
}

async fn check(args: &PagenineArgs, previous_thread: Option<data::Thread>) -> Option<data::Thread> {
    let refresh = match &previous_thread {
        Some(thread) => thread.check_if_needs_refresh(),
        None => true,
    };
    let thread = if refresh {
        get_current_thread(&args.board, &args.title).await?
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
