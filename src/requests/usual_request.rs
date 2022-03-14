use std::sync::{atomic::Ordering, Arc};

use colored::Colorize;

pub async fn send_usual_request(data: Arc<crate::LoadTestingTool>) {
    match reqwest::get(data.url.to_string()).await {
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
    }
}
