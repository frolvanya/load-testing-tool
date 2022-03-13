use hyper::{Client, Uri};

pub async fn website_is_up(url: Uri) -> Result<(), crate::WebsiteError> {
    let client = Client::new();
    let response = client.get(url).await;

    match response {
        Ok(response_res) => {
            if response_res.status().is_success() {
                return Ok(());
            }

            Err(crate::WebsiteError::BadResponse)
        }
        Err(_) => Err(crate::WebsiteError::WebsiteUnaccessible),
    }
}
