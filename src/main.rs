use clap::Parser;
use colored::Colorize;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};

use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Instant;

mod ctrlc_handler;
mod display;
mod proxies;
mod requests;

pub enum WebsiteError {
    WebsiteUnaccessible,
    BadResponse,
}

/// Load Testing Tool
#[derive(Parser, Debug)]
struct Args {
    /// Website URL to test
    #[clap(short, long)]
    url: String,

    /// Needs to use proxy servers
    #[clap(short = 'p', long = "use-proxy", takes_value = false)]
    use_proxy: bool,

    /// Start Load Tesing Tool without website status checking
    #[clap(short = 's', long = "no-status-check", takes_value = false)]
    no_status_check: bool,

    /// Do not display errors
    #[clap(short = 'e', long = "no-error-mode", takes_value = false)]
    no_error_mode: bool,

    /// Set up a concurrency
    #[clap(short, long)]
    concurrency: usize,
}

pub struct LoadTestingTool {
    url: String,
    total_requests: AtomicU64,
    failed_requests: AtomicU64,
    use_proxy: bool,
    error_mode: bool,
    concurrency: usize,
    start_attack_time: Instant,
}

impl LoadTestingTool {
    async fn attack(self: &Arc<Self>) {
        let cloned_self = self.clone();
        tokio::spawn(async move {
            ctrlc_handler::ctrl_c_handler(cloned_self).await;
        });

        let mut proxies = Vec::new();
        if let Ok(lines) = proxies::read_proxies::read_lines("./proxies.txt") {
            for line in lines.flatten() {
                proxies.push(line);
            }
        }

        let mut spawned_tasks = FuturesUnordered::new();

        loop {
            if spawned_tasks.len() > self.concurrency {
                spawned_tasks.next().await;
            }

            let cloned_self = self.clone();
            if cloned_self.use_proxy {
                let taken_proxy = proxies::random_proxy::take_random_proxy(proxies.clone());

                spawned_tasks.push(
                    (async move {
                        requests::proxy_request::send_proxy_request(cloned_self, taken_proxy).await
                    })
                    .boxed(),
                );
            } else {
                spawned_tasks.push(
                    (async move { requests::usual_request::send_usual_request(cloned_self).await })
                        .boxed(),
                );
            }
        }
    }
}

async fn start_load_testing_tool(
    url: String,
    concurrency: usize,
    use_proxy: bool,
    error_mode: bool,
) {
    display::time::display_time();
    println!(
        "{} {}",
        "Load Testing Tool is running at".green(),
        url.clone().to_string().bold()
    );
    Arc::new(LoadTestingTool {
        url: url.clone(),
        total_requests: AtomicU64::new(0),
        failed_requests: AtomicU64::new(0),
        use_proxy,
        error_mode,
        concurrency,
        start_attack_time: Instant::now(),
    })
    .attack()
    .await
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let website_url = args.url;
    let use_proxy = args.use_proxy;
    let concurrency = args.concurrency;
    let error_mode = args.no_error_mode;

    if args.no_status_check {
        start_load_testing_tool(website_url.clone(), concurrency, use_proxy, error_mode).await;
    }

    match requests::website_status::website_is_up(website_url.clone()).await {
        Ok(()) => {
            start_load_testing_tool(website_url.clone(), concurrency, use_proxy, error_mode).await;
        }
        Err(WebsiteError::WebsiteUnaccessible) => {
            display::error::display_error(
                "Unable to ping website. Make sure the link is spelled correctly!".to_string(),
                error_mode,
            );
        }
        Err(WebsiteError::BadResponse) => {
            display::error::display_error(
                "GET request to the site ended with an failure response!".to_string(),
                error_mode,
            );
        }
    }
}
