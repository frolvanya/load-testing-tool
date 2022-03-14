pub async fn website_is_up(url: String) -> Result<(), crate::WebsiteError> {
    match reqwest::get(url).await {
        Ok(response) => {
            if response.status().is_success() {
                return Ok(());
            }

            Err(crate::WebsiteError::BadResponse)
        }
        Err(_) => Err(crate::WebsiteError::WebsiteUnaccessible),
    }
}
