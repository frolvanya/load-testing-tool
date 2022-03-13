use std::sync::{atomic::Ordering, Arc};

use colored::Colorize;
use hyper::Client;
use hyper_tls::HttpsConnector;

pub async fn send_usual_request(data: Arc<crate::LoadTestingTool>) {
    let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());

    match client.get(data.url.clone()).await {
        Ok(_) => {
            data.spawned_requests.fetch_add(1, Ordering::SeqCst);

            if data.spawned_requests.load(Ordering::SeqCst) % 1000 == 0 {
                crate::display::time::display_time();

                let request_info = format!(
                    "Request №{} was successfuly sent",
                    data.spawned_requests.load(Ordering::SeqCst),
                );
                println!("{}", request_info.green());
            }
        }
        Err(e) => crate::display::error::display_error(
            format!("Unable to send request, due to {}", e),
            data.error_mode,
        ),
    }
}
