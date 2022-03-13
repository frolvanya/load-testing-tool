#![feature(integer_atomics)]

use chrono::{Local, Timelike};
use clap::Parser;
use colored::Colorize;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use hyper::client::HttpConnector;
use hyper::{Body, Client, Method, Request, Uri};
use hyper_proxy::{Intercept, Proxy, ProxyConnector};
use hyper_socks2::SocksConnector;
use hyper_tls::HttpsConnector;
use rand::Rng;

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::atomic::{AtomicU128, Ordering};
use std::sync::Arc;
use std::time::Instant;

enum WebsiteError {
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

struct LoadTestingTool {
    url: String,
    spawned_requests: AtomicU128,
}

impl LoadTestingTool {
    async fn attack(self: &Arc<Self>, concurrency: usize, activate_proxy: bool, error_mode: bool) {
        let start_attack_time = Instant::now();

        let cloned_self = self.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();

            println!();
            display_time();
            println!("{}", "Load Testing Tool was stoped by user".green());

            display_time();
            println!(
                "{}",
                format!(
                    "{} {} {}",
                    "Program worked for".green(),
                    format!("{:.02}", start_attack_time.elapsed().as_secs_f64() / 60.).bold(),
                    "min".green()
                )
            );

            display_time();
            println!(
                "{}",
                format!(
                    "{} {}",
                    "Average requests per second:".green(),
                    format!(
                        "{:.02}",
                        cloned_self.spawned_requests.load(Ordering::SeqCst) as f64
                            / start_attack_time.elapsed().as_secs_f64()
                    )
                    .bold()
                )
            );
            std::process::exit(1);
        });

        let mut proxies = Vec::new();
        if let Ok(lines) = read_lines("./proxies.txt") {
            for line in lines.flatten() {
                proxies.push(line);
            }
        }

        let mut spawned_tasks = FuturesUnordered::new();
        loop {
            if spawned_tasks.len() > concurrency {
                spawned_tasks.next().await;
            }

            let self_cloned = self.clone();
            if activate_proxy {
                let mut proxy_successfully_parsed = true;
                let taken_proxy = match take_random_proxy(proxies.clone()).parse::<Uri>() {
                    Ok(proxy_uri) => proxy_uri,
                    Err(e) => {
                        display_error(
                            format!("Unable to parse taken proxy, due to: {}", e),
                            error_mode,
                        );

                        proxy_successfully_parsed = false;
                        Uri::from_static("error")
                    }
                };

                if proxy_successfully_parsed {
                    spawned_tasks.push(tokio::spawn(async move {
                        let mut everything_is_fine = true;

                        let parsed_uri = match self_cloned.url.parse::<Uri>() {
                            Ok(unwrapped_uri) => unwrapped_uri,
                            Err(e) => {
                                display_error(
                                    format!("Unable to consider taken proxy as URI, due to {}", e),
                                    error_mode,
                                );

                                everything_is_fine = false;
                                Uri::from_static("error")
                            }
                        };

                        if everything_is_fine {
                            let connector = HttpsConnector::new();

                            let proxy = SocksConnector {
                                proxy_addr: taken_proxy,
                                auth: None,
                                connector,
                            };

                            let client = Client::builder().build::<_, Body>(proxy);

                            let req = match Request::get(parsed_uri)
                                .method(Method::GET)
                                .body(Body::empty())
                            {
                                Ok(request) => request,
                                Err(e) => {
                                    display_error(
                                        format!("Unable to configure request, due to: {}", e),
                                        error_mode,
                                    );

                                    everything_is_fine = false;
                                    Request::new(Body::empty())
                                }
                            };

                            if everything_is_fine {
                                match client.request(req).await {
                                    Ok(_) => {
                                        self_cloned.spawned_requests.fetch_add(1, Ordering::SeqCst);

                                        if self_cloned.spawned_requests.load(Ordering::SeqCst)
                                            % 1000
                                            == 0
                                        {
                                            display_time();

                                            let request_info = format!(
                                                "Request №{} was successfuly sent",
                                                self_cloned.spawned_requests.load(Ordering::SeqCst),
                                            );
                                            println!("{}", request_info.green());
                                        }
                                    }
                                    Err(e) => display_error(
                                        format!("Unable to send request, due to {}", e),
                                        error_mode,
                                    ),
                                }
                            }
                        }
                    }));
                }
            } else {
                spawned_tasks.push(tokio::spawn(async move {
                    let mut everything_is_fine = true;
                    let parsed_uri = match self_cloned.url.parse::<Uri>() {
                        Ok(unwrapped_uri) => unwrapped_uri,
                        Err(e) => {
                            display_error(
                                format!("Unable to consider taken proxy as URI, due to {}", e),
                                error_mode,
                            );

                            everything_is_fine = false;
                            Uri::from_static("error")
                        }
                    };

                    if everything_is_fine {
                        let https = HttpsConnector::new();
                        let client = Client::builder().build::<_, hyper::Body>(https);

                        match client.get(parsed_uri).await {
                            Ok(_) => {
                                self_cloned.spawned_requests.fetch_add(1, Ordering::SeqCst);

                                if self_cloned.spawned_requests.load(Ordering::SeqCst) % 1000 == 0 {
                                    display_time();

                                    let request_info = format!(
                                        "Request №{} was successfuly sent",
                                        self_cloned.spawned_requests.load(Ordering::SeqCst),
                                    );
                                    println!("{}", request_info.green());
                                }
                            }
                            Err(e) => display_error(
                                format!("Unable to send request, due to {}", e),
                                error_mode,
                            ),
                        }
                    }
                }));
            }
        }
    }
}

