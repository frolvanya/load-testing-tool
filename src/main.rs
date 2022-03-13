#![feature(integer_atomics)]

use clap::Parser;
use colored::Colorize;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use hyper::Uri;

use std::sync::atomic::AtomicU128;
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
    url: Uri,
    spawned_requests: AtomicU128,
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
            if spawned_tasks.len() > self.concurrency.clone() {
                spawned_tasks.next().await;
            }

            let cloned_self = self.clone();
            if cloned_self.use_proxy {
                let mut proxy_successfully_parsed = true;

                let taken_proxy = match proxies::random_proxy::take_random_proxy(proxies.clone())
                    .parse::<Uri>()
                {
                    Ok(proxy_uri) => proxy_uri,
                    Err(e) => {
                        display::error::display_error(
                            format!("Unable to parse taken proxy, due to: {}", e),
                            cloned_self.error_mode,
                        );

                        proxy_successfully_parsed = false;
                        Uri::from_static("error")
                    }
                };

                if proxy_successfully_parsed {
                    spawned_tasks.push(
                        (async move {
                            requests::proxy_request::send_proxy_request(cloned_self, taken_proxy)
                                .await
                        })
                        .boxed(),
                    );
                }
            } else {
                spawned_tasks.push(
                    (async move { requests::usual_request::send_usual_request(cloned_self).await })
                        .boxed(),
                );
            }
        }
    }
}

async fn start_load_testing_tool(url: Uri, concurrency: usize, use_proxy: bool, error_mode: bool) {
    display::time::display_time();
    println!(
        "{} {}",
        "Load Testing Tool is running at".green(),
        url.clone().to_string().bold()
    );
    Arc::new(LoadTestingTool {
        url: url.clone(),
        spawned_requests: AtomicU128::new(0),
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

    let error_mode = args.no_error_mode;
    let website_url = match args.url.parse::<Uri>() {
        Ok(url) => url,
        Err(e) => {
            display::error::display_error(
                format!("Unable to parse taken url, due to: {}", e),
                error_mode,
            );
            std::process::exit(1);
        }
    };
    let use_proxy = args.use_proxy;
    let concurrency = args.concurrency;

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
