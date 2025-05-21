use crate::import::ImporterError;
use reqwest::{Client, Response};
use url::Url;

pub struct PinataFetcher {
    client: Client,
    base_url: Url,
}

impl Default for PinataFetcher {
    fn default() -> Self {
        Self {
            base_url: Url::parse("https://hub.pinata.cloud/v1/castsByFid").unwrap(),
            client: Client::new(),
        }
    }
}

impl PinataFetcher {
    pub fn with_base_url(self, url: Url) -> Self {
        Self {
            base_url: url,
            ..self
        }
    }

    pub async fn api_request_for_id(&self, id: u64) -> Result<Response, ImporterError> {
        self.client
            .get(
                Url::parse_with_params(self.base_url.as_str(), &[("fid", id.to_string())])
                    .map_err(|_| ImporterError::InvalidEndpoint)?,
            )
            .send()
            .await
            .map_err(|_| ImporterError::FailedApiRequest)
    }
}