fn display_time() {
    let now = Local::now().time();
    let time = format!(
        "[{:02}:{:02}:{:02}] ",
        now.hour(),
        now.minute(),
        now.second()
    );

    print!("{} ", time.blue());
}

fn display_error(text: String, error_mode: bool) {
    if !error_mode {
        display_time();
        println!("{}", text.red().bold());
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn take_random_proxy(proxies: Vec<String>) -> String {
    let mut rng = rand::thread_rng();

    let rand_index = rng.gen_range(0..proxies.len());
    proxies[rand_index].clone()
}

async fn website_is_up(url: String) -> Result<(), WebsiteError> {
    let response = reqwest::get(url).await;

    match response {
        Ok(response_res) => {
            if response_res.status().is_success() {
                return Ok(());
            }

            Err(WebsiteError::BadResponse)
        }
        Err(_) => Err(WebsiteError::WebsiteUnaccessible),
    }
}

async fn start_load_testing_tool(
    url: String,
    concurrency: usize,
    activate_proxy: bool,
    error_mode: bool,
) {
    display_time();
    println!(
        "{} {}",
        "Load Testing Tool is running at".green(),
        url.clone().bold()
    );
    Arc::new(LoadTestingTool {
        url: url.clone(),
        spawned_requests: AtomicU128::new(0),
    })
    .attack(concurrency, activate_proxy, error_mode)
    .await
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let website_url = args.url;
    let use_proxy = args.use_proxy;
    let error_mode = args.no_error_mode;
    let concurrency = args.concurrency;

    if args.no_status_check {
        start_load_testing_tool(website_url.clone(), concurrency, use_proxy, error_mode).await;
    }

    match website_is_up(website_url.clone()).await {
        Ok(()) => {
            start_load_testing_tool(website_url.clone(), concurrency, use_proxy, error_mode).await;
        }
        Err(WebsiteError::WebsiteUnaccessible) => {
            display_error(
                "Unable to ping website. Make sure the link is spelled correctly!".to_string(),
                error_mode,
            );
        }
        Err(WebsiteError::BadResponse) => {
            display_error(
                "GET request to the site ended with an failure response!".to_string(),
                error_mode,
            );
        }
    }
}
