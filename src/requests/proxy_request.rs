use std::sync::{atomic::Ordering, Arc};

use colored::Colorize;
use hyper::{Body, Client, Method, Request, Uri};
use hyper_socks2::SocksConnector;
use hyper_tls::HttpsConnector;

pub async fn send_proxy_request(data: Arc<crate::LoadTestingTool>, taken_proxy: Uri) {
    let mut everything_is_fine = true;

    let connector = HttpsConnector::new();

    let proxy = SocksConnector {
        proxy_addr: taken_proxy.clone(),
        auth: None,
        connector,
    };

    let client = Client::builder().build::<_, Body>(proxy);

    let req = match Request::get(data.url.clone())
        .method(Method::GET)
        .body(Body::empty())
    {
        Ok(request) => request,
        Err(e) => {
            crate::display::error::display_error(
                format!("Unable to configure request, due to: {}", e),
                data.error_mode,
            );

            everything_is_fine = false;
            Request::new(Body::empty())
        }
    };

    if everything_is_fine {
        match client.request(req).await {
            Ok(_) => {
                data.spawned_requests.fetch_add(1, Ordering::SeqCst);

                if data.spawned_requests.load(Ordering::SeqCst) % 1000 == 0 {
                    crate::display::time::display_time();

                    let request_info = format!(
                        "Request №{} was successfuly sent from",
                        data.spawned_requests.load(Ordering::SeqCst),
                    );
                    println!(
                        "{} {}",
                        request_info.green(),
                        taken_proxy.to_string().bold()
                    );
                }
            }
            Err(e) => {
                data.failed_requests.fetch_add(1, Ordering::SeqCst);
                crate::display::error::display_error(
                    format!("Unable to send request, due to {}", e),
                    data.error_mode,
                );
            }
        }
    }
}
