use std::sync::{atomic::Ordering, Arc};

use colored::Colorize;
use reqwest::{Client, Proxy};

pub async fn send_proxy_request(data: Arc<crate::LoadTestingTool>, taken_proxy: String) {
    match Proxy::http(taken_proxy) {
        Ok(connected_proxy) => {
            match Client::builder().proxy(connected_proxy).build() {
                Ok(built_client) => match built_client.get(data.url.clone()).send().await {
                    Ok(response) => {
                        if !response.status().is_success() {
                            data.failed_requests.fetch_add(1, Ordering::SeqCst);
                        }

                        data.total_requests.fetch_add(1, Ordering::SeqCst);

                        if data.total_requests.load(Ordering::SeqCst) % 1000 == 0 {
                            crate::display::time::display_time();

                            let request_info = format!(
                                "Request №{} was successfuly sent",
                                data.total_requests.load(Ordering::SeqCst),
                            );
                            println!("{}", request_info.green());
                        }
                    }
                    Err(e) => {
                        data.failed_requests.fetch_add(1, Ordering::SeqCst);
                        crate::display::error::display_error(
                            format!("Unable to send request, due to {}", e),
                            data.error_mode,
                        )
                    }
                },
                Err(e) => crate::display::error::display_error(
                    format!("Unable to build client, due to: {}", e),
                    data.error_mode,
                ),
            };
        }
        Err(e) => crate::display::error::display_error(
            format!("Unable to connect proxy, due to: {}", e),
            data.error_mode,
        ),
    };
}
